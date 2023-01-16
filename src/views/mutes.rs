#[get("/api/v1/mutes")]
pub async fn mutes(
    user: super::oauth::TokenClaims, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, super::Error> {
    if !user.has_scope("read:mutes") || !user.has_scope("follow") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
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
    user: super::oauth::TokenClaims, account_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, super::Error> {
    if !user.has_scope("write:mutes") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let _account_id = match uuid::Uuid::parse_str(&account_id) {
        Ok(id) => id,
        Err(_) => return Err(super::Error {
            code: rocket::http::Status::NotFound,
            error: fl!(localizer, "account-not-found")
        })
    };

    Err(super::Error {
        code: rocket::http::Status::ServiceUnavailable,
        error: fl!(localizer, "service-unavailable")
    })
}

#[get("/api/v1/accounts/<_account_id>/unmute")]
pub async fn get_unmute_account(
    _account_id: &str
) -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[post("/api/v1/accounts/<account_id>/unmute")]
pub async fn unmute_account(
    user: super::oauth::TokenClaims, account_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, super::Error> {
    if !user.has_scope("write:mutes") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let _account_id = match uuid::Uuid::parse_str(&account_id) {
        Ok(id) => id,
        Err(_) => return Err(super::Error {
            code: rocket::http::Status::NotFound,
            error: fl!(localizer, "account-not-found")
        })
    };

    Err(super::Error {
        code: rocket::http::Status::ServiceUnavailable,
        error: fl!(localizer, "service-unavailable")
    })
}