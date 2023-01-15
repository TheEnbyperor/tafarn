use diesel::prelude::*;
use chrono::prelude::*;
use futures::StreamExt;
use crate::models;

#[async_recursion::async_recursion]
pub async fn render_status(
    config: &crate::AppConfig, db: &crate::DbConn, status: models::Status,
    req_account: Option<&'async_recursion models::Account>,
) -> Result<super::objs::Status, rocket::http::Status> {
    let req_account_id = req_account.map(|a| a.id);

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
            .get_result::<models::Account>(c)
    }).await?;

    let in_reply_to = match status.in_reply_to_id {
        Some(id) => Some(crate::db_run(db, move |c| -> QueryResult<_> {
            crate::schema::statuses::dsl::statuses.find(id).inner_join(
                crate::schema::accounts::table.on(
                    crate::schema::accounts::dsl::id.eq(crate::schema::statuses::dsl::account_id)
                )
            )
                .get_result::<(models::Status, models::Account)>(c)
        }).await?),
        None => None
    };

    let boost = match status.boost_of_id {
        Some(id) => Some(crate::db_run(db, move |c| -> QueryResult<_> {
            crate::schema::statuses::dsl::statuses.find(id)
                .get_result::<models::Status>(c)
        }).await?),
        None => None
    };

    let (like_count, boost_count, replies_count) = crate::db_run(&db, move |c| -> QueryResult<_> {
        let lc = crate::schema::likes::dsl::likes.filter(
            crate::schema::likes::dsl::status.eq(status.id)
        ).count().get_result::<i64>(c)?;
        let bc = crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::boost_of_id.eq(status.id)
        ).filter(
            crate::schema::statuses::dsl::deleted_at.is_null()
        ).count().get_result::<i64>(c)?;
        let rc = crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::in_reply_to_id.eq(status.id)
        ).filter(
            crate::schema::statuses::dsl::deleted_at.is_null()
        ).count().get_result::<i64>(c)?;
        Ok((lc, bc, rc))
    }).await?;

    let media_attachments: Vec<(models::MediaAttachment, models::Media)> =
        crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::media_attachments::dsl::media_attachments.filter(
            crate::schema::media_attachments::dsl::status.eq(status.id)
        ).inner_join(
            crate::schema::media::table.on(
                crate::schema::media::dsl::id.eq(crate::schema::media_attachments::dsl::media)
            )
        ).get_results(c)
    }).await?;

    let boosted = match req_account_id {
        Some(account) => {
            if status.account_id == account && status.boost_of_id.is_some() {
                Some(true)
            } else {
                Some(crate::db_run(db, move |c| -> QueryResult<_> {
                    crate::schema::statuses::dsl::statuses.filter(
                        crate::schema::statuses::dsl::boost_of_id.eq(status.id)
                    ).filter(
                        crate::schema::statuses::dsl::account_id.eq(account)
                    ).filter(
                        crate::schema::statuses::dsl::deleted_at.is_null()
                    ).count().get_result::<i64>(c)
                }).await? > 0)
            }
        },
        None => None
    };
    let liked = match req_account_id {
        Some(account) => {
            Some(crate::db_run(&db, move |c| -> QueryResult<_> {
                crate::schema::likes::dsl::likes.filter(
                    crate::schema::likes::dsl::status.eq(status.id)
                ).filter(
                    crate::schema::likes::dsl::account.eq(account)
                ).count().get_result::<i64>(c)
            }).await? > 0)
        },
        None => None
    };
    let bookmarked = match req_account_id {
        Some(account) => {
            Some(crate::db_run(&db, move |c| -> QueryResult<_> {
                crate::schema::bookmarks::dsl::bookmarks.filter(
                    crate::schema::bookmarks::dsl::status.eq(status.id)
                ).filter(
                    crate::schema::bookmarks::dsl::account.eq(account)
                ).count().get_result::<i64>(c)
            }).await? > 0)
        },
        None => None
    };
    let pinned = match req_account_id {
        Some(account) => {
            Some(crate::db_run(&db, move |c| -> QueryResult<_> {
                crate::schema::pins::dsl::pins.filter(
                    crate::schema::pins::dsl::status.eq(status.id)
                ).filter(
                    crate::schema::pins::dsl::account.eq(account)
                ).count().get_result::<i64>(c)
            }).await? > 0)
        },
        None => None
    };

    Ok(super::objs::Status {
        id: status.iid.to_string(),
        uri: status.url(&config.uri),
        created_at: Utc.from_utc_datetime(&status.created_at),
        account: super::accounts::render_account(config, db, account).await?,
        content: status.text,
        visibility,
        sensitive: status.sensitive,
        spoiler_text: status.spoiler_text,
        media_attachments: media_attachments.into_iter()
            .map(|(_, m)| super::media::render_media_attachment(m, config)).collect::<Result<Vec<_>, _>>()?,
        mentions: vec![],
        tags: vec![],
        emojis: vec![],
        reblogs_count: boost_count as u64,
        favourites_count: like_count as u64,
        replies_count: replies_count as u64,
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
        favourited: liked,
        reblogged: boosted,
        muted: req_account.map(|_| false),
        bookmarked,
        pinned,
    })
}

