#[get("/api/v1/domain_blocks")]
pub async fn domain_blocks(
    user: super::oauth::TokenClaims, localizer: crate::i18n::Localizer,
) -> Result<rocket::serde::json::Json<Vec<String>>, super::Error> {
    if !user.has_scope("read:blocks") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[derive(Deserialize, FromForm)]
pub struct DomainBlock {
    domain: String,
}

#[post("/api/v1/domain_blocks", data = "<_form>")]
pub async fn create_domain_block(
    user: super::oauth::TokenClaims, _form: rocket::form::Form<DomainBlock>,
    localizer: crate::i18n::Localizer,
) -> Result<rocket::serde::json::Json<()>, super::Error> {
    if !user.has_scope("write:filters") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Err(super::Error {
        code: rocket::http::Status::ServiceUnavailable,
        error: fl!(localizer, "service-unavailable")
    })
}

#[delete("/api/v1/domain_block", data = "<_form>")]
pub async fn delete_domain_block(
    user: super::oauth::TokenClaims, _form: rocket::form::Form<DomainBlock>,
    localizer: crate::i18n::Localizer,
) -> Result<rocket::serde::json::Json<()>, super::Error> {
    if !user.has_scope("write:lists") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Err(super::Error {
        code: rocket::http::Status::ServiceUnavailable,
        error: fl!(localizer, "service-unavailable")
    })
}