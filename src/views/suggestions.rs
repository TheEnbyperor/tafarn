#[get("/api/v1/suggestions")]
pub async fn suggestions(
    user: super::oauth::TokenClaims, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, super::Error> {
    if !user.has_scope("read") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[delete("/api/v1/suggestions/<_acct_id>")]
pub async fn delete_suggestion(
    user: super::oauth::TokenClaims, _acct_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<()>, super::Error> {
    if !user.has_scope("read") {
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