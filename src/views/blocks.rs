#[get("/api/v1/blocks")]
pub async fn blocks(
    user: super::oauth::TokenClaims, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, super::Error> {
    if !user.has_scope("read:blocks") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
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
    user: super::oauth::TokenClaims, account_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, super::Error> {
    if !user.has_scope("write:blocks") {
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

#[get("/api/v1/accounts/<_account_id>/unblock")]
pub async fn get_unblock_account(
    _account_id: &str
) -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[post("/api/v1/accounts/<account_id>/unblock")]
pub async fn unblock_account(
    user: super::oauth::TokenClaims, account_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, super::Error> {
    if !user.has_scope("write:blocks") {
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