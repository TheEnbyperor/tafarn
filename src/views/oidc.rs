use chrono::Duration;
use openidconnect::{OAuth2TokenResponse, TokenResponse};
use rocket::http::{CookieJar, SameSite};
use rocket::Request;
use rocket::response::Redirect;
use chrono::prelude::*;
use diesel::prelude::*;

type OIDCIdTokenFields = openidconnect::IdTokenFields<
    AdditionalClaims, openidconnect::EmptyExtraTokenFields, openidconnect::core::CoreGenderClaim,
    openidconnect::core::CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJwsSigningAlgorithm, openidconnect::core::CoreJsonWebKeyType
>;
pub type OIDCIdTokenClaims = openidconnect::IdTokenClaims<
    AdditionalClaims, openidconnect::core::CoreGenderClaim
>;
type OIDCTokenResponse = openidconnect::StandardTokenResponse<
    OIDCIdTokenFields, openidconnect::core::CoreTokenType
>;
type OIDCClient = openidconnect::Client<
    AdditionalClaims, openidconnect::core::CoreAuthDisplay, openidconnect::core::CoreGenderClaim,
    openidconnect::core::CoreJweContentEncryptionAlgorithm, openidconnect::core::CoreJwsSigningAlgorithm,
    openidconnect::core::CoreJsonWebKeyType, openidconnect::core::CoreJsonWebKeyUse,
    openidconnect::core::CoreJsonWebKey, openidconnect::core::CoreAuthPrompt,
    openidconnect::StandardErrorResponse<openidconnect::core::CoreErrorResponseType>,
    OIDCTokenResponse, openidconnect::core::CoreTokenType,
    openidconnect::core::CoreTokenIntrospectionResponse, openidconnect::core::CoreRevocableToken,
    openidconnect::core::CoreRevocationErrorResponse
>;

pub struct OIDCUser {
    pub access_token: openidconnect::AccessToken,
    pub claims: OIDCIdTokenClaims,
}

