use diesel::prelude::*;
use chrono::prelude::*;
use rocket::futures;
use futures::StreamExt;

pub async fn get_account(db: &crate::DbConn, user: &super::oauth::TokenClaims) -> Result<crate::models::Account, rocket::http::Status> {
    let sub = user.subject.clone();
    crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::accounts::dsl::accounts.filter(
            crate::schema::accounts::dsl::owned_by.eq(sub)
        ).first(c)
    }).await
}

pub async fn init_account(db: crate::DbConn, user: &super::oidc::OIDCIdTokenClaims) -> Result<(), rocket::http::Status> {
    let sub = user.subject().to_string();
    let account = crate::db_run(&db, move |c| -> QueryResult<_> {
        Ok(crate::schema::accounts::dsl::accounts.filter(
            crate::schema::accounts::dsl::owned_by.eq(sub)
        ).count().get_result::<i64>(c)? > 0)
    }).await?;

    if !account {
        let pref = user.preferred_username().map(|u| u.to_string())
            .or(user.given_name().and_then(|g| g.get(None)).map(|g| g.to_string().to_lowercase()))
            .unwrap_or_else(|| user.subject().to_string());
        let username = crate::db_run(&db, move |c| -> QueryResult<_> {
            if crate::schema::accounts::dsl::accounts.filter(
                crate::schema::accounts::dsl::username.eq(&pref)
            ).filter(
                crate::schema::accounts::dsl::local.eq(true)
            ).count().get_result::<i64>(c)? > 0 {
                let i = 1;
                loop {
                    let username = format!("{}{}", pref, i);
                    if crate::schema::accounts::dsl::accounts.filter(
                        crate::schema::accounts::dsl::username.eq(&username)
                    ).filter(
                        crate::schema::accounts::dsl::local.eq(true)
                    ).count().get_result::<i64>(c)? == 0 {
                        break Ok(username);
                    }
                }
            } else {
                Ok(pref)
            }
        }).await?;

        let private_key = String::from_utf8(
            match match openssl::rsa::Rsa::generate(2048) {
                Ok(k) => k,
                Err(e) => {
                    error!("Unable to generate RSA key: {}", e);
                    return Err(rocket::http::Status::InternalServerError);
                }
            }.private_key_to_pem() {
                Ok(k) => k,
                Err(e) => {
                    error!("Unable to convert RSA key to PEM: {}", e);
                    return Err(rocket::http::Status::InternalServerError);
                }
            }
        ).unwrap();

        let account = crate::models::NewAccount {
            id: uuid::Uuid::new_v4(),
            owned_by: Some(user.subject().to_string()),
            display_name: user.name().and_then(|n| n.get(None)).map(|n| n.to_string())
                .unwrap_or_else(|| username.clone()),
            default_sensitive: Some(false),
            default_language: Some("en".to_string()),
            discoverable: Some(true),
            follower_count: 0,
            following_count: 0,
            bio: "".to_string(),
            locked: false,
            bot: false,
            group: false,
            created_at: Utc::now().naive_utc(),
            username,
            statuses_count: 0,
            private_key: Some(private_key),
            local: true,
            inbox_url: None,
            outbox_url: None,
            shared_inbox_url: None,
            actor: None,
            updated_at: Utc::now().naive_utc(),
            url: None,
            avatar_file: None,
            avatar_content_type: None,
            avatar_remote_url: None,
            header_file: None,
            header_content_type: None,
            header_remote_url: None,
            follower_collection_url: None,
        };
        crate::db_run(&db, move |c| -> diesel::result::QueryResult<_> {
            diesel::insert_into(crate::schema::accounts::table)
                .values(account)
                .execute(c)
        }).await?;
    }

    Ok(())
}

