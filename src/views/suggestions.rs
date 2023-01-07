#[get("/api/v1/suggestions")]
pub async fn suggestions(
    user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, rocket::http::Status> {
    if !user.has_scope("read") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[delete("/api/v1/suggestions/<_acct_id>")]
pub async fn delete_suggestion(
    user: super::oauth::TokenClaims, _acct_id: String
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("read") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}