pub async fn can_view(
    status: &models::Status, account: Option<&models::Account>, db: &crate::DbConn
) -> Result<bool, rocket::http::Status> {
    if status.visible {
        Ok(true)
    } else {
        if let Some(account) = account {
            if account.id == status.account_id {
                return Ok(true);
            }

            let account_id = account.id;
            let status_id = status.id;
            if crate::db_run(db, move |c| -> QueryResult<_> {
                crate::schema::status_audiences::dsl::status_audiences.filter(
                    crate::schema::status_audiences::dsl::status_id.eq(&status_id)
                ).filter(
                    crate::schema::status_audiences::dsl::account.eq(&account_id)
                ).count().get_result::<i64>(c)
            }).await? > 0 {
                return Ok(true);
            }

            if crate::db_run(db, move |c| -> QueryResult<_> {
                crate::schema::status_audiences::dsl::status_audiences.filter(
                    crate::schema::status_audiences::dsl::status_id.eq(&status_id)
                ).inner_join(crate::schema::following::table.on(
                    crate::schema::status_audiences::dsl::account_followers.eq(
                        crate::schema::following::dsl::followee.nullable()
                    )
                )).filter(
                    crate::schema::following::dsl::follower.eq(&account_id)
                ).count().get_result::<i64>(c)
            }).await? > 0 {
                return Ok(true);
            }

            Ok(false)
        } else {
            Ok(false)
        }
    }
}

pub async fn get_status_and_check_visibility(
    status_id: &str, account: Option<&models::Account>,
    db: &crate::DbConn,
) -> Result<crate::models::Status, rocket::http::Status> {
    let status_id = match status_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let status = match crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::iid.eq(status_id)
        ).get_result::<models::Status>(c).optional()
    }).await? {
        Some(m) => m,
        None => return Err(rocket::http::Status::NotFound)
    };

    if status.deleted_at.is_some() {
        return Err(rocket::http::Status::Gone);
    }

    if can_view(&status, account, db).await? {
        Ok(status)
    } else {
        Err(rocket::http::Status::NotFound)
    }
}


#[derive(FromForm)]
pub struct StatusForm<'a> {
    status: Option<&'a str>,
    media_ids: Option<Vec<&'a str>>,
    in_reply_to_id: Option<&'a str>,
    sensitive: Option<&'a str>,
    spoiler_text: Option<&'a str>,
    language: Option<&'a str>,
    visibility: Option<&'a str>,
}

#[derive(Deserialize)]
pub struct StatusJson<'a> {
    #[serde(default)]
    status: Option<&'a str>,
    #[serde(default)]
    media_ids: Option<Vec<&'a str>>,
    #[serde(default)]
    in_reply_to_id: Option<&'a str>,
    #[serde(default)]
    sensitive: Option<bool>,
    #[serde(default)]
    spoiler_text: Option<&'a str>,
    #[serde(default)]
    language: Option<&'a str>,
    #[serde(default)]
    visibility: Option<&'a str>,
}

#[derive(Debug)]
pub struct CreateStatus<'a> {
    status: Option<&'a str>,
    media_ids: Vec<uuid::Uuid>,
    in_reply_to_id: Option<i32>,
    sensitive: Option<bool>,
    spoiler_text: Option<&'a str>,
    language: Option<&'a str>,
    visibility: super::objs::StatusVisibility,
}

