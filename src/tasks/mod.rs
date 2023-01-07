use crate::views::activity_streams;

pub mod inbox;
pub mod accounts;
pub mod relationships;
pub mod delivery;

lazy_static::lazy_static! {
    pub static ref CONFIG: std::sync::RwLock<Option<Config>> = std::sync::RwLock::new(None);
}

#[derive(Clone)]
pub struct Config {
    pub db: std::sync::Arc<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
    pub celery: std::sync::Arc<crate::CeleryApp>,
    pub uri: String,
}

#[inline]
pub(crate) fn config() -> Config {
    CONFIG.read().unwrap().as_ref().unwrap().clone()
}

async fn fetch_object<T: serde::de::DeserializeOwned>(uri: String) -> Option<T> {
    match backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
        match crate::AS_CLIENT.get(&uri).send().await {
            Ok(r) => {
                match r.error_for_status() {
                    Ok(r) => {
                        match r.json::<T>().await {
                            Ok(r) => Ok(r),
                            Err(e) => Err(backoff::Error::Permanent(e))
                        }
                    },
                    Err(e) => {
                        if e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
                            info!("Too many requests on {}, retrying", uri);
                            Err(backoff::Error::Transient { err: e, retry_after: None })
                        } else {
                            Err(backoff::Error::Permanent(e))
                        }
                    }
                }
            }
            Err(e) => Err(backoff::Error::Permanent(e)),
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
        activity_streams::ReferenceOrObject::Reference(uri) => fetch_object(uri).await
    }
}

#[inline]
pub async fn resolve_object_or_link(obj: activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>) -> Option<activity_streams::Object> {
    match resolve_object(obj).await? {
        activity_streams::ObjectOrLink::Object(o) => Some(o),
        activity_streams::ObjectOrLink::Link(uri) => fetch_object(uri.href?).await
    }
}

#[inline]
pub fn resolve_url(obj: activity_streams::URLOrLink) -> Option<String> {
    match obj {
        activity_streams::URLOrLink::URL(u) => Some(u),
        activity_streams::URLOrLink::Link(l) => l.href
    }
}