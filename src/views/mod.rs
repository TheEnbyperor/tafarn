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
pub mod objs;
pub mod activity_streams;

pub fn parse_bool(s: Option<&str>, default: bool) -> Result<bool, rocket::http::Status> {
    Ok(match s {
        None => default,
        Some("true") => true,
        Some("1") => true,
        Some("0") => false,
        Some("false") => false,
        _ => return Err(rocket::http::Status::BadRequest)
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