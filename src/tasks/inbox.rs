use crate::views::activity_streams;
use crate::views::activity_streams::ObjectID;
use celery::prelude::*;
use diesel::prelude::*;
use super::{resolve_object_or_link};

#[celery::task]
pub async fn process_activity(activity: activity_streams::Object, signature: activity_streams::Signature) -> TaskResult<()> {
    let db = super::config().db.clone();
    match &activity {
        activity_streams::Object::Accept(a) |
        activity_streams::Object::TentativeAccept(a) |
        activity_streams::Object::Add(a) |
        activity_streams::Object::Arrive(a) |
        activity_streams::Object::Create(a) |
        activity_streams::Object::Delete(a) |
        activity_streams::Object::Follow(a) |
        activity_streams::Object::Ignore(a) |
        activity_streams::Object::Join(a) |
        activity_streams::Object::Leave(a) |
        activity_streams::Object::Like(a) |
        activity_streams::Object::Offer(a) |
        activity_streams::Object::Invite(a) |
        activity_streams::Object::Reject(a) |
        activity_streams::Object::TentativeReject(a) |
        activity_streams::Object::Remove(a) |
        activity_streams::Object::Undo(a) |
        activity_streams::Object::Update(a) |
        activity_streams::Object::View(a) |
        activity_streams::Object::Listen(a) |
        activity_streams::Object::Read(a) |
        activity_streams::Object::Move(a) |
        activity_streams::Object::Travel(a) |
        activity_streams::Object::Announce(a) |
        activity_streams::Object::Block(a) |
        activity_streams::Object::Flag(a) |
        activity_streams::Object::Dislike(a) |
        activity_streams::Object::Question(a) => {
            let actor = match &a.actor {
                Some(a) => a.clone(),
                None => {
                    warn!("Activity \"{}\" has no actor", a.id_or_default());
                    return Ok(());
                }
            };

            let account = super::accounts::find_account(actor, true).await?;

            if account.local {
                warn!("Activity \"{}\" has local actor \"{}\"", a.id_or_default(), a.id_or_default());
                return Ok(());
            }

            let public_key: crate::models::PublicKey = match tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::public_keys::dsl::public_keys.filter(
                    crate::schema::public_keys::dsl::key_id.eq(&signature.key_id)
                ).get_result(&c).optional().with_expected_err(|| "Unable to fetch public key")
            })? {
                Some(k) => k,
                None => {
                    warn!("Activity \"{}\" has unknown public key \"{}\"", a.id_or_default(), signature.key_id);
                    return Ok(());
                }
            };
            let pkey = openssl::pkey::PKey::public_key_from_pem(public_key.key.as_bytes()).with_unexpected_err(|| "Unable to parse public key")?;

            if !signature.verify(&pkey) {
                warn!("Activity \"{}\" signature verification failed with key \"{}\"", a.id_or_default(), signature.key_id);
                return Ok(());
            }

            let celery = super::config().celery;
            match activity {
                activity_streams::Object::Follow(a) => {
                    celery.send_task(
                        super::relationships::process_follow::new(a.clone(), account)
                    ).await.with_expected_err(|| "Unable to send task")?;
                }
                activity_streams::Object::Update(a) => {
                    match &a.object {
                        Some(o) => {
                            let o = match resolve_object_or_link(o.clone()).await {
                                Some(o) => o,
                                None => {
                                    return Err(TaskError::ExpectedError(format!("Unable to resolve object {:?}", o)));
                                }
                            };
                            if matches!(
                                o,
                                activity_streams::Object::Person(_) |
                                activity_streams::Object::Service(_) |
                                activity_streams::Object::Organization(_) |
                                activity_streams::Object::Application(_) |
                                activity_streams::Object::Group(_)
                            ) {
                                celery.send_task(
                                    super::accounts::update_account_from_object::new(o, false)
                                ).await.with_expected_err(|| "Unable to send task")?;
                            } else {
                                warn!("Object does not support update: {:?}", a);
                            }
                        }
                        None => {
                            warn!("Undo activity does not have object: {:?}", a);
                        }
                    }
                }
                activity_streams::Object::Undo(a) => {
                    match &a.object {
                        Some(o) => {
                            match resolve_object_or_link(o.clone()).await {
                                Some(activity_streams::Object::Follow(a)) => {
                                    celery.send_task(
                                        super::relationships::process_undo_follow::new(a.clone(), account)
                                    ).await.with_expected_err(|| "Unable to send task")?;
                                }
                                Some(_) => {
                                    warn!("Object does not support undo: {:?}", a);
                                }
                                None => {
                                    return Err(TaskError::ExpectedError(format!("Unable to resolve object {:?}", o)));
                                }
                            }
                        }
                        None => {
                            warn!("Undo activity does not have object: {:?}", a);
                        }
                    }
                }
                activity_streams::Object::Accept(a) => {
                    match &a.object {
                        Some(o) => {
                            match resolve_object_or_link(o.clone()).await {
                                Some(activity_streams::Object::Follow(a)) => {
                                    celery.send_task(
                                        super::relationships::process_accept_follow::new(a.clone(), account)
                                    ).await.with_expected_err(|| "Unable to send task")?;
                                }
                                Some(_) => {
                                    warn!("Activity does not support accept: {:?}", a);
                                }
                                None => {
                                    return Err(TaskError::ExpectedError(format!("Unable to resolve object {:?}", o)));
                                }
                            }
                        }
                        None => {
                            warn!("Accept activity does not have object: {:?}", a);
                        }
                    }
                }
                activity_streams::Object::Reject(a) => {
                    match &a.object {
                        Some(o) => {
                            match resolve_object_or_link(o.clone()).await {
                                Some(activity_streams::Object::Follow(a)) => {
                                    celery.send_task(
                                        super::relationships::process_reject_follow::new(a.clone(), account)
                                    ).await.with_expected_err(|| "Unable to send task")?;
                                }
                                Some(_) => {
                                    warn!("Activity does not support reject: {:?}", a);
                                }
                                None => {
                                    return Err(TaskError::ExpectedError(format!("Unable to resolve object {:?}", o)));
                                }
                            }
                        }
                        None => {
                            warn!("Reject activity does not have object: {:?}", a);
                        }
                    }
                }
                a => warn!("Activity is not supported: {:?}", a)
            }

            Ok(())
        }
        _ => {
            warn!("Unknown activity type: {:?}", activity);
            Ok(())
        }
    }
}