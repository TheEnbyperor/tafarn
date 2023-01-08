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
        id: notification.iid.to_string(),
        notification_type: notification.notification_type,
        created_at: Utc.from_utc_datetime(&notification.created_at),
        account: super::accounts::render_account(config, &db, account).await?,
        status: None,
        report: None,
    })
}

#[get("/api/v1/notifications?<limit>&<types>&<exclude_types>&<account_id>&<min_id>&<max_id>")]
pub async fn notifications(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    min_id: Option<i32>, max_id: Option<i32>, limit: Option<u64>,
    types: Option<Vec<String>>, exclude_types: Option<Vec<String>>,
    account_id: Option<String>, host: &rocket::http::uri::Host<'_>,
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Notification>>>, rocket::http::Status> {
    if !user.has_scope("read:notifications") {
        return Err(rocket::http::Status::Forbidden);
    }

    let limit = limit.unwrap_or(15);
    if limit > 500 {
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
        ).limit(limit as i64).order_by(crate::schema::notifications::created_at.desc()).into_boxed();
        if let Some(types) = types {
            q = q.filter(crate::schema::notifications::dsl::notification_type.eq_any(types));
        }
        if let Some(types) = exclude_types {
            q = q.filter(crate::schema::notifications::dsl::notification_type.ne_all(types));
        }
        if let Some(account_id) = account_id {
            q = q.filter(crate::schema::notifications::dsl::cause.eq(account_id));
        }
        if let Some(min_id) = min_id {
            q = q.filter(crate::schema::notifications::dsl::iid.gt(min_id));
        }
        if let Some(max_id) = max_id {
            q = q.filter(crate::schema::notifications::dsl::iid.lt(max_id));
        }
        q.load(c)
    }).await?;

    let mut links = vec![];

    if let Some(last_id) = notifications.last().map(|a| a.iid) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/notifications?max_id={}", host.to_string(), last_id)
        });
    }
    if let Some(first_id) = notifications.first().map(|a| a.iid) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/notifications?min_id={}", host.to_string(), first_id)
        });
    }

    Ok(super::LinkedResponse {
        inner: rocket::serde::json::Json(
            futures::stream::iter(notifications.into_iter())
                .map(|n| render_notification(&db, config, n))
                .buffered(10)
                .collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>, _>>()?
        ),
        links
    })
}

async fn get_notification_and_check_visibility(
    notification_id: &str, account: &crate::models::Account, db: &crate::DbConn
) -> Result<crate::models::Notification, rocket::http::Status> {
    let notification_id = match notification_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let notification: crate::models::Notification = match crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::notifications::dsl::notifications.filter(
            crate::schema::notifications::dsl::iid.eq(notification_id)
        ).get_result(c).optional()
    }).await? {
        Some(a) => a,
        None => return Err(rocket::http::Status::NotFound)
    };

    if notification.account != account.id {
        return Err(rocket::http::Status::Forbidden);
    }

    Ok(notification)
}

#[get("/api/v1/notifications/<notification_id>")]
pub async fn notification(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    notification_id: String
) -> Result<rocket::serde::json::Json<super::objs::Notification>, rocket::http::Status> {
    if !user.has_scope("read:notifications") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let notification = get_notification_and_check_visibility(&notification_id, &account, &db).await?;

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

#[post("/api/v1/notifications/<notification_id>/dimiss")]
pub async fn dismiss_notification(
    db: crate::DbConn, user: super::oauth::TokenClaims, notification_id: String
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("write:notifications") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let _notification = get_notification_and_check_visibility(&notification_id, &account, &db).await?;

    Ok(rocket::serde::json::Json(()))
}