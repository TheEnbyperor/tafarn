pub struct CSRFFairing;
#[derive(Debug)]
pub struct CSRFToken(Vec<u8>);

#[rocket::async_trait]
impl rocket::fairing::Fairing for CSRFFairing {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "CSRF",
            kind: rocket::fairing::Kind::Request,
        }
    }

    async fn on_request(&self, request: &mut rocket::Request<'_>, _: &mut rocket::Data<'_>) {
        debug!("Request cookies: {:#?}", request.cookies());
        if let Some(_) = request.cookies()
            .get_private("csrf_token")
            .and_then(|c| base64::decode_config(c.value(), base64::URL_SAFE).ok()) {
            return;
        }

        use rand::Rng;
        let values: Vec<u8> = rand::thread_rng()
            .sample_iter(rand::distributions::Standard)
            .take(64)
            .collect();

        let encoded = base64::encode_config(&values[..], base64::URL_SAFE);

        request.cookies().add_private(
            rocket::http::Cookie::build("csrf_token", encoded)
                .http_only(true)
                .expires(time::OffsetDateTime::now_utc() + time::Duration::hours(6))
                .finish(),
        );
    }
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for CSRFToken {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let cookies = request.cookies();
        let csrf_cookie = match cookies
            .get_private("csrf_token")
            .or_else(|| cookies.get_pending("csrf_token"))
            .and_then(|c| base64::decode_config(c.value(), base64::URL_SAFE).ok()) {
            Some(c) => c,
            None => {
                use rand::Rng;
                let values: Vec<u8> = rand::thread_rng()
                    .sample_iter(rand::distributions::Standard)
                    .take(64)
                    .collect();


                request.cookies().add_private(
                    rocket::http::Cookie::build("csrf_token", base64::encode_config(&values[..], base64::URL_SAFE))
                        .http_only(true)
                        .expires(time::OffsetDateTime::now_utc() + time::Duration::hours(6))
                        .finish(),
                );

                values
            }
        };

        rocket::request::Outcome::Success(CSRFToken(csrf_cookie))
    }
}

impl CSRFToken {
    pub fn verify(&self, token: &str) -> bool {
        match base64::decode_config(token, base64::URL_SAFE).ok() {
            Some(t) => t == self.0,
            None => false
        }
    }
}

impl ToString for CSRFToken {
    fn to_string(&self) -> String {
        base64::encode_config(&self.0, base64::URL_SAFE)
    }
}