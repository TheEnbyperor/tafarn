use celery::prelude::*;
use diesel::prelude::*;
use crate::models;

#[derive(Serialize, Deserialize, Clone)]
struct NotificationData {
    title: String,
    body: String,
    preferred_locale: String,
    access_token: String,
    notification_id: i32,
    notification_type: String,
    icon: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Notification {
    endpoint: String,
    p256dh: String,
    auth: String,
    data: NotificationData,
}

#[celery::task]
pub async fn notify(notification: models::Notification) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    let (account, cause, status, is_followed, is_following) = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        let a = crate::schema::accounts::dsl::accounts
            .filter(crate::schema::accounts::dsl::id.eq(notification.account))
            .get_result::<models::Account>(&c).with_expected_err(|| "Unable to get account")?;
        let ca = crate::schema::accounts::dsl::accounts
            .filter(crate::schema::accounts::dsl::id.eq(notification.cause))
            .get_result::<models::Account>(&c).with_expected_err(|| "Unable to get account")?;
        let s = match notification.status {
            Some(sid) => Some(crate::schema::statuses::dsl::statuses
                .filter(crate::schema::statuses::dsl::id.eq(sid))
                .get_result::<models::Status>(&c).with_expected_err(|| "Unable to get status")?),
            None => None
        };

        let is_followed = crate::schema::following::dsl::following
            .filter(crate::schema::following::dsl::follower.eq(notification.account))
            .filter(crate::schema::following::dsl::followee.eq(notification.cause))
            .count().get_result::<i64>(&c).with_expected_err(|| "Unable to check followed set")? > 0;
        let is_following = crate::schema::following::dsl::following
            .filter(crate::schema::following::dsl::followee.eq(notification.account))
            .filter(crate::schema::following::dsl::follower.eq(notification.cause))
            .count().get_result::<i64>(&c).with_expected_err(|| "Unable to check following set")? > 0;

        Ok((a, ca, s, is_followed, is_following))
    })?;

    let subscriptions = match notification.notification_type.as_str() {
        "follow" => {
            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::web_push_subscriptions::dsl::web_push_subscriptions
                    .filter(crate::schema::web_push_subscriptions::dsl::follow.eq(true))
                    .filter(crate::schema::web_push_subscriptions::dsl::account_id.eq(notification.account))
                    .get_results(&c).with_expected_err(|| "Unable to get subscriptions")
            })?
        }
        "favourite" => {
            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::web_push_subscriptions::dsl::web_push_subscriptions
                    .filter(crate::schema::web_push_subscriptions::dsl::favourite.eq(true))
                    .filter(crate::schema::web_push_subscriptions::dsl::account_id.eq(notification.account))
                    .get_results::<models::WebPushSubscription>(&c).with_expected_err(|| "Unable to get subscriptions")
            })?
        }
        "reblog" => {
            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::web_push_subscriptions::dsl::web_push_subscriptions
                    .filter(crate::schema::web_push_subscriptions::dsl::reblog.eq(true))
                    .filter(crate::schema::web_push_subscriptions::dsl::account_id.eq(notification.account))
                    .get_results::<models::WebPushSubscription>(&c).with_expected_err(|| "Unable to get subscriptions")
            })?
        }
        "mention" => {
            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::web_push_subscriptions::dsl::web_push_subscriptions
                    .filter(crate::schema::web_push_subscriptions::dsl::mention.eq(true))
                    .filter(crate::schema::web_push_subscriptions::dsl::account_id.eq(notification.account))
                    .get_results::<models::WebPushSubscription>(&c).with_expected_err(|| "Unable to get subscriptions")
            })?
        }
        _ => {
            warn!("Unknown notification type: {}", notification.notification_type);
            return Ok(());
        }
    }.into_iter().filter(|s| match s.policy.as_str() {
        "all" => true,
        "follower" => is_following,
        "followed" => is_followed,
        _ => false
    }).collect::<Vec<models::WebPushSubscription>>();

    let localizer = crate::i18n::Localizer::get_lang_opt(account.default_language.as_deref());

    let notification_data = match notification.notification_type.as_str() {
        "follow" => {
            NotificationData {
                notification_id: notification.iid,
                notification_type: "follow".to_string(),
                title: fl!(localizer, "follow-notification", name = account.display_name),
                icon: cause.avatar_file.as_ref().map(|f| format!("https://{}/media/{}", config.uri, f)),
                body: cause.bio,
                access_token: "".to_string(),
                preferred_locale: cause.default_language.clone().unwrap_or_else(|| "en".to_string()),
            }
        },
        "favourite" => {
            NotificationData {
                notification_id: notification.iid,
                notification_type: "favourite".to_string(),
                title: fl!(localizer, "favourite-notification", name = account.display_name),
                icon: cause.avatar_file.as_ref().map(|f| format!("https://{}/media/{}", config.uri, f)),
                body: status.as_ref().map(|s| s.text.clone()).unwrap_or_else(|| "".to_string()),
                access_token: "".to_string(),
                preferred_locale: cause.default_language.clone().unwrap_or_else(|| "en".to_string()),
            }
        }
        "reblog" => {
            NotificationData {
                notification_id: notification.iid,
                notification_type: "reblog".to_string(),
                title: fl!(localizer, "reblog-notification", name = account.display_name),
                icon: cause.avatar_file.as_ref().map(|f| format!("https://{}/media/{}", config.uri, f)),
                body: status.as_ref().map(|s| s.text.clone()).unwrap_or_else(|| "".to_string()),
                access_token: "".to_string(),
                preferred_locale: cause.default_language.clone().unwrap_or_else(|| "en".to_string()),
            }
        }
        "mention" => {
            NotificationData {
                notification_id: notification.iid,
                notification_type: "mention".to_string(),
                title: fl!(localizer, "mention-notification", name = account.display_name),
                icon: cause.avatar_file.as_ref().map(|f| format!("https://{}/media/{}", config.uri, f)),
                body: status.as_ref().map(|s| s.text.clone()).unwrap_or_else(|| "".to_string()),
                access_token: "".to_string(),
                preferred_locale: cause.default_language.clone().unwrap_or_else(|| "en".to_string()),
            }
        }
        _ => unreachable!()
    };

    for sub in subscriptions {
        config.celery.send_task(deliver_notification::new(Notification {
            data: notification_data.clone(),
            endpoint: sub.endpoint,
            p256dh: sub.p256dh,
            auth: sub.auth,
        }, sub.id)).await.with_expected_err(|| "Unable to submit notification delivery task")?;
    }

    Ok(())
}