impl<'a> TryFrom<StatusForm<'a>> for CreateStatus<'a> {
    type Error = rocket::http::Status;

    fn try_from(value: StatusForm<'a>) -> Result<Self, Self::Error> {
        Ok(CreateStatus {
            status: value.status,
            media_ids: value.media_ids.iter().flatten()
                .map(|x| uuid::Uuid::parse_str(x))
                .collect::<Result<Vec<_>, _>>().map_err(|_| rocket::http::Status::UnprocessableEntity)?,
            in_reply_to_id: value.in_reply_to_id.map(|x| x.parse::<i32>())
                .transpose().map_err(|_| rocket::http::Status::UnprocessableEntity)?,
            sensitive: if value.sensitive.is_some() {
                Some(super::parse_bool(value.sensitive, false)?)
            } else {
                None
            },
            spoiler_text: value.spoiler_text,
            language: value.language,
            visibility: match value.visibility {
                Some(v) => match super::objs::StatusVisibility::from_str(v) {
                    Some(v) => v,
                    None => return Err(rocket::http::Status::UnprocessableEntity)
                },
                None => super::objs::StatusVisibility::Public
            }
        })
    }
}

impl<'a> TryFrom<StatusJson<'a>> for CreateStatus<'a> {
    type Error = rocket::http::Status;

    fn try_from(value: StatusJson<'a>) -> Result<Self, Self::Error> {
        Ok(CreateStatus {
            status: value.status,
            media_ids: value.media_ids.iter().flatten()
                .map(|x| uuid::Uuid::parse_str(x))
                .collect::<Result<Vec<_>, _>>().map_err(|_| rocket::http::Status::UnprocessableEntity)?,
            in_reply_to_id: value.in_reply_to_id.map(|x| x.parse::<i32>())
                .transpose().map_err(|_| rocket::http::Status::UnprocessableEntity)?,
            sensitive: value.sensitive,
            spoiler_text: value.spoiler_text,
            language: value.language,
            visibility: match value.visibility {
                Some(v) => match super::objs::StatusVisibility::from_str(v) {
                    Some(v) => v,
                    None => return Err(rocket::http::Status::UnprocessableEntity)
                },
                None => super::objs::StatusVisibility::Public
            }
        })
    }
}

#[post("/api/v1/statuses", data = "<form>", rank = 1)]
pub async fn create_status_form(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    form: rocket::form::Form<StatusForm<'_>>, celery: &rocket::State<crate::CeleryApp>
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    _create_status(db, config, user, form.into_inner().try_into()?, celery).await
}

#[post("/api/v1/statuses", data = "<form>", rank = 2)]
pub async fn create_status_json(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    form: rocket::serde::json::Json<StatusJson<'_>>, celery: &rocket::State<crate::CeleryApp>
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    _create_status(db, config, user, form.into_inner().try_into()?, celery).await
}

