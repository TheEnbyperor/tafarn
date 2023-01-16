use crate::AppConfig;
use rocket_dyn_templates::{Template, context};
use rand::Rng;
use diesel::prelude::*;
use chrono::prelude::*;

#[derive(Clone, Debug)]
pub struct TokenClaims {
    pub issuer: String,
    pub subject: String,
    pub audience: String,
    pub not_before: i64,
    pub issued_at: i64,
    pub json_web_token_id: uuid::Uuid,
    pub scopes: Vec<String>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CustomTokenClaims {
    pub scopes: Vec<String>
}

impl TokenClaims {
    pub fn has_scope(&self, scope: &str) -> bool {
        if self.scopes.iter().any(|s| s == scope) {
            return true;
        }
        if let Some(s) = API_SCOPES.get(scope) {
            for scope in s.parent {
                if self.scopes.iter().any(|s| s == scope)  {
                    return true;
                }
            }
        }
        false
    }

    pub async fn get_account(&self, db: &crate::DbConn) -> Result<crate::models::Account, rocket::http::Status> {
        let sub = self.subject.clone();
        crate::db_run(db, move |c| -> diesel::result::QueryResult<_> {
            crate::schema::accounts::dsl::accounts.filter(
                crate::schema::accounts::dsl::owned_by.eq(sub)
            ).get_result::<crate::models::Account>(c)
        }).await
    }

    fn sign(&self, key: &jwt_simple::algorithms::HS512Key) -> String {
        use jwt_simple::algorithms::MACLike;

        key.authenticate(jwt_simple::claims::JWTClaims {
            issuer: Some(self.issuer.clone()),
            subject: Some(self.subject.clone()),
            audiences: Some( jwt_simple::claims::Audiences::AsString(self.audience.clone())),
            invalid_before: Some(jwt_simple::prelude::Duration::from_secs(self.not_before as u64)),
            issued_at: Some(jwt_simple::prelude::Duration::from_secs(self.issued_at as u64)),
            expires_at: None,
            nonce: None,
            jwt_id: Some(self.json_web_token_id.to_string()),
            custom: CustomTokenClaims {
                scopes: self.scopes.clone()
            }
        }).unwrap()
    }

    fn verify(token: &str, config: &AppConfig) -> Result<Self, rocket::http::Status> {
        use jwt_simple::algorithms::MACLike;

        let mut aud = std::collections::HashSet::new();
        aud.insert(format!("https://{}", config.uri));
        let claims: jwt_simple::claims::JWTClaims<CustomTokenClaims> =
            match config.jwt_secret.verify_token(token, Some(jwt_simple::common::VerificationOptions {
            reject_before: None,
            accept_future: false,
            allowed_audiences: Some(aud),
            ..Default::default()
        })) {
            Ok(c) => c,
            Err(_) => return Err(rocket::http::Status::Unauthorized)
        };

        Ok(Self {
            issuer: claims.issuer.unwrap(),
            subject: claims.subject.unwrap(),
            audience: claims.audiences.unwrap().into_string().unwrap_or_default(),
            not_before: claims.invalid_before.unwrap().as_secs() as i64,
            issued_at: claims.issued_at.unwrap().as_secs() as i64,
            json_web_token_id: uuid::Uuid::parse_str(&claims.jwt_id.unwrap()).unwrap(),
            scopes: claims.custom.scopes
        })
    }
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for TokenClaims {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let config = match request.guard::<&rocket::State<AppConfig>>().await {
            rocket::request::Outcome::Success(a) => a,
            rocket::request::Outcome::Forward(()) => return rocket::request::Outcome::Forward(()),
            rocket::request::Outcome::Failure(e) => return rocket::request::Outcome::Failure(e)
        };
        let db = match request.guard::<crate::DbConn>().await {
            rocket::request::Outcome::Success(a) => a,
            rocket::request::Outcome::Forward(()) => return rocket::request::Outcome::Forward(()),
            rocket::request::Outcome::Failure(e) => return rocket::request::Outcome::Failure(e)
        };

        let authorization_txt = match request.headers().get_one("Authorization") {
            Some(t) => t,
            None => return rocket::request::Outcome::Failure((rocket::http::Status::Unauthorized, ())),
        };
        let (authorization_tye, authorization) = match authorization_txt.split_once(" ") {
            Some(t) => t,
            None => return rocket::request::Outcome::Failure((rocket::http::Status::Unauthorized, ())),
        };

        if authorization_tye != "Bearer" {
            return rocket::request::Outcome::Failure((rocket::http::Status::Unauthorized, ()));
        }

        let claims = match TokenClaims::verify(authorization, &config) {
            Ok(c) => c,
            Err(_) => return rocket::request::Outcome::Failure((rocket::http::Status::Unauthorized, ())),
        };

        let token_obj: crate::models::OAuthToken = match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
            crate::schema::oauth_token::dsl::oauth_token.find(claims.json_web_token_id).first(c).optional()
        }).await {
            Ok(Some(c)) => c,
            Ok(None) => return rocket::request::Outcome::Failure((rocket::http::Status::Unauthorized, ())),
            Err(_) => return rocket::request::Outcome::Failure((rocket::http::Status::InternalServerError, ()))
        };

