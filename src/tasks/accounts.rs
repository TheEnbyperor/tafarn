use crate::models;
use crate::views::activity_streams;
use celery::prelude::*;
use diesel::prelude::*;
use itertools::Itertools;
use futures::stream::StreamExt;
use super::{resolve_url, resolve_object, fetch_object};

async fn fetch_image(img: &activity_streams::ReferenceOrObject<activity_streams::ImageOrLink>) -> Option<(String, String, String)> {
    let avatar = resolve_object(img.clone()).await?;
    if let activity_streams::ImageOrLink::Image(image) = avatar {
        let url = image.url?;
        let content_type = image.media_type?;
        let format = image::ImageFormat::from_mime_type(&content_type)?;
        if format != image::ImageFormat::Png && format != image::ImageFormat::Jpeg &&
            format != image::ImageFormat::Gif {
            warn!("Unsupported image format: {}", content_type);
            return None;
        }
        let url = match url {
            activity_streams::URLOrLink::URL(url) => url,
            activity_streams::URLOrLink::Link(l) => l.href?,
        };
        match crate::AS_CLIENT.get(&url).send().await {
            Ok(r) => match r.error_for_status() {
                Ok(r) => match r.bytes().await {
                    Ok(b) => {
                        let image_id = uuid::Uuid::new_v4();
                        let image_name = format!("{}.{}", image_id.to_string(), format.extensions_str()[0]);
                        let image_path = format!("./media/{}", image_name);
                        match std::fs::write(&image_path, &b) {
                            Ok(_) => {
                                Some((image_name, url, content_type))
                            },
                            Err(e) => {
                                error!("Unable to write avatar file: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Unable to fetch avatar \"{}\": {}", url, e);
                        None
                    }
                }
                Err(e) => {
                    warn!("Unable to fetch avatar f\"{}\": {}", url, e);
                    None
                }
            }
            Err(e) => {
                warn!("Unable to fetch avatar \"{}\": {}", url, e);
                None
            }
        }
    } else {
        None
    }
}

async fn _update_account(
    object: activity_streams::Object, new_account: bool, follow_graph: bool,
) -> TaskResult<models::Account> {
    let db = super::config().db.clone();
    let is_bot = matches!(object, activity_streams::Object::Service(_) | activity_streams::Object::Application(_));
    let is_group = matches!(object, activity_streams::Object::Group(_));
    match object {
        activity_streams::Object::Person(ref a) |
        activity_streams::Object::Service(ref a) |
        activity_streams::Object::Organization(ref a) |
        activity_streams::Object::Application(ref a) |
        activity_streams::Object::Group(ref a) => {
            let account: Option<models::Account> = if new_account {
                None
            } else {
                tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    crate::schema::accounts::dsl::accounts.filter(
                        crate::schema::accounts::dsl::actor.eq(&a.common.id)
                    ).get_result(&c).optional().with_expected_err(|| "Unable to fetch account")
                })?
            };

            let shared_inbox = match &a.endpoints {
                Some(e) => resolve_object(e.clone()).await.and_then(|e| e.shared_inbox),
                None => None
            };
            let avatar = match &a.common.icon {
                Some(i) => fetch_image(i).await,
                None => None
            };
            let header = match &a.common.image {
                Some(i) => fetch_image(i).await,
                None => None
            };

            let new_account = match account {
                Some(mut existing_account) => {
                    if existing_account.local {
                        warn!("Account \"{}\" is local, ignoring update", existing_account.id);
                        return Ok(existing_account);
                    }
                    existing_account.actor = a.common.id.clone();
                    existing_account.bot = is_bot;
                    existing_account.group = is_group;
                    existing_account.display_name = a.common.name.clone().unwrap_or(existing_account.display_name);
                    existing_account.username = a.preferred_username.clone().unwrap_or(existing_account.username);
                    existing_account.bio = a.common.summary.clone().unwrap_or(existing_account.bio);
                    existing_account.inbox_url = Some(a.inbox.clone());
                    existing_account.outbox_url = Some(a.outbox.clone());
                    existing_account.locked = a.manually_approves_followers.unwrap_or_default();
                    existing_account.created_at = a.common.published.map(|p| p.naive_utc()).unwrap_or(existing_account.created_at);
                    existing_account.url = a.common.url.clone().and_then(resolve_url).or(existing_account.url);
                    existing_account.locked = a.manually_approves_followers.unwrap_or(existing_account.locked);
                    existing_account.shared_inbox_url = shared_inbox;

                    if let Some((file, url, format)) = avatar {
                        existing_account.avatar_file = Some(file);
                        existing_account.avatar_content_type = Some(format);
                        existing_account.avatar_remote_url = Some(url);
                    }
                    if let Some((file, url, format)) = header {
                        existing_account.header_file = Some(file);
                        existing_account.header_content_type = Some(format);
                        existing_account.header_remote_url = Some(url);
                    }

                    tokio::task::block_in_place(|| -> TaskResult<_> {
                        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                        diesel::update(crate::schema::accounts::dsl::accounts.find(existing_account.id))
                            .set(&existing_account)
                            .execute(&c).with_expected_err(|| "Unable to update account")
                    })?;

                    existing_account
                },
                None => {
                    let mut new_account = models::NewAccount {
                        id: uuid::Uuid::new_v4(),
                        actor: a.common.id.clone(),
                        username: a.preferred_username.clone().unwrap_or_default(),
                        display_name: a.common.name.clone().unwrap_or_default(),
                        bio: a.common.summary.clone().unwrap_or_default(),
                        locked: a.manually_approves_followers.unwrap_or_default(),
                        bot: is_bot,
                        group: is_group,
                        created_at: a.common.published.map(|p| p.naive_utc()).unwrap_or_else(|| chrono::Utc::now().naive_utc()),
                        updated_at: chrono::Utc::now().naive_utc(),
                        default_sensitive: None,
                        default_language: None,
                        discoverable: None,
                        follower_count: 0,
                        following_count: 0,
                        statuses_count: 0,
                        owned_by: None,
                        private_key: None,
                        local: false,
                        inbox_url: Some(a.inbox.clone()),
                        outbox_url: Some(a.outbox.clone()),
                        shared_inbox_url: shared_inbox,
                        url: a.common.url.clone().and_then(resolve_url),
                        avatar_file: None,
                        avatar_content_type: None,
                        avatar_remote_url: None,
                        header_file: None,
                        header_content_type: None,
                        header_remote_url: None,
                    };

                    if let Some((file, url, format)) = avatar {
                        new_account.avatar_file = Some(file);
                        new_account.avatar_content_type = Some(format);
                        new_account.avatar_remote_url = Some(url);
                    }
                    if let Some((file, url, format)) = header {
                        new_account.header_file = Some(file);
                        new_account.header_content_type = Some(format);
                        new_account.header_remote_url = Some(url);
                    }

                    tokio::task::block_in_place(|| -> TaskResult<_> {
                        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                        diesel::insert_into(crate::schema::accounts::dsl::accounts)
                            .values(&new_account)
                            .get_result(&c).with_expected_err(|| "Unable to insert account")
                    })?
                }
            };

            for key in a.public_key.as_slice().into_iter() {
                let key = match resolve_object(key.clone()).await {
                    Some(k) => k,
                    None => {
                        warn!("Unable to resolve public key: {:?}", key);
                        continue;
                    }
                };
                let key_id = match key.id {
                    Some(i) => i,
                    None => continue
                };
                match key.owner {
                    Some(o) => match o.id() {
                        Some(i) => if i != new_account.actor.as_deref().unwrap_or_default() {
                            continue
                        },
                        None => continue
                    }
                    None => continue
                }
                let key_pem = match key.public_key_pem {
                    Some(p) => p,
                    None => continue
                };
                let pkey = openssl::pkey::PKey::public_key_from_pem(key_pem.as_bytes())
                    .with_unexpected_err(|| "Unable to parse public key")?;
                let key = models::PublicKey {
                    id: uuid::Uuid::new_v4(),
                    key_id,
                    user_id: new_account.id,
                    key: String::from_utf8(
                        pkey.public_key_to_pem().with_unexpected_err(|| "Unable to serialize public key")?
                    ).unwrap(),
                };
                tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    diesel::insert_into(crate::schema::public_keys::dsl::public_keys)
                        .values(&key)
                        .execute(&c).with_expected_err(|| "Unable to insert public key")
                })?;
            }

            let mut pvs = vec![];
            for attachment in a.common.attachment.as_slice().into_iter() {
                let attachment = match match resolve_object(attachment.clone()).await {
                    Some(a) => a,
                    None => {
                        warn!("Unable to resolve attachment: {:?}", attachment);
                        continue;
                    }
                } {
                    activity_streams::ObjectOrLink::Object(o) => o,
                    activity_streams::ObjectOrLink::Link(l) => {
                        warn!("Attachment is a link: {:?}", l);
                        continue;
                    }
                };
                match attachment {
                    activity_streams::Object::PropertyValue(pv) => {
                        pvs.push((pv.name.unwrap_or_default(), pv.value.unwrap_or_default()));
                    },
                    o => {
                        warn!("Account attachment is unsupported: {:?}", o);
                        continue;
                    }
                }
            }
            tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                c.transaction::<(), diesel::result::Error, _>(|| {
                    diesel::delete(crate::schema::account_fields::table.filter(
                        crate::schema::account_fields::dsl::account_id.eq(new_account.id)
                    )).execute(&c)?;

                    diesel::insert_into(crate::schema::account_fields::table)
                        .values(pvs.into_iter().enumerate().map(|(i, f)| models::AccountField {
                            id: uuid::Uuid::new_v4(),
                            account_id: new_account.id,
                            name: f.0,
                            value: f.1,
                            sort_order: i as i32
                        }).collect::<Vec<_>>())
                        .execute(&c)?;
                    Ok(())
                }).with_expected_err(|| "Unable to update account fields")
            })?;

            if follow_graph {
                let celery = super::config().celery;
                celery.send_task(
                    update_account_relations::new(new_account.clone(), a.followers.clone(), a.following.clone())
                ).await.with_expected_err(|| "Unable to send task")?;
            }

            Ok(new_account)
        }
        o => Err(TaskError::UnexpectedError(format!("Invalid object: {:?}", o)))
    }
}

