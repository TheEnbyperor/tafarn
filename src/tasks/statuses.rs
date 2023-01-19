use chrono::prelude::*;
use diesel::prelude::*;
use celery::prelude::*;
use crate::models;
use crate::tasks::{fetch_object, resolve_object, resolve_object_or_link, resolve_url};
use crate::views::activity_streams::{self, ObjectID};
use futures::StreamExt;

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

fn image_format_to_content_type(format: image::ImageFormat) -> &'static str {
    match format {
        image::ImageFormat::Bmp => "image/bmp",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::Ico => "image/x-icon",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Pnm => "image/x-portable-bitmap",
        image::ImageFormat::Tiff => "image/tiff",
        image::ImageFormat::WebP => "image/webp",
        image::ImageFormat::Dds => "image/vnd.ms-dds",
        image::ImageFormat::Avif => "image/avif",
        image::ImageFormat::Tga => "image/x-targa",
        image::ImageFormat::Hdr => "image/vnd.radiance",
        image::ImageFormat::OpenExr => "image/x-exr",
        _ => "application/octet-stream"
    }
}

struct Attachment {
    remote_url: String,
    file_name: String,
    file_path: std::path::PathBuf,
    format: AttachmentFormat,
}

#[non_exhaustive]
enum AttachmentFormat {
    Image(image::ImageFormat),
}

impl AttachmentFormat {
    fn content_type(&self) -> &'static str {
        match self {
            AttachmentFormat::Image(f) => image_format_to_content_type(*f)
        }
    }
}

