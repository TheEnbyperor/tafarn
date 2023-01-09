use chrono::prelude::*;
use diesel::prelude::*;
use celery::prelude::*;
use crate::models;
use crate::tasks::{fetch_object, resolve_object_or_link, resolve_url};
use crate::views::activity_streams::{self, ObjectID};

pub async fn get_status(status: activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>) -> TaskResult<models::Status> {
    let config = super::config();
    let db = config.db.clone();

    let object = match status {
        activity_streams::ReferenceOrObject::Object(o) => match *o {
            activity_streams::ObjectOrLink::Object(o) => activity_streams::ReferenceOrObject::Object(Box::new(o)),
            activity_streams::ObjectOrLink::Link(l) => activity_streams::ReferenceOrObject::Reference(match l.href {
                Some(l) => l,
                None => {
                    return Err(TaskError::UnexpectedError(format!("Object link does not have href: {:?}", l)));
                }
            })
        },
        activity_streams::ReferenceOrObject::Reference(r) => activity_streams::ReferenceOrObject::Reference(r)
    };

    match object {
        activity_streams::ReferenceOrObject::Reference(r) => {
            let local_regex = regex::Regex::new(&format!("https://{}/as/status/(?P<id>.+)", config.uri)).unwrap();
            if let Some(cap) = local_regex.captures(&r) {
                let id = cap.name("id").unwrap().as_str();
                let id = match uuid::Uuid::parse_str(id) {
                    Ok(id) => id,
                    Err(e) => {
                        return Err(TaskError::UnexpectedError(format!("Unable to parse UUID: {}", e)));
                    }
                };
                let status: models::Status = tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    crate::schema::statuses::dsl::statuses.filter(
                        crate::schema::statuses::dsl::id.eq(id)
                    ).get_result(&c).with_expected_err(|| "Unable to fetch status")
                })?;
                return Ok(status);
            }

            let status: Option<models::Status> = tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::statuses::dsl::statuses.filter(
                    crate::schema::statuses::dsl::url.eq(&r)
                ).get_result(&c).optional().with_expected_err(|| "Unable to fetch status")
            })?;

            if let Some(status) = status {
                Ok(status)
            } else {
                let object: activity_streams::Object = match fetch_object(&r).await {
                    Some(o) => o,
                    None => return Err(TaskError::ExpectedError(format!("Error fetching object {}", r)))
                };

                _update_status(object, None, true).await
            }
        }
        activity_streams::ReferenceOrObject::Object(o) => {
            _update_status(*o, None, false).await
        }
    }
}

struct Audiences {
    to_public: bool,
    cc_public: bool,
    audiences: Vec<models::StatusAudience>,
}

async fn resolve_audiences(object: &activity_streams::ObjectCommon, status_id: uuid::Uuid) -> TaskResult<Audiences> {
    let config = super::config();

    let to_public = object.to.as_slice().iter().any(|t| match t {
        activity_streams::ReferenceOrObject::Reference(r) => r == "https://www.w3.org/ns/activitystreams#Public",
        _ => false
    });
    let cc_public = object.cc.as_slice().iter().any(|t| match t {
        activity_streams::ReferenceOrObject::Reference(r) => r == "https://www.w3.org/ns/activitystreams#Public",
        _ => false
    });

    let mut audiences = vec![];
    let resolve_audience =
        |aud: activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>, mention,
         db: std::sync::Arc<r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>>| async move {
            if let Some(id) = aud.id() {
                if id == "https://www.w3.org/ns/activitystreams#Public" {
                    return Ok(None);
                }
                if let Some(acc) =
                tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    crate::schema::accounts::dsl::accounts.filter(
                        crate::schema::accounts::dsl::follower_collection_url.eq(id)
                    ).get_result::<models::Account>(&c).optional().with_expected_err(|| "Unable to fetch account")
                })? {
                    return Ok(Some(models::StatusAudience {
                        id: uuid::Uuid::new_v4(),
                        status_id,
                        mention,
                        account: None,
                        account_followers: Some(acc.id),
                    }));
                } else {
                    if let Some(acc) = super::accounts::find_account(aud, true).await? {
                        return Ok(Some(models::StatusAudience {
                            id: uuid::Uuid::new_v4(),
                            status_id,
                            mention,
                            account: Some(acc.id),
                            account_followers: None,
                        }));
                    }
                }
            }
            Ok(None)
        };

    for aud in object.to.as_slice() {
        if let Some(aud) = resolve_audience(aud.clone(), true, config.db.clone()).await? {
            audiences.push(aud);
        }
    }
    for aud in object.cc.as_slice() {
        if let Some(aud) = resolve_audience(aud.clone(), false, config.db.clone()).await? {
            audiences.push(aud);
        }
    }

    Ok(Audiences {
        to_public,
        cc_public,
        audiences,
    })
}