struct CollectionStream {
    collection: activity_streams::Collection,
    page_cache: Vec<activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>>,
    next_page: Option<activity_streams::ReferenceOrObject<activity_streams::CollectionPageOrLink>>,
    resolve_fut: Option<futures::future::BoxFuture<'static, Option<activity_streams::CollectionPageOrLink>>>,
    fetch_link_fut: Option<futures::future::BoxFuture<'static, Option<activity_streams::CollectionPage>>>,
}

impl CollectionStream {
    fn poll_fetch_link(
        mut self: std::pin::Pin<&mut Self>, cx: &mut futures::task::Context<'_>
    ) -> futures::task::Poll<Option<activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>>> {
        match std::future::Future::poll(self.fetch_link_fut.as_mut().unwrap().as_mut(), cx) {
            futures::task::Poll::Ready(page) => {
                if let Some(page) = page {
                    self.next_page = page.next;
                    if let Some(items) = page.common.items {
                        self.page_cache.extend(items);
                    }
                    futures::task::Poll::Ready(self.page_cache.pop())
                } else {
                    warn!("Unable to fetch collection page");
                    futures::task::Poll::Ready(None)
                }
            }
            futures::task::Poll::Pending => futures::task::Poll::Pending
        }
    }

    fn poll_resolve_object(
        mut self: std::pin::Pin<&mut Self>, cx: &mut futures::task::Context<'_>
    ) -> futures::task::Poll<Option<activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>>> {
        match std::future::Future::poll(self.resolve_fut.as_mut().unwrap().as_mut(), cx) {
            futures::task::Poll::Ready(page) => {
                self.resolve_fut = None;
                match page {
                    Some(o) => {
                        let page = match match o {
                            activity_streams::CollectionPageOrLink::Link(l) => {
                                if let Some(l) = l.href {
                                    println!("Fetching {}", l);
                                    let fut = fetch_object(l);
                                    self.fetch_link_fut = Some(Box::pin(fut));
                                    return self.poll_fetch_link(cx);
                                } else {
                                    None
                                }
                            }
                            activity_streams::CollectionPageOrLink::CollectionPage(o) |
                            activity_streams::CollectionPageOrLink::OrderedCollectionPage(o) => {
                                Some(o)
                            }
                        } {
                            Some(p) => p,
                            None => {
                                warn!("Unable to fetch collection page");
                                return futures::task::Poll::Ready(None);
                            }
                        };
                        self.next_page = page.next;
                        if let Some(items) = page.common.items {
                            self.page_cache.extend(items);
                        }
                        futures::task::Poll::Ready(self.page_cache.pop())
                    }
                    None => {
                        warn!("Unable to resolve object: {:?}", self.next_page);
                        futures::task::Poll::Ready(None)
                    }
                }
            }
            futures::task::Poll::Pending => futures::task::Poll::Pending
        }
    }
}