async fn _download_object(obj: &activity_streams::ObjectOrLink) -> Option<Attachment> {
    let config = super::config();

    match obj {
        activity_streams::ObjectOrLink::Object(activity_streams::Object::Document(doc)) |
        activity_streams::ObjectOrLink::Object(activity_streams::Object::Image(doc)) => {
            let url = doc.url.clone()?;
            let content_type = doc.media_type.as_deref()?;
            let format = image::ImageFormat::from_mime_type(content_type)?;
            if format != image::ImageFormat::Png && format != image::ImageFormat::Jpeg &&
                format != image::ImageFormat::Gif {
                warn!("Unsupported attachment format: {}", content_type);
                return None;
            }
            let url = match url {
                activity_streams::URLOrLink::URL(url) => url,
                activity_streams::URLOrLink::Link(l) => l.href?,
            };
            let url = match reqwest::Url::parse(&url) {
                Ok(url) => url,
                Err(e) => {
                    warn!("Unable to parse URL {}: {}", url, e);
                    return None;
                }
            };
            match super::authenticated_get(url.clone()).await {
                Ok(r) => match r.error_for_status() {
                    Ok(r) => match r.bytes().await {
                        Ok(b) => {
                            let (doc_name, doc_path) = crate::gen_media_path(&config.media_path, format.extensions_str()[0]);
                            match std::fs::write(&doc_path, &b) {
                                Ok(_) => {
                                    Some(Attachment {
                                        remote_url: url.to_string(),
                                        file_name: doc_name,
                                        file_path: doc_path,
                                        format: AttachmentFormat::Image(format),
                                    })
                                }
                                Err(e) => {
                                    error!("Unable to write attachment file: {}", e);
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Unable to fetch attachment \"{}\": {}", url, e);
                            None
                        }
                    }
                    Err(e) => {
                        warn!("Unable to fetch attachment \"{}\": {}", url, e);
                        None
                    }
                }
                Err(e) => {
                    warn!("Unable to fetch attachment \"{}\": {}", url, e);
                    None
                }
            }
        }
        _ => None
    }
}

async fn fetch_attachment(attachment: activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>) -> Option<models::Media> {
    let config = super::config();
    let db = config.db.clone();

    let attachment = resolve_object(attachment).await?;
    let f = _download_object(&attachment).await?;
    if let activity_streams::ObjectOrLink::Object(activity_streams::Object::Document(doc)) |
    activity_streams::ObjectOrLink::Object(activity_streams::Object::Image(doc)) = attachment {
        let (preview_doc_name, preview_format) = if let Some(preview) = doc.preview {
            let preview = resolve_object(preview).await?;
            let doc = _download_object(&preview).await?;
            (Some(doc.file_name), Some(doc.format))
        } else {
            match f.format {
                AttachmentFormat::Image(format) => {
                    let mut image_r = image::io::Reader::open(&f.file_path).ok()?;
                    image_r.set_format(format);
                    if let Some(image) = image_r.decode().ok() {
                        let (doc_name, doc_path) = crate::gen_media_path(&config.media_path, format.extensions_str()[0]);
                        let preview_image = image.thumbnail(crate::PREVIEW_DIMENSION, crate::PREVIEW_DIMENSION);
                        let mut out_image_bytes: Vec<u8> = Vec::new();
                        preview_image.write_to(&mut std::io::Cursor::new(&mut out_image_bytes), image::ImageOutputFormat::Jpeg(80)).ok()?;
                        std::fs::write(&doc_path, &out_image_bytes).ok()?;
                        (Some(doc_name), Some(AttachmentFormat::Image(image::ImageFormat::Jpeg)))
                    } else {
                        (None, None)
                    }
                }
            }
        };


        let new_media = models::Media {
            id: uuid::Uuid::new_v4(),
            media_type: "image".to_string(),
            file: Some(f.file_name),
            content_type: Some(f.format.content_type().to_string()),
            remote_url: Some(f.remote_url.to_string()),
            preview_file: preview_doc_name,
            preview_content_type: preview_format.map(|f| f.content_type().to_string()),
            blurhash: doc.blurhash,
            focus_x: doc.focal_points.map(|f| f.0),
            focus_y: doc.focal_points.map(|f| f.1),
            original_width: doc.width.map(|w| w as i32),
            original_height: doc.height.map(|h| h as i32),
            preview_width: None,
            preview_height: None,
            created_at: doc.published.unwrap_or_else(Utc::now).naive_utc(),
            description: doc.summary,
            owned_by: None,
        };

        tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            diesel::insert_into(crate::schema::media::dsl::media)
                .values(&new_media)
                .execute(&c).with_expected_err(|| "Unable to insert media")
        }).ok()?;

        Some(new_media)
    } else {
        None
    }
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

            let tags = futures::stream::iter(o.tag.to_vec())
                .filter_map(|t| resolve_object_or_link(t))
                .collect::<Vec<_>>().await;

            println!("{:?}", tags);

            let summary = o.summary.as_deref()
                .map(|s| {
                    sanitize_html::sanitize_str(&crate::HTML_RULES, s)
                        .with_unexpected_err(|| "Unable to sanitize summary")
                }).transpose()?;
            let content = o.content.as_deref()
                .map(|s| {
                    sanitize_html::sanitize_str(&crate::HTML_RULES, s)
                        .with_unexpected_err(|| "Unable to sanitize content")
                }).transpose()?;

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
            let created_at = o.published.unwrap_or_else(|| Utc::now());

            let mut mentions = vec![];
            for t in &tags {
                if let activity_streams::Object::Mention(m) = t {
                    if let Some(acct_actor) = &m.href {
                        if let Some(a) = super::accounts::find_account(
                            activity_streams::ReferenceOrObject::Reference(acct_actor.clone()), true
                        ).await? {
                            mentions.push(models::StatusMention {
                                id: uuid::Uuid::new_v4(),
                                status: status_id,
                                account: a.id
                            })
                        }
                    }
                }
            }

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
                    existing_status.text = content.unwrap_or_default();
                    existing_status.created_at = o.published.map(|p| p.naive_utc())
                        .unwrap_or(existing_status.created_at);
                    existing_status.updated_at = o.updated.map(|u| u.naive_utc())
                        .unwrap_or(existing_status.updated_at);
                    existing_status.spoiler_text = summary.unwrap_or_default();
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
                        text: content.unwrap_or_default(),
                        created_at: created_at.naive_utc(),
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
                        spoiler_text: summary.unwrap_or_default(),
                        language: None,
                        local: false,
                        account_id: account.id,
                        deleted_at: None,
                        edited_at: None,
                        public: audiences.to_public,
                        visible: audiences.to_public || audiences.cc_public,
                        text_source: None,
                        spoiler_text_source: None,
                    };


                    let mut media_attachments = vec![];
                    for attachment in o.attachment.to_vec() {
                        if let Some(media) = fetch_attachment(attachment).await {
                            media_attachments.push(models::MediaAttachment {
                                status: new_status.id,
                                media: media.id,
                            })
                        }
                    }

                    tokio::task::block_in_place(|| -> TaskResult<_> {
                        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                        let s = diesel::insert_into(crate::schema::statuses::dsl::statuses)
                            .values(&new_status)
                            .get_result(&c).with_expected_err(|| "Unable to insert status")?;
                        diesel::insert_into(crate::schema::media_attachments::dsl::media_attachments)
                            .values(&media_attachments)
                            .execute(&c).with_expected_err(|| "Unable to insert media attachments")?;
                        Ok(s)
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

                    diesel::delete(crate::schema::status_mentions::table.filter(
                        crate::schema::status_mentions::dsl::status.eq(new_status.id)
                    )).execute(&c)?;

                    diesel::insert_into(crate::schema::status_mentions::table)
                        .values(&mentions)
                        .execute(&c)?;
                    Ok(())
                }).with_expected_err(|| "Unable to update status audiences and mentions")
            })?;

            if is_new_status {
                config.celery.send_task(
                    insert_into_timelines::new(new_status.clone(), audiences.audiences.clone())
                ).await.with_expected_err(|| "Unable to send task")?;
                if let Some(replies) = o.replies {
                    config.celery.send_task(get_replies::new(replies))
                        .await.with_expected_err(|| "Unable to send task")?;
                }

                for aud in &audiences.audiences {
                    if aud.mention {
                        if let Some(account_id) = aud.account {
                            let notification = tokio::task::block_in_place(|| -> TaskResult<_> {
                                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                                diesel::insert_into(crate::schema::notifications::dsl::notifications)
                                    .values(models::NewNotification {
                                        id: uuid::Uuid::new_v4(),
                                        notification_type: "mention".to_string(),
                                        account: account_id,
                                        cause: account.id,
                                        status: Some(new_status.id),
                                        created_at: created_at.naive_utc(),
                                    })
                                    .on_conflict_do_nothing()
                                    .get_result::<models::Notification>(&c).with_expected_err(|| "Unable to insert notification")
                            })?;
                            config.celery.send_task(super::notifications::notify::new(notification))
                                .await.with_expected_err(|| "Unable to submit notification task")?;
                        }
                    }
                }

                for mention in &mentions {
                    let notification = tokio::task::block_in_place(|| -> TaskResult<_> {
                        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                        diesel::insert_into(crate::schema::notifications::dsl::notifications)
                            .values(models::NewNotification {
                                id: uuid::Uuid::new_v4(),
                                notification_type: "mention".to_string(),
                                account: mention.account,
                                cause: account.id,
                                status: Some(new_status.id),
                                created_at: created_at.naive_utc(),
                            })
                            .on_conflict_do_nothing()
                            .get_result::<models::Notification>(&c).with_expected_err(|| "Unable to insert notification")
                    })?;
                    config.celery.send_task(super::notifications::notify::new(notification))
                        .await.with_expected_err(|| "Unable to submit notification task")?;
                }
            }
            Ok(new_status)
        }
        o => Err(TaskError::UnexpectedError(format!("Invalid object, not an status: {:?}", o)))
    }
}

