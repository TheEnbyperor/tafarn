use celery::error::TaskError;
use celery::prelude::TaskResultExt;
use celery::task::TaskResult;
use crate::views::activity_streams;

pub mod inbox;
pub mod accounts;
pub mod relationships;
pub mod delivery;
pub mod notifications;
pub mod statuses;

const SIGNED_HEADERS: [&str; 4] = ["host", "date", "digest", "content-type"];

lazy_static::lazy_static! {
    pub static ref CONFIG: std::sync::RwLock<Option<Config>> = std::sync::RwLock::new(None);
}

#[derive(Clone)]
pub struct Config {
    pub db: std::sync::Arc<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
    pub celery: std::sync::Arc<crate::CeleryApp>,
    pub uri: String,
    pub vapid_key: Vec<u8>,
    pub web_push_client: std::sync::Arc<web_push_old::WebPushClient>,
    pub as_key: std::sync::Arc<openssl::pkey::PKey<openssl::pkey::Private>>,
}

#[inline]
pub(crate) fn config() -> Config {
    CONFIG.read().unwrap().as_ref().unwrap().clone()
}

fn sign_request(req: &mut reqwest::Request, pkey: &openssl::pkey::PKeyRef<openssl::pkey::Private>, key_id: String) -> Result<(), String> {
    let mut signed_data = vec![
        format!("(request-target): {} {}", req.method().as_str().to_lowercase(), req.url().path()),
    ];
    let mut signed_headers = vec!["(request-target)".to_string()];
    for (header_name, header_value) in req.headers().iter() {
        if SIGNED_HEADERS.iter().any(|h| header_name == h) {
            signed_data.push(format!(
                "{}: {}",
                header_name.as_str().to_lowercase(),
                header_value.to_str().map_err(|e| format!("Unable to convert header to string: {}", e))?
            ));
            signed_headers.push(header_name.as_str().to_lowercase());
        }
    }

    let signed_data = signed_data.join("\n").into_bytes();
    let mut signer = openssl::sign::Signer::new(openssl::hash::MessageDigest::sha256(), &pkey)
        .map_err(|e| format!("Unable to create signer: {}", e))?;
    let signature = signer.sign_oneshot_to_vec(&signed_data)
        .map_err(|e| format!("Unable to sign request: {}", e))?;
    req.headers_mut().insert("Signature", format!(
        "keyId=\"{}\",algorithm=\"rsa-sha256\",headers=\"{}\",signature=\"{}\"",
        key_id, signed_headers.join(" "),
        base64::encode(signature)
    ).parse().map_err(|e| format!("Unable to parse signature header: {}", e))?);

    Ok(())
}

async fn fetch_object<'a, T: serde::de::DeserializeOwned, U: Into<std::borrow::Cow<'a, str>>>(uri: U) -> Option<T> {
    let uri = uri.into();
    let config = config();
    let pkey = config.as_key;

    let url = match reqwest::Url::parse(&uri) {
        Ok(url) => url,
        Err(e) => {
            warn!("Unable to parse URL {}: {}", uri, e);
            return None;
        }
    };
    let host = url.host_str().map(|h| h.to_string())?;
    let date = chrono::Utc::now().naive_utc().format("%a, %d %h %Y %H:%M:%S GMT").to_string();

    match backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
        let mut req = match crate::AS_CLIENT.get(url.clone())
            .header("Host", &host)
            .header("Date", &date)
            .build() {
            Ok(req) => req,
            Err(e) => return Err(backoff::Error::Permanent(e.to_string()))
        };

        match sign_request(&mut req, &pkey, format!("https://{}/as/system#key", config.uri)) {
            Ok(()) => (),
            Err(e) => return Err(backoff::Error::Permanent(e))
        }

        match crate::AS_CLIENT.execute(req).await {
            Ok(r) => {
                match r.error_for_status() {
                    Ok(r) => {
                        match r.json::<T>().await {
                            Ok(r) => Ok(r),
                            Err(e) => Err(backoff::Error::Permanent(e.to_string()))
                        }
                    },
                    Err(e) => {
                        if e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
                            info!("Too many requests on {}, retrying", uri);
                            Err(backoff::Error::Transient { err: e.to_string(), retry_after: None })
                        } else {
                            Err(backoff::Error::Permanent(e.to_string()))
                        }
                    }
                }
            }
            Err(e) => Err(backoff::Error::Permanent(e.to_string())),
        }
    }).await {
        Ok(r) => Some(r),
        Err(e) => {
            warn!("Failed to fetch object {}: {}", uri, e);
            None
        }
    }
}

#[inline]
pub async fn resolve_object<T: serde::de::DeserializeOwned>(obj: activity_streams::ReferenceOrObject<T>) -> Option<T> {
    match obj {
        activity_streams::ReferenceOrObject::Object(o) => Some(*o),
        activity_streams::ReferenceOrObject::Reference(uri) => fetch_object(&uri).await
    }
}

#[inline]
pub async fn resolve_object_or_link(obj: activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>) -> Option<activity_streams::Object> {
    match resolve_object(obj).await? {
        activity_streams::ObjectOrLink::Object(o) => Some(o),
        activity_streams::ObjectOrLink::Link(uri) => fetch_object(&uri.href?).await
    }
}

#[inline]
pub fn resolve_url(obj: activity_streams::URLOrLink) -> Option<String> {
    match obj {
        activity_streams::URLOrLink::URL(u) => Some(u),
        activity_streams::URLOrLink::Link(l) => l.href
    }
}