        if token_obj.revoked {
            return rocket::request::Outcome::Failure((rocket::http::Status::Unauthorized, ()));
        }

        rocket::request::Outcome::Success(claims)
    }
}

#[derive(Serialize)]
struct APIScope {
    description: &'static str,
    parent: &'static [&'static str],
}

const API_SCOPES: phf::Map<&'static str, APIScope> = phf::phf_map! {
    "read" => APIScope {
        description: "scope-read",
        parent: &[]
    },
    "read:accounts" => APIScope {
        description: "scope-read-accounts",
        parent: &["read"]
    },
    "read:blocks" => APIScope {
        description: "scope-read-blocks",
        parent: &["read", "follow"]
    },
    "read:bookmarks" => APIScope {
        description: "scope-read-bookmarks",
        parent: &["read"]
    },
    "read:favourites" => APIScope {
        description: "scope-read-favourites",
        parent: &["read"]
    },
    "read:filters" => APIScope {
        description: "scope-read-filters",
        parent: &["read"]
    },
    "read:follows" => APIScope {
        description: "scope-read-follows",
        parent: &["read", "follow"]
    },
    "read:lists" => APIScope {
        description: "scope-read-lists",
        parent: &["read"]
    },
    "read:mutes" => APIScope {
        description: "scope-read-mutes",
        parent: &["read", "follow"]
    },
    "read:notifications" => APIScope {
        description: "scope-read-notifications",
        parent: &["read"]
    },
    "read:search" => APIScope {
        description: "scope-read-search",
        parent: &["read"]
    },
    "read:statuses" => APIScope {
        description: "scope-read-statuses",
        parent: &["read"]
    },
    "write" => APIScope {
        description: "scope-write",
        parent: &[]
    },
    "write:accounts" => APIScope {
        description: "scope-write-accounts",
        parent: &["write"]
    },
    "write:blocks" => APIScope {
        description: "scope-write-blocks",
        parent: &["write", "follow"]
    },
    "write:bookmarks" => APIScope {
        description: "scope-write-bookmarks",
        parent: &["write"]
    },
    "write:conversations" => APIScope {
        description: "scope-write-conversations",
        parent: &["write"]
    },
    "write:favourites" => APIScope {
        description: "scope-write-favourites",
        parent: &["write"]
    },
    "write:filters" => APIScope {
        description: "scope-write-filters",
        parent:  &["write"]
    },
    "write:follows" => APIScope {
        description: "scope-write-follows",
        parent: &["write", "follow"]
    },
    "write:lists" => APIScope {
        description: "scope-write-lists",
        parent:  &["write"]
    },
    "write:media" => APIScope {
        description: "scope-write-media",
        parent: &["write"]
    },
    "write:mutes" => APIScope {
        description: "scope-write-mutes",
        parent: &["write", "follow"]
    },
    "write:notifications" => APIScope {
        description: "scope-write-notifications",
        parent: &["write"]
    },
    "write:reports" => APIScope {
        description: "scope-write-reports",
        parent: &["write"]
    },
    "write:statuses" => APIScope {
        description: "scope-write-statuses",
        parent: &["write"]
    },
    "follow" => APIScope {
        description: "scope-follow",
        parent: &[]
    },
    "push" => APIScope {
        description: "scope-push",
        parent: &[]
    }
};