#[async_recursion::async_recursion]
async fn _update_status(
    object: activity_streams::Object, account: Option<models::Account>, new_status: bool,
) -> TaskResult<models::Status> {
    let config = super::config();
    let db = config.db.clone();

    match object {
        activity_streams::Object::Note(o) => {
            let id = match &o.id {
                Some(id) => id,
                None => return Err(TaskError::UnexpectedError(format!("Object has no ID: {:?}", o)))
            };
            let status: Option<models::Status> = if new_status {
                None
            } else {
                tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    crate::schema::statuses::dsl::statuses.filter(
                        crate::schema::statuses::dsl::url.eq(id)
                    ).get_result(&c).optional().with_expected_err(|| "Unable to fetch status")
                })?
            };

            let is_new_status = status.is_none();
            let status_id = status.as_ref().map(|s| s.id).unwrap_or_else(|| uuid::Uuid::new_v4());
            let audiences = resolve_audiences(&o, status_id).await?;
            let in_reply_to_id = o.in_reply_to.as_ref()
                .and_then(|r| r.id().map(|s| s.to_string()));
            let in_reply_to = match o.in_reply_to {
                Some(irt) => match get_status(irt).await {
                    Ok(s) => Some(s),
                    Err(e) => {
                        warn!("Error fetching in reply to status: {:?}", e);
                        None
                    }
                },
                None => None,
            };

            let account = match account {
                Some(account) => account,
                None => match o.attributed_to {
                    Some(a) => match super::accounts::find_account(a, true).await? {
                        Some(a) => a,
                        None => return Err(TaskError::UnexpectedError(format!("Unable to find account for object \"{}\"", id)))
                    },
                    None => {
                        return Err(TaskError::UnexpectedError(format!("Object \"{}\" has no attributed_to", id)));
                    }
                }
            };

            let new_status = match status {
                Some(mut existing_status) => {
                    if existing_status.local {
                        warn!("Status \"{}\" is local, ignoring update", existing_status.id);
                        return Ok(existing_status);
                    }

                    if existing_status.account_id != account.id {
                        warn!("Status \"{}\" is not owned by account \"{}\", ignoring update", existing_status.id, account.id);
                        return Ok(existing_status);
                    }

                    existing_status.uri = o.url.clone().and_then(resolve_url);
                    existing_status.text = o.content.unwrap_or_default();
                    existing_status.created_at = o.published.map(|p| p.naive_utc())
                        .unwrap_or(existing_status.created_at);
                    existing_status.updated_at = o.updated.map(|u| u.naive_utc())
                        .unwrap_or(existing_status.updated_at);
                    existing_status.spoiler_text = o.summary.unwrap_or_default();
                    existing_status.public = audiences.to_public;
                    existing_status.visible = audiences.to_public || audiences.cc_public;
                    existing_status.in_reply_to_url = if in_reply_to.is_some() {
                        None
                    } else {
                        in_reply_to_id
                    };
                    existing_status.in_reply_to_id = in_reply_to.map(|s| s.id);

                    tokio::task::block_in_place(|| -> TaskResult<_> {
                        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                        diesel::update(crate::schema::statuses::dsl::statuses.find(existing_status.id))
                            .set(&existing_status)
                            .execute(&c).with_expected_err(|| "Unable to update status")
                    })?;

                    existing_status
                }
                None => {
                    let new_status = models::NewStatus {
                        id: status_id,
                        url: id.to_string(),
                        uri: o.url.clone().and_then(resolve_url),
                        text: o.content.unwrap_or_default(),
                        created_at: o.published.unwrap_or_else(|| Utc::now()).naive_utc(),
                        updated_at: o.updated.or(o.published).unwrap_or_else(|| Utc::now()).naive_utc(),
                        in_reply_to_url: if in_reply_to.is_some() {
                            None
                        } else {
                            in_reply_to_id
                        },
                        in_reply_to_id: in_reply_to.map(|s| s.id),
                        boost_of_id: None,
                        boost_of_url: None,
                        sensitive: false,
                        spoiler_text: o.summary.unwrap_or_default(),
                        language: None,
                        local: false,
                        account_id: account.id,
                        deleted_at: None,
                        edited_at: None,
                        public: audiences.to_public,
                        visible: audiences.to_public || audiences.cc_public,
                    };

                    tokio::task::block_in_place(|| -> TaskResult<_> {
                        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                        diesel::insert_into(crate::schema::statuses::dsl::statuses)
                            .values(&new_status)
                            .get_result(&c).with_expected_err(|| "Unable to insert status")
                    })?
                }
            };

            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                c.transaction::<(), diesel::result::Error, _>(|| {
                    diesel::delete(crate::schema::status_audiences::table.filter(
                        crate::schema::status_audiences::dsl::status_id.eq(new_status.id)
                    )).execute(&c)?;

                    diesel::insert_into(crate::schema::status_audiences::table)
                        .values(&audiences.audiences)
                        .execute(&c)?;
                    Ok(())
                }).with_expected_err(|| "Unable to update status audiences")
            })?;

            if is_new_status {
                config.celery.send_task(
                    insert_into_timelines::new(new_status.clone(), audiences.audiences)
                ).await.with_expected_err(|| "Unable to send task")?;
            }

            Ok(new_status)
        }
        o => Err(TaskError::UnexpectedError(format!("Invalid object, not an status: {:?}", o)))
    }
}