#[celery::task]
pub async fn get_replies(
    collection: activity_streams::ReferenceOrObject<activity_streams::Collection>
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();
    let collection = match resolve_object(collection).await {
        Some(c) => c,
        None => {
            warn!("Unable to resolve replies collection");
            return Ok(());
        }
    };
    let mut cs = super::collection::fetch_entire_collection(activity_streams::Object::Collection(collection))?;

    while let Some(reply) = cs.next().await {
        let id = match reply.id() {
            Some(id) => id,
            None => continue,
        };

        let is_new_status = tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            crate::schema::statuses::dsl::statuses.filter(
                crate::schema::statuses::dsl::url.eq(id)
            ).count().get_result::<i64>(&c).with_expected_err(|| "Unable to fetch status")
        })? == 0;

        if is_new_status {
            let obj = match resolve_object_or_link(reply).await {
                Some(o) => o,
                None => return Ok(())
            };
            _update_status(obj, None, true).await?;
        }
    }

    Ok(())
}

#[celery::task]
pub async fn create_status(
    activity: activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>, account: models::Account,
) -> TaskResult<()> {
    let obj = match resolve_object_or_link(activity).await {
        Some(o) => o,
        None => return Ok(())
    };

    _update_status(obj, Some(account), true).await?;
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
    let created_at = activity.common.published.unwrap_or_else(|| Utc::now());

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
            if let Some(obj) = &boost_of {
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
                created_at: created_at.naive_utc(),
                updated_at: activity.common.updated.or(activity.common.published).unwrap_or_else(|| Utc::now()).naive_utc(),
                in_reply_to_id: None,
                in_reply_to_url: None,
                boost_of_url: if boost_of.is_some() { None } else { Some(boost_of_id) },
                boost_of_id: boost_of.as_ref().map(|s| s.id),
                sensitive: false,
                spoiler_text: "".to_string(),
                language: None,
                local: false,
                account_id: account.id,
                deleted_at: None,
                edited_at: None,
                public: audiences.to_public,
                visible: audiences.to_public || audiences.cc_public,
                text_source: None,
                spoiler_text_source: None,
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

        if let Some(boost_of) = &boost_of {
            if boost_of.local {
                let notification = tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    diesel::insert_into(crate::schema::notifications::dsl::notifications)
                        .values(models::NewNotification {
                            id: uuid::Uuid::new_v4(),
                            notification_type: "reblog".to_string(),
                            account: boost_of.account_id,
                            cause: account.id,
                            status: Some(boost_of.id),
                            created_at: created_at.naive_utc(),
                        })
                        .on_conflict_do_nothing()
                        .get_result::<models::Notification>(&c).with_expected_err(|| "Unable to insert notification")
                })?;
                config.celery.send_task(super::notifications::notify::new(notification))
                    .await.with_expected_err(|| "Unable to submit notification task")?;
            }
        }
    }

    Ok(())
}

