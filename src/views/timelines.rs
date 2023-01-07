use crate::AppConfig;

#[get("/api/v1/timelines/home?<max_id>&<since_id>&<min_id>&<limit>")]
pub async fn timeline_home(
    config: &rocket::State<AppConfig>, max_id: Option<String>, since_id: Option<String>,
    min_id: Option<String>, limit: Option<u64>, user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<super::objs::Status>>, rocket::http::Status> {
    if !user.has_scope("read:statuses") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[get("/api/v1/timelines/tag/<hashtag>?<any>&<all>&<none>&<local>&<remote>&<only_media>&<max_id>&<since_id>&<min_id>&<limit>")]
pub async fn timeline_hashtag(
    config: &rocket::State<AppConfig>, hashtag: &str, any: Option<Vec<&str>>, all: Option<Vec<&str>>,
    none: Option<Vec<&str>>, local: Option<&str>, remote: Option<&str>,
    only_media: Option<&str>, max_id: Option<String>, since_id: Option<String>,
    min_id: Option<String>, limit: Option<u64>
) -> Result<rocket::serde::json::Json<Vec<super::objs::Status>>, rocket::http::Status> {
    let _local = super::parse_bool(local, false)?;
    let _remote = super::parse_bool(remote, false)?;
    let _only_media = super::parse_bool(only_media, false)?;

    Ok(rocket::serde::json::Json(vec![]))
}

#[get("/api/v1/timelines/public?<local>&<remote>&<only_media>&<max_id>&<since_id>&<min_id>&<limit>")]
pub async fn timeline_public(
    config: &rocket::State<AppConfig>, local: Option<&str>, remote: Option<&str>, only_media: Option<&str>,
    max_id: Option<String>, since_id: Option<String>, min_id: Option<String>, limit: Option<u64>,
) -> Result<rocket::serde::json::Json<Vec<super::objs::Status>>, rocket::http::Status> {
    let _local = super::parse_bool(local, false)?;
    let _remote = super::parse_bool(remote, false)?;
    let _only_media = super::parse_bool(only_media, false)?;

    Ok(rocket::serde::json::Json(vec![]))
}