pub async fn render_account(
    config: &crate::AppConfig, db: &crate::DbConn, account: crate::models::Account
) -> Result<super::objs::Account, rocket::http::Status> {
    let fields: Vec<crate::models::AccountField> = crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::account_fields::dsl::account_fields.filter(
            crate::schema::account_fields::dsl::account_id.eq(account.id)
        ).order_by(crate::schema::account_fields::dsl::sort_order.asc()).get_results(c)
    }).await?;

    let domain = account.url.as_deref().and_then(
        |u| reqwest::Url::parse(u).ok()?.domain().map(|d| d.to_string())
    );

    let follower_count = if account.local {
        crate::db_run(db, move |c| -> QueryResult<_> {
            crate::schema::following::dsl::following.filter(
                crate::schema::following::dsl::followee.eq(account.id).and(
                    crate::schema::following::dsl::pending.eq(false)
                )
            ).count().get_result(c)
        }).await?
    } else {
        account.follower_count as i64
    };

    let following_count = if account.local {
        crate::db_run(db, move |c| -> QueryResult<_> {
            crate::schema::following::dsl::following.filter(
                crate::schema::following::dsl::follower.eq(account.id).and(
                    crate::schema::following::dsl::pending.eq(false)
                )
            ).count().get_result(c)
        }).await?
    } else {
        account.following_count as i64
    };

    Ok(super::objs::Account {
        id: account.iid.to_string(),
        username: account.username.clone(),
        acct: match domain {
            Some(d) => format!("{}@{}", account.username, d),
            None => account.username
        },
        display_name: account.display_name,
        locked: account.locked,
        bot: account.bot,
        created_at: Utc.from_local_datetime(&account.created_at).unwrap(),
        note: account.bio,
        url: if account.local {
            Some(format!("https://{}/users/{}", config.uri, account.id.to_string()))
        } else {
            account.url
        },
        avatar: match &account.avatar_file {
            Some(a) => format!("https://{}/media/{}", config.uri, a),
            None => format!("https://{}/static/missing.png", config.uri),
        },
        avatar_static: match &account.avatar_file {
            Some(a) => format!("https://{}/media/{}", config.uri, a),
            None => format!("https://{}/static/missing.png", config.uri),
        },
        header: match &account.header_file {
            Some(a) => format!("https://{}/media/{}", config.uri, a),
            None => format!("https://{}/static/header.png", config.uri),
        },
        header_static: match &account.header_file {
            Some(a) => format!("https://{}/media/{}", config.uri, a),
            None => format!("https://{}/static/header.png", config.uri),
        },
        followers_count: follower_count as u64,
        following_count: following_count as u64,
        statuses_count: account.statuses_count as u64,
        last_status_at: None,
        fields: fields.into_iter().map(|f| super::objs::Field {
            name: f.name,
            value: f.value,
            verified_at: None
        }).collect(),
        emojis: vec![],
        discoverable: Some(account.discoverable.unwrap_or(true)),
        group: account.group,
        limited: None,
        moved: None,
        noindex: None,
        suspended: None,
    })
}

#[get("/api/v1/accounts/verify_credentials")]
pub async fn verify_credentials(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims
) -> Result<rocket::serde::json::Json<super::objs::CredentialAccount>, rocket::http::Status> {
    if !user.has_scope("read:accounts") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = get_account(&db, &user).await?;

    Ok(rocket::serde::json::Json(super::objs::CredentialAccount {
        source: super::objs::AccountSource {
            note: account.bio.clone(),
            fields: vec![],
            privacy: "public".to_string(),
            sensitive: false,
            language: "en".to_string(),
            follow_requests_count: 0
        },
        base: render_account(config, &db, account).await?,
    }))
}

#[derive(FromForm)]
pub struct AccountUpdateForm<'a> {
    discoverable: Option<bool>,
    bot: Option<bool>,
    display_name: Option<String>,
    note: Option<String>,
    locked: Option<bool>,
    source: AccountUpdateSourceForm,
    #[field(default = Vec::new())]
    fields_attributes: Option<Vec<AccountUpdateFieldForm>>,
    avatar: Option<rocket::fs::TempFile<'a>>,
    header: Option<rocket::fs::TempFile<'a>>,
}

#[derive(FromForm)]
pub struct AccountUpdateSourceForm {
    privacy: Option<String>,
    sensitive: Option<bool>,
    language: Option<String>
}

#[derive(FromForm)]
pub struct AccountUpdateFieldForm {
    name: String,
    value: String,
}