#[celery::task]
pub async fn create_like(
    activity: activity_streams::ActivityCommon, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    let id = match &activity.common.id {
        Some(id) => id,
        None => return Err(TaskError::UnexpectedError(format!("Announce has no ID: {:?}", activity)))
    };
    if tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::likes::dsl::likes.filter(
            crate::schema::likes::dsl::url.eq(&id)
        ).count().get_result::<i64>(&c).with_expected_err(|| "Unable to fetch like")
    })? == 0 {
        let like_of_object = match activity.object {
            Some(obj) => obj,
            None => return Err(TaskError::UnexpectedError(format!("Like has no object: {:?}", activity)))
        };
        let like_of_id = match like_of_object.id() {
            Some(o) => o.to_string(),
            None => return Err(TaskError::UnexpectedError(format!("Like object has no ID")))
        };
        let like_of = match get_status(like_of_object).await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("Error fetching like of status: {:?}", e);
                None
            }
        };

        let created_at = activity.common.published.unwrap_or_else(|| Utc::now());
        let new_like = models::NewLike {
            id: uuid::Uuid::new_v4(),
            account: account.id,
            status: like_of.as_ref().map(|s| s.id),
            status_url: if like_of.is_none() { Some(like_of_id) } else { None },
            local: false,
            url: Some(id.to_string()),
            created_at: created_at.naive_utc(),
        };
        tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            diesel::insert_into(crate::schema::likes::dsl::likes)
                .values(&new_like)
                .execute(&c).with_expected_err(|| "Unable to insert like")
        })?;
        if let Some(like_of) = like_of {
            if like_of.local {
                let notification = tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    diesel::insert_into(crate::schema::notifications::dsl::notifications)
                        .values(models::NewNotification {
                            id: uuid::Uuid::new_v4(),
                            notification_type: "favourite".to_string(),
                            account: like_of.account_id,
                            cause: account.id,
                            status: Some(like_of.id),
                            created_at: created_at.naive_utc(),
                        })
                        .on_conflict_do_nothing()
                        .get_result::<models::Notification>(&c).with_expected_err(|| "Unable to insert notification")
                })?;
                config.celery.send_task(super::notifications::notify::new(notification))
                    .await.with_expected_err(|| "Unable to submit notification task")?;
            }
        }
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
pub async fn delete_status_by_id(id: String, account: models::Account) -> TaskResult<()> {
    _delete_status_by_id(id.as_str(), account, Utc::now()).await
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
pub async fn undo_like(
    activity: activity_streams::ActivityCommon, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let db = config.db.clone();

    let id = match &activity.common.id {
        Some(id) => id,
        None => return Err(TaskError::UnexpectedError(format!("Activity has no ID: {:?}", activity)))
    };

    if let Some(like) = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::likes::dsl::likes.filter(
            crate::schema::likes::dsl::url.eq(id)
        ).get_result::<models::Like>(&c).optional().with_expected_err(|| "Unable to fetch status")
    })? {
        if like.local {
            warn!("Like \"{}\" is local, ignoring delete", like.id);
            return Ok(());
        }

        if like.account != account.id {
            warn!("Like \"{}\" is not owned by account \"{}\", ignoring delete", like.id, account.id);
            return Ok(());
        }

        tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            diesel::delete(&like)
                .execute(&c).with_expected_err(|| "Unable to delete like")
        })?;
    }
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
                            account_id: acct.id,
                        })
                        .execute(&c).with_expected_err(|| "Unable to insert into home timeline")?;
                }
                Ok(())
            })?;
        } else if let Some(acct) = aud.account_followers {
            let followers = tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                let mut q = crate::schema::following::dsl::following.filter(
                    crate::schema::following::dsl::followee.eq(acct)
                ).into_boxed();
                if status.boost_of_id.is_some() {
                    q = q.filter(crate::schema::following::dsl::reblogs.eq(true));
                }
                q.get_results::<models::Following>(&c).with_expected_err(|| "Unable to fetch followers")
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

pub struct ASAudiences {
    pub to: Vec<activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>>,
    pub cc: Vec<activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>>,
    audiences: Vec<models::StatusAudience>,
    delivery_accounts: Vec<models::Account>,
}

impl ASAudiences {
    pub fn is_visible(&self) -> bool {
        self.cc.iter().any(|a| a.id() == Some("https://www.w3.org/ns/activitystreams#Public")) ||
            self.to.iter().any(|a| a.id() == Some("https://www.w3.org/ns/activitystreams#Public"))
    }
}

pub async fn make_audiences(status: &models::Status, resolve_delivery: bool) -> TaskResult<ASAudiences> {
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

    let audiences = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        let audiences = crate::schema::status_audiences::dsl::status_audiences.filter(
            crate::schema::status_audiences::dsl::status_id.eq(status.id)
        ).get_results::<models::StatusAudience>(&c).with_expected_err(|| "Unable to get boot audiences")?;

        for aud in &audiences {
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
                continue;
            };

            if aud.mention {
                to.push(activity_streams::ReferenceOrObject::Reference(reference));
            } else {
                cc.push(activity_streams::ReferenceOrObject::Reference(reference));
            }
        }

        Ok(audiences)
    })?;

    Ok(ASAudiences {
        to,
        cc,
        audiences,
        delivery_accounts,
    })
}

