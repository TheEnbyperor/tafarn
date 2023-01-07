#[get("/api/v1/filters")]
pub async fn filters(
    user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<super::objs::Filter>>, rocket::http::Status> {
    if !user.has_scope("read:filters") {
        return Err(rocket::http::Status::Forbidden);
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
    user: super::oauth::TokenClaims, _form: rocket::form::Form<FilterCreateForm>
) -> Result<rocket::serde::json::Json<super::objs::Filter>, rocket::http::Status> {
    if !user.has_scope("write:filters") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[get("/api/v1/filters/<_filter_id>")]
pub async fn filter(
    user: super::oauth::TokenClaims, _filter_id: String
) -> Result<rocket::serde::json::Json<super::objs::Filter>, rocket::http::Status> {
    if !user.has_scope("read:filters") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(super::objs::Filter {}))
}

#[post("/api/v1/filters/<_filter_id>", data = "<_form>")]
pub async fn update_filter(
    user: super::oauth::TokenClaims, _filter_id: String, _form: rocket::form::Form<FilterCreateForm>
) -> Result<rocket::serde::json::Json<super::objs::Filter>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[delete("/api/v1/filters/<_filter_id>")]
pub async fn delete_filter(
    user: super::oauth::TokenClaims, _filter_id: String
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}