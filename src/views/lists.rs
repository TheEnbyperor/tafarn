#[get("/api/v1/lists")]
pub async fn lists(
    user: super::oauth::TokenClaims, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<Vec<super::objs::List>>, super::Error> {
    if !user.has_scope("read:lists") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[derive(Deserialize, FromForm)]
pub struct ListCreateForm {
    title: String,
    #[serde(default)]
    replies_policy: super::objs::ListRepliesPolicy
}

#[post("/api/v1/lists", data = "<_form>")]
pub async fn create_list(
    user: super::oauth::TokenClaims, _form: rocket::form::Form<ListCreateForm>, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::List>, super::Error> {
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

#[get("/api/v1/lists/<_list_id>")]
pub async fn list(
    user: super::oauth::TokenClaims, _list_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::List>, super::Error> {
    if !user.has_scope("read:lists") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(super::objs::List {}))
}

#[post("/api/v1/lists/<_list_id>", data = "<_form>")]
pub async fn update_list(
    user: super::oauth::TokenClaims, _list_id: String, _form: rocket::form::Form<ListCreateForm>,
    localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::List>, super::Error> {
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

#[delete("/api/v1/lists/<_list_id>")]
pub async fn delete_list(
    user: super::oauth::TokenClaims, _list_id: String, localizer: crate::i18n::Localizer
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

#[get("/api/v1/lists/<_list_id>/accounts")]
pub async fn list_accounts(
    user: super::oauth::TokenClaims, _list_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, super::Error> {
    if !user.has_scope("read:lists") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[derive(FromForm)]
pub struct ListAccountsForm {
    account_ids: Vec<String>
}

#[post("/api/v1/lists/<_list_id>/accounts", data = "<_form>")]
pub async fn list_add_accounts(
    user: super::oauth::TokenClaims, _list_id: String, _form: rocket::form::Form<ListAccountsForm>,
    localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::List>, super::Error> {
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

#[delete("/api/v1/lists/<_list_id>/accounts", data = "<_form>")]
pub async fn list_delete_accounts(
    user: super::oauth::TokenClaims, _list_id: String, _form: rocket::form::Form<ListAccountsForm>,
    localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::List>, super::Error> {
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