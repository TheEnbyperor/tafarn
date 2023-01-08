use crate::AppConfig;
use diesel::prelude::*;

#[derive(Deserialize)]
pub struct WebPushSubscription {
    subscription: WebPushSubscriptionData,
    data: WebPushData,
    #[serde(default)]
    policy: WebPushPolicy,
}

#[derive(Deserialize)]
pub struct WebPushSubscriptionData {
    endpoint: String,
    keys: WebPushKeys,
}

#[derive(Deserialize)]
pub struct WebPushKeys {
    p256dh: String,
    auth: String,
}

#[derive(Deserialize)]
pub struct WebPushData {
    alerts: super::objs::WebPushAlerts,
}

#[derive(Deserialize)]
pub enum WebPushPolicy {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "followed")]
    Followed,
    #[serde(rename = "follower")]
    Follower,
    #[serde(rename = "none")]
    None,
}

impl Default for WebPushPolicy {
    fn default() -> Self {
        Self::All
    }
}

impl ToString for WebPushPolicy {
    fn to_string(&self) -> String {
        match self {
            WebPushPolicy::All => "all".to_string(),
            WebPushPolicy::Followed => "followed".to_string(),
            WebPushPolicy::Follower => "follower".to_string(),
            WebPushPolicy::None => "none".to_string(),
        }
    }
}

fn render_subscription(
    subscription: crate::models::WebPushSubscription, config: &AppConfig
) -> rocket::serde::json::Json<super::objs::WebPushSubscription> {
    rocket::serde::json::Json(super::objs::WebPushSubscription {
        id: subscription.id.to_string(),
        endpoint: subscription.endpoint,
        alerts: super::objs::WebPushAlerts {
            follow: subscription.follow,
            favourite: subscription.favourite,
            reblog: subscription.reblog,
            mention: subscription.mention,
            poll: subscription.poll,
            status: subscription.status,
            follow_request: subscription.follow_request,
            update: subscription.update,
            admin_sign_up: subscription.admin_sign_up,
            admin_report: subscription.admin_report,
        },
        server_key: base64::encode_config(config.web_push_signature.get_public_key(), base64::URL_SAFE_NO_PAD),
    })
}

#[post("/api/v1/push/subscription", data = "<data>")]
pub async fn create_subscription(
    db: crate::DbConn, config: &rocket::State<AppConfig>,
    user: super::oauth::TokenClaims, data: rocket::serde::json::Json<WebPushSubscription>,
) -> Result<rocket::serde::json::Json<super::objs::WebPushSubscription>, rocket::http::Status> {
    if !user.has_scope("push") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = user.get_account(&db).await?;

    let new_subscription = crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
        let new_subscription = crate::models::WebPushSubscription {
            id: uuid::Uuid::new_v4(),
            token_id: user.json_web_token_id,
            account_id: account.id,
            endpoint: data.subscription.endpoint.clone(),
            p256dh: data.subscription.keys.p256dh.clone(),
            auth: data.subscription.keys.auth.clone(),
            follow: data.data.alerts.follow,
            favourite: data.data.alerts.favourite,
            reblog: data.data.alerts.reblog,
            mention: data.data.alerts.mention,
            poll: data.data.alerts.poll,
            status: data.data.alerts.status,
            follow_request: data.data.alerts.follow_request,
            update: data.data.alerts.update,
            admin_sign_up: data.data.alerts.admin_sign_up,
            admin_report: data.data.alerts.admin_sign_up,
            policy: data.policy.to_string(),
        };

        c.transaction(|| -> diesel::result::QueryResult<_> {
            diesel::delete(crate::schema::web_push_subscriptions::dsl::web_push_subscriptions.filter(
                crate::schema::web_push_subscriptions::dsl::token_id.eq(user.json_web_token_id)
            )).execute(c)?;

            diesel::insert_into(crate::schema::web_push_subscriptions::dsl::web_push_subscriptions)
                .values(&new_subscription)
                .execute(c)?;

            Ok(())
        })?;

        Ok(new_subscription)
    }).await?;

    Ok(render_subscription(new_subscription, config))
}

#[get("/api/v1/push/subscription")]
pub async fn get_subscription(
    db: crate::DbConn, user: super::oauth::TokenClaims, config: &rocket::State<AppConfig>,
) -> Result<rocket::serde::json::Json<super::objs::WebPushSubscription>, rocket::http::Status> {
    if !user.has_scope("push") {
        return Err(rocket::http::Status::Forbidden);
    }

    let subscription: crate::models::WebPushSubscription = crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
        crate::schema::web_push_subscriptions::dsl::web_push_subscriptions.filter(
            crate::schema::web_push_subscriptions::dsl::token_id.eq(user.json_web_token_id)
        ).get_result(c)
    }).await?;


    Ok(render_subscription(subscription, config))
}

#[derive(Deserialize)]
pub struct WebPushSubscriptionUpdate {
    data: WebPushData,
    policy: Option<WebPushPolicy>,
}

#[put("/api/v1/push/subscription", data = "<data>")]
pub async fn update_subscription(
    db: crate::DbConn, user: super::oauth::TokenClaims, config: &rocket::State<AppConfig>,
    data: rocket::serde::json::Json<WebPushSubscriptionUpdate>,
) -> Result<rocket::serde::json::Json<super::objs::WebPushSubscription>, rocket::http::Status> {
    if !user.has_scope("push") {
        return Err(rocket::http::Status::Forbidden);
    }

    use crate::schema::web_push_subscriptions;
    #[derive(AsChangeset)]
    #[table_name="web_push_subscriptions"]
    pub struct WebPushSubscriptionUpdate {
        follow: bool,
        favourite: bool,
        reblog: bool,
        mention: bool,
        poll: bool,
        status: bool,
        follow_request: bool,
        update: bool,
        admin_sign_up: bool,
        admin_report: bool,
        policy: Option<String>,
    }

    let subscription: crate::models::WebPushSubscription = crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
        diesel::update(crate::schema::web_push_subscriptions::dsl::web_push_subscriptions.filter(
            crate::schema::web_push_subscriptions::dsl::token_id.eq(user.json_web_token_id)
        ))
            .set(WebPushSubscriptionUpdate {
                policy: data.policy.as_ref().map(|p| p.to_string()),
                follow: data.data.alerts.follow,
                favourite: data.data.alerts.favourite,
                reblog: data.data.alerts.reblog,
                mention: data.data.alerts.mention,
                poll: data.data.alerts.poll,
                status: data.data.alerts.status,
                follow_request: data.data.alerts.follow_request,
                update: data.data.alerts.update,
                admin_sign_up: data.data.alerts.admin_sign_up,
                admin_report: data.data.alerts.admin_report,
            })
            .get_result(c)
    }).await?;


    Ok(render_subscription(subscription, config))
}

#[delete("/api/v1/push/subscription")]
pub async fn delete_subscription(
    db: crate::DbConn, user: super::oauth::TokenClaims,
) -> Result<rocket::serde::json::Json<()>, rocket::http::Status> {
    if !user.has_scope("push") {
        return Err(rocket::http::Status::Forbidden);
    }

    crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
        diesel::delete(crate::schema::web_push_subscriptions::dsl::web_push_subscriptions.filter(
                crate::schema::web_push_subscriptions::dsl::token_id.eq(user.json_web_token_id)
            )).execute(c)
    }).await?;

    Ok(rocket::serde::json::Json(()))
}