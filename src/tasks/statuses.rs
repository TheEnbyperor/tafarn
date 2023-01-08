use chrono::prelude::*;
use diesel::prelude::*;
use celery::prelude::*;
use crate::models;
use crate::tasks::{resolve_object_or_link, resolve_url};
use crate::views::activity_streams;

async fn _update_status(
    object: activity_streams::Object, account: models::Account,
) -> TaskResult<models::Status> {
    let config = super::config();
    let db = config.db.clone();

    match object {
        activity_streams::Object::Note(o) => {
            let id = match o.id {
                Some(id) => id,
                None => return Err(TaskError::UnexpectedError(format!("Object has no ID: {:?}", o)))
            };
            let status: Option<models::Status> = tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::statuses::dsl::statuses.filter(
                    crate::schema::statuses::dsl::url.eq(&id)
                ).get_result(&c).optional().with_expected_err(|| "Unable to fetch status")
            })?;

            let to_public = o.to.as_slice().iter().any(|t| match t {
                activity_streams::ReferenceOrObject::Reference(r) => r == "https://www.w3.org/ns/activitystreams#Public",
                _ => false
            });
            let cc_public = o.cc.as_slice().iter().any(|t| match t {
                activity_streams::ReferenceOrObject::Reference(r) => r == "https://www.w3.org/ns/activitystreams#Public",
                _ => false
            });

            let is_new_status = status.is_none();

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
                    existing_status.public = to_public;
                    existing_status.visible = to_public || cc_public;

                    tokio::task::block_in_place(|| -> TaskResult<_> {
                        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                        diesel::update(crate::schema::statuses::dsl::statuses.find(existing_status.id))
                            .set(&existing_status)
                            .execute(&c).with_expected_err(|| "Unable to update status")
                    })?;

                    existing_status
                },
                None => {
                    let new_status = models::NewStatus {
                        id: uuid::Uuid::new_v4(),
                        url: id,
                        uri: o.url.clone().and_then(resolve_url),
                        text: o.content.unwrap_or_default(),
                        created_at: o.published.unwrap_or_else(|| Utc::now()).naive_utc(),
                        updated_at: o.updated.or(o.published).unwrap_or_else(|| Utc::now()).naive_utc(),
                        in_reply_to_id: None,
                        boot_of_id: None,
                        sensitive: false,
                        spoiler_text: o.summary.unwrap_or_default(),
                        language: None,
                        local: false,
                        account_id: account.id,
                        deleted_at: None,
                        edited_at: None,
                        public: to_public,
                        visible: to_public || cc_public
                    };

                    tokio::task::block_in_place(|| -> TaskResult<_> {
                        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                        diesel::insert_into(crate::schema::statuses::dsl::statuses)
                            .values(&new_status)
                            .get_result(&c).with_expected_err(|| "Unable to insert status")
                    })?
                }
            };

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
                            status_id: new_status.id,
                            mention,
                            account: None,
                            account_followers: Some(acc.id)
                        }));
                    } else {
                        if let Some(acc) = super::accounts::find_account(aud, true).await? {
                            return Ok(Some(models::StatusAudience {
                                id: uuid::Uuid::new_v4(),
                                status_id: new_status.id,
                                mention,
                                account: Some(acc.id),
                                account_followers: None
                            }));
                        }
                    }
                }
                Ok(None)
            };

            for aud in o.to.to_vec() {
                if let Some (aud) = resolve_audience(aud, true, db.clone()).await? {
                    audiences.push(aud);
                }
            }
            for aud in o.cc.to_vec() {
                if let Some (aud) = resolve_audience(aud, false, db.clone()).await? {
                    audiences.push(aud);
                }
            }

            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                c.transaction::<(), diesel::result::Error, _>(|| {
                    diesel::delete(crate::schema::status_audiences::table.filter(
                        crate::schema::status_audiences::dsl::status_id.eq(new_status.id)
                    )).execute(&c)?;

                    diesel::insert_into(crate::schema::status_audiences::table)
                        .values(&audiences)
                        .execute(&c)?;
                    Ok(())
                }).with_expected_err(|| "Unable to update status audiences")
            })?;

            if is_new_status {
                config.celery.send_task(
                    insert_into_timelines::new(new_status.clone(), audiences)
                ).await.with_expected_err(|| "Unable to send task")?;
            }

            Ok(new_status)
        },
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

    _update_status(obj, account).await?;
    Ok(())
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
                            account_id: acct.id
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
                                account_id: acct.id
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