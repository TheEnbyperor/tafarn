use crate::AppConfig;

#[get("/api/v1/conversations?<max_id>&<since_id>&<min_id>&<limit>")]
pub async fn conversations(
    config: &rocket::State<AppConfig>, max_id: Option<String>, since_id: Option<String>,
    min_id: Option<String>, limit: Option<u64>, user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<super::objs::Conversation>>, rocket::http::Status> {
    if !user.has_scope("read:statuses") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[delete("/api/v1/conversations/<id>")]
pub async fn delete_conversation(
    config: &rocket::State<AppConfig>, id: &str, user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:conversations") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[post("/api/v1/conversations/<id>/read")]
pub async fn read_conversation(
    config: &rocket::State<AppConfig>, id: &str, user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:conversations") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}