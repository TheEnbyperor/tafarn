use rocket::Request;

pub mod oauth;
pub mod timelines;
pub mod meta;
pub mod accounts;
pub mod oidc;
pub mod lists;
pub mod filters;
pub mod domain_blocks;
pub mod follow_requests;
pub mod suggestions;
pub mod notifications;
pub mod web_push;
pub mod instance;
pub mod conversations;
pub mod search;
pub mod mutes;
pub mod blocks;
pub mod media;
pub mod statuses;
pub mod bookmarks;
pub mod favourites;
pub mod objs;
pub mod activity_streams;
pub mod nodeinfo;

pub fn parse_bool(s: Option<&str>, default: bool, localizer: &crate::i18n::Localizer) -> Result<bool, Error> {
    Ok(match s {
        None => default,
        Some("true") => true,
        Some("1") => true,
        Some("0") => false,
        Some("false") => false,
        _ => return Err(Error {
            code: rocket::http::Status::BadRequest,
            error: fl!(localizer, "invalid-request")
        })
    })
}

pub struct LinkedResponse<T> {
    pub inner: T,
    pub links: Vec<Link>
}

#[derive(Debug)]
pub struct Link {
    pub rel: String,
    pub href: String
}

impl <'r, 'o: 'r, T: rocket::response::Responder<'r, 'o>> rocket::response::Responder<'r, 'o> for LinkedResponse<T> {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'o> {
        let mut response = self.inner.respond_to(request)?;
        if !self.links.is_empty() {
            let mut links = vec![];
            for link in self.links {
                links.push(format!(
                    "<{}>; rel=\"{}\"",
                    percent_encoding::utf8_percent_encode(&link.href, percent_encoding::CONTROLS),
                    percent_encoding::utf8_percent_encode(&link.rel, percent_encoding::CONTROLS)
                ));
            }
            response.set_raw_header("Link", links.join(", "));
        }
        Ok(response)
    }
}

pub struct Error {
    pub code: rocket::http::Status,
    pub error: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String
}

impl <'r, 'o: 'r> rocket::response::Responder<'r, 'o> for Error {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'o> {
        let body = serde_json::to_vec(&ErrorResponse {
            error: self.error
        }).unwrap();

        rocket::Response::build()
            .status(self.code)
            .sized_body(body.len(), std::io::Cursor::new(body))
            .header(rocket::http::ContentType::JSON)
            .ok()
    }
}

impl From<Error> for rocket::http::Status {
    fn from(from: Error) -> rocket::http::Status {
        from.code
    }
}