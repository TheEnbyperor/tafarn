use crate::AppConfig;

#[get("/api/v1/conversations?<max_id>&<since_id>&<min_id>&<limit>")]
pub async fn conversations(
    _config: &rocket::State<AppConfig>, max_id: Option<String>, since_id: Option<String>,
    min_id: Option<String>, limit: Option<u64>, user: super::oauth::TokenClaims,
    localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<Vec<super::objs::Conversation>>, super::Error> {
    if !user.has_scope("read:statuses") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[delete("/api/v1/conversations/<_id>")]
pub async fn delete_conversation(
    _config: &rocket::State<AppConfig>, _id: &str, user: super::oauth::TokenClaims,
    localizer: crate::i18n::Localizer,
) -> Result<rocket::serde::json::Json<()>, super::Error> {
    if !user.has_scope("write:conversations") {
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

#[post("/api/v1/conversations/<_id>/read")]
pub async fn read_conversation(
    _config: &rocket::State<AppConfig>, _id: &str, user: super::oauth::TokenClaims,
    localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<()>, super::Error> {
    if !user.has_scope("write:conversations") {
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