#[celery::task]
pub async fn create_status(
    activity: activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>, account: models::Account,
) -> TaskResult<()> {
    let obj = match resolve_object_or_link(activity).await {
        Some(o) => o,
        None => return Ok(())
    };

    _update_status(obj, Some(account), false).await?;
    Ok(())
}

#[celery::task]
pub async fn create_announce(
    activity: activity_streams::ActivityCommon, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    let id = match &activity.common.id {
        Some(id) => id,
        None => return Err(TaskError::UnexpectedError(format!("Announce has no ID: {:?}", activity)))
    };
    let status: Option<models::Status> = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::url.eq(&id)
        ).get_result(&c).optional().with_expected_err(|| "Unable to fetch status")
    })?;

    let is_new_status = status.is_none();
    let status_id = status.as_ref().map(|s| s.id).unwrap_or_else(|| uuid::Uuid::new_v4());
    let audiences = resolve_audiences(&activity.common, status_id).await?;
    let boost_of_object = match activity.object {
        Some(obj) => obj,
        None => return Err(TaskError::UnexpectedError(format!("Announce has no object: {:?}", activity)))
    };
    let boost_of_id = match boost_of_object.id() {
        Some(o) => o.to_string(),
        None => return Err(TaskError::UnexpectedError(format!("Announce object has no ID")))
    };
    let boost_of = match get_status(boost_of_object).await {
        Ok(s) => Some(s),
        Err(e) => {
            warn!("Error fetching boost of status: {:?}", e);
            None
        }
    };

    let new_status = match status {
        Some(mut existing_status) => {
            if existing_status.local {
                warn!("Status \"{}\" is local, ignoring update", existing_status.id);
                return Ok(());
            }

            if existing_status.account_id != account.id {
                warn!("Status \"{}\" is not owned by account \"{}\", ignoring update", existing_status.id, account.id);
                return Ok(());
            }

            existing_status.created_at = activity.common.published.map(|p| p.naive_utc())
                .unwrap_or(existing_status.created_at);
            existing_status.updated_at = activity.common.updated.map(|u| u.naive_utc())
                .unwrap_or(existing_status.updated_at);
            existing_status.public = audiences.to_public;
            existing_status.visible = audiences.to_public || audiences.cc_public;
            if let Some(obj) = boost_of {
                existing_status.boost_of_id = Some(obj.id);
                existing_status.boost_of_url = None;
            }

            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                diesel::update(crate::schema::statuses::dsl::statuses.find(existing_status.id))
                    .set(&existing_status)
                    .execute(&c).with_expected_err(|| "Unable to update status")
            })?;

            existing_status
        }
        None => {
            let new_status = models::NewStatus {
                id: status_id,
                url: id.to_string(),
                uri: None,
                text: "".to_string(),
                created_at: activity.common.published.unwrap_or_else(|| Utc::now()).naive_utc(),
                updated_at: activity.common.updated.or(activity.common.published).unwrap_or_else(|| Utc::now()).naive_utc(),
                in_reply_to_id: None,
                in_reply_to_url: None,
                boost_of_url: if boost_of.is_some() { None } else { Some(boost_of_id) },
                boost_of_id: boost_of.map(|s| s.id),
                sensitive: false,
                spoiler_text: "".to_string(),
                language: None,
                local: false,
                account_id: account.id,
                deleted_at: None,
                edited_at: None,
                public: audiences.to_public,
                visible: audiences.to_public || audiences.cc_public,
            };

            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                diesel::insert_into(crate::schema::statuses::dsl::statuses)
                    .values(&new_status)
                    .get_result(&c).with_expected_err(|| "Unable to insert status")
            })?
        }
    };

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        c.transaction::<(), diesel::result::Error, _>(|| {
            diesel::delete(crate::schema::status_audiences::table.filter(
                crate::schema::status_audiences::dsl::status_id.eq(new_status.id)
            )).execute(&c)?;

            diesel::insert_into(crate::schema::status_audiences::table)
                .values(&audiences.audiences)
                .execute(&c)?;
            Ok(())
        }).with_expected_err(|| "Unable to update status audiences")
    })?;

    if is_new_status {
        config.celery.send_task(
            insert_into_timelines::new(new_status.clone(), audiences.audiences)
        ).await.with_expected_err(|| "Unable to send task")?;
    }

    Ok(())
}