#[derive(FromForm, Deserialize)]
pub struct AppsCreate {
    client_name: String,
    redirect_uris: String,
    website: Option<String>,
    scopes: Option<String>,
}

#[post("/api/v1/apps", data = "<form>", rank = 1)]
pub async fn api_apps_form(
    db: crate::DbConn, config: &rocket::State<AppConfig>, form: rocket::form::Form<AppsCreate>,
) -> Result<rocket::serde::json::Json<super::objs::App>, rocket::http::Status> {
    _api_apps(db, config, form.into_inner()).await
}

#[post("/api/v1/apps", data = "<form>", rank = 2)]
pub async fn api_apps_json(
    db: crate::DbConn, config: &rocket::State<AppConfig>, form: rocket::serde::json::Json<AppsCreate>,
) -> Result<rocket::serde::json::Json<super::objs::App>, rocket::http::Status> {
    _api_apps(db, config, form.into_inner()).await
}

pub async fn _api_apps(
    db: crate::DbConn, config: &rocket::State<AppConfig>, form: AppsCreate,
) -> Result<rocket::serde::json::Json<super::objs::App>, rocket::http::Status> {
    if form.client_name.trim().is_empty() {
        return Err(rocket::http::Status::UnprocessableEntity);
    }

    let redirect_uri = match rocket::http::uri::Absolute::parse(&form.redirect_uris) {
        Ok(uri) => uri.into_normalized().to_string(),
        Err(_) => return Err(rocket::http::Status::UnprocessableEntity),
    };

    let website = form.website.map(|w| match rocket::http::uri::Absolute::parse(&w) {
        Ok(uri) => {
            if uri.scheme() != "http" && uri.scheme() != "https" {
                return Err(rocket::http::Status::UnprocessableEntity);
            }

            Ok(uri.into_normalized().to_string())
        }
        Err(_) => Err(rocket::http::Status::UnprocessableEntity),
    }).transpose()?;

    let scopes = match form.scopes {
        Some(s) => {
            let mut scopes = s.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>();
            scopes.sort();
            scopes.dedup();
            scopes
        }
        None => vec!["read".to_string()],
    };

    for scope in &scopes {
        if !API_SCOPES.contains_key(scope) {
            return Err(rocket::http::Status::UnprocessableEntity);
        }
    }

    let new_app = crate::models::Apps {
        id: uuid::Uuid::new_v4(),
        name: form.client_name.to_string(),
        website,
        redirect_uri,
        client_secret: base64::encode_config(
            rand::thread_rng()
                .sample_iter(rand::distributions::Standard)
                .take(64)
                .collect::<Vec<u8>>(),
            base64::URL_SAFE_NO_PAD,
        ),
    };

    let new_app = crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
        c.transaction(|| {
            diesel::insert_into(crate::schema::apps::dsl::apps)
                .values(&new_app)
                .execute(c)?;

            for scope in scopes {
                diesel::insert_into(crate::schema::app_scopes::dsl::app_scopes)
                    .values(crate::models::AppScopes {
                        app_id: new_app.id,
                        scope: scope.to_string(),
                    })
                    .execute(c)?;
            }
            Ok(new_app)
        })
    }).await?;

    Ok(rocket::serde::json::Json(super::objs::App {
        id: new_app.id.clone(),
        name: new_app.name,
        website: new_app.website,
        redirect_uri: new_app.redirect_uri,
        client_id: new_app.id.to_string(),
        client_secret: new_app.client_secret,
        vapid_key: Some(base64::encode_config(config.web_push_signature.get_public_key(), base64::URL_SAFE_NO_PAD)),
    }))
}

#[derive(Serialize, Deserialize)]
struct OAuthConsentState {
    client_id: uuid::Uuid,
    scopes: Vec<String>,
    return_to: String,
    redirect_uri: url::Url,
    state: Option<String>,
}

#[derive(Responder)]
pub enum OAuthAuthorizeResponse {
    #[response(status = 400)]
    Template(Template),
    #[response(status = 200)]
    TemplateOk(Template),
    RocketRedirect(rocket::response::Redirect),
    Redirect(super::oidc::OIDCAuthorizeRedirect),
}