pub fn as_render_status(
    status: &models::Status, account: &models::Account, aud: &ASAudiences,
) -> TaskResult<activity_streams::Object> {
    let config = super::config();
    let db = config.db.clone();

    if let Some(deleted_at) = &status.deleted_at {
        return Ok(activity_streams::Object::Tombstone(activity_streams::Tombstone {
            common: activity_streams::ObjectCommon {
                id: Some(status.url(&config.uri)),
                ..Default::default()
            },
            former_type: Some("Note".to_string()),
            deleted: Some(Utc.from_utc_datetime(deleted_at)),
        }));
    }

    let attachments: Vec<(models::MediaAttachment, models::Media)> = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        let attachments = crate::schema::media_attachments::dsl::media_attachments.filter(
            crate::schema::media_attachments::dsl::status.eq(status.id)
        ).inner_join(
            crate::schema::media::table.on(
                crate::schema::media::dsl::id.eq(crate::schema::media_attachments::dsl::media)
            )
        ).get_results(&c).with_expected_err(|| "Unable to get attachments")?;

        Ok(attachments)
    })?;

    Ok(activity_streams::Object::Note(activity_streams::ObjectCommon {
        id: Some(status.url(&config.uri)),
        published: Some(Utc.from_utc_datetime(&status.created_at)),
        updated: Some(Utc.from_utc_datetime(&status.updated_at)),
        to: activity_streams::Pluralisable::List(aud.to.clone()),
        cc: activity_streams::Pluralisable::List(aud.cc.clone()),
        attributed_to: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        sensitive: Some(status.sensitive),
        content: Some(status.text.clone()),
        content_map: status.language.as_ref().map(|l| activity_streams::LanguageMap::from([
            (l.clone(), status.text.clone())
        ])),
        summary: if status.spoiler_text.is_empty() {
            None
        } else {
            Some(status.spoiler_text.clone())
        },
        summary_map: status.language.as_ref().and_then(|l| if status.spoiler_text.is_empty() {
            None
        } else {
            Some(activity_streams::LanguageMap::from([
                (l.clone(), status.spoiler_text.clone())
            ]))
        }),
        attachment: activity_streams::Pluralisable::List(attachments.into_iter()
            .map(|(_, m)| activity_streams::ReferenceOrObject::Object(Box::new(activity_streams::ObjectOrLink::Object(
                activity_streams::Object::Document(activity_streams::ObjectCommon {
                    url: m.file.map(|f| activity_streams::URLOrLink::URL(format!("https://{}/media/{}", config.uri, f))),
                    summary: m.description,
                    blurhash: m.blurhash,
                    width: m.original_width.map(|w| w as u64),
                    height: m.original_height.map(|h| h as u64),
                    media_type: m.content_type,
                    published: Some(Utc.from_utc_datetime(&m.created_at)),
                    focal_points: match (m.focus_x, m.focus_y) {
                        (Some(x), Some(y)) => Some((x, y)),
                        _ => None
                    },
                    preview: m.preview_file.map(|p| activity_streams::ReferenceOrObject::Object(Box::new(
                        activity_streams::ObjectOrLink::Object(activity_streams::Object::Document(activity_streams::ObjectCommon {
                            url: Some(activity_streams::URLOrLink::URL(format!("https://{}/media/{}", config.uri, p))),
                            media_type: m.preview_content_type,
                            width: m.preview_width.map(|w| w as u64),
                            height: m.preview_height.map(|h| h as u64),
                            ..Default::default()
                        }))
                    ))),
                    ..Default::default()
                })
            )))).collect()),
        ..Default::default()
    }))
}

