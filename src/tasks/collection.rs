use celery::prelude::*;
use crate::views::activity_streams;
use super::{fetch_object, resolve_object};

pub struct CollectionStream {
    pub collection: activity_streams::Collection,
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

pub fn fetch_entire_collection(
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