pub async fn _create_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    form: CreateStatus<'_>, celery: &rocket::State<crate::CeleryApp>
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if !user.has_scope("write:statuses") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let status_source = form.status.unwrap_or("");
    let status_text = comrak::markdown_to_html(status_source, &crate::COMRAK_OPTIONS);
    let language = form.language.or(account.default_language.as_deref())
        .map(|x| x.to_string());
    let in_reply_to = match form.in_reply_to_id {
        Some(id) => match crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::statuses::dsl::statuses
                .filter(crate::schema::statuses::dsl::iid.eq(id))
                .get_result::<models::Status>(c).optional()
        }).await? {
            Some(s) => Some(s),
            None => return Err(rocket::http::Status::UnprocessableEntity)
        },
        None => None
    };

    let mut media = vec![];
    for id in form.media_ids {
        match crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::media::dsl::media
                .filter(crate::schema::media::dsl::id.eq(id))
                .get_result::<models::Media>(c).optional()
        }).await? {
            Some(m) => {
                if m.owned_by.as_deref() != Some(&user.subject) {
                    return Err(rocket::http::Status::Forbidden);
                }
                media.push(m);
            },
            None => return Err(rocket::http::Status::UnprocessableEntity)
        }
    }

    if status_text.is_empty() && media.is_empty() {
        return Err(rocket::http::Status::UnprocessableEntity);
    }

    let mut new_status_audiences = vec![];
    let new_status = models::NewStatus {
        id: uuid::Uuid::new_v4(),
        url: "".to_string(),
        uri: None,
        text: status_text,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
        in_reply_to_id: in_reply_to.map(|x| x.id),
        in_reply_to_url: None,
        boost_of_url: None,
        boost_of_id: None,
        sensitive: form.sensitive.or(account.default_sensitive).unwrap_or(false),
        spoiler_text: form.spoiler_text.unwrap_or_default().to_string(),
        language,
        local: true,
        account_id: account.id,
        deleted_at: None,
        edited_at: None,
        public: form.visibility == super::objs::StatusVisibility::Public,
        visible: form.visibility == super::objs::StatusVisibility::Public ||
            form.visibility == super::objs::StatusVisibility::Unlisted,
        text_source: Some(status_source.to_string()),
        spoiler_text_source: Some(form.spoiler_text.unwrap_or_default().to_string()),
    };
    if form.visibility == super::objs::StatusVisibility::Public ||
        form.visibility == super::objs::StatusVisibility::Unlisted ||
        form.visibility == super::objs::StatusVisibility::Private {
        new_status_audiences.push(models::StatusAudience {
            id: uuid::Uuid::new_v4(),
            status_id: new_status.id,
            mention: false,
            account: None,
            account_followers: Some(account.id)
        });
    }

    let new_status_media = media.iter().map(|m| models::MediaAttachment {
        status: new_status.id,
        media: m.id
    }).collect::<Vec<_>>();

    let s = crate::db_run(&db, move |c| -> QueryResult<_> {
        c.transaction::<_, diesel::result::Error, _>(|| {
            let s = diesel::insert_into(crate::schema::statuses::dsl::statuses)
                .values(new_status)
                .get_result::<models::Status>(c)?;
            diesel::insert_into(crate::schema::status_audiences::dsl::status_audiences)
                .values(new_status_audiences)
                .execute(c)?;
            diesel::insert_into(crate::schema::media_attachments::dsl::media_attachments)
                .values(new_status_media)
                .execute(c)?;
            Ok(s)
        })
    }).await?;

    match celery.send_task(
        super::super::tasks::statuses::deliver_status::new(s.clone(), account.clone())
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    Ok(rocket::serde::json::Json(render_status(config, &db, s, Some(&account)).await?))
}

#[get("/api/v1/statuses/<status_id>")]
pub async fn get_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>,
    user: Option<super::oauth::TokenClaims>, status_id: String,
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if let Some(user) = &user {
        if !user.has_scope("read:statuses") {
            return Err(rocket::http::Status::Forbidden);
        }
    }

    let account = match &user {
        Some(u) => Some(super::accounts::get_account(&db, u).await?),
        None => None
    };

    let status = get_status_and_check_visibility(&status_id, account.as_ref(), &db).await?;

    Ok(rocket::serde::json::Json(render_status(config, &db, status, account.as_ref()).await?))
}

#[delete("/api/v1/statuses/<status_id>")]
pub async fn delete_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    celery: &rocket::State<crate::CeleryApp>, status_id: String,
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if !user.has_scope("write:statuses") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let status = get_status_and_check_visibility(&status_id, Some(&account), &db).await?;

    if status.account_id != account.id {
        return Err(rocket::http::Status::Forbidden);
    }

    let status = crate::db_run(&db, move |c| -> QueryResult<_> {
        diesel::update(&status)
            .set(crate::schema::statuses::dsl::deleted_at.eq(Utc::now().naive_utc()))
            .get_result::<models::Status>(c)
    }).await?;

    match celery.send_task(
        super::super::tasks::statuses::deliver_status_delete::new(status.clone(), account.clone())
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    Ok(rocket::serde::json::Json(render_status(config, &db, status, Some(&account)).await?))
}