#[patch("/api/v1/accounts/update_credentials", data = "<form>")]
pub async fn update_credentials(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    form: rocket::form::Form<AccountUpdateForm<'_>>,
) -> Result<rocket::serde::json::Json<super::objs::Account>, rocket::http::Status> {
    if !user.has_scope("write:accounts") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = get_account(&db, &user).await?;

    use crate::schema::accounts;
    #[derive(AsChangeset)]
    #[table_name = "accounts"]
    struct AccountUpdate<'a> {
        display_name: Option<String>,
        bio: Option<String>,
        locked: Option<bool>,
        bot: Option<bool>,
        discoverable: Option<bool>,
        default_sensitive: Option<bool>,
        default_language: Option<String>,
        avatar_file: Option<String>,
        avatar_content_type: Option<&'a str>,
        header_file: Option<String>,
        header_content_type: Option<&'a str>,
    }

    if let Some(default_language) = &form.source.language {
        if default_language.len() != 2 {
            return Err(rocket::http::Status::BadRequest);
        }
    }

    let mut upd = AccountUpdate {
        display_name: form.display_name.as_ref().map(|x| x.to_string()),
        bio: form.note.as_ref().map(|x| x.to_string()),
        locked: form.locked,
        bot: form.bot,
        discoverable: form.discoverable,
        default_sensitive: form.source.sensitive,
        default_language: form.source.language.as_ref().map(|x| x.to_string()),
        avatar_file: None,
        avatar_content_type: None,
        header_file: None,
        header_content_type: None,
    };

    if let Some(avatar) = &form.avatar {
        let format = match avatar.content_type() {
            Some(f) => match image::ImageFormat::from_mime_type(f.to_string()) {
                Some(f) => f,
                None => return Err(rocket::http::Status::UnprocessableEntity)
            },
            None => return Err(rocket::http::Status::BadRequest)
        };
        match format {
            image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::Gif => {},
            _ => return Err(rocket::http::Status::UnprocessableEntity)
        }
        let mut image_r = image::io::Reader::open(match avatar.path() {
            Some(p) => p,
            None => return Err(rocket::http::Status::InternalServerError)
        }).map_err(|e| rocket::http::Status::InternalServerError)?;
        image_r.set_format(format);
        let mut image = image_r.decode().map_err(|e| rocket::http::Status::BadRequest)?;
        image = image.resize_to_fill(crate::AVATAR_SIZE, crate::AVATAR_SIZE, image::imageops::FilterType::Lanczos3);
        let mut out_image_bytes: Vec<u8> = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut out_image_bytes), image::ImageOutputFormat::Png)
            .map_err(|_| rocket::http::Status::InternalServerError)?;
        let image_id = uuid::Uuid::new_v4();
        let image_name = format!("{}.png", image_id.to_string());
        let image_path = format!("./media/{}", image_name);
        std::fs::write(&image_path, &out_image_bytes).map_err(|_| rocket::http::Status::InternalServerError)?;
        upd.avatar_file = Some(image_name);
        upd.avatar_content_type = Some("image/png");
    }

    if let Some(header) = &form.header {
        let format = match header.content_type() {
            Some(f) => match image::ImageFormat::from_mime_type(f.to_string()) {
                Some(f) => f,
                None => return Err(rocket::http::Status::BadRequest)
            },
            None => return Err(rocket::http::Status::BadRequest)
        };
        match format {
            image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::Gif => {},
            _ => return Err(rocket::http::Status::BadRequest)
        }
        let mut image_r = image::io::Reader::open(match header.path() {
            Some(p) => p,
            None => return Err(rocket::http::Status::InternalServerError)
        }).map_err(|e| rocket::http::Status::InternalServerError)?;
        image_r.set_format(format);
        let mut image = image_r.decode().map_err(|e| rocket::http::Status::BadRequest)?;
        image = image.resize_to_fill(crate::HEADER_WIDTH, crate::HEADER_HEIGHT, image::imageops::FilterType::Lanczos3);
        let mut out_image_bytes: Vec<u8> = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut out_image_bytes), image::ImageOutputFormat::Png)
            .map_err(|_| rocket::http::Status::InternalServerError)?;
        let image_id = uuid::Uuid::new_v4();
        let image_name = format!("{}.png", image_id.to_string());
        let image_path = format!("./media/{}", image_name);
        std::fs::write(&image_path, &out_image_bytes).map_err(|_| rocket::http::Status::InternalServerError)?;
        upd.header_file = Some(image_name);
        upd.header_content_type = Some("image/png");
    }

    let attributes = form.fields_attributes.as_ref().map(|a| {
        a.into_iter().enumerate().map(|(i, f)| crate::models::AccountField {
            id: uuid::Uuid::new_v4(),
            account_id: account.id,
            name: f.name.clone(),
            value: f.value.clone(),
            sort_order: i as i32
        }).collect::<Vec<_>>()
    });

    let account: crate::models::Account = crate::db_run(&db, move |c| -> QueryResult<_> {
        c.transaction::<_, _, _>(|| {
            if let Some(attributes) = attributes {
                diesel::delete(crate::schema::account_fields::table.filter(
                    crate::schema::account_fields::dsl::account_id.eq(account.id)
                )).execute(c)?;

                diesel::insert_into(crate::schema::account_fields::table)
                    .values(attributes)
                    .execute(c)?;
            }

            diesel::update(accounts::dsl::accounts.find(account.id)).set(&upd).get_result(c)
        })
    }).await?;

    Ok(rocket::serde::json::Json(render_account(config, &db, account).await?))
}

