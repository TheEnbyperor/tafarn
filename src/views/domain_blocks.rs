#[get("/api/v1/domain_blocks")]
pub async fn domain_blocks(
    user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<String>>, rocket::http::Status> {
    if !user.has_scope("read:blocks") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[derive(Deserialize, FromForm)]
pub struct DomainBlock {
    domain: String,
}

#[post("/api/v1/domain_blocks", data = "<_form>")]
pub async fn create_domain_block(
    user: super::oauth::TokenClaims, _form: rocket::form::Form<DomainBlock>
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:filters") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[delete("/api/v1/domain_block", data = "<_form>")]
pub async fn delete_domain_block(
    user: super::oauth::TokenClaims, _form: rocket::form::Form<DomainBlock>
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}