#[get("/api/v1/statuses/<status_id>/context")]
pub async fn status_context(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>,
    user: Option<super::oauth::TokenClaims>, status_id: String,
) -> Result<rocket::serde::json::Json<super::objs::Context>, rocket::http::Status> {
    if let Some(user) = &user {
        if !user.has_scope("read:statuses") {
            return Err(rocket::http::Status::Forbidden);
        }
    }

    let account = match &user {
        Some(u) => Some(super::accounts::get_account(&db, u).await?),
        None => None
    };

    let status = get_status_and_check_visibility(&status_id, account.as_ref(), &db).await?;

    let (ancestors, descendants) = crate::db_run(&db, move |c| -> QueryResult<_> {
        let mut descendants = vec![];
        let mut ancestors = vec![];

        let mut in_reply_to = status.in_reply_to_id;
        while let Some(irt) = in_reply_to {
            let irt_status = crate::schema::statuses::dsl::statuses.find(irt).filter(
                crate::schema::statuses::dsl::deleted_at.is_null()
            ).get_result::<models::Status>(c).optional()?;
            match irt_status {
                Some(irt_status) => {
                    in_reply_to = irt_status.in_reply_to_id;
                    ancestors.push(irt_status);
                }
                None => {
                    in_reply_to = None
                }
            }
        }

        let mut descendant_ids = vec![status.id];
        while let Some(did) = descendant_ids.pop() {
            let de_statuses = crate::schema::statuses::dsl::statuses.filter(
                crate::schema::statuses::in_reply_to_id.eq(did)
            ).filter(
                crate::schema::statuses::dsl::deleted_at.is_null()
            ).get_results::<models::Status>(c)?;
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

#[get("/api/v1/statuses/<status_id>/reblogged_by?<limit>&<min_id>&<max_id>")]
pub async fn status_boosted_by(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>,
    user: Option<super::oauth::TokenClaims>, status_id: String,
    limit: Option<u64>, min_id: Option<i32>, max_id: Option<i32>, host: &rocket::http::uri::Host<'_>,
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Account>>>, rocket::http::Status> {
    if let Some(user) = &user {
        if !user.has_scope("read:statuses") {
            return Err(rocket::http::Status::Forbidden);
        }
    }

    let account = match &user {
        Some(u) => Some(super::accounts::get_account(&db, u).await?),
        None => None
    };

    let limit = limit.unwrap_or(40);
    if limit > 500 {
        return Err(rocket::http::Status::BadRequest);
    }

    let status = get_status_and_check_visibility(&status_id, account.as_ref(), &db).await?;

    let boosted_by = crate::db_run(&db, move |c| -> QueryResult<_> {
        let mut sel = crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::boost_of_id.eq(status.id)
        ).filter(
            crate::schema::statuses::dsl::deleted_at.is_null()
        ).inner_join(
            crate::schema::accounts::table.on(
                crate::schema::accounts::dsl::id.eq(crate::schema::statuses::dsl::account_id)
            )
        ).order_by(crate::schema::accounts::dsl::iid.desc()).limit(limit as i64).into_boxed();
        if let Some(min_id) = min_id {
            sel = sel.filter(crate::schema::accounts::dsl::iid.gt(min_id));
        }
        if let Some(max_id) = max_id {
            sel = sel.filter(crate::schema::accounts::dsl::iid.lt(max_id));
        }
        sel.get_results::<(models::Status, models::Account)>(c)
    }).await?;

    let mut links = vec![];

    if let Some(last_id) = boosted_by.first().map(|a| a.0.iid) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/statuses/{}/reblogged_by?min_id={}", host.to_string(), status_id, last_id)
        });
    }
    if let Some(first_id) = boosted_by.last().map(|a| a.0.iid) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/statuses/{}/reblogged_by?max_id={}", host.to_string(), status_id, first_id)
        });
    }

    Ok(super::LinkedResponse {
        inner: rocket::serde::json::Json(futures::stream::iter(boosted_by.into_iter()).map(|(_, a)| {
            super::accounts::render_account(config, &db, a)
        }).buffer_unordered(10).collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>, _>>()?),
        links
    })
}