async fn get_account_from_db(account_id: &str, db: &crate::DbConn) -> Result<crate::models::Account, rocket::http::Status> {
    let account_id = match account_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let account: crate::models::Account = match crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::accounts::dsl::accounts.filter(
            crate::schema::accounts::dsl::iid.eq(account_id)
        ).get_result(c).optional()
    }).await? {
        Some(a) => a,
        None => return Err(rocket::http::Status::NotFound)
    };

    Ok(account)
}

#[get("/api/v1/accounts/<account_id>")]
pub async fn account(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, account_id: String
) -> Result<rocket::serde::json::Json<super::objs::Account>, rocket::http::Status> {
    let account = get_account_from_db(&account_id, &db).await?;

    Ok(rocket::serde::json::Json(render_account(config, &db, account).await?))
}

#[get("/api/v1/accounts/<account_id>/statuses?<limit>&<exclude_replies>&<only_media>&<exclude_reblogs>&<pinned>&<min_id>&<max_id>")]
pub async fn account_statuses(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, account_id: String,
    user: Option<super::oauth::TokenClaims>,
    limit: Option<u64>, exclude_replies: Option<&str>, only_media: Option<&str>,
    exclude_reblogs: Option<&str>, pinned: Option<&str>,
    max_id: Option<i32>, min_id: Option<i32>,
    host: &rocket::http::uri::Host<'_>,
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Status>>>, rocket::http::Status> {
    let account = get_account_from_db(&account_id, &db).await?;
    let req_account = match &user {
        Some(u) => Some(get_account(&db, u).await?),
        None => None
    };

    let exclude_replies = super::parse_bool(exclude_replies, false)?;
    let _only_media = super::parse_bool(only_media, false)?;
    let exclude_reblogs = super::parse_bool(exclude_reblogs, false)?;
    let pinned = super::parse_bool(pinned, false)?;

    let limit = limit.unwrap_or(20);
    if limit > 500 {
        return Err(rocket::http::Status::BadRequest);
    }

    let statuses: Vec<crate::models::Status> =
        crate::db_run(&db, move |c| -> QueryResult<_> {
            let mut sel = crate::schema::statuses::dsl::statuses.order_by(
                crate::schema::statuses::dsl::created_at.desc()
            ).filter(
                crate::schema::statuses::dsl::account_id.eq(&account.id)
            ).filter(
                crate::schema::statuses::dsl::deleted_at.is_null()
            ).filter(
                crate::schema::statuses::dsl::boost_of_url.is_null()
            ).limit(limit as i64).into_boxed();
            if let Some(min_id) = min_id {
                sel = sel.filter(crate::schema::statuses::dsl::iid.gt(min_id));
            }
            if let Some(max_id) = max_id {
                sel = sel.filter(crate::schema::statuses::dsl::iid.lt(max_id));
            }
            if exclude_replies {
                sel = sel.filter(
                    crate::schema::statuses::dsl::in_reply_to_id.is_null().and(
                        crate::schema::statuses::dsl::in_reply_to_url.is_null()
                    )
                );
            }
            if exclude_reblogs {
                sel = sel.filter(
                    crate::schema::statuses::dsl::boost_of_id.is_null().and(
                        crate::schema::statuses::dsl::boost_of_url.is_null()
                    )
                );
            }
            if pinned {
                sel = sel.filter(crate::schema::statuses::dsl::id.eq_any(
                    crate::schema::pins::dsl::pins.filter(
                        crate::schema::pins::dsl::account.eq(&account.id)
                    ).select(crate::schema::pins::dsl::status)
                ));
            }
            sel.get_results(c)
        }).await?;

    let mut out_statuses = vec![];
    for status in statuses {
        if super::statuses::can_view(&status, req_account.as_ref(), &db).await? {
            out_statuses.push(status);
        }
    }

    let mut links = vec![];

    if let Some(last_id) = out_statuses.last().map(|a| a.iid) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/accounts/{}/statuses?max_id={}", host.to_string(), account_id, last_id)
        });
    }
    if let Some(first_id) = out_statuses.first().map(|a| a.iid) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/accounts/{}/statuses?min_id={}", host.to_string(), account_id, first_id)
        });
    }

    Ok(super::LinkedResponse {
        inner: rocket::serde::json::Json(
            futures::stream::iter(out_statuses).map(|status| {
                super::statuses::render_status(config, &db, status, req_account.as_ref())
            }).buffered(10).collect::<Vec<_>>().await
                .into_iter().collect::<Result<Vec<_>, _>>()?
        ),
        links
    })
}