async fn _delete_status_by_id(id: &str, account: models::Account, deleted: DateTime<Utc>) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    if let Some(mut status) = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::statuses::dsl::statuses.filter(
            crate::schema::statuses::dsl::url.eq(id)
        ).get_result::<models::Status>(&c).optional().with_expected_err(|| "Unable to fetch status")
    })? {
        if status.local {
            warn!("Status \"{}\" is local, ignoring delete", status.id);
            return Ok(());
        }

        if status.account_id != account.id {
            warn!("Status \"{}\" is not owned by account \"{}\", ignoring delete", status.id, account.id);
            return Ok(());
        }

        status.deleted_at = Some(deleted.naive_utc());

        tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            diesel::update(crate::schema::statuses::dsl::statuses.find(status.id))
                .set(&status)
                .execute(&c).with_expected_err(|| "Unable to update status")
        })?;
    }
    Ok(())
}

#[celery::task]
pub async fn delete_status(
    tombstone: activity_streams::Tombstone, account: models::Account,
) -> TaskResult<()> {
    let id = match &tombstone.common.id {
        Some(id) => id,
        None => return Err(TaskError::UnexpectedError(format!("Tombstone has no ID: {:?}", tombstone)))
    };
    _delete_status_by_id(id.as_str(), account, tombstone.deleted.unwrap_or_else(Utc::now)).await
}

#[celery::task]
pub async fn undo_announce(
    activity: activity_streams::ActivityCommon, account: models::Account,
) -> TaskResult<()> {
    let id = match &activity.common.id {
        Some(id) => id,
        None => return Err(TaskError::UnexpectedError(format!("Activity has no ID: {:?}", activity)))
    };
    _delete_status_by_id(id.as_str(), account, activity.common.published.unwrap_or_else(Utc::now)).await
}

#[celery::task]
pub async fn insert_into_timelines(
    status: models::Status, audiences: Vec<models::StatusAudience>,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    if status.public {
        tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            diesel::insert_into(crate::schema::public_timeline::table)
                .values(models::NewPublicTimelineEntry {
                    status_id: status.id
                })
                .execute(&c).with_expected_err(|| "Unable to insert into public timeline")
        })?;
    }

    for aud in audiences {
        if let Some(acct) = aud.account {
            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                let acct = crate::schema::accounts::dsl::accounts.find(acct)
                    .get_result::<models::Account>(&c).with_expected_err(|| "Unable to fetch account")?;
                if acct.local {
                    diesel::insert_into(crate::schema::home_timeline::table)
                        .values(models::NewHomeTimelineEntry {
                            status_id: status.id,
                            account_id: acct.id,
                        })
                        .execute(&c).with_expected_err(|| "Unable to insert into home timeline")?;
                }
                Ok(())
            })?;
        } else if let Some(acct) = aud.account_followers {
            let followers = tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::following::dsl::following.filter(
                    crate::schema::following::dsl::followee.eq(acct)
                ).get_results::<models::Following>(&c).with_expected_err(|| "Unable to fetch followers")
            })?;
            for follower in followers {
                tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    let acct = crate::schema::accounts::dsl::accounts.find(follower.follower)
                        .get_result::<models::Account>(&c).with_expected_err(|| "Unable to fetch account")?;
                    if acct.local {
                        diesel::insert_into(crate::schema::home_timeline::table)
                            .values(models::NewHomeTimelineEntry {
                                status_id: status.id,
                                account_id: acct.id,
                            })
                            .execute(&c).with_expected_err(|| "Unable to insert into home timeline")?;
                    }
                    Ok(())
                })?;
            }
        }
    }

    Ok(())
}