#[get("/api/v1/statuses/<status_id>/favourited_by?<limit>&<min_id>&<max_id>")]
pub async fn status_liked_by(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>,
    user: Option<super::oauth::TokenClaims>, status_id: String,
    limit: Option<u64>, min_id: Option<i32>, max_id: Option<i32>, host: &rocket::http::uri::Host<'_>,
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Account>>>, rocket::http::Status> {
    if let Some(user) = &user {
        if !user.has_scope("read:statuses") {
            return Err(rocket::http::Status::Forbidden);
        }
    }

    let account = match &user {
        Some(u) => Some(super::accounts::get_account(&db, u).await?),
        None => None
    };

    let limit = limit.unwrap_or(40);
    if limit > 500 {
        return Err(rocket::http::Status::BadRequest);
    }

    let status = get_status_and_check_visibility(&status_id, account.as_ref(), &db).await?;

    let boosted_by = crate::db_run(&db, move |c| -> QueryResult<_> {
        let mut sel = crate::schema::likes::dsl::likes.filter(
            crate::schema::likes::dsl::status.eq(status.id)
        ).inner_join(
            crate::schema::accounts::table.on(
                crate::schema::accounts::dsl::id.eq(crate::schema::likes::dsl::account)
            )
        ).order_by(crate::schema::likes::dsl::iid.desc()).limit(limit as i64).into_boxed();
        if let Some(min_id) = min_id {
            sel = sel.filter(crate::schema::likes::dsl::iid.gt(min_id));
        }
        if let Some(max_id) = max_id {
            sel = sel.filter(crate::schema::likes::dsl::iid.lt(max_id));
        }
        sel.get_results::<(models::Like, models::Account)>(c)
    }).await?;

    let mut links = vec![];

    if let Some(last_id) = boosted_by.first().map(|a| a.0.iid) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/statuses/{}/favourited_by?min_id={}", host.to_string(), status_id, last_id)
        });
    }
    if let Some(first_id) = boosted_by.last().map(|a| a.0.iid) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/statuses/{}/favourited_by?max_id={}", host.to_string(), status_id, first_id)
        });
    }

    Ok(super::LinkedResponse {
        inner: rocket::serde::json::Json(futures::stream::iter(boosted_by.into_iter()).map(|(_, a)| {
            super::accounts::render_account(config, &db, a)
        }).buffer_unordered(10).collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>, _>>()?),
        links
    })
}

#[derive(FromForm)]
pub struct BoostForm<'a> {
    visibility: Option<&'a str>,
}

#[post("/api/v1/statuses/<status_id>/reblog", data = "<form>")]
pub async fn boost_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    status_id: String, form: Option<rocket::form::Form<BoostForm<'_>>>,
    celery: &rocket::State<crate::CeleryApp>
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if !user.has_scope("write:statuses") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let status = get_status_and_check_visibility(&status_id, Some(&account), &db).await?;

    if let Some(status) = crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::boost_of_id.eq(status.id)
        ).filter(
            crate::schema::statuses::dsl::account_id.eq(&account.id)
        ).filter(
            crate::schema::statuses::dsl::deleted_at.is_null()
        ).get_result::<models::Status>(c).optional()
    }).await? {
        return Ok(rocket::serde::json::Json(render_status(config, &db, status, Some(&account)).await?));
    }

    let audience = match form.and_then(|f| f.visibility) {
        Some(v) => match super::objs::StatusVisibility::from_str(v) {
            Some(v) => v,
            None => return Err(rocket::http::Status::UnprocessableEntity)
        },
        None => super::objs::StatusVisibility::Public
    };

    match audience {
        super::objs::StatusVisibility::Public |
        super::objs::StatusVisibility::Unlisted |
        super::objs::StatusVisibility::Private => (),
        _ => return Err(rocket::http::Status::BadRequest)
    }

    let new_status = models::NewStatus {
        id: uuid::Uuid::new_v4(),
        url: "".to_string(),
        uri: None,
        text: "".to_string(),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
        in_reply_to_id: None,
        in_reply_to_url: None,
        boost_of_url: None,
        boost_of_id: Some(status.id),
        sensitive: false,
        spoiler_text: "".to_string(),
        language: None,
        local: true,
        account_id: account.id,
        deleted_at: None,
        edited_at: None,
        public: audience == super::objs::StatusVisibility::Public,
        visible: audience == super::objs::StatusVisibility::Public ||
            audience == super::objs::StatusVisibility::Unlisted,
        text_source: None,
        spoiler_text_source: None,
    };
    let new_audience_followers = models::StatusAudience {
        id: uuid::Uuid::new_v4(),
        status_id: new_status.id,
        mention: false,
        account: None,
        account_followers: Some(account.id)
    };
    let new_audience_account = models::StatusAudience {
        id: uuid::Uuid::new_v4(),
        status_id: new_status.id,
        mention: false,
        account: Some(status.account_id),
        account_followers: None,
    };
    let s = crate::db_run(&db, move |c| -> QueryResult<_> {
        c.transaction::<_, diesel::result::Error, _>(|| {
            let s = diesel::insert_into(crate::schema::statuses::dsl::statuses)
                .values(new_status)
                .get_result::<models::Status>(c)?;
            diesel::insert_into(crate::schema::status_audiences::dsl::status_audiences)
                .values(vec![new_audience_followers, new_audience_account])
                .execute(c)?;
            Ok(s)
        })
    }).await?;

    match celery.send_task(
        super::super::tasks::statuses::deliver_boost::new(s.clone(), status, account.clone())
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    Ok(rocket::serde::json::Json(render_status(config, &db, s, Some(&account)).await?))
}

