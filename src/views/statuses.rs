use diesel::prelude::*;
use chrono::prelude::*;
use futures::StreamExt;

#[async_recursion::async_recursion]
pub async fn render_status(
    config: &crate::AppConfig, db: &crate::DbConn, status: crate::models::Status,
    req_account: Option<&'async_recursion crate::models::Account>
) -> Result<super::objs::Status, rocket::http::Status> {
    let visibility = if status.public {
        super::objs::StatusVisibility::Public
    } else if status.visible {
        super::objs::StatusVisibility::Unlisted
    } else {
        if crate::db_run(db, move |c| -> QueryResult<_> {
            crate::schema::status_audiences::dsl::status_audiences.filter(
                crate::schema::status_audiences::dsl::status_id.eq(status.id)
            ).filter(
                crate::schema::status_audiences::dsl::account_followers.eq(status.account_id)
            ).count().get_result::<i64>(c)
        }).await? > 0 {
            super::objs::StatusVisibility::Private
        } else {
            super::objs::StatusVisibility::Direct
        }
    };

    let account = crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::accounts::dsl::accounts.find(status.account_id)
            .get_result::<crate::models::Account>(c)
    }).await?;

    let in_reply_to = match status.in_reply_to_id {
        Some(id) => Some(crate::db_run(db, move |c| -> QueryResult<_> {
            crate::schema::statuses::dsl::statuses.find(id).inner_join(
                crate::schema::accounts::table.on(
                    crate::schema::accounts::dsl::id.eq(crate::schema::statuses::dsl::account_id)
                )
            )
                .get_result::<(crate::models::Status, crate::models::Account)>(c)
        }).await?),
        None => None
    };

    let boost = match status.boot_of_id {
        Some(id) => Some(crate::db_run(db, move |c| -> QueryResult<_> {
            crate::schema::statuses::dsl::statuses.find(id)
                .get_result::<crate::models::Status>(c)
        }).await?),
        None => None
    };

    Ok(super::objs::Status {
        id: status.iid.to_string(),
        uri: status.url,
        created_at: Utc.from_utc_datetime(&status.created_at),
        account: super::accounts::render_account(config, db, account).await?,
        content: status.text,
        visibility,
        sensitive: status.sensitive,
        spoiler_text: status.spoiler_text,
        media_attachments: vec![],
        mentions: vec![],
        tags: vec![],
        emojis: vec![],
        reblogs_count: 0,
        favourites_count: 0,
        replies_count: 0,
        url: status.uri,
        in_reply_to_id: in_reply_to.as_ref().map(|x| x.0.iid.to_string()),
        in_reply_to_account_id: in_reply_to.as_ref().map(|x| x.1.iid.to_string()),
        reblog: match boost {
            Some(boost) => Some(Box::new(render_status(config, db, boost, req_account).await?)),
            None => None
        },
        poll: None,
        card: None,
        language: status.language,
        edited_at: status.edited_at.map(|x| Utc.from_utc_datetime(&x)),
        favourited: req_account.map(|_| false),
        reblogged: req_account.map(|_| false),
        muted: req_account.map(|_| false),
        bookmarked: req_account.map(|_| false),
        pinned: req_account.map(|_| false),
    })
}

async fn get_status_and_check_visibility(
    status_id: String, _account: Option<&crate::models::Account>,
    db: &crate::DbConn,
) -> Result<crate::models::Status, rocket::http::Status> {
    let status_id = match status_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let status = match crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::iid.eq(status_id)
        )
            .get_result::<crate::models::Status>(c).optional()
    }).await? {
        Some(m) => m,
        None => return Err(rocket::http::Status::NotFound)
    };

    if !status.visible {
        return Err(rocket::http::Status::NotFound);
    }

    Ok(status)
}

#[get("/api/v1/statuses/<status_id>")]
pub async fn get_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>,
    user: Option<super::oauth::TokenClaims>, status_id: String
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    let account = match &user {
        Some(u) => Some(super::accounts::get_account(&db, u).await?),
        None => None
    };

    let status = get_status_and_check_visibility(status_id, account.as_ref(), &db).await?;

    Ok(rocket::serde::json::Json(render_status(config, &db, status, account.as_ref()).await?))
}


#[get("/api/v1/statuses/<status_id>/context")]
pub async fn status_context(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>,
    user: Option<super::oauth::TokenClaims>, status_id: String
) -> Result<rocket::serde::json::Json<super::objs::Context>, rocket::http::Status> {
    let account = match &user {
        Some(u) => Some(super::accounts::get_account(&db, u).await?),
        None => None
    };

    let status = get_status_and_check_visibility(status_id, account.as_ref(), &db).await?;

    let (ancestors, descendants) = crate::db_run(&db, move |c| -> QueryResult<_> {
        let mut descendants = vec![];
        let mut ancestors = vec![];

        let mut in_reply_to = status.in_reply_to_id;
        while let Some(irt) = in_reply_to {
            let irt_status = crate::schema::statuses::dsl::statuses.find(irt)
                    .get_result::<crate::models::Status>(c)?;
            in_reply_to = irt_status.in_reply_to_id;
            ancestors.push(irt_status);
        }

        let mut descendant_ids = vec![status.id];
        while let Some(did) = descendant_ids.pop() {
            let de_statuses = crate::schema::statuses::dsl::statuses.filter(
                    crate::schema::statuses::in_reply_to_id.eq(did)
                ).get_results::<crate::models::Status>(c)?;
            for de_status in de_statuses {
                descendant_ids.push(de_status.id);
                descendants.push(de_status);
            }
        }
        Ok((ancestors, descendants))
    }).await?;

    Ok(rocket::serde::json::Json(super::objs::Context {
        ancestors: futures::stream::iter(ancestors).map(|status| {
            render_status(config, &db, status, account.as_ref())
        }).buffer_unordered(10).collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>, _>>()?,
        descendants: futures::stream::iter(descendants).map(|status| {
            render_status(config, &db, status, account.as_ref())
        }).buffer_unordered(10).collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>, _>>()?,
    }))
}