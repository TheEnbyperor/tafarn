use diesel::prelude::*;
use chrono::prelude::*;
use futures::StreamExt;

pub async fn render_notification(
    db: &crate::DbConn, config: &crate::AppConfig, notification: crate::models::Notification
) -> Result<super::objs::Notification, rocket::http::Status> {
    let account = crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::accounts::dsl::accounts.find(notification.cause).get_result(c)
    }).await?;

    Ok(super::objs::Notification {
        id: notification.id.to_string(),
        notification_type: notification.notification_type,
        created_at: Utc.from_utc_datetime(&notification.created_at),
        account: super::accounts::render_account(config, &db, account).await?,
        status: None,
        report: None,
    })
}

#[get("/api/v1/notifications?<limit>&<types>&<exclude_types>&<account_id>")]
pub async fn notifications(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    limit: Option<u64>, types: Option<Vec<String>>, exclude_types: Option<Vec<String>>,
    account_id: Option<String>
) -> Result<rocket::serde::json::Json<Vec<super::objs::Notification>>, rocket::http::Status> {
    if !user.has_scope("read:notifications") {
        return Err(rocket::http::Status::Forbidden);
    }

    let limit = limit.unwrap_or(15);
    if limit > 50 {
        return Err(rocket::http::Status::BadRequest);
    }

    let account_id = match account_id {
        Some(id) => match uuid::Uuid::parse_str(&id) {
            Ok(id) => Some(id),
            Err(_) => return Err(rocket::http::Status::BadRequest)
        },
        None => None
    };

    let account = super::accounts::get_account(&db, &user).await?;
    let notifications: Vec<crate::models::Notification> = crate::db_run(&db, move |c| -> QueryResult<_> {
        let mut q = crate::schema::notifications::dsl::notifications.filter(
            crate::schema::notifications::dsl::account.eq(&account.id)
        ).into_boxed();
        if let Some(types) = types {
            q = q.filter(crate::schema::notifications::dsl::notification_type.eq_any(types));
        }
        if let Some(types) = exclude_types {
            q = q.filter(crate::schema::notifications::dsl::notification_type.ne_all(types));
        }
        if let Some(account_id) = account_id {
            q = q.filter(crate::schema::notifications::dsl::cause.eq(account_id));
        }
        q.limit(limit as i64).load(c)
    }).await?;

    Ok(rocket::serde::json::Json(
        futures::stream::iter(notifications.into_iter())
        .map(|n| render_notification(&db, config, n))
            .buffered(10)
        .collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>, _>>()?
    ))
}

#[get("/api/v1/notifications/<notification_id>")]
pub async fn notification(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    notification_id: String
) -> Result<rocket::serde::json::Json<super::objs::Notification>, rocket::http::Status> {
    let notification_id = match uuid::Uuid::parse_str(&notification_id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    if !user.has_scope("read:notifications") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let notification: crate::models::Notification = match crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::notifications::dsl::notifications.find(&notification_id).get_result(c).optional()
    }).await? {
        Some(n) => n,
        None => return Err(rocket::http::Status::NotFound)
    };

    if notification.account != account.id {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(render_notification(&db, config, notification).await?))
}

#[post("/api/v1/notifications/clear")]
pub async fn clear_notifications(
    user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:notifications") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(()))
}

#[post("/api/v1/notifications/<_notification_id>/dimiss")]
pub async fn dismiss_notification(
    user: super::oauth::TokenClaims, _notification_id: String
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:notifications") {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(rocket::serde::json::Json(()))
}