#[get("/api/v1/accounts/<account_id>/followers?<limit>&<min_id>&<max_id>")]
pub async fn account_followers(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, account_id: String,
    limit: Option<u64>, min_id: Option<i32>, max_id: Option<i32>, host: &rocket::http::uri::Host<'_>,
) -> Result<super::LinkedResponse<rocket::serde::json::Json<Vec<super::objs::Account>>>, rocket::http::Status> {
    let account = get_account_from_db(&account_id, &db).await?;

    let limit = limit.unwrap_or(40);
    if limit > 500 {
        return Err(rocket::http::Status::BadRequest);
    }

    let followers: Vec<crate::models::Account> = crate::db_run(&db, move |c| -> QueryResult<_> {
        let mut sel = crate::schema::accounts::dsl::accounts.filter(
            crate::schema::accounts::dsl::id.eq_any(
                crate::schema::following::dsl::following.select(crate::schema::following::dsl::follower).filter(
                    crate::schema::following::dsl::followee.eq(&account.id).and(
                        crate::schema::following::dsl::pending.eq(false)
                    )
                )
            )
        ).order_by(crate::schema::accounts::dsl::iid.desc()).limit(limit as i64).into_boxed();
        if let Some(min_id) = min_id {
            sel = sel.filter(crate::schema::accounts::dsl::iid.gt(min_id));
        }
        if let Some(max_id) = max_id {
            sel = sel.filter(crate::schema::accounts::dsl::iid.lt(max_id));
        }
        sel.get_results(c)
    }).await?;

    let mut links = vec![];

    if let Some(last_id) = followers.first().map(|a| a.iid) {
        links.push(super::Link {
            rel: "next".to_string(),
            href: format!("https://{}/api/v1/accounts/{}/followers?min_id={}", host.to_string(), account_id, last_id)
        });
    }
    if let Some(first_id) = followers.last().map(|a| a.iid) {
        links.push(super::Link {
            rel: "prev".to_string(),
            href: format!("https://{}/api/v1/accounts/{}/followers?max_id={}", host.to_string(), account_id, first_id)
        });
    }

    Ok(super::LinkedResponse {
        inner: rocket::serde::json::Json(futures::future::try_join_all(
            followers.into_iter().map(|a| render_account(config, &db, a)).collect::<Vec<_>>()
        ).await?),
        links,
    })
}