struct ASAudiences {
    to: Vec<activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>>,
    cc: Vec<activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>>,
    delivery_accounts: Vec<models::Account>
}

async fn make_audiences(status: &models::Status, resolve_delivery: bool) -> TaskResult<ASAudiences> {
    let config = super::config();
    let db = config.db.clone();

    let mut to = vec![];
    let mut cc = vec![];
    let mut delivery_accounts = vec![];

    if status.public {
        to.push(activity_streams::ReferenceOrObject::Reference("https://www.w3.org/ns/activitystreams#Public".to_string()));
    } else if status.visible {
        cc.push(activity_streams::ReferenceOrObject::Reference("https://www.w3.org/ns/activitystreams#Public".to_string()));
    }

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        let audiences = crate::schema::status_audiences::dsl::status_audiences.filter(
            crate::schema::status_audiences::dsl::status_id.eq(status.id)
        ).get_results::<models::StatusAudience>(&c).with_expected_err(|| "Unable to get boot audiences")?;

        for aud in audiences {
            let reference = if let Some(acct) = aud.account {
                let a = crate::schema::accounts::dsl::accounts.find(acct)
                    .get_result::<models::Account>(&c)
                    .with_expected_err(|| "Unable to get account")?;
                let r = a.actor_id(&config.uri);
                if resolve_delivery {
                    delivery_accounts.push(a);
                }
                r
            } else if let Some(acct) = aud.account_followers {
                let a = crate::schema::accounts::dsl::accounts.find(acct)
                    .get_result::<models::Account>(&c)
                    .with_expected_err(|| "Unable to get account")?;
                let r = a.follower_collection(&config.uri);
                if resolve_delivery {
                    let f = crate::schema::following::dsl::following.filter(
                        crate::schema::following::dsl::followee.eq(a.id)
                    ).filter(
                        crate::schema::following::dsl::pending.eq(false)
                    ).inner_join(
                        crate::schema::accounts::table.on(
                            crate::schema::accounts::dsl::id.eq(crate::schema::following::dsl::follower)
                        )
                    ).get_results::<(models::Following, models::Account)>(&c)
                        .with_expected_err(|| "Unable to get account followers")?.into_iter().map(|f| f.1);
                    delivery_accounts.extend(f.into_iter());
                }
                r
            } else {
                continue
            };

            if aud.mention {
                to.push(activity_streams::ReferenceOrObject::Reference(reference));
            } else {
                cc.push(activity_streams::ReferenceOrObject::Reference(reference));
            }
        }

        Ok(())
    })?;

    Ok(ASAudiences {
        to,
        cc,
        delivery_accounts
    })
}

#[celery::task]
pub async fn deliver_boost(
    status: models::Status, boosted_status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let aud = make_audiences(&status, true).await?;

    let activity = activity_streams::Object::Announce(activity_streams::ActivityCommon {
        common: activity_streams::ObjectCommon {
            id: Some(status.url(&config.uri)),
            published: Some(Utc.from_utc_datetime(&status.created_at)),
            to: activity_streams::Pluralisable::List(aud.to),
            cc: activity_streams::Pluralisable::List(aud.cc),
            ..Default::default()
        },
        actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        object: Some(activity_streams::ReferenceOrObject::Reference(boosted_status.url)),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    });

    super::delivery::deliver_dedupe_inboxes(activity, aud.delivery_accounts, account).await?;

    Ok(())
}

#[celery::task]
pub async fn deliver_undo_boost(
    status: models::Status, boosted_status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let aud = make_audiences(&status, true).await?;

    let activity = activity_streams::Object::Undo(activity_streams::ActivityCommon {
        common: activity_streams::ObjectCommon {
            id: Some(format!("https://{}/as/transient/{}", config.uri, uuid::Uuid::new_v4())),
            to: activity_streams::Pluralisable::List(aud.to.clone()),
            cc: activity_streams::Pluralisable::List(aud.cc.clone()),
            published: status.deleted_at.as_ref().map(|d| Utc.from_utc_datetime(d)),
            ..Default::default()
        },
        actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        object: Some(activity_streams::ReferenceOrObject::Object(Box::new(
            activity_streams::ObjectOrLink::Object(activity_streams::Object::Announce(activity_streams::ActivityCommon {
                common: activity_streams::ObjectCommon {
                    id: Some(status.url(&config.uri)),
                    published: Some(Utc.from_utc_datetime(&status.created_at)),
                    to: activity_streams::Pluralisable::List(aud.to),
                    cc: activity_streams::Pluralisable::List(aud.cc),
                    ..Default::default()
                },
                actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
                object: Some(activity_streams::ReferenceOrObject::Reference(boosted_status.url)),
                target: None,
                result: None,
                origin: None,
                instrument: None,
            }))
        ))),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    });

    super::delivery::deliver_dedupe_inboxes(activity, aud.delivery_accounts, account).await?;

    Ok(())
}