#[get("/oauth/authorize?<response_type>&<client_id>&<redirect_uri>&<scope>&<force_login>&<state>")]
pub async fn oauth_authorize(
    config: &rocket::State<AppConfig>, db: crate::DbConn, response_type: &str, client_id: &str,
    redirect_uri: &str, scope: Option<&str>, force_login: Option<&str>, state: Option<&str>,
    oidc_app: &rocket::State<super::oidc::OIDCApplication>, oidc_user: Option<super::oidc::OIDCUser>,
    origin: &rocket::http::uri::Origin<'_>, csrf_token: crate::csrf::CSRFToken,
    cookies: &rocket::http::CookieJar<'_>, localizer: crate::i18n::Localizer
) -> Result<OAuthAuthorizeResponse, rocket::http::Status> {
    let client_id = match uuid::Uuid::parse_str(client_id) {
        Ok(id) => id,
        Err(_) => return Ok(OAuthAuthorizeResponse::Template(Template::render("oauth-error", context! {
            message: fl!(localizer, "client-not-found"),
            lang: localizer
        })))
    };

    let (app, app_scopes) = match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
        Ok((
            crate::schema::apps::dsl::apps.find(client_id).first::<crate::models::Apps>(c).optional()?,
            crate::schema::app_scopes::dsl::app_scopes.filter(
                crate::schema::app_scopes::dsl::app_id.eq(client_id)
            ).load::<crate::models::AppScopes>(c)?
        ))
    }).await? {
        (Some(a), s) => (a, s),
        (None, _) => return Ok(OAuthAuthorizeResponse::Template(Template::render("oauth-error", context! {
            message: fl!(localizer, "client-not-found"),
            lang: localizer
        }))),
    };

    let c_redirect_uri = redirect_uri.to_string();
    let mut redirect_uri = match url::Url::parse(redirect_uri) {
        Ok(u) => u,
        Err(_) => return Ok(OAuthAuthorizeResponse::Template(Template::render("oauth-error", context! {
            message: fl!(localizer, "invalid-redirect-uri"),
            lang: localizer
        })))
    };

    if redirect_uri != url::Url::parse(&app.redirect_uri).unwrap() {
        return Ok(OAuthAuthorizeResponse::Template(Template::render("oauth-error", context! {
            message: fl!(localizer, "invalid-redirect-uri"),
            lang: localizer
        })));
    }

    if let Some(state) = state {
        redirect_uri.query_pairs_mut().append_pair("state", state);
    }

    let force_login = match force_login {
        None => false,
        Some("true") => true,
        Some("1") => true,
        Some("0") => false,
        Some("false") => false,
        _ => {
            redirect_uri.query_pairs_mut().append_pair("error", "invalid_request");
            return Ok(OAuthAuthorizeResponse::RocketRedirect(rocket::response::Redirect::to(redirect_uri.to_string())));
        }
    };

    if response_type != "code" {
        redirect_uri.query_pairs_mut().append_pair("error", "unsupported_response_type");
        return Ok(OAuthAuthorizeResponse::RocketRedirect(rocket::response::Redirect::to(redirect_uri.to_string())));
    }

    let scopes = match scope {
        Some(s) => {
            let mut scopes = s.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>();
            scopes.dedup();
            scopes
        }
        None => vec!["read".to_string()],
    };

    for scope in &scopes {
        if !app_scopes.iter().any(|s| s.scope == *scope) {
            redirect_uri.query_pairs_mut().append_pair("error", "invalid_scope");
            return Ok(OAuthAuthorizeResponse::RocketRedirect(rocket::response::Redirect::to(redirect_uri.to_string())));
        }
    }

    if force_login || oidc_user.is_none() {
        Ok(OAuthAuthorizeResponse::Redirect(match oidc_app.authorize(
            &origin.to_string(), &format!("https://{}", config.uri),
        ) {
            Ok(r) => r,
            Err(_) => {
                redirect_uri.query_pairs_mut().append_pair("error", "server_error");
                return Ok(OAuthAuthorizeResponse::RocketRedirect(rocket::response::Redirect::to(redirect_uri.to_string())));
            }
        }))
    } else {
        let oidc_user = oidc_user.unwrap();
        let user_id = oidc_user.claims.subject().to_string();

        let c_user_id = user_id.clone();
        let consent = crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::oauth_consents::dsl::oauth_consents.filter(
                crate::schema::oauth_consents::dsl::app_id.eq(app.id)
            ).filter(
                crate::schema::oauth_consents::dsl::user_id.eq(&c_user_id)
            ).first::<crate::models::OAuthConsents>(c).optional()
        }).await?;

        let should_consent = match consent {
            Some(consent) => {
                let consent_scopes = crate::db_run(&db, move |c| -> QueryResult<_> {
                    crate::schema::oauth_consent_scopes::dsl::oauth_consent_scopes.filter(
                        crate::schema::oauth_consent_scopes::dsl::consent_id.eq(consent.id)
                    ).load::<crate::models::AppScopes>(c)
                }).await?;

                !scopes.iter().all(|s| consent_scopes.iter().any(|c| c.scope == *s))
            }
            None => true
        };

        if should_consent {
            cookies.add_private(
                rocket::http::Cookie::build("oauth_consent", serde_json::to_string(&OAuthConsentState {
                    return_to: origin.to_string(),
                    scopes: scopes.clone(),
                    client_id: app.id,
                    redirect_uri,
                    state: state.map(|s| s.to_string()),
                }).unwrap())
                    .http_only(true)
                    .secure(true)
                    .same_site(rocket::http::SameSite::Strict)
                    .expires(time::OffsetDateTime::now_utc() + time::Duration::minutes(60))
                    .finish()
            );

            Ok(OAuthAuthorizeResponse::TemplateOk(Template::render("oauth-consent", context! {
                name: app.name,
                website: app.website,
                scopes: scopes.into_iter().filter_map(|s| API_SCOPES.get(&s)).collect::<Vec<_>>(),
                csrf_token: csrf_token.to_string(),
                lang: localizer
            })))
        } else {
            return Ok(match crate::db_run(&db, move |c| -> QueryResult<_> {
                c.transaction(|| {
                    let id = uuid::Uuid::new_v4();
                    diesel::insert_into(crate::schema::oauth_codes::dsl::oauth_codes)
                        .values(&crate::models::OAuthCodes {
                            id: id.clone(),
                            client_id: app.id,
                            user_id,
                            time: Utc::now().naive_utc(),
                            redirect_uri: c_redirect_uri,
                        })
                        .execute(c)?;

                    for scope in scopes {
                        diesel::insert_into(crate::schema::oauth_code_scopes::dsl::oauth_code_scopes)
                            .values(crate::models::OAuthCodeScopes {
                                code_id: id.clone(),
                                scope: scope.to_string(),
                            })
                            .execute(c)?;
                    }
                    Ok(id)
                })
            }).await {
                Ok(id) => {
                    redirect_uri.query_pairs_mut().append_pair("code", &id.to_string());
                    OAuthAuthorizeResponse::RocketRedirect(rocket::response::Redirect::to(redirect_uri.to_string()))
                }
                Err(_) => {
                    redirect_uri.query_pairs_mut().append_pair("error", "server_error");
                    OAuthAuthorizeResponse::RocketRedirect(rocket::response::Redirect::to(redirect_uri.to_string()))
                }
            });
        }
    }
}


