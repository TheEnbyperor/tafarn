use diesel::prelude::*;
use chrono::prelude::*;
use futures::StreamExt;
use crate::models;

#[get("/api/v1/favourites?<limit>&<min_id>&<max_id>")]
pub async fn favourites(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    limit: Option<u64>, max_id: Option<i64>, min_id: Option<i64>,
    host: &rocket::http::uri::Host<'_>, localizer: crate::i18n::Localizer
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Status>>>, super::Error> {
    if !user.has_scope("read:favourites") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let limit = limit.unwrap_or(20);
    if limit > 500 {
        return Err( super::Error {
            code: rocket::http::Status::UnprocessableEntity,
            error: fl!(localizer, "limit-too-large")
        });
    }

    let account = super::accounts::get_account(&db, &localizer, &user).await?;

    let statuses: Vec<(models::Like, models::Status)> =
        crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
            let mut sel = crate::schema::likes::dsl::likes.order_by(
                crate::schema::likes::dsl::iid.desc()
            ).filter(
                crate::schema::likes::dsl::account.eq(&account.id)
            ).inner_join(
                crate::schema::statuses::table.on(
                    crate::schema::likes::dsl::status.eq(crate::schema::statuses::dsl::id.nullable())
                )
            ).filter(
                crate::schema::statuses::dsl::deleted_at.is_null()
            ).filter(
                crate::schema::statuses::dsl::boost_of_url.is_null()
            ).limit(limit as i64).into_boxed();
            if let Some(min_id) = min_id {
                sel = sel.filter(crate::schema::likes::dsl::iid.gt(min_id));
            }
            if let Some(max_id) = max_id {
                sel = sel.filter(crate::schema::likes::dsl::iid.lt(max_id));
            }
            sel.get_results(c)
        }).await?;

    let mut links = vec![];

    if let Some(last_id) = statuses.last().map(|a| a.0.iid) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/favourites?max_id={}", host.to_string(), last_id)
        });
    }
    if let Some(first_id) = statuses.first().map(|a| a.0.iid) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/favourites?min_id={}", host.to_string(), first_id)
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

#[post("/api/v1/statuses/<status_id>/favourite")]
pub async fn like_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    status_id: String, celery: &rocket::State<crate::CeleryApp>, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::Status>, super::Error> {
    if !user.has_scope("write:favourites") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let account = super::accounts::get_account(&db, &localizer, &user).await?;
    let status = super::statuses::get_status_and_check_visibility(&status_id, Some(&account), &db, &localizer).await?;

    if crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
        crate::schema::likes::dsl::likes.filter(
            crate::schema::likes::dsl::status.eq(status.id)
        ).filter(
            crate::schema::likes::dsl::account.eq(&account.id)
        ).count().get_result::<i64>(c)
    }).await? > 0 {
        return Ok(rocket::serde::json::Json(super::statuses::render_status(config, &db, status, &localizer, Some(&account)).await?));
    }

    let new_like = models::NewLike {
        id: uuid::Uuid::new_v4(),
        account: account.id,
        status: Some(status.id),
        status_url: None,
        created_at: Utc::now().naive_utc(),
        local: true,
        url: None,
    };
    let like = crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
        diesel::insert_into(crate::schema::likes::dsl::likes)
            .values(new_like)
            .get_result::<models::Like>(c)
    }).await?;

    match celery.send_task(
        super::super::tasks::statuses::deliver_like::new(like, status.clone(), account.clone())
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(super::Error {
                code: rocket::http::Status::InternalServerError,
                error: fl!(localizer, "internal-server-error")
            });
        }
    };

    Ok(rocket::serde::json::Json(super::statuses::render_status(config, &db, status, &localizer, Some(&account)).await?))
}

#[post("/api/v1/statuses/<status_id>/unfavourite")]
pub async fn unlike_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    status_id: String, celery: &rocket::State<crate::CeleryApp>, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::Status>, super::Error> {
    if !user.has_scope("write:favourites") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let account = super::accounts::get_account(&db, &localizer, &user).await?;
    let status = super::statuses::get_status_and_check_visibility(&status_id, Some(&account), &db, &localizer).await?;

    if let Some(like) = crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
        crate::schema::likes::dsl::likes.filter(
            crate::schema::likes::dsl::status.eq(status.id)
        ).filter(
            crate::schema::likes::dsl::account.eq(&account.id)
        ).get_result::<models::Like>(c).optional()
    }).await? {
        let like_id = like.id;
        crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
            diesel::delete(crate::schema::likes::dsl::likes.find(like_id))
                .execute(c)
        }).await?;

        match celery.send_task(
            super::super::tasks::statuses::deliver_undo_like::new(like, status.clone(), account.clone())
        ).await {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to submit celery task: {:?}", err);
                return Err(super::Error {
                    code: rocket::http::Status::InternalServerError,
                    error: fl!(localizer, "internal-server-error")
                });
            }
        };
    }

    Ok(rocket::serde::json::Json(super::statuses::render_status(config, &db, status, &localizer, Some(&account)).await?))
}