pub fn as_render_status_activity(
    status: &models::Status, account: &models::Account, aud: &ASAudiences,
) -> TaskResult<activity_streams::Object> {
    let config = super::config();

    Ok(activity_streams::Object::Create(activity_streams::ActivityCommon {
        common: activity_streams::ObjectCommon {
            id: Some(status.activity_url(&config.uri).unwrap_or_default()),
            published: Some(Utc.from_utc_datetime(&status.created_at)),
            to: activity_streams::Pluralisable::List(aud.to.clone()),
            cc: activity_streams::Pluralisable::List(aud.cc.clone()),
            ..Default::default()
        },
        actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        object: Some(activity_streams::ReferenceOrObject::Object(Box::new(activity_streams::ObjectOrLink::Object(
            as_render_status(status, account, aud)?
        )))),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    }))
}

#[celery::task]
pub async fn deliver_status(
    status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let aud = make_audiences(&status, true).await?;
    let activity = as_render_status_activity(&status, &account, &aud)?;
    config.celery.send_task(
        insert_into_timelines::new(status, aud.audiences)
    ).await.with_expected_err(|| "Unable to submit timelines task")?;
    super::delivery::deliver_dedupe_inboxes(activity, aud.delivery_accounts, account).await?;
    Ok(())
}

#[celery::task]
pub async fn deliver_status_delete(
    status: models::Status, account: models::Account,
) -> TaskResult<()> {
    if let Some(deleted_at) = &status.deleted_at {
        let config = super::config();
        let aud = make_audiences(&status, true).await?;

        let activity = activity_streams::Object::Delete(activity_streams::ActivityCommon {
            common: activity_streams::ObjectCommon {
                id: Some(format!("https://{}/as/transient/{}", config.uri, uuid::Uuid::new_v4())),
                published: Some(Utc.from_utc_datetime(deleted_at)),
                to: activity_streams::Pluralisable::List(aud.to.clone()),
                cc: activity_streams::Pluralisable::List(aud.cc.clone()),
                ..Default::default()
            },
            actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
            object: Some(activity_streams::ReferenceOrObject::Object(Box::new(activity_streams::ObjectOrLink::Object(
                as_render_status(&status, &account, &aud)?
            )))),
            target: None,
            result: None,
            origin: None,
            instrument: None,
        });

        super::delivery::deliver_dedupe_inboxes(activity, aud.delivery_accounts, account).await?;
    }

    Ok(())
}