impl futures::stream::Stream for CollectionStream {
    type Item = activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut futures::task::Context<'_>) -> futures::task::Poll<Option<Self::Item>> {
        if let Some(item) = self.page_cache.pop() {
            futures::task::Poll::Ready(Some(item))
        } else {
            if self.fetch_link_fut.is_some() {
                self.poll_fetch_link(cx)
            } else if self.resolve_fut.is_some() {
                self.poll_resolve_object(cx)
            } else {
                if let Some(obj) = &self.next_page {
                    println!("Resolving object: {:?}", obj);
                    self.resolve_fut = Some(Box::pin(resolve_object(obj.clone())));
                    self.poll_resolve_object(cx)
                } else {
                    futures::task::Poll::Ready(None)
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.page_cache.len(), self.collection.total_items.map(|t| t as usize))
    }
}

fn fetch_entire_collection(
    collection: activity_streams::Object
) -> TaskResult<CollectionStream> {
    match collection {
        activity_streams::Object::Collection(c) |
        activity_streams::Object::OrderedCollection(c) => {
            if let Some(items) = &c.items {
                let items = items.clone();
                Ok(CollectionStream {
                    next_page: None,
                    page_cache: items,
                    collection: c,
                    resolve_fut: None,
                    fetch_link_fut: None,
                })
            } else if let Some(first) = &c.first {
                Ok(CollectionStream {
                    next_page: Some(first.clone()),
                    page_cache: vec![],
                    collection: c,
                    resolve_fut: None,
                    fetch_link_fut: None,
                })
            } else {
                Ok(CollectionStream {
                    next_page: None,
                    page_cache: vec![],
                    collection: c,
                    resolve_fut: None,
                    fetch_link_fut: None,
                })
            }
        }
        o => Err(TaskError::UnexpectedError(format!("Not a collection: {:?}", o)))
    }
}

#[derive(Hash, Debug, Eq)]
enum NonMatchingOption<T> {
    None,
    Some(T)
}

impl<T: PartialEq> PartialEq for NonMatchingOption<T> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            NonMatchingOption::None => false,
            NonMatchingOption::Some(s) => match other {
                NonMatchingOption::None => false,
                NonMatchingOption::Some(o) => s == o
            }
        }
    }
}