#[get("/api/v1/accounts/<account_id>/following?<limit>")]
pub async fn account_following(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, account_id: String,
    limit: Option<usize>
) -> Result<rocket::serde::json::Json<Vec<super::objs::Account>>, rocket::http::Status> {
    let account = get_account_from_db(&account_id, &db).await?;

    let limit = limit.unwrap_or(40);
    if limit > 500 {
        return Err(rocket::http::Status::BadRequest);
    }

    let following: Vec<crate::models::Account> = crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::accounts::dsl::accounts.filter(
            crate::schema::accounts::dsl::id.eq_any(
                crate::schema::following::dsl::following.select(crate::schema::following::dsl::followee).filter(
                    crate::schema::following::dsl::follower.eq(&account.id).and(
                        crate::schema::following::dsl::pending.eq(false)
                    )
                )
            )
        ).limit(limit as i64).get_results(c)
    }).await?;

    Ok(rocket::serde::json::Json(futures::future::try_join_all(
        following.into_iter().map(|a| render_account(config, &db, a)).collect::<Vec<_>>()
    ).await?))
}

#[get("/api/v1/accounts/<account_id>/lists")]
pub async fn lists(
    db: crate::DbConn, _config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    account_id: String
) -> Result<rocket::serde::json::Json<Vec<super::objs::List>>, rocket::http::Status> {
    if !user.has_scope("read:lists") {
        return Err(rocket::http::Status::Forbidden);
    }

    let _account = get_account_from_db(&account_id, &db).await?;

    Ok(rocket::serde::json::Json(vec![]))
}

async fn render_relationship<'a>(
    db: &'a crate::DbConn, own_account: &'a crate::models::Account,
    other_account: std::borrow::Cow<'a, crate::models::Account>,
) -> Result<super::objs::Relationship, rocket::http::Status> {
    let own_account_id = own_account.id;
    let other_account_id = other_account.id;
    let following = crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::following::dsl::following.filter(
            crate::schema::following::dsl::follower.eq(own_account_id).and(
                crate::schema::following::dsl::followee.eq(other_account_id)
            ).and(
                crate::schema::following::dsl::pending.eq(false)
            )
        ).count().get_result::<i64>(c)
    }).await?;
    let following_pending = crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::following::dsl::following.filter(
            crate::schema::following::dsl::follower.eq(own_account_id).and(
                crate::schema::following::dsl::followee.eq(other_account_id)
            ).and(
                crate::schema::following::dsl::pending.eq(true)
            )
        ).count().get_result::<i64>(c)
    }).await?;
    let followed_by = crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::following::dsl::following.filter(
            crate::schema::following::dsl::followee.eq(own_account_id).and(
                crate::schema::following::dsl::follower.eq(other_account_id)
            ).and(
                crate::schema::following::dsl::pending.eq(false)
            )
        ).count().get_result::<i64>(c)
    }).await?;
    let note: Option<crate::models::AccountNote> = crate::db_run(db, move |c| -> QueryResult<_> {
        crate::schema::account_notes::dsl::account_notes.filter(
            crate::schema::account_notes::dsl::account.eq(other_account_id)
        ).filter(
            crate::schema::account_notes::dsl::owner.eq(own_account_id)
        ).get_result(c).optional()
    }).await?;

    Ok(super::objs::Relationship {
        id: other_account.iid.to_string(),
        following: following > 0,
        followed_by: followed_by > 0,
        blocking: false,
        blocked_by: false,
        muting: false,
        muting_notifications: false,
        requested: following_pending > 0,
        domain_blocking: false,
        showing_reblogs: true,
        notifying: false,
        endorsed: false,
        languages: vec![],
        note: note.map(|n| n.note),
    })
}

