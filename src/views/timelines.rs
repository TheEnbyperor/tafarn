use diesel::prelude::*;
use futures::StreamExt;

#[get("/api/v1/timelines/home?<max_id>&<since_id>&<min_id>&<limit>")]
pub async fn timeline_home(
    config: &rocket::State<crate::AppConfig>, db: crate::DbConn, user: super::oauth::TokenClaims,
    max_id: Option<i32>, since_id: Option<i32>, min_id: Option<i32>,
    limit: Option<u64>, host: &rocket::http::uri::Host<'_>, localizer: crate::i18n::Localizer
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Status>>>, super::Error> {
    if !user.has_scope("read:statuses") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let limit = limit.unwrap_or(20);
    if limit > 500 {
        return Err(super::Error {
            code: rocket::http::Status::BadRequest,
            error: fl!(localizer, "limit-too-large")
        });
    }

    let account = super::accounts::get_account(&db, &localizer, &user).await?;

    let statuses: Vec<(crate::models::HomeTimelineEntry, crate::models::Status)> =
        crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
            let mut sel = crate::schema::home_timeline::dsl::home_timeline.filter(
                crate::schema::home_timeline::dsl::account_id.eq(&account.id)
            ).filter(
                crate::schema::statuses::dsl::deleted_at.is_null()
            ).filter(
                crate::schema::statuses::dsl::boost_of_url.is_null()
            ).order_by(
                crate::schema::home_timeline::dsl::id.desc()
            ).limit(limit as i64).inner_join(crate::schema::statuses::table.on(
                crate::schema::statuses::dsl::id.eq(crate::schema::home_timeline::dsl::status_id)
            )).into_boxed();
            if let Some(min_id) = min_id {
                sel = sel.filter(crate::schema::home_timeline::dsl::id.gt(min_id));
            }
            if let Some(max_id) = max_id {
                sel = sel.filter(crate::schema::home_timeline::dsl::id.lt(max_id));
            }
            if let Some(since_id) = since_id {
                sel = sel.filter(crate::schema::home_timeline::dsl::id.gt(since_id));
            }
            sel.get_results(c)
        }).await?;

    let mut links = vec![];

    if let Some(last_id) = statuses.last().map(|a| a.0.id) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/timelines/home?max_id={}", host.to_string(), last_id)
        });
    }
    if let Some(first_id) = statuses.first().map(|a| a.0.id) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/timelines/home?min_id={}", host.to_string(), first_id)
        });
    }

    Ok(super::LinkedResponse {
        inner: rocket::serde::json::Json(
            futures::stream::iter(statuses).map(|status| {
                super::statuses::render_status(config, &db, status.1, &localizer, Some(&account))
            }).buffered(10).collect::<Vec<_>>().await
                .into_iter().collect::<Result<Vec<_>, _>>()?
        ),
        links
    })
}

#[get("/api/v1/timelines/tag/<hashtag>?<any>&<all>&<none>&<local>&<remote>&<only_media>&<max_id>&<since_id>&<min_id>&<limit>")]
pub async fn timeline_hashtag(
    config: &rocket::State<crate::AppConfig>, hashtag: &str, any: Option<Vec<&str>>, all: Option<Vec<&str>>,
    none: Option<Vec<&str>>, local: Option<&str>, remote: Option<&str>,
    only_media: Option<&str>, max_id: Option<String>, since_id: Option<i32>,
    min_id: Option<String>, limit: Option<u64>, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<Vec<super::objs::Status>>, super::Error> {
    let _local = super::parse_bool(local, false, &localizer)?;
    let _remote = super::parse_bool(remote, false, &localizer)?;
    let _only_media = super::parse_bool(only_media, false, &localizer)?;

    Ok(rocket::serde::json::Json(vec![]))
}

#[get("/api/v1/timelines/public?<local>&<remote>&<only_media>&<max_id>&<since_id>&<min_id>&<limit>")]
pub async fn timeline_public(
    config: &rocket::State<crate::AppConfig>, db: crate::DbConn, user: Option<super::oauth::TokenClaims>,
    local: Option<&str>, remote: Option<&str>, only_media: Option<&str>,
    max_id: Option<i32>, since_id: Option<i32>, min_id: Option<i32>, limit: Option<u64>,
    host: &rocket::http::uri::Host<'_>, localizer: crate::i18n::Localizer
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Status>>>, super::Error> {
    let local = super::parse_bool(local, false, &localizer)?;
    let remote = super::parse_bool(remote, false, &localizer)?;
    let _only_media = super::parse_bool(only_media, false, &localizer)?;

    if let Some(user) = &user {
        if !user.has_scope("read:statuses") {
            return Err(super::Error {
                code: rocket::http::Status::Forbidden,
                error: fl!(localizer, "error-no-permission")
            });
        }
    }

    let limit = limit.unwrap_or(20);
    if limit > 500 {
        return Err(super::Error {
            code: rocket::http::Status::BadRequest,
            error: fl!(localizer, "limit-too-large")
        });
    }

    let account = match &user {
        Some(u) => Some(super::accounts::get_account(&db, &localizer, u).await?),
        None => None
    };

    let statuses: Vec<(crate::models::PublicTimelineEntry, crate::models::Status)> =
        crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
            let mut sel = crate::schema::public_timeline::dsl::public_timeline.order_by(
                crate::schema::public_timeline::dsl::id.desc()
            ).filter(
                crate::schema::statuses::dsl::deleted_at.is_null()
            ).filter(
                crate::schema::statuses::dsl::boost_of_url.is_null()
            ).limit(limit as i64).inner_join(crate::schema::statuses::table.on(
                crate::schema::statuses::dsl::id.eq(crate::schema::public_timeline::dsl::status_id)
            )).into_boxed();
            if let Some(min_id) = min_id {
                sel = sel.filter(crate::schema::public_timeline::dsl::id.gt(min_id));
            }
            if let Some(max_id) = max_id {
                sel = sel.filter(crate::schema::public_timeline::dsl::id.lt(max_id));
            }
            if let Some(since_id) = since_id {
                sel = sel.filter(crate::schema::public_timeline::dsl::id.gt(since_id));
            }
            if local {
                sel = sel.filter(crate::schema::statuses::dsl::local.eq(true));
            }
            if remote {
                sel = sel.filter(crate::schema::statuses::dsl::local.eq(false));
            }
            sel.get_results(c)
        }).await?;

    let mut links = vec![];

    if let Some(last_id) = statuses.last().map(|a| a.0.id) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/timelines/public?max_id={}", host.to_string(), last_id)
        });
    }
    if let Some(first_id) = statuses.first().map(|a| a.0.id) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/timelines/public?min_id={}", host.to_string(), first_id)
        });
    }

    Ok(super::LinkedResponse {
        inner: rocket::serde::json::Json(
            futures::stream::iter(statuses).map(|status| {
                super::statuses::render_status(config, &db, status.1, &localizer, account.as_ref())
            }).buffered(10).collect::<Vec<_>>().await
                .into_iter().collect::<Result<Vec<_>, _>>()?
        ),
        links
    })
}