pub fn as_render_boost(
    status: &models::Status, boosted_status: &models::Status, account: &models::Account, aud: &ASAudiences,
) -> activity_streams::Object {
    let config = super::config();

    activity_streams::Object::Announce(activity_streams::ActivityCommon {
        common: activity_streams::ObjectCommon {
            id: Some(status.url(&config.uri)),
            published: Some(Utc.from_utc_datetime(&status.created_at)),
            to: activity_streams::Pluralisable::List(aud.to.clone()),
            cc: activity_streams::Pluralisable::List(aud.cc.clone()),
            ..Default::default()
        },
        actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        object: Some(activity_streams::ReferenceOrObject::Reference(boosted_status.url.clone())),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    })
}

#[celery::task]
pub async fn deliver_boost(
    status: models::Status, boosted_status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let aud = make_audiences(&status, true).await?;
    let activity = as_render_boost(&status, &boosted_status, &account, &aud);
    config.celery.send_task(
        insert_into_timelines::new(status, aud.audiences)
    ).await.with_expected_err(|| "Unable to submit timelines task")?;
    super::delivery::deliver_dedupe_inboxes(activity, aud.delivery_accounts, account).await?;
    Ok(())
}

#[celery::task]
pub async fn deliver_undo_boost(
    status: models::Status, boosted_status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();
    let aud = make_audiences(&status, true).await?;
    let boost_activity = as_render_boost(&status, &boosted_status, &account, &aud);

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
            activity_streams::ObjectOrLink::Object(boost_activity)
        ))),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    });

    super::delivery::deliver_dedupe_inboxes(activity, aud.delivery_accounts, account).await?;

    Ok(())
}


