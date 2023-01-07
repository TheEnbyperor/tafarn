use crate::models;
use crate::views::activity_streams;
use celery::prelude::*;
use chrono::prelude::*;
use diesel::prelude::*;
use crate::views::activity_streams::ObjectID;

#[celery::task]
pub async fn process_follow(
    activity: activity_streams::ActivityCommon, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();
    let object = match &activity.object {
        Some(o) => o,
        None => {
            warn!("Follow activity \"{}\" has no object", activity.id_or_default());
            return Ok(());
        }
    };
    let followed_account = super::accounts::find_account(object.clone(), false).await?;
    let created_at = activity.common.published.unwrap_or_else(|| Utc::now());

    if followed_account.local {
        if let Some(inbox) = account.inbox_url {
            let celery = super::config().celery;
            let a = followed_account.clone();
            let task = super::delivery::deliver_object::new(activity_streams::Object::Accept(activity_streams::ActivityCommon {
                common: activity_streams::ObjectCommon {
                    id: Some(format!("https://{}/as/transient/{}", config.uri, uuid::Uuid::new_v4())),
                    published: Some(created_at),
                    ..Default::default()
                },
                actor: followed_account.actor.map(activity_streams::ReferenceOrObject::Reference),
                object: Some(activity_streams::ReferenceOrObject::Object(
                    Box::new(activity_streams::ObjectOrLink::Object(
                        activity_streams::Object::Follow(activity.clone())
                    ))
                )),
                target: None,
                result: None,
                origin: None,
                instrument: None,
            }), inbox, a);
            celery.send_task(task).await.with_expected_err(|| "Unable to submit delivery task")?;
        } else {
            warn!("Account \"{}\" has no inbox URL", account.id);
        }

        tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            diesel::insert_into(crate::schema::notifications::dsl::notifications)
                .values(models::NewNotification {
                    id: uuid::Uuid::new_v4(),
                    notification_type: "follow".to_string(),
                    account: followed_account.id,
                    cause: account.id,
                    status: None,
                    created_at: created_at.naive_utc(),
                })
                .on_conflict_do_nothing()
                .execute(&c).with_expected_err(|| "Unable to insert following")?;
            diesel::insert_into(crate::schema::following::dsl::following)
                .values(models::NewFollowing {
                    id: uuid::Uuid::new_v4(),
                    follower: account.id,
                    followee: followed_account.id,
                    created_at: created_at.naive_utc(),
                    pending: false,
                })
                .on_conflict_do_nothing()
                .execute(&c).with_expected_err(|| "Unable to insert following")?;
            Ok(())
        })?;
    } else {
        info!("Follow activity \"{}\" has non-local object {:?}", activity.id_or_default(), object);
    }

    Ok(())
}

#[celery::task]
pub async fn process_undo_follow(
    activity: activity_streams::ActivityCommon, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();
    let object = match &activity.object {
        Some(o) => o,
        None => {
            warn!("Undo follow activity \"{}\" has no object", activity.id_or_default());
            return Ok(());
        }
    };
    let followed_account = super::accounts::find_account(object.clone(), false).await?;

    if !followed_account.local {
        info!("Undo follow activity \"{}\" has non-local object {:?}", activity.id_or_default(), object);
    }

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        diesel::delete(crate::schema::following::dsl::following
            .filter(
                crate::schema::following::dsl::follower.eq(account.id)
                    .and(crate::schema::following::dsl::followee.eq(followed_account.id))
            ))
            .execute(&c).with_expected_err(|| "Unable to delete following")?;
        Ok(())
    })?;

    Ok(())
}

#[celery::task]
pub async fn follow_account(
    following_id: uuid::Uuid, follower: models::Account, followee: models::Account, created_at: DateTime<Utc>,
) -> TaskResult<()> {
    let config = super::config();

    if follower.local {
        if let Some(inbox) = followee.inbox_url {
            let celery = super::config().celery;
            let a = follower.clone();
            let task = super::delivery::deliver_object::new(activity_streams::Object::Follow(activity_streams::ActivityCommon {
                common: activity_streams::ObjectCommon {
                    id: Some(format!("https://{}/as/follow/{}", config.uri, following_id)),
                    published: Some(created_at),
                    ..Default::default()
                },
                actor: follower.actor.map(activity_streams::ReferenceOrObject::Reference),
                object: followee.actor.map(activity_streams::ReferenceOrObject::Reference),
                target: None,
                result: None,
                origin: None,
                instrument: None,
            }), inbox, a);
            celery.send_task(task).await.with_expected_err(|| "Unable to submit delivery task")?;
        } else {
            warn!("Account \"{}\" has no inbox URL", followee.id);
        }
    } else {
        warn!("Account \"{}\" is not local, not generating follow activity", follower.id);
    }
    Ok(())
}