impl<T> From<Option<T>> for NonMatchingOption<T> {
    fn from(o: Option<T>) -> Self {
        match o {
            Some(o) => NonMatchingOption::Some(o),
            None => NonMatchingOption::None
        }
    }
}

#[celery::task]
pub async fn update_account_relations(
    account: models::Account, followers: Option<String>, following: Option<String>
) -> TaskResult<()> {
    let mut account = account;
    let db = super::config().db.clone();

    let mut accounts = std::collections::HashMap::<String, models::Account>::new();

    let has_followers = followers.is_some();
    let has_following = following.is_some();
    let (followers, following) = match (followers, following) {
        (None, None) => (None, None),
        (Some(followers), None) =>
            (fetch_object(followers).await, None),
        (None, Some(following)) =>
            (None, fetch_object(following).await),
        (Some(followers), Some(following)) =>
            futures::future::join(fetch_object(followers), fetch_object(following)).await,
    };

    if has_followers && followers.is_none() {
        return Err(TaskError::ExpectedError("Unable to fetch followers".to_string()))
    }
    if has_following && following.is_none() {
        return Err(TaskError::ExpectedError("Unable to fetch following".to_string()))
    }

    let mut to_fetch = vec![];

    let followers = match followers {
        Some(c) => {
            let cs = fetch_entire_collection(c)?;
            if let Some(t) = cs.collection.total_items {
                account.follower_count = t as i32;
            }
            Some(cs)
        },
        None => None
    };
    let following = match following {
        Some(c) => {
            let cs = fetch_entire_collection(c)?;
            if let Some(t) = cs.collection.total_items {
                account.following_count = t as i32;
            }
            Some(cs)
        },
        None => None
    };

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let con = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        diesel::update(crate::schema::accounts::dsl::accounts.find(account.id))
            .set(&account)
            .execute(&con).with_expected_err(|| "Unable to update account")
    })?;

    let (followers, following) = match (followers, following) {
        (None, None) => (None, None),
        (Some(followers), None) =>
            (Some(followers.collect::<Vec<_>>().await), None),
        (None, Some(following)) =>
            (None, Some(following.collect::<Vec<_>>().await)),
        (Some(followers), Some(following)) => {
            let (p1, p2) = futures::future::join(followers.collect::<Vec<_>>(), following.collect::<Vec<_>>()).await;
            (Some(p1), Some(p2))
        },
    };

    let followers = if let Some(page) = followers {
        let f = page.iter().filter_map(|i| i.id().map(|s| s.to_string())).collect::<Vec<_>>();
        to_fetch.extend(page);
        Some(f)
    } else {
        None
    };
    let following = if let Some(page) = following {
        let f = page.iter().filter_map(|i| i.id().map(|s| s.to_string())).collect::<Vec<_>>();
        to_fetch.extend(page);
        Some(f)
    } else {
        None
    };

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let con = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        diesel::update(crate::schema::accounts::dsl::accounts.find(account.id))
            .set(&account)
            .execute(&con).with_expected_err(|| "Unable to update account")
    })?;

    let to_fetch = to_fetch.into_iter()
        .unique_by(|i| NonMatchingOption::from(
            i.id().map(|s| s.to_string())
        ));

    let mut s = futures::stream::iter(to_fetch)
        .map(|item| async move {
            (item.id().map(|s| s.to_string()), find_account(item, false).await)
        })
        .buffer_unordered(10);
    while let Some((id, account)) = s.next().await {
        match account {
            Ok(account) =>if let Some(id) = id {
                accounts.insert(id.to_string(), account);
            },
            Err(e) => {
                log::error!("Unable to fetch account {:?}: {}", id, e);
            }
        }
    }

    tokio::task::block_in_place(|| -> TaskResult<_> {
        let con = db.get().with_expected_err(|| "Unable to get DB pool connection")?;

        if let Some(followers) = followers {
            for follower in followers {
                if let Some(follower_account) = accounts.get(&follower) {
                    diesel::insert_into(crate::schema::following::dsl::following)
                        .values(models::NewFollowing {
                            id: uuid::Uuid::new_v4(),
                            follower: follower_account.id,
                            followee: account.id,
                            created_at: chrono::Utc::now().naive_utc(),
                            pending: false
                        })
                        .on_conflict_do_nothing()
                        .execute(&con).with_expected_err(|| "Unable to insert following")?;
                }
            }
        }

        if let Some(following) = following {
            for follow in following {
                if let Some(followed_account) = accounts.get(&follow) {
                    diesel::insert_into(crate::schema::following::dsl::following)
                        .values(models::NewFollowing {
                            id: uuid::Uuid::new_v4(),
                            follower: account.id,
                            followee: followed_account.id,
                            created_at: chrono::Utc::now().naive_utc(),
                            pending: false
                        })
                        .on_conflict_do_nothing()
                        .execute(&con).with_expected_err(|| "Unable to insert following")?;
                }
            }
        }

        Ok(())
    })?;

    Ok(())
}

