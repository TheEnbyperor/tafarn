#[get("/api/v1/mutes")]
pub async fn mutes(
    user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, rocket::http::Status> {
    if !user.has_scope("read:mutes") || !user.has_scope("follow") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[get("/api/v1/accounts/<_account_id>/mute")]
pub async fn get_mute_account(
    _account_id: &str
) -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[post("/api/v1/accounts/<account_id>/mute")]
pub async fn mute_account(
    user: super::oauth::TokenClaims, account_id: String
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, rocket::http::Status> {
    if !user.has_scope("write:mutes") {
        return Err(rocket::http::Status::Forbidden);
    }

    let _account_id = match uuid::Uuid::parse_str(&account_id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    Err(rocket::http::Status::ServiceUnavailable)
}

#[get("/api/v1/accounts/<_account_id>/unmute")]
pub async fn get_unmute_account(
    _account_id: &str
) -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[post("/api/v1/accounts/<account_id>/unmute")]
pub async fn unmute_account(
    user: super::oauth::TokenClaims, account_id: String
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, rocket::http::Status> {
    if !user.has_scope("write:mutes") {
        return Err(rocket::http::Status::Forbidden);
    }

    let _account_id = match uuid::Uuid::parse_str(&account_id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    Err(rocket::http::Status::ServiceUnavailable)
}