#[celery::task]
pub async fn unfollow_account(
    following: models::Following, follower: models::Account, followee: models::Account
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    if follower.local {
        if let Some(inbox) = followee.inbox_url {
            let celery = super::config().celery;
            let a = follower.clone();
            let actor = follower.actor.map(activity_streams::ReferenceOrObject::Reference);
            let task = super::delivery::deliver_object::new(activity_streams::Object::Undo(activity_streams::ActivityCommon {
                common: activity_streams::ObjectCommon {
                    id: Some(format!("https://{}/as/transient/{}", config.uri, uuid::Uuid::new_v4())),
                    ..Default::default()
                },
                actor: actor.clone(),
                object: Some(activity_streams::ReferenceOrObject::Object(Box::new(
                    activity_streams::ObjectOrLink::Object(activity_streams::Object::Follow(activity_streams::ActivityCommon {
                        common: activity_streams::ObjectCommon {
                            id: Some(format!("https://{}/as/follow/{}", config.uri, following.id)),
                            published: Some(Utc.from_utc_datetime(&following.created_at)),
                            ..Default::default()
                        },
                        actor,
                        object: followee.actor.map(activity_streams::ReferenceOrObject::Reference),
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
            }), inbox, a);
            celery.send_task(task).await.with_expected_err(|| "Unable to submit delivery task")?;
        } else {
            warn!("Account \"{}\" has no inbox URL", followee.id);
        }
    } else {
        warn!("Account \"{}\" is not local, not generating unfollow activity", follower.id);
    }

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        diesel::delete(crate::schema::following::dsl::following
            .filter(
                crate::schema::following::dsl::follower.eq(follower.id)
                    .and(crate::schema::following::dsl::followee.eq(followee.id))
            ))
            .execute(&c).with_expected_err(|| "Unable to delete following")?;
        Ok(())
    })?;

    Ok(())
}

#[celery::task]
pub async fn process_accept_follow(
    activity: activity_streams::ActivityCommon, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();
    let actor = match &activity.actor {
        Some(o) => o,
        None => {
            warn!("Accept follow activity \"{}\" has no object", activity.id_or_default());
            return Ok(());
        }
    };
    let object = match &activity.object {
        Some(o) => o,
        None => {
            warn!("Accept ollow activity \"{}\" has no object", activity.id_or_default());
            return Ok(());
        }
    };
    let following_account = super::accounts::find_account(actor.clone(), false).await?;
    let followed_account = super::accounts::find_account(object.clone(), false).await?;

    if !following_account.local {
        info!("Accept follow activity \"{}\" has non-local actor {:?}", activity.id_or_default(), object);
    }

    if account.id != followed_account.id {
        warn!("Accept follow activity \"{}\" has inconsistent actor", activity.id_or_default());
        return Ok(());
    }

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        diesel::update(crate::schema::following::dsl::following.filter(
            crate::schema::following::dsl::followee.eq(account.id)
                .and(crate::schema::following::dsl::follower.eq(following_account.id))
        )).set(crate::schema::following::dsl::pending.eq(false))
            .execute(&c).with_expected_err(|| "Unable to update following")?;
        Ok(())
    })?;

    Ok(())
}

#[celery::task]
pub async fn process_reject_follow(
    activity: activity_streams::ActivityCommon, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();
    let actor = match &activity.actor {
        Some(o) => o,
        None => {
            warn!("Reject follow activity \"{}\" has no actor", activity.id_or_default());
            return Ok(());
        }
    };
    let object = match &activity.object {
        Some(o) => o,
        None => {
            warn!("Reject follow activity \"{}\" has no object", activity.id_or_default());
            return Ok(());
        }
    };
    let following_account = super::accounts::find_account(actor.clone(), false).await?;
    let followed_account = super::accounts::find_account(object.clone(), false).await?;

    if !following_account.local {
        info!("Reject follow activity \"{}\" has non-local actor {:?}", activity.id_or_default(), object);
    }

    if account.id != followed_account.id {
        warn!("Reject follow activity \"{}\" has inconsistent actor", activity.id_or_default());
        return Ok(())
    }

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        diesel::delete(crate::schema::following::dsl::following
            .filter(
                crate::schema::following::dsl::followee.eq(account.id)
                    .and(crate::schema::following::dsl::follower.eq(following_account.id))
            ))
            .execute(&c).with_expected_err(|| "Unable to insert following")?;
        Ok(())
    })?;

    Ok(())
}