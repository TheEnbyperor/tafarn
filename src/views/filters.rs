#[get("/api/v1/filters")]
pub async fn filters(
    user: super::oauth::TokenClaims, localizer: crate::i18n::Localizer,
) -> Result<rocket::serde::json::Json<Vec<super::objs::Filter>>, super::Error> {
    if !user.has_scope("read:filters") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[derive(Deserialize, FromForm)]
pub struct FilterCreateForm {
    phrase: String,
    context: Vec<FilterContexts>,
    irreversible: Option<bool>,
    whole_word: Option<bool>,
    expires_in: Option<u64>,
}

#[derive(Serialize, Deserialize, FromFormField, Debug)]
pub enum FilterContexts {
    #[serde(rename = "home")]
    Home,
    #[serde(rename = "notifications")]
    Notifications,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "thread")]
    Thread
}

#[post("/api/v1/filters", data = "<_form>")]
pub async fn create_filter(
    user: super::oauth::TokenClaims, _form: rocket::form::Form<FilterCreateForm>,
    localizer: crate::i18n::Localizer,
) -> Result<rocket::serde::json::Json<super::objs::Filter>, super::Error> {
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

#[get("/api/v1/filters/<_filter_id>")]
pub async fn filter(
    user: super::oauth::TokenClaims, _filter_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::Filter>, super::Error> {
    if !user.has_scope("read:filters") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    Ok(rocket::serde::json::Json(super::objs::Filter {}))
}

#[post("/api/v1/filters/<_filter_id>", data = "<_form>")]
pub async fn update_filter(
    user: super::oauth::TokenClaims, _filter_id: String, _form: rocket::form::Form<FilterCreateForm>,
    localizer: crate::i18n::Localizer,
) -> Result<rocket::serde::json::Json<super::objs::Filter>, super::Error> {
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

#[delete("/api/v1/filters/<_filter_id>")]
pub async fn delete_filter(
    user: super::oauth::TokenClaims, _filter_id: String, localizer: crate::i18n::Localizer
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