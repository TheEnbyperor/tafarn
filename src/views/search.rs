use diesel::prelude::*;
use chrono::prelude::*;

lazy_static! {
    static ref WEBFINGER_RE: regex::Regex = regex::Regex::new("@?(?P<acct>.+@(?P<domain>.+))").unwrap();
}

#[get("/api/v2/search?<q>&<limit>&<offset>&<resolve>&<following>&<type>")]
pub async fn search(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>,
    user: Option<super::oauth::TokenClaims>, q: String, limit: Option<u64>, offset: Option<u64>,
    following: Option<&str>, resolve: Option<&str>, r#type: Option<&str>
) -> Result<rocket::serde::json::Json<super::objs::Search>, rocket::http::Status> {
    let limit = limit.unwrap_or(20);
    if limit > 50 {
        return Err(rocket::http::Status::BadRequest);
    }
    let following = super::parse_bool(following, false)?;
    let resolve = super::parse_bool(resolve, false)?;

    if resolve {
        if user.is_none() {
            return Err(rocket::http::Status::Forbidden);
        }

        if let Some((domain, q)) = if let Ok(url) = url::Url::parse(&q) {
            url.domain().map(|d| (d.to_string(), url.to_string()))
        } else if let Some(cap) = WEBFINGER_RE.captures(&q) {
            Some((
                     cap.name("domain").unwrap().as_str().to_string(),
                     cap.name("acct").unwrap().as_str().to_string()
            ))
        } else {
            None
        } {
            let url = format!("https://{}/.well-known/webfinger?resource={}", domain, q);
            if let Ok(res) = crate::AS_CLIENT.get(&url).send().await {
                if let Ok(jrd) = res.json::<crate::views::meta::JRD>().await {
                    if let Some(actor) = jrd.links.into_iter()
                        .filter(|l| l.rel == "self")
                        .find(|l| l.type_.as_deref() == Some("application/activity+json"))
                        .map(|l| l.href.as_ref().unwrap().clone()) {
                        match crate::tasks::accounts::find_account(
                            super::activity_streams::ReferenceOrObject::Reference(actor), true
                        ).await {
                            Ok(Some(account)) => {
                                return Ok(rocket::serde::json::Json(super::objs::Search {
                                    accounts: vec![super::accounts::render_account(config, &db, account).await?],
                                    hashtags: vec![],
                                    statuses: vec![]
                                }));
                            },
                            Ok(None) => {},
                            Err(e) => {
                                warn!("Error resolving search: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    let account = match &user {
        Some(user) => {
            if !user.has_scope("read:search") {
                return Err(rocket::http::Status::Forbidden);
            }
            Some(super::accounts::get_account(&db, user).await?)
        },
        None => None,
    };
    let only_following = match account {
        Some(account) => if following {
            Some(account.id)
        } else {
            None
        },
        None => if following {
            return Err(rocket::http::Status::Forbidden);
        } else {
            None
        }
    };

    let accounts: Vec<crate::models::Account> = if r#type.is_none() || r#type == Some("accounts") {
        crate::db_run(&db, move |c| -> QueryResult<_> {
            let q = q.replace("%", "\\%").replace("_", "\\_");
            let ilike = format!("%{}%", q);
            let ilike_sort = format!("{}%", q);
            let mut query = crate::schema::accounts::dsl::accounts.filter(
                crate::schema::accounts::dsl::username.ilike(&ilike)
                    .or(crate::schema::accounts::dsl::display_name.ilike(&ilike))
            ).order_by((
                crate::schema::accounts::dsl::username.not_ilike(&ilike_sort),
                crate::schema::accounts::dsl::display_name.not_ilike(&ilike_sort),
            )).limit(limit as i64).into_boxed();
            if let Some(following) = only_following {
                query = query.filter(crate::schema::accounts::dsl::id.eq_any(
                    crate::schema::following::dsl::following.select(
                        crate::schema::following::dsl::followee
                    ).filter(
                        crate::schema::following::dsl::follower.eq(following)
                    )
                ));
            }
            if let Some(offset) = offset {
                query = query.offset(offset as i64);
            }
            query.get_results(c)
        }).await?
    } else {
        vec![]
    };

    Ok(rocket::serde::json::Json(super::objs::Search {
        accounts: futures::future::try_join_all(
            accounts.into_iter().map(|a| super::accounts::render_account(config, &db, a)).collect::<Vec<_>>()
        ).await?,
        hashtags: vec![],
        statuses: vec![],
    }))
}