#[derive(Responder)]
pub enum OAuthConsentResponse {
    #[response(status = 400)]
    Template(Template),
    Redirect(rocket::response::Redirect),
}

#[derive(FromForm)]
pub struct OAuthConsentForm<'r> {
    csrf_token: &'r str,
    consent: &'r str,
}

#[post("/oauth/consent", data = "<form>")]
pub async fn oauth_consent(
    db: crate::DbConn, oidc_user: super::oidc::OIDCUser, csrf_token: crate::csrf::CSRFToken,
    cookies: &rocket::http::CookieJar<'_>, form: rocket::form::Form<OAuthConsentForm<'_>>,
    localizer: crate::i18n::Localizer,
) -> OAuthConsentResponse {
    if !csrf_token.verify(form.csrf_token) {
        return OAuthConsentResponse::Template(Template::render("oauth-error", context! {
            message: fl!(localizer, "invalid-csrf-token"),
            lang: localizer
        }));
    }

    let state_txt = match cookies.get_private("oauth_consent") {
        Some(t) => t,
        None => return OAuthConsentResponse::Template(Template::render("oauth-error", context! {
            message: fl!(localizer, "invalid-state"),
            lang: localizer
        }))
    };
    let mut state_obj: OAuthConsentState = match serde_json::from_str(state_txt.value()) {
        Ok(s) => s,
        Err(_) => return OAuthConsentResponse::Template(Template::render("oauth-error", context! {
            message: fl!(localizer, "invalid-state"),
            lang: localizer
        }))
    };
    cookies.remove_private(state_txt);

    match form.consent {
        "no" => {
            state_obj.redirect_uri.query_pairs_mut().append_pair("error", "access_denied");
            OAuthConsentResponse::Redirect(rocket::response::Redirect::to(state_obj.redirect_uri.to_string()))
        }
        "yes" => {
            match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
                c.transaction(|| {
                    let id = uuid::Uuid::new_v4();
                    diesel::insert_into(crate::schema::oauth_consents::dsl::oauth_consents)
                        .values(&crate::models::OAuthConsents {
                            id: id.clone(),
                            app_id: state_obj.client_id,
                            user_id: oidc_user.claims.subject().to_string(),
                            time: Utc::now().naive_utc(),
                        })
                        .execute(c)?;

                    for scope in state_obj.scopes {
                        diesel::insert_into(crate::schema::oauth_consent_scopes::dsl::oauth_consent_scopes)
                            .values(crate::models::OAuthConsentScopes {
                                consent_id: id.clone(),
                                scope: scope.to_string(),
                            })
                            .execute(c)?;
                    }
                    Ok(())
                })
            }).await {
                Ok(_) => {
                    OAuthConsentResponse::Redirect(rocket::response::Redirect::to(state_obj.return_to))
                }
                Err(_) => {
                    state_obj.redirect_uri.query_pairs_mut().append_pair("error", "server_error");
                    OAuthConsentResponse::Redirect(rocket::response::Redirect::to(state_obj.redirect_uri.to_string()))
                }
            }
        }
        _ => OAuthConsentResponse::Template(Template::render("oauth-error", context! {
            message: fl!(localizer, "invalid-consent"),
            lang: localizer
        }))
    }
}