pub async fn update_account_from_url(
    account: String, follow_graph: bool,
) -> TaskResult<models::Account> {
    let object: activity_streams::Object = match fetch_object(account.clone()).await {
        Some(o) => o,
        None => return Err(TaskError::ExpectedError(format!("Error fetching object {}", account)))
    };

    _update_account(object, false, follow_graph).await
}

#[celery::task]
pub async fn update_account(
    account: String, no_graph: bool
) -> TaskResult<models::Account> {
    update_account_from_url(account, !no_graph).await
}

#[celery::task]
pub async fn update_account_from_object(
    account: activity_streams::Object, no_graph: bool
) -> TaskResult<models::Account> {
    _update_account(account, false,!no_graph).await
}

#[celery::task]
pub async fn update_accounts(no_graph: bool) -> TaskResult<()> {
    let db = super::config().db.clone();

    let accounts: Vec<models::Account> = tokio::task::block_in_place(|| -> TaskResult<_> {
        let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
        crate::schema::accounts::dsl::accounts
            .filter(crate::schema::accounts::dsl::local.eq(false))
            .load(&c).with_expected_err(|| "Unable to fetch accounts")
    })?;

    futures::future::join_all(
        accounts.into_iter().map(|account| update_account_from_url(account.actor.unwrap(), !no_graph))
    ).await;
    Ok(())
}

