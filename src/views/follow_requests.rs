#[get("/api/v1/follow_requests")]
pub async fn follow_requests(
    user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, rocket::http::Status> {
    if !user.has_scope("read:follows") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[post("/api/v1/follow_requests/<_acct_id>/accept")]
pub async fn accept_follow_request(
    user: super::oauth::TokenClaims, _acct_id: String
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:filters") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[post("/api/v1/follow_requests/<_acct_id>/reject")]
pub async fn reject_follow_request(
    user: super::oauth::TokenClaims, _acct_id: String
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}