#[derive(Clone, Debug)]
pub struct ClientAuth {
    client: crate::models::Apps
}

impl ClientAuth {
    async fn from_request(
        db: &crate::DbConn, basic_auth: Option<rocket_basicauth::BasicAuth>, client_id: Option<&str>, client_secret: Option<&str>
    ) -> Result<ClientAuth, (rocket::http::Status, rocket::serde::json::Json<OAuthError>)> {
        let (client_id, client_secret) = match basic_auth {
            Some(b) => (b.username, b.password),
            None => match (client_id, client_secret) {
                (Some(id), Some(secret)) => (id.to_string(), secret.to_string()),
                _ => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                    error: "invalid_request".to_string(),
                    error_description: Some("Missing client credentials".to_string()),
                    error_uri: None,
                })))
            }
        };

        let client_id = match uuid::Uuid::parse_str(&client_id) {
            Ok(id) => id,
            Err(_) => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                error: "invalid_client".to_string(),
                error_description: Some("Invalid client ID".to_string()),
                error_uri: None,
            })))
        };

        let client_obj: crate::models::Apps = match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
            crate::schema::apps::dsl::apps.find(client_id).first(c)
        }).await {
            Ok(c) => c,
            Err(_) => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                error: "invalid_client".to_string(),
                error_description: Some("Unknown client".to_string()),
                error_uri: None,
            })))
        };

        if client_obj.client_secret != client_secret {
            return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                error: "invalid_client".to_string(),
                error_description: Some("Invalid client credentials".to_string()),
                error_uri: None,
            })));
        }

        Ok(ClientAuth {
            client: client_obj
        })
    }
}