pub async fn make_like_audiences(like: &models::Like, liked_status: &models::Status, account: &models::Account, resolve_delivery: bool) -> TaskResult<ASAudiences> {
    let config = super::config();
    let db = config.db.clone();

    let (liked_status_account, aud) =
        tokio::task::block_in_place(|| -> TaskResult<_> {
            let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
            let a = crate::schema::accounts::dsl::accounts.find(liked_status.account_id)
                .get_result::<models::Account>(&c).with_expected_err(|| "Unable to get account")?;
            let f = if resolve_delivery {
                let mut f = crate::schema::accounts::dsl::accounts.filter(
                    crate::schema::accounts::dsl::id.eq_any(
                        crate::schema::following::dsl::following.filter(
                            crate::schema::following::dsl::followee.eq(like.account)
                        ).filter(
                            crate::schema::following::dsl::pending.eq(false)
                        ).select(crate::schema::following::dsl::follower)
                    )
                ).get_results::<models::Account>(&c).with_expected_err(|| "Unable to get followers")?;
                f.push(a.clone());
                f
            } else {
                vec![]
            };
            Ok((a, f))
        })?;

    let to = vec![
        activity_streams::ReferenceOrObject::Reference(liked_status_account.actor_id(&config.uri))
    ];
    let mut cc = vec![
        activity_streams::ReferenceOrObject::Reference(account.follower_collection(&config.uri))
    ];

    if liked_status.visible {
        cc.push(activity_streams::ReferenceOrObject::Reference("https://www.w3.org/ns/activitystreams#Public".to_string()));
    }

    Ok(ASAudiences {
        to,
        cc,
        audiences: vec![],
        delivery_accounts: aud,
    })
}

pub async fn as_render_like(
    like: &models::Like, liked_status: &models::Status, account: &models::Account, aud: &ASAudiences,
) -> TaskResult<activity_streams::Object> {
    let config = super::config();

    Ok(activity_streams::Object::Like(activity_streams::ActivityCommon {
        common: activity_streams::ObjectCommon {
            id: Some(like.url(&config.uri)),
            published: Some(Utc.from_utc_datetime(&like.created_at)),
            to: activity_streams::Pluralisable::List(aud.to.clone()),
            cc: activity_streams::Pluralisable::List(aud.cc.clone()),
            ..Default::default()
        },
        actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        object: Some(activity_streams::ReferenceOrObject::Reference(liked_status.url.clone())),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    }))
}

#[celery::task]
pub async fn deliver_like(
    like: models::Like, liked_status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let aud = make_like_audiences(&like, &liked_status, &account, true).await?;
    let activity = as_render_like(&like, &liked_status, &account, &aud).await?;

    super::delivery::deliver_dedupe_inboxes(activity, aud.delivery_accounts, account).await?;

    Ok(())
}

#[celery::task]
pub async fn deliver_undo_like(
    like: models::Like, liked_status: models::Status, account: models::Account,
) -> TaskResult<()> {
    let config = super::config();

    let aud = make_like_audiences(&like, &liked_status, &account, true).await?;
    let like_activity = as_render_like(&like, &liked_status, &account, &aud).await?;

    let activity = activity_streams::Object::Undo(activity_streams::ActivityCommon {
        common: activity_streams::ObjectCommon {
            id: Some(format!("https://{}/as/transient/{}", config.uri, uuid::Uuid::new_v4())),
            to: activity_streams::Pluralisable::List(aud.to.clone()),
            cc: activity_streams::Pluralisable::List(aud.cc.clone()),
            ..Default::default()
        },
        actor: Some(activity_streams::ReferenceOrObject::Reference(account.actor_id(&config.uri))),
        object: Some(activity_streams::ReferenceOrObject::Object(Box::new(
            activity_streams::ObjectOrLink::Object(like_activity)
        ))),
        target: None,
        result: None,
        origin: None,
        instrument: None,
    });

    super::delivery::deliver_dedupe_inboxes(activity, aud.delivery_accounts, account).await?;

    Ok(())
}