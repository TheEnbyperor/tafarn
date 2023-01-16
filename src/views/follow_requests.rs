#[get("/api/v1/follow_requests")]
pub async fn follow_requests(
    user: super::oauth::TokenClaims, localizer: crate::i18n::Localizer,
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, super::Error> {
    if !user.has_scope("read:follows") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[post("/api/v1/follow_requests/<_acct_id>/accept")]
pub async fn accept_follow_request(
    user: super::oauth::TokenClaims, _acct_id: String, localizer: crate::i18n::Localizer
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

#[post("/api/v1/follow_requests/<_acct_id>/reject")]
pub async fn reject_follow_request(
    user: super::oauth::TokenClaims, _acct_id: String, localizer: crate::i18n::Localizer
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