#[get("/api/v1/accounts/relationships?<id>")]
pub async fn relationships(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    id: Vec<String>
) -> Result<rocket::serde::json::Json<Vec<super::objs::Relationship>>, rocket::http::Status> {
    if !user.has_scope("read:follows") {
        return Err(rocket::http::Status::Forbidden);
    }

    let ids = match id.into_iter()
        .map(|id| id.parse::<i32>())
        .collect::<Result<Vec<_>, _>>() {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let accounts = futures::stream::iter(ids.into_iter().map(|id| {
        crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::accounts::dsl::accounts.filter(
                crate::schema::accounts::dsl::iid.eq(id)
            ).first::<crate::models::Account>(c)
        })
    })).buffer_unordered(10).collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>, _>>()?;

    let account = get_account(&db, &user).await?;

    let relationships = futures::stream::iter(accounts.into_iter())
        .map(|acct| render_relationship(&db, &account, std::borrow::Cow::Owned(acct)))
        .buffer_unordered(10).collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>, _>>()?;

    Ok(rocket::serde::json::Json(relationships))
}

#[get("/api/v1/accounts/familiar_followers?<id>")]
pub async fn familiar_followers(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    id: Vec<String>
) -> Result<rocket::serde::json::Json<Vec<super::objs::FamiliarFollowers>>, rocket::http::Status> {
    if !user.has_scope("read:follows") {
        return Err(rocket::http::Status::Forbidden);
    }

    let ids = match id.into_iter()
        .map(|id| uuid::Uuid::parse_str(&id))
        .collect::<Result<Vec<_>, _>>() {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let account = get_account(&db, &user).await?;

    let account_followers: Vec<uuid::Uuid> = crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::following::dsl::following.filter(
            crate::schema::following::dsl::followee.eq(&account.id).and(
                crate::schema::following::dsl::pending.eq(false)
            )
        ).select(crate::schema::following::dsl::follower).get_results(c)
    }).await?;

    let mut familiar_followers = vec![];
    for id in ids {
        let f = account_followers.clone();
        let followed_by: Vec<_> = crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::accounts::dsl::accounts.filter(
                crate::schema::accounts::dsl::id.eq_any(
                    crate::schema::following::dsl::following.select(
                        crate::schema::following::dsl::follower
                    ).filter(
                    crate::schema::following::dsl::followee.eq(id).and(
                        crate::schema::following::dsl::follower.eq_any(f)
                        ).and(
                            crate::schema::following::dsl::pending.eq(false)
                        )
                    )
                )
            ).get_results::<crate::models::Account>(c)
        }).await?;

        familiar_followers.push(super::objs::FamiliarFollowers {
            id: id.to_string(),
            accounts: futures::future::try_join_all(
                followed_by.into_iter().map(|a| render_account(config, &db, a)).collect::<Vec<_>>()
            ).await?
        });
    }

    Ok(rocket::serde::json::Json(familiar_followers))
}

#[derive(FromForm)]
pub struct FollowAccountForm<'a> {
    reblogs: Option<&'a str>,
    notify: Option<&'a str>,
    language: Option<Vec<&'a str>>
}

#[post("/api/v1/accounts/<account_id>/follow", data = "<_form>")]
pub async fn follow_account(
    db: crate::DbConn, user: super::oauth::TokenClaims, account_id: String,
    celery: &rocket::State<crate::CeleryApp>,
    _form: Option<rocket::form::Form<FollowAccountForm<'_>>>
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, rocket::http::Status> {
    if !user.has_scope("write:follows") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = get_account(&db, &user).await?;
    let followed_account = get_account_from_db(&account_id, &db).await?;

    if crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::following::dsl::following.filter(
            crate::schema::following::dsl::follower.eq(&account.id).and(
                crate::schema::following::dsl::followee.eq(&followed_account.id)
            )
        ).count().get_result::<i64>(c)
    }).await? > 0 {
        return render_relationship(&db, &account, std::borrow::Cow::Borrowed(&followed_account))
            .await.map(rocket::serde::json::Json);
    }

    let mut relationship =
        render_relationship(&db, &account, std::borrow::Cow::Borrowed(&followed_account)).await?;
    relationship.following = !followed_account.locked;
    relationship.requested = followed_account.locked;

    let following_id = uuid::Uuid::new_v4();
    let created = Utc::now();

    crate::db_run(&db, move |c| -> QueryResult<_> {
        diesel::insert_into(crate::schema::following::dsl::following).values(
            crate::models::NewFollowing {
                id: following_id.clone(),
                follower: account.id,
                followee: followed_account.id,
                created_at: created.naive_utc(),
                pending: true
            }
        ).execute(c)
    }).await?;

    match celery.send_task(
        super::super::tasks::relationships::follow_account::new(following_id, account, followed_account, created)
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    Ok(rocket::serde::json::Json(relationship))
}

