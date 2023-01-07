#[get("/api/v1/lists")]
pub async fn lists(
    user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<Vec<super::objs::List>>, rocket::http::Status> {
    if !user.has_scope("read:lists") {
        return Err(rocket::http::Status::Forbidden);
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
    user: super::oauth::TokenClaims, _form: rocket::form::Form<ListCreateForm>
) -> Result<rocket::serde::json::Json<super::objs::List>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[get("/api/v1/lists/<_list_id>")]
pub async fn list(
    user: super::oauth::TokenClaims, _list_id: String
) -> Result<rocket::serde::json::Json<super::objs::List>, rocket::http::Status> {
    if !user.has_scope("read:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(super::objs::List {}))
}

#[post("/api/v1/lists/<_list_id>", data = "<_form>")]
pub async fn update_list(
    user: super::oauth::TokenClaims, _list_id: String, _form: rocket::form::Form<ListCreateForm>
) -> Result<rocket::serde::json::Json<super::objs::List>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[delete("/api/v1/lists/<_list_id>")]
pub async fn delete_list(
    user: super::oauth::TokenClaims, _list_id: String
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[get("/api/v1/lists/<_list_id>/accounts")]
pub async fn list_accounts(
    user: super::oauth::TokenClaims, _list_id: String
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, rocket::http::Status> {
    if !user.has_scope("read:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(vec![]))
}

#[derive(FromForm)]
pub struct ListAccountsForm {
    account_ids: Vec<String>
}

#[post("/api/v1/lists/<_list_id>/accounts", data = "<_form>")]
pub async fn list_add_accounts(
    user: super::oauth::TokenClaims, _list_id: String, _form: rocket::form::Form<ListAccountsForm>
) -> Result<rocket::serde::json::Json<super::objs::List>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}

#[delete("/api/v1/lists/<_list_id>/accounts", data = "<_form>")]
pub async fn list_delete_accounts(
    user: super::oauth::TokenClaims, _list_id: String, _form: rocket::form::Form<ListAccountsForm>
) -> Result<rocket::serde::json::Json<super::objs::List>, rocket::http::Status> {
    if !user.has_scope("write:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    Err(rocket::http::Status::ServiceUnavailable)
}