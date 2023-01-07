#[get("/api/v1/blocks")]
pub async fn blocks(
    user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, rocket::http::Status> {
    if !user.has_scope("read:blocks") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[get("/api/v1/accounts/<_account_id>/block")]
pub async fn get_block_account(
    _account_id: &str
) -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[post("/api/v1/accounts/<account_id>/block")]
pub async fn block_account(
    user: super::oauth::TokenClaims, account_id: String
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, rocket::http::Status> {
    if !user.has_scope("write:blocks") {
        return Err(rocket::http::Status::Forbidden);
    }

    let _account_id = match uuid::Uuid::parse_str(&account_id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    Err(rocket::http::Status::ServiceUnavailable)
}

#[get("/api/v1/accounts/<_account_id>/unblock")]
pub async fn get_unblock_account(
    _account_id: &str
) -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[post("/api/v1/accounts/<account_id>/unblock")]
pub async fn unblock_account(
    user: super::oauth::TokenClaims, account_id: String
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, rocket::http::Status> {
    if !user.has_scope("write:blocks") {
        return Err(rocket::http::Status::Forbidden);
    }

    let _account_id = match uuid::Uuid::parse_str(&account_id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    Err(rocket::http::Status::ServiceUnavailable)
}