#[derive(FromForm, Deserialize)]
pub struct OAuthTokenForm {
    grant_type: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct OAuthToken {
    access_token: String,
    token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct OAuthError {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_uri: Option<String>,
}

#[post("/oauth/token", data = "<form>", rank = 1)]
pub async fn oauth_token_form(
    config: &rocket::State<AppConfig>, db: crate::DbConn, form: rocket::form::Form<OAuthTokenForm>,
    basic_auth: Option<rocket_basicauth::BasicAuth>
) -> Result<rocket::serde::json::Json<OAuthToken>, (rocket::http::Status, rocket::serde::json::Json<OAuthError>)> {
    _oauth_token(config, db, form.into_inner(), basic_auth).await
}

#[post("/oauth/token", data = "<form>", rank = 2)]
pub async fn oauth_token_json(
    config: &rocket::State<AppConfig>, db: crate::DbConn, form: rocket::serde::json::Json<OAuthTokenForm>,
    basic_auth: Option<rocket_basicauth::BasicAuth>
) -> Result<rocket::serde::json::Json<OAuthToken>, (rocket::http::Status, rocket::serde::json::Json<OAuthError>)> {
    _oauth_token(config, db, form.into_inner(), basic_auth).await
}

pub async fn _oauth_token(
    config: &rocket::State<AppConfig>, db: crate::DbConn, form: OAuthTokenForm,
    basic_auth: Option<rocket_basicauth::BasicAuth>
) -> Result<rocket::serde::json::Json<OAuthToken>, (rocket::http::Status, rocket::serde::json::Json<OAuthError>)> {
    match form.grant_type.as_str() {
        "authorization_code" => {
            let code = match form.code {
                Some(c) => c,
                None => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                    error: "invalid_request".to_string(),
                    error_description: Some("Missing code".to_string()),
                    error_uri: None,
                })))
            };
            let redirect_uri = match form.redirect_uri {
                Some(r) => r,
                None => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                    error: "invalid_request".to_string(),
                    error_description: Some("Missing redirect_uri".to_string()),
                    error_uri: None,
                })))
            };

            let client_obj = ClientAuth::from_request(
                &db, basic_auth, form.client_id.as_deref(), form.client_secret.as_deref()
            ).await?.client;

            let code_id = match uuid::Uuid::parse_str(&code) {
                Ok(id) => id,
                Err(_) => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                    error: "invalid_grant".to_string(),
                    error_description: Some("Invalid code".to_string()),
                    error_uri: None,
                })))
            };

            let code_obj: crate::models::OAuthCodes = match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
                crate::schema::oauth_codes::dsl::oauth_codes.find(code_id).first(c)
            }).await {
                Ok(c) => c,
                Err(_) => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                    error: "invalid_grant".to_string(),
                    error_description: Some("Invalid code".to_string()),
                    error_uri: None,
                })))
            };

            let scopes: Vec<String> = match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
                crate::schema::oauth_code_scopes::dsl::oauth_code_scopes
                    .filter(crate::schema::oauth_code_scopes::dsl::code_id.eq(code_id))
                    .select(crate::schema::oauth_code_scopes::dsl::scope)
                    .load(c)
            }).await {
                Ok(s) => s,
                Err(_) => return Err((rocket::http::Status::InternalServerError, rocket::serde::json::Json(OAuthError {
                    error: "server_error".to_string(),
                    error_description: None,
                    error_uri: None,
                })))
            };

            match crate::db_run(&db, move |c| -> QueryResult<()> {
                diesel::delete(crate::schema::oauth_code_scopes::dsl::oauth_code_scopes.filter(
                crate::schema::oauth_code_scopes::dsl::code_id.eq(code_id)
                )).execute(c)?;
                Ok(())
            }).await {
                Ok(_) => {}
                Err(_) => return Err((rocket::http::Status::InternalServerError, rocket::serde::json::Json(OAuthError {
                    error: "server_error".to_string(),
                    error_description: None,
                    error_uri: None,
                })))
            };

            match crate::db_run(&db, move |c| -> QueryResult<()> {
                diesel::delete(crate::schema::oauth_codes::dsl::oauth_codes.find(code_id))
                    .execute(c)?;
                Ok(())
            }).await {
                Ok(_) => {}
                Err(_) => return Err((rocket::http::Status::InternalServerError, rocket::serde::json::Json(OAuthError {
                    error: "server_error".to_string(),
                    error_description: None,
                    error_uri: None,
                })))
            };

            if code_obj.client_id != client_obj.id {
                return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                    error: "invalid_grant".to_string(),
                    error_description: Some("Code issued to another client".to_string()),
                    error_uri: None,
                })));
            }

            if code_obj.redirect_uri != redirect_uri {
                return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                    error: "invalid_grant".to_string(),
                    error_description: Some("Invalid redirect_uri".to_string()),
                    error_uri: None,
                })));
            }

            if code_obj.time + chrono::Duration::seconds(60) < Utc::now().naive_utc() {
                return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
                    error: "invalid_grant".to_string(),
                    error_description: Some("Code expired".to_string()),
                    error_uri: None,
                })));
            }

            let c_scopes = scopes.clone();
            let new_token: crate::models::OAuthToken = match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
                let new_token = crate::models::OAuthToken {
                    id: uuid::Uuid::new_v4(),
                    client_id: client_obj.id,
                    user_id: code_obj.user_id,
                    time: Utc::now().naive_utc(),
                    revoked: false,
                };

                c.transaction(|| -> diesel::result::QueryResult<_> {
                    diesel::insert_into(crate::schema::oauth_token::dsl::oauth_token)
                        .values(&new_token)
                        .execute(c)?;

                    for scope in c_scopes {
                        diesel::insert_into(crate::schema::oauth_token_scopes::dsl::oauth_token_scopes)
                            .values(crate::models::OAuthTokenScopes {
                                token_id: new_token.id.clone(),
                                scope: scope.to_string(),
                            })
                            .execute(c)?;
                    }
                    Ok(())
                })?;

                Ok(new_token)
            }).await {
                Ok(t) => t,
                Err(_) => return Err((rocket::http::Status::InternalServerError, rocket::serde::json::Json(OAuthError {
                    error: "server_error".to_string(),
                    error_description: None,
                    error_uri: None,
                })))
            };

            let now = Utc::now().naive_utc();
            let token_claims = TokenClaims {
                issuer: format!("https://{}", config.uri),
                subject: new_token.user_id,
                audience: format!("https://{}", config.uri),
                issued_at:  now.timestamp(),
                not_before: now.timestamp(),
                json_web_token_id: new_token.id,
                scopes: scopes.clone(),
            };

            Ok(rocket::serde::json::Json(OAuthToken {
                access_token: token_claims.sign(&config.jwt_secret),
                token_type: "Bearer".to_string(),
                scope: Some(scopes.join(" ")),
                created_at: Some(new_token.time.timestamp()),
            }))
        }
        _ => Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
            error: "unsupported_grant_type".to_string(),
            error_description: None,
            error_uri: None,
        })))
    }
}