fn refresh_nonce_verifier(_: Option<&openidconnect::Nonce>) -> Result<(), String> {
    Ok(())
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for OIDCUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let oidc_app = match request.guard::<&rocket::State<OIDCApplication>>().await {
            rocket::request::Outcome::Success(a) => a,
            rocket::request::Outcome::Forward(()) => return rocket::request::Outcome::Forward(()),
            rocket::request::Outcome::Failure(e) => return rocket::request::Outcome::Failure(e)
        };
        let db = match request.guard::<crate::DbConn>().await {
            rocket::request::Outcome::Success(a) => a,
            rocket::request::Outcome::Forward(()) => return rocket::request::Outcome::Forward(()),
            rocket::request::Outcome::Failure(e) => return rocket::request::Outcome::Failure(e)
        };

        let state_txt = match request.cookies().get_private("oidc_login") {
            Some(t) => t,
            None => return rocket::request::Outcome::Forward(()),
        };
        let session_id = match uuid::Uuid::parse_str(state_txt.value()) {
            Ok(i) => i,
            Err(_) => return rocket::request::Outcome::Forward(()),
        };

        let session = match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
            crate::schema::session::dsl::session.find(session_id).first::<crate::models::Session>(c)
        }).await {
            Ok(s) => s,
            Err(err) => {
                error!("Unable to retrieve session from database: {}", err);
                return rocket::request::Outcome::Forward(())
            }
        };

        let claims = match serde_json::from_str(&session.claims) {
            Ok(c) => c,
            Err(_) => return rocket::request::Outcome::Forward(()),
        };

        let now = Utc::now();
        match (session.expires_at, session.refresh_token) {
            (None, _) => rocket::request::Outcome::Success(OIDCUser {
                access_token: openidconnect::AccessToken::new(session.access_token),
                claims,
            }),
            (Some(exp), _) if Utc.from_utc_datetime(&exp) > now => rocket::request::Outcome::Success(OIDCUser {
                access_token: openidconnect::AccessToken::new(session.access_token),
                claims,
            }),
            (_, None) => rocket::request::Outcome::Forward(()),
            (_, Some(refresh_token)) => {
                match oidc_app.client.exchange_refresh_token(&openidconnect::RefreshToken::new(refresh_token))
                    .request_async(openidconnect::reqwest::async_http_client).await {
                    Ok(auth_res) => {
                        let id_token = match auth_res.id_token() {
                            Some(i) => i.clone(),
                            None => return rocket::request::Outcome::Failure((rocket::http::Status::InternalServerError, ()))
                        };

                        let id_claims = match id_token
                            .into_claims(&oidc_app.client.id_token_verifier(), refresh_nonce_verifier) {
                            Ok(c) => c,
                            Err(err) => {
                                warn!("Unable to verify claims: {}", err);
                                return rocket::request::Outcome::Failure((rocket::http::Status::InternalServerError, ()));
                            }
                        };

                        let new_session = crate::models::Session {
                            id: session.id,
                            access_token: auth_res.access_token().secret().to_string(),
                            expires_at: match auth_res.expires_in() {
                                Some(d) => Some((Utc::now() + match Duration::from_std(d) {
                                    Ok(d) => d,
                                    Err(_) => return rocket::request::Outcome::Failure((rocket::http::Status::InternalServerError, ()))
                                }).naive_utc()),
                                None => None
                            },
                            refresh_token: auth_res.refresh_token().map(|r| r.secret().to_string()),
                            claims: serde_json::to_string(&id_claims).unwrap(),
                        };

                        match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
                            diesel::update(crate::schema::session::dsl::session)
                                .set(&new_session)
                                .execute(c)
                        }).await  {
                            Ok(_) => {},
                            Err(err) => {
                                error!("Unable to update session in database: {}", err);
                                return rocket::request::Outcome::Failure((rocket::http::Status::InternalServerError, ()));
                            }
                        }

                        rocket::request::Outcome::Success(OIDCUser {
                            access_token: auth_res.access_token().clone(),
                            claims: id_claims,
                        })
                    }
                    Err(err) => {
                        warn!("Unable to refresh token: {}", err);
                        rocket::request::Outcome::Forward(())
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ClientRoles {
    roles: Vec<String>
}

impl ClientRoles {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdditionalClaims {
    #[serde(default)]
    resource_access: std::collections::HashMap<String, ClientRoles>
}

impl AdditionalClaims {
    pub fn own_roles(&mut self, client_id: &str) -> &ClientRoles {
        if !self.resource_access.contains_key(client_id) {
            self.resource_access.insert(client_id.to_string(), ClientRoles::default());
        }
        self.resource_access.get(client_id).unwrap()
    }


    pub fn has_role(&self, client_id: &str, role: &str) -> bool {
        self.resource_access.get(client_id).map_or(false, |r| r.has_role(role))
    }
}

impl openidconnect::AdditionalClaims for AdditionalClaims {}

pub struct OIDCApplication {
    client: OIDCClient,
    client_id: String,
}

impl OIDCApplication {
    pub async fn new(
        issuer: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<Self, String> {
        let provider_metadata = openidconnect::core::CoreProviderMetadata::discover_async(
            openidconnect::IssuerUrl::new(issuer.to_string())
                .map_err(|err| format!("Invalid issuer URI: {}", err))?,
            openidconnect::reqwest::async_http_client,
        ).await.map_err(|err| format!("Failed to discover OIDC server: {}", err))?;

        let client = OIDCClient::from_provider_metadata(
            provider_metadata,
            openidconnect::ClientId::new(client_id.to_string()),
            Some(openidconnect::ClientSecret::new(client_secret.to_string())),
        );

        Ok(Self {
            client,
            client_id: client_id.to_string(),
        })
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn authorize(&self, return_uri: &str, external_uri: &str) -> Result<OIDCAuthorizeRedirect, String> {
        let redirect_uri = openidconnect::RedirectUrl::new(format!("{}/oidc/redirect", external_uri))
            .map_err(|err| format!("Invalid redirect URI: {}", err))?;
        let (pkce_challenge, pkce_verifier) = openidconnect::PkceCodeChallenge::new_random_sha256();
        let (url, csrf_token, nonce) = self.client.authorize_url(
            openidconnect::core::CoreAuthenticationFlow::AuthorizationCode,
            openidconnect::CsrfToken::new_random,
            openidconnect::Nonce::new_random,
        )
            .set_pkce_challenge(pkce_challenge)
            .set_redirect_uri(std::borrow::Cow::Owned(redirect_uri.clone()))
            .url();

        Ok(OIDCAuthorizeRedirect {
            state: OIDCAuthorizeState {
                pkce_verifier,
                csrf_token,
                nonce,
                redirect_uri,
                return_uri: return_uri.to_string(),
            },
            redirect_uri: url.to_string(),
        })
    }
}

#[derive(Serialize, Deserialize)]
struct OIDCAuthorizeState {
    pkce_verifier: openidconnect::PkceCodeVerifier,
    csrf_token: openidconnect::CsrfToken,
    nonce: openidconnect::Nonce,
    redirect_uri: openidconnect::RedirectUrl,
    return_uri: String,
}

pub struct OIDCAuthorizeRedirect {
    state: OIDCAuthorizeState,
    redirect_uri: String,
}

impl<'r> rocket::response::Responder<'r, 'static> for OIDCAuthorizeRedirect {
    fn respond_to(self, r: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        r.cookies().add_private(
            rocket::http::Cookie::build("oidc_auth_state", serde_json::to_string(&self.state).unwrap())
                .http_only(true)
                .secure(true)
                .same_site(SameSite::None)
                .expires(time::OffsetDateTime::now_utc() + time::Duration::hours(6))
                .finish()
        );

        rocket::Response::build()
            .status(rocket::http::Status::TemporaryRedirect)
            .raw_header("Location", self.redirect_uri)
            .ok()
    }
}

#[get("/oidc/redirect?<code>&<state>")]
pub async fn oidc_redirect(
    cookies: &CookieJar<'_>, oidc_app: &rocket::State<OIDCApplication>,
    code: String, state: &str, db: crate::DbConn, lang: crate::i18n::Languages,
) -> Result<Redirect, rocket::http::Status> {
    let state_txt = match cookies.get_private("oidc_auth_state") {
        Some(t) => t,
        None => return Err(rocket::http::Status::BadRequest)
    };
    let state_obj: OIDCAuthorizeState = match serde_json::from_str(state_txt.value()) {
        Ok(s) => s,
        Err(_) => return Err(rocket::http::Status::InternalServerError)
    };
    cookies.remove_private(state_txt);

    if state != state_obj.csrf_token.secret() {
        return Err(rocket::http::Status::BadRequest);
    }

    let code = openidconnect::AuthorizationCode::new(code);
    let auth_res = match oidc_app.client.exchange_code(code)
        .set_pkce_verifier(state_obj.pkce_verifier)
        .set_redirect_uri(std::borrow::Cow::Owned(state_obj.redirect_uri))
        .request_async(openidconnect::reqwest::async_http_client).await {
        Ok(r) => r,
        Err(err) => {
            warn!("Unable to exchange auth code: {}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    if auth_res.token_type() != &openidconnect::core::CoreTokenType::Bearer {
        return Err(rocket::http::Status::BadRequest);
    }

    let id_token = match auth_res.id_token() {
        Some(i) => i.clone(),
        None => return Err(rocket::http::Status::InternalServerError)
    };

    let id_claims = match id_token.into_claims(&oidc_app.client.id_token_verifier(), &state_obj.nonce) {
        Ok(c) => c,
        Err(err) => {
            warn!("Unable to verify claims: {}", err);
            return Err(rocket::http::Status::BadRequest);
        }
    };

    let session_id = uuid::Uuid::new_v4();
    let session = crate::models::Session {
        id: session_id.clone(),
        access_token: auth_res.access_token().secret().to_string(),
        expires_at: match auth_res.expires_in() {
            Some(d) => Some((Utc::now() + match Duration::from_std(d) {
                Ok(d) => d,
                Err(_) => return Err(rocket::http::Status::InternalServerError)
            }).naive_utc()),
            None => None
        },
        refresh_token: auth_res.refresh_token().map(|r| r.secret().to_string()),
        claims: serde_json::to_string(&id_claims).unwrap(),
    };

    crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
        diesel::insert_into(crate::schema::session::dsl::session)
            .values(&session)
            .execute(c)
    }).await?;

    cookies.add_private(
        rocket::http::Cookie::build("oidc_login", session_id.to_string())
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .expires(time::OffsetDateTime::now_utc() + time::Duration::weeks(52))
            .finish()
    );

    super::accounts::init_account(db, &id_claims, &lang).await?;

    Ok(Redirect::temporary(state_obj.return_uri))
}