#[post("/api/v1/accounts/<account_id>/unfollow")]
pub async fn unfollow_account(
    db: crate::DbConn, user: super::oauth::TokenClaims, account_id: String,
    celery: &rocket::State<crate::CeleryApp>
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, rocket::http::Status> {
    if !user.has_scope("write:follows") {
        return Err(rocket::http::Status::Forbidden);
    }

    let account = get_account(&db, &user).await?;
    let followed_account = get_account_from_db(&account_id, &db).await?;

    let following = match crate::db_run(&db, move |c| -> QueryResult<_> {
        crate::schema::following::dsl::following.filter(
            crate::schema::following::dsl::follower.eq(&account.id).and(
                crate::schema::following::dsl::followee.eq(followed_account.id)
            )
        ).get_result::<crate::models::Following>(c).optional()
    }).await? {
        Some(f) => f,
        None => return render_relationship(&db, &account, std::borrow::Cow::Borrowed(&followed_account))
            .await.map(rocket::serde::json::Json)
    };

    let mut relationship =
        render_relationship(&db, &account, std::borrow::Cow::Borrowed(&followed_account)).await?;
    relationship.following = false;
    relationship.requested = false;

    match celery.send_task(
        super::super::tasks::relationships::unfollow_account::new(following, account, followed_account)
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    Ok(rocket::serde::json::Json(relationship))
}

#[derive(FromForm)]
pub struct NoteForm<'a> {
    comment: Option<&'a str>,
}

#[post("/api/v1/accounts/<account_id>/note", data = "<form>")]
pub async fn note(
    db: crate::DbConn, user: super::oauth::TokenClaims, account_id: String,
    form: Option<rocket::form::Form<NoteForm<'_>>>
) -> Result<rocket::serde::json::Json<super::objs::Relationship>, rocket::http::Status> {
    if !user.has_scope("write:accounts") {
        return Err(rocket::http::Status::Forbidden);
    }

    let note = match form {
        Some(f) => match f.comment {
            Some(n) => {
                let n = n.trim();
                if n.is_empty() {
                    None
                } else {
                    Some(n.to_string())
                }
            },
            None => None
        }
        None => None,
    };

    let account = get_account(&db, &user).await?;
    let note_account = get_account_from_db(&account_id, &db).await?;

    match note {
        Some(note) => {
            crate::db_run(&db, move |c| -> QueryResult<_> {
                diesel::insert_into(crate::schema::account_notes::dsl::account_notes).values(
                    crate::models::AccountNote {
                        account: note_account.id,
                        owner: account.id,
                        note: note.clone(),
                    }
                ).on_conflict(
                    (crate::schema::account_notes::dsl::account, crate::schema::account_notes::dsl::owner)
                ).do_update().set(
                    crate::schema::account_notes::dsl::note.eq(note)
                ).execute(c)
            }).await?;
        },
        None => {
            crate::db_run(&db, move |c| -> QueryResult<_> {
                diesel::delete(
                    crate::schema::account_notes::dsl::account_notes.filter(
                        crate::schema::account_notes::dsl::account.eq(note_account.id)
                    ).filter(
                        crate::schema::account_notes::dsl::owner.eq(account.id)
                    )
                ).execute(c)
            }).await?;
        }
    }

    let relationship = render_relationship(&db, &account, std::borrow::Cow::Borrowed(&note_account)).await?;

    Ok(rocket::serde::json::Json(relationship))
}