#[derive(FromForm)]
pub struct OAuthRevokeForm<'r> {
    token: &'r str,
    client_id: Option<&'r str>,
    client_secret: Option<&'r str>,
}


#[post("/oauth/revoke", data = "<form>")]
pub async fn oauth_revoke(
    config: &rocket::State<AppConfig>, db: crate::DbConn, form: rocket::form::Form<OAuthRevokeForm<'_>>,
    basic_auth: Option<rocket_basicauth::BasicAuth>
) -> Result<rocket::serde::json::Json<()>, (rocket::http::Status, rocket::serde::json::Json<OAuthError>)> {
    let client_obj = ClientAuth::from_request(&db, basic_auth, form.client_id, form.client_secret).await?.client;

    let claims = match TokenClaims::verify(form.token, &config) {
        Ok(c) => c,
        Err(_) => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
            error: "invalid_request".to_string(),
            error_description: None,
            error_uri: None,
        }))),
    };

    let token_obj: crate::models::OAuthToken = match crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
        crate::schema::oauth_token::dsl::oauth_token.find(&claims.json_web_token_id).first(c)
    }).await {
        Ok(c) => c,
        Err(_) => return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
            error: "invalid_request".to_string(),
            error_description: None,
            error_uri: None,
        })))
    };

    if token_obj.client_id != client_obj.id {
        return Err((rocket::http::Status::BadRequest, rocket::serde::json::Json(OAuthError {
            error: "invalid_grant".to_string(),
            error_description: None,
            error_uri: None,
        })))
    }

    match crate::db_run(&db, move |c| -> diesel::result::QueryResult<()> {
        diesel::update(crate::schema::oauth_token::dsl::oauth_token.find(&claims.json_web_token_id))
            .set(crate::schema::oauth_token::dsl::revoked.eq(true))
            .execute(c)?;
        Ok(())
    }).await {
        Ok(_) => {}
        Err(_) => return Err((rocket::http::Status::InternalServerError, rocket::serde::json::Json(OAuthError {
            error: "server_error".to_string(),
            error_description: None,
            error_uri: None,
        })))
    };

    Ok(rocket::serde::json::Json(()))
}