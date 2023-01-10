use diesel::prelude::*;
use futures::StreamExt;
use crate::models;

#[get("/api/v1/bookmarks?<limit>&<min_id>&<max_id>")]
pub async fn bookmarks(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    limit: Option<u64>, max_id: Option<i32>, min_id: Option<i32>,
    host: &rocket::http::uri::Host<'_>,
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Status>>>, rocket::http::Status> {
    if !user.has_scope("read:bookmarks") {
        return Err(rocket::http::Status::Forbidden);
    }

    let limit = limit.unwrap_or(20);
    if limit > 500 {
        return Err(rocket::http::Status::BadRequest);
    }

    let account = super::accounts::get_account(&db, &user).await?;

    let statuses: Vec<(models::Bookmark, models::Status)> =
        crate::db_run(&db, move |c| -> QueryResult<_> {
            let mut sel = crate::schema::bookmarks::dsl::bookmarks.order_by(
                crate::schema::bookmarks::dsl::iid.desc()
            ).filter(
                crate::schema::bookmarks::dsl::account.eq(&account.id)
            ).inner_join(
                crate::schema::statuses::table.on(
                    crate::schema::bookmarks::dsl::status.eq(crate::schema::statuses::dsl::id)
                )
            ).filter(
                crate::schema::statuses::dsl::deleted_at.is_null()
            ).filter(
                crate::schema::statuses::dsl::boost_of_url.is_null()
            ).limit(limit as i64).into_boxed();
            if let Some(min_id) = min_id {
                sel = sel.filter(crate::schema::bookmarks::dsl::iid.gt(min_id));
            }
            if let Some(max_id) = max_id {
                sel = sel.filter(crate::schema::bookmarks::dsl::iid.lt(max_id));
            }
            sel.get_results(c)
        }).await?;

    let mut links = vec![];

    if let Some(last_id) = statuses.last().map(|a| a.0.iid) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/bookmarks?max_id={}", host.to_string(), last_id)
        });
    }
    if let Some(first_id) = statuses.first().map(|a| a.0.iid) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/bookmarks?min_id={}", host.to_string(), first_id)
        });
    }

    Ok(super::LinkedResponse {
        inner: rocket::serde::json::Json(
            futures::stream::iter(statuses).map(|status| {
                super::statuses::render_status(config, &db, status.1, Some(&account))
            }).buffered(10).collect::<Vec<_>>().await
                .into_iter().collect::<Result<Vec<_>, _>>()?
        ),
        links
    })
}

#[post("/api/v1/statuses/<status_id>/bookmark")]
pub async fn bookmark_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    status_id: String
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if !user.has_scope("write:bookmarks") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let status = super::statuses::get_status_and_check_visibility(&status_id, Some(&account), &db).await?;

    if crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::bookmarks::dsl::bookmarks.filter(
            crate::schema::bookmarks::dsl::status.eq(status.id)
        ).filter(
            crate::schema::bookmarks::dsl::account.eq(&account.id)
        ).count().get_result::<i64>(c)
    }).await? > 0 {
        return Ok(rocket::serde::json::Json(super::statuses::render_status(config, &db, status, Some(&account)).await?));
    }

    let new_bookmark = models::NewBookmark {
        id: uuid::Uuid::new_v4(),
        account: account.id,
        status: status.id,
    };
    crate::db_run(&db, move |c| -> QueryResult<_> {
        diesel::insert_into(crate::schema::bookmarks::dsl::bookmarks)
            .values(new_bookmark)
            .execute(c)
    }).await?;

    Ok(rocket::serde::json::Json(super::statuses::render_status(config, &db, status, Some(&account)).await?))
}

#[post("/api/v1/statuses/<status_id>/unbookmark")]
pub async fn unbookmark_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    status_id: String,
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if !user.has_scope("write:bookmarks") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let status = super::statuses::get_status_and_check_visibility(&status_id, Some(&account), &db).await?;

    crate::db_run(&db, move |c| -> QueryResult<_> {
        diesel::delete(crate::schema::bookmarks::dsl::bookmarks.filter(
            crate::schema::bookmarks::dsl::status.eq(status.id)
        ).filter(
            crate::schema::bookmarks::dsl::account.eq(&account.id)
        )).execute(c)
    }).await?;

    Ok(rocket::serde::json::Json(super::statuses::render_status(config, &db, status, Some(&account)).await?))
}