#[post("/api/v1/statuses/<status_id>/unreblog")]
pub async fn unboost_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    status_id: String, celery: &rocket::State<crate::CeleryApp>
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if !user.has_scope("write:statuses") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let status = get_status_and_check_visibility(&status_id, Some(&account), &db).await?;

    let mut boost_status: models::Status = match crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::boost_of_id.eq(status.id)
        ).filter(
            crate::schema::statuses::dsl::account_id.eq(&account.id)
        ).filter(
            crate::schema::statuses::dsl::deleted_at.is_null()
        ).get_result(c).optional()
    }).await? {
        Some(s) => s,
        None => return Ok(rocket::serde::json::Json(render_status(config, &db, status, Some(&account)).await?))
    };

    boost_status.deleted_at = Some(Utc::now().naive_utc());

    let boost_status = crate::db_run(&db, move |c| -> QueryResult<_> {
        diesel::update(&boost_status)
            .set(&boost_status)
            .get_result::<models::Status>(c)
    }).await?;

    match celery.send_task(
        super::super::tasks::statuses::deliver_undo_boost::new(boost_status, status.clone(), account.clone())
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    Ok(rocket::serde::json::Json(render_status(config, &db, status, Some(&account)).await?))
}

#[post("/api/v1/statuses/<status_id>/pin")]
pub async fn pin_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    status_id: String
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if !user.has_scope("write:accounts") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let status = get_status_and_check_visibility(&status_id, Some(&account), &db).await?;

    if crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::pins::dsl::pins.filter(
            crate::schema::pins::dsl::status.eq(status.id)
        ).filter(
            crate::schema::pins::dsl::account.eq(&account.id)
        ).count().get_result::<i64>(c)
    }).await? > 0 {
        return Ok(rocket::serde::json::Json(render_status(config, &db, status, Some(&account)).await?));
    }

    let new_pin = models::NewPin {
        id: uuid::Uuid::new_v4(),
        account: account.id,
        status: status.id,
    };
    crate::db_run(&db, move |c| -> QueryResult<_> {
        diesel::insert_into(crate::schema::pins::dsl::pins)
            .values(new_pin)
            .execute(c)
    }).await?;

    Ok(rocket::serde::json::Json(render_status(config, &db, status, Some(&account)).await?))
}

#[post("/api/v1/statuses/<status_id>/unpin")]
pub async fn unpin_status(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    status_id: String,
) -> Result<rocket::serde::json::Json<super::objs::Status>, rocket::http::Status> {
    if !user.has_scope("write:accounts") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = super::accounts::get_account(&db, &user).await?;
    let status = get_status_and_check_visibility(&status_id, Some(&account), &db).await?;

    crate::db_run(&db, move |c| -> QueryResult<_> {
        diesel::delete(crate::schema::pins::dsl::pins.filter(
            crate::schema::pins::dsl::status.eq(status.id)
        ).filter(
            crate::schema::pins::dsl::account.eq(&account.id)
        )).execute(c)
    }).await?;

    Ok(rocket::serde::json::Json(render_status(config, &db, status, Some(&account)).await?))
}