pub async fn find_account(
    activity: activity_streams::ReferenceOrObject<activity_streams::ObjectOrLink>,
    follow_graph: bool
) -> TaskResult<models::Account> {
    let config = super::config();
    let db = config.db.clone();

    let object = match activity {
        activity_streams::ReferenceOrObject::Object(o) => match *o {
            activity_streams::ObjectOrLink::Object(o) => activity_streams::ReferenceOrObject::Object(Box::new(o)),
            activity_streams::ObjectOrLink::Link(l) => activity_streams::ReferenceOrObject::Reference(match l.href {
                Some(l) => l,
                None => {
                    return Err(TaskError::UnexpectedError(format!("Actor link does not have href: {:?}", l)));
                }
            })
        },
        activity_streams::ReferenceOrObject::Reference(r) => activity_streams::ReferenceOrObject::Reference(r)
    };

    match object {
        activity_streams::ReferenceOrObject::Reference(r) => {
            let local_regex = regex::Regex::new(&format!("https://{}/as/users/(?P<id>.+)", config.uri)).unwrap();
            if let Some(cap) = local_regex.captures(&r) {
                let id = cap.name("id").unwrap().as_str();
                let id = match uuid::Uuid::parse_str(id) {
                    Ok(id) => id,
                    Err(e) => {
                        return Err(TaskError::UnexpectedError(format!("Unable to parse UUID: {}", e)));
                    }
                };
                let account: models::Account = tokio::task::block_in_place(|| -> TaskResult<_> {
                    let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                    crate::schema::accounts::dsl::accounts.filter(
                        crate::schema::accounts::dsl::id.eq(id)
                    ).get_result(&c).with_expected_err(|| "Unable to fetch account")
                })?;
                return Ok(account);
            }

            let account: Option<models::Account> = tokio::task::block_in_place(|| -> TaskResult<_> {
                let c = db.get().with_expected_err(|| "Unable to get DB pool connection")?;
                crate::schema::accounts::dsl::accounts.filter(
                    crate::schema::accounts::dsl::actor.eq(&r)
                ).get_result(&c).optional().with_expected_err(|| "Unable to fetch account")
            })?;

            if let Some(account) = account {
                Ok(account)
            } else {
                let object: activity_streams::Object = match fetch_object(r.clone()).await {
                    Some(o) => o,
                    None => return Err(TaskError::ExpectedError(format!("Error fetching object {}", r)))
                };

                _update_account(object, true, follow_graph).await
            }
        }
        activity_streams::ReferenceOrObject::Object(o) => {
            _update_account(*o, false, follow_graph).await
        }
    }
}