#[celery::task]
pub async fn deliver_notification(notification: Notification, subscription_id: uuid::Uuid) -> TaskResult<()> {
    let config = super::config();

    let subscription = web_push_old::SubscriptionInfo::new(
        notification.endpoint,
        notification.p256dh,
        notification.auth,
    );
    let vapid_signature_builder = web_push_old::VapidSignatureBuilder::from_pem(
        config.vapid_key.as_slice(), &subscription
    ).with_expected_err(|| "Unable to create VAPID signature builder")?;
    let vapid_signature = vapid_signature_builder.build().with_expected_err(|| "Unable to build VAPID signature")?;

    let payload = serde_json::to_vec(&notification.data).unwrap();

    let mut builder = web_push_old::WebPushMessageBuilder::new(&subscription)
        .with_expected_err(|| "Unable to create WebPushMessageBuilder")?;
    builder.set_ttl(48 * 60 * 60); // 48 hours
    builder.set_vapid_signature(vapid_signature);
    builder.set_payload(web_push_old::ContentEncoding::AesGcm, &payload);

    let message = builder.build().with_unexpected_err(|| "Unable to build WebPushMessage")?;
    let req = build_request(message).with_unexpected_err(|| "Unable to build WebPushRequest")?;
    let res = crate::AS_CLIENT.execute(req).await.with_expected_err(|| "Unable to execute WebPushRequest")?;
    let status = res.status();
    if status.is_success() {
        return Ok(());
    }

    if status == reqwest::StatusCode::GONE || status == reqwest::StatusCode::NOT_FOUND
        || status == reqwest::StatusCode::FORBIDDEN {
        info!("Removing invalid subscription {}", subscription_id);
        tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = config.db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            diesel::delete(crate::schema::web_push_subscriptions::dsl::web_push_subscriptions
                .filter(crate::schema::web_push_subscriptions::dsl::id.eq(subscription_id))
            ).execute(&c).with_expected_err(|| "Unable to delete subscription")
        })?;
        return Ok(());
    }

    return Err(TaskError::ExpectedError(format!("Unable to send WebPushMessage: {}", status)));
}

fn build_request(message: web_push_old::WebPushMessage) -> reqwest::Result<reqwest::Request> {
    let mut builder = crate::AS_CLIENT.post(message.endpoint.to_string())
        .header("Urgency", "normal")
        .header("TTL", format!("{}", message.ttl).as_bytes());

    if let Some(payload) = message.payload {
        builder = builder
            .header("Content-Encoding", payload.content_encoding)
            .header(
                "Content-Length",
                format!("{}", payload.content.len() as u64).as_bytes(),
            )
            .header("Content-Type", "application/octet-stream");

        for (k, v) in payload.crypto_headers.into_iter() {
            let v: &str = v.as_ref();
            builder = builder.header(k, v);
        }

        builder = builder.body(payload.content)
    } else {
        builder = builder.body("");
    }

    builder.build()
}