#[celery::task]
pub async fn deliver_like(
    like: models::Like, liked_status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    let liked_status_account = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::accounts::dsl::accounts.find(liked_status.account_id)
            .get_result::<models::Account>(&c).with_expected_err(|| "Unable to get account")
    })?;

    let mut aud = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::accounts::dsl::accounts.filter(
            crate::schema::accounts::dsl::id.eq_any(
                crate::schema::following::dsl::following.filter(
                    crate::schema::following::dsl::followee.eq(account.id)
                ).select(crate::schema::following::dsl::follower)
            )
        ).get_results::<models::Account>(&c).with_expected_err(|| "Unable to get followers")
    })?;
    aud.push(liked_status_account.clone());

    let activity = activity_streams::Object::Like(activity_streams::ActivityCommon {
        common: activity_streams::ObjectCommon {
            id: Some(like.url(&config.uri)),
            published: Some(Utc.from_utc_datetime(&like.created_at)),
            to: activity_streams::Pluralisable::Object(activity_streams::ReferenceOrObject::Reference(
                liked_status_account.actor_id(&config.uri)
            )),
            cc: activity_streams::Pluralisable::Object(activity_streams::ReferenceOrObject::Reference(
                account.follower_collection(&config.uri)
            )),
            ..Default::default()
        },
        actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        object: Some(activity_streams::ReferenceOrObject::Reference(liked_status.url)),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    });

    super::delivery::deliver_dedupe_inboxes(activity, aud, account).await?;

    Ok(())
}

#[celery::task]
pub async fn deliver_undo_like(
    like: models::Like, liked_status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    let liked_status_account = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::accounts::dsl::accounts.find(liked_status.account_id)
            .get_result::<models::Account>(&c).with_expected_err(|| "Unable to get account")
    })?;

    let mut aud = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::accounts::dsl::accounts.filter(
            crate::schema::accounts::dsl::id.eq_any(
                crate::schema::following::dsl::following.filter(
                    crate::schema::following::dsl::followee.eq(account.id)
                ).select(crate::schema::following::dsl::follower)
            )
        ).get_results::<models::Account>(&c).with_expected_err(|| "Unable to get followers")
    })?;
    aud.push(liked_status_account.clone());

    let activity = activity_streams::Object::Undo(activity_streams::ActivityCommon {
        common: activity_streams::ObjectCommon {
            id: Some(format!("https://{}/as/transient/{}", config.uri, uuid::Uuid::new_v4())),
            to: activity_streams::Pluralisable::Object(activity_streams::ReferenceOrObject::Reference(
                liked_status_account.actor_id(&config.uri)
            )),
            cc: activity_streams::Pluralisable::Object(activity_streams::ReferenceOrObject::Reference(
                account.follower_collection(&config.uri)
            )),
            ..Default::default()
        },
        actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        object: Some(activity_streams::ReferenceOrObject::Object(Box::new(
            activity_streams::ObjectOrLink::Object(activity_streams::Object::Like(activity_streams::ActivityCommon {
                common: activity_streams::ObjectCommon {
                    id: Some(like.url(&config.uri)),
                    published: Some(Utc.from_utc_datetime(&like.created_at)),
                    to: activity_streams::Pluralisable::Object(activity_streams::ReferenceOrObject::Reference(
                        liked_status_account.actor_id(&config.uri)
                    )),
                    cc: activity_streams::Pluralisable::Object(activity_streams::ReferenceOrObject::Reference(
                        account.follower_collection(&config.uri)
                    )),
                    ..Default::default()
                },
                actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
                object: Some(activity_streams::ReferenceOrObject::Reference(liked_status.url)),
                target: None,
                result: None,
                origin: None,
                instrument: None,
            }))
        ))),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    });

    super::delivery::deliver_dedupe_inboxes(activity, aud, account).await?;

    Ok(())
}