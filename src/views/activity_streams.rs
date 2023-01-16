use crate::AppConfig;
use diesel::prelude::*;
use chrono::prelude::*;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
enum DigestAlgorithm {
    SHA512,
    SHA256,
    SHA1,
    MD5,
}

impl DigestAlgorithm {
    fn from_str(from: &str) -> Option<Self> {
        match from {
            "SHA-512" => Some(Self::SHA512),
            "SHA-256" => Some(Self::SHA256),
            "SHA-1" => Some(Self::SHA1),
            "MD5" => Some(Self::MD5),
            _ => None
        }
    }
}

pub trait ObjectID {
    fn id(&self) -> Option<&str>;

    fn id_or_default(&self) -> &str {
        self.id().unwrap_or("<no ID>")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ReferenceOrObject<T> {
    Reference(String),
    Object(Box<T>),
}

impl<T: ObjectID> ObjectID for ReferenceOrObject<T> {
    fn id(&self) -> Option<&str> {
        match self {
            ReferenceOrObject::Object(o) => o.id(),
            ReferenceOrObject::Reference(r) => Some(r),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Pluralisable<T> {
    Object(T),
    List(Vec<T>),
    #[serde(skip_deserializing)]
    None,
}

impl<T> Default for Pluralisable<T> {
    fn default() -> Self {
        Self::List(vec![])
    }
}

impl<T> Pluralisable<T> {
    fn is_none(&self) -> bool {
        match self {
            Self::Object(_) => false,
            Self::List(l) => l.is_empty(),
            Self::None => true,
        }
    }

    pub fn to_vec(self) -> Vec<T> {
        match self {
            Self::Object(o) => vec![o],
            Self::List(l) => l,
            Self::None => vec![],
        }
    }

    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::Object(o) => std::slice::from_ref(o),
            Self::List(l) => l.as_slice(),
            Self::None => &[],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Object {
    Object(ObjectCommon),
    Activity(ActivityCommon),
    Collection(Collection),
    OrderedCollection(Collection),
    CollectionPage(CollectionPage),
    OrderedCollectionPage(CollectionPage),
    Accept(ActivityCommon),
    TentativeAccept(ActivityCommon),
    Add(ActivityCommon),
    Arrive(ActivityCommon),
    Create(ActivityCommon),
    Delete(ActivityCommon),
    Follow(ActivityCommon),
    Ignore(ActivityCommon),
    Join(ActivityCommon),
    Leave(ActivityCommon),
    Like(ActivityCommon),
    Offer(ActivityCommon),
    Invite(ActivityCommon),
    Reject(ActivityCommon),
    TentativeReject(ActivityCommon),
    Remove(ActivityCommon),
    Undo(ActivityCommon),
    Update(ActivityCommon),
    View(ActivityCommon),
    Listen(ActivityCommon),
    Read(ActivityCommon),
    Move(ActivityCommon),
    Travel(ActivityCommon),
    Announce(ActivityCommon),
    Block(ActivityCommon),
    Flag(ActivityCommon),
    Dislike(ActivityCommon),
    Question(ActivityCommon),
    Application(Actor),
    Group(Actor),
    Organization(Actor),
    Person(Actor),
    Service(Actor),
    Relationship(Relationship),
    Article(ObjectCommon),
    Document(ObjectCommon),
    Audio(ObjectCommon),
    Image(ObjectCommon),
    Video(ObjectCommon),
    Note(ObjectCommon),
    Page(ObjectCommon),
    Event(ObjectCommon),
    Place(Place),
    Mention(Link),
    Profile(Profile),
    Tombstone(Tombstone),
    PropertyValue(PropertyValue),
}

impl ObjectID for Object {
    fn id(&self) -> Option<&str> {
        match self {
            Object::Object(o) => o.id.as_deref(),
            Object::Collection(o) |
            Object::OrderedCollection(o) => o.common.id.as_deref(),
            Object::CollectionPage(o) |
            Object::OrderedCollectionPage(o) => o.common.common.id.as_deref(),
            Object::Activity(o) |
            Object::Accept(o) |
            Object::TentativeAccept(o) |
            Object::Add(o) |
            Object::Arrive(o) |
            Object::Create(o) |
            Object::Delete(o) |
            Object::Follow(o) |
            Object::Ignore(o) |
            Object::Join(o) |
            Object::Leave(o) |
            Object::Like(o) |
            Object::Offer(o) |
            Object::Invite(o) |
            Object::Reject(o) |
            Object::TentativeReject(o) |
            Object::Remove(o) |
            Object::Undo(o) |
            Object::Update(o) |
            Object::View(o) |
            Object::Listen(o) |
            Object::Read(o) |
            Object::Move(o) |
            Object::Travel(o) |
            Object::Announce(o) |
            Object::Block(o) |
            Object::Flag(o) |
            Object::Dislike(o) |
            Object::Question(o) => o.id(),
            Object::Application(o) |
            Object::Group(o) |
            Object::Organization(o) |
            Object::Person(o) |
            Object::Service(o) => o.common.id.as_deref(),
            Object::Relationship(o) => o.common.id.as_deref(),
            Object::Article(o) |
            Object::Document(o) |
            Object::Audio(o) |
            Object::Image(o) |
            Object::Video(o) |
            Object::Note(o) |
            Object::Page(o) |
            Object::Event(o) => o.id.as_deref(),
            Object::Place(o) => o.common.id.as_deref(),
            Object::Mention(o) => o.href.as_deref(),
            Object::Profile(o) => o.common.id.as_deref(),
            Object::Tombstone(o) => o.common.id.as_deref(),
            Object::PropertyValue(o) => None,
        }
    }
}

impl Object {
    pub fn to_json(&self) -> String {
        let mut body = serde_json::to_value(&self).unwrap();

        if let serde_json::Value::Object(obj) = &mut body {
            obj.insert("@context".to_string(), serde_json::json!([
                "https://www.w3.org/ns/activitystreams",
                "https://w3id.org/security/v1",
                {
                    "toot": "http://joinmastodon.org/ns#",
                    "schema": "http://schema.org#",
                    "sensitive": "as:sensitive",
                    "manuallyApprovesFollowers": "as:manuallyApprovesFollowers",
                    "PropertyValue": "schema:PropertyValue",
                    "value": "schema:value",
                    "discoverable": "toot:discoverable",
                    "focalPoint": {
                        "@container": "@list",
                        "@id": "toot:focalPoint"
                    },
                    "featured": {
                        "@id": "toot:featured",
                        "@type": "@id"
                    },
                    "featuredTags": {
                        "@id": "toot:featuredTags",
                        "@type": "@id"
                    },
                    "alsoKnownAs": {
                        "@id": "as:alsoKnownAs",
                        "@type": "@id"
                    },
                    "movedTo": {
                        "@id": "as:movedTo",
                        "@type": "@id"
                    },
                    "blurhash": "toot:blurhash",
                }
            ]));
        }

        serde_json::to_string(&body).unwrap()
    }
}

impl<'r> rocket::response::Responder<'r, 'static> for Object {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let body = self.to_json();

        let mut res = rocket::response::Response::new();
        res.set_status(rocket::http::Status::Ok);
        res.set_raw_header("Content-Type", "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"");
        res.set_sized_body(body.len(), std::io::Cursor::new(body));

        Ok(res)
    }
}

#[rocket::async_trait]
impl<'r> rocket::data::FromData<'r> for Object {
    type Error = String;

    async fn from_data(req: &'r rocket::Request<'_>, data: rocket::data::Data<'r>) -> rocket::data::Outcome<'r, Self> {
        let mut needs_context = true;

        match req.content_type() {
            None => return rocket::data::Outcome::Forward(data),
            Some(ct) => match (ct.top().as_str(), ct.sub().as_str()) {
                ("application", "json") => {}
                ("application", "ld+json") => {}
                ("application", "activity+json") => {
                    needs_context = false;
                }
                _ => return rocket::data::Outcome::Forward(data),
            }
        }

        let mut digests: Vec<(DigestAlgorithm, &str)> = req.headers()
            .get("Digest")
            .filter_map(|d| d.split_once("="))
            .map(|(alg, digest)| (DigestAlgorithm::from_str(alg), digest))
            .filter_map(|(alg, digest)| alg.map(|alg| (alg, digest)))
            .collect();

        digests.sort_by_key(|(alg, _)| alg.clone());

        let data = match data.open(1 * rocket::data::ByteUnit::GiB).into_bytes().await {
            Ok(s) => s,
            Err(e) => return rocket::data::Outcome::Failure((rocket::http::Status::BadRequest, format!("Failed to read request body: {}", e))),
        };

        if !data.is_complete() {
            return rocket::data::Outcome::Failure((rocket::http::Status::PayloadTooLarge, "Payload too large".to_string()));
        }

        if let Some((alg, digest)) = digests.get(0) {
            let digest = match base64::decode(digest) {
                Ok(digest) => digest,
                Err(e) => return rocket::data::Outcome::Failure((rocket::http::Status::BadRequest, format!("Failed to decode digest: {}", e))),
            };

            let own_digest = match alg {
                DigestAlgorithm::SHA512 => {
                    openssl::hash::hash(openssl::hash::MessageDigest::sha512(), &data).unwrap().to_vec()
                }
                DigestAlgorithm::SHA256 => {
                    openssl::hash::hash(openssl::hash::MessageDigest::sha256(), &data).unwrap().to_vec()
                }
                DigestAlgorithm::SHA1 => {
                    openssl::hash::hash(openssl::hash::MessageDigest::sha1(), &data).unwrap().to_vec()
                }
                DigestAlgorithm::MD5 => {
                    openssl::hash::hash(openssl::hash::MessageDigest::md5(), &data).unwrap().to_vec()
                }
            };

            if own_digest != digest {
                return rocket::data::Outcome::Failure(
                    (rocket::http::Status::UnprocessableEntity, format!("Digest mismatch: {:?} != {:?}", own_digest, digest))
                );
            }
        }

        let data: serde_json::Value = match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(e) => {
                return rocket::data::Outcome::Failure(
                    (rocket::http::Status::UnprocessableEntity, format!("Failed to parse JSON: {}", e))
                );
            }
        };

        if needs_context {
            match data {
                serde_json::Value::Object(ref o) => {
                    match o.get("@context") {
                        Some(serde_json::Value::String(s)) => {
                            if s != "https://www.w3.org/ns/activitystreams" {
                                return rocket::data::Outcome::Failure(
                                    (rocket::http::Status::UnprocessableEntity, format!("Invalid @context: {}", s))
                                );
                            }
                        }
                        Some(serde_json::Value::Array(s)) => {
                            if !s.iter().any(|v| *v == serde_json::Value::String("https://www.w3.org/ns/activitystreams".to_string())) {
                                return rocket::data::Outcome::Failure(
                                    (rocket::http::Status::UnprocessableEntity, format!("Invalid @context: {:?}", s))
                                );
                            }
                        }
                        _ => return rocket::data::Outcome::Failure(
                            (rocket::http::Status::UnprocessableEntity, format!("Missing @context"))
                        ),
                    }
                }
                _ => return rocket::data::Outcome::Failure(
                    (rocket::http::Status::UnprocessableEntity, format!("Not an object"))
                ),
            }
        }

        let data: Object = match serde_json::from_value(data) {
            Ok(d) => d,
            Err(e) => return rocket::data::Outcome::Failure(
                (rocket::http::Status::UnprocessableEntity, format!("Failed to parse object: {}", e))
            ),
        };

        rocket::data::Outcome::Success(data)
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ObjectOrLink {
    Object(Object),
    Link(Link),
}

impl ObjectID for ObjectOrLink {
    fn id(&self) -> Option<&str> {
        match self {
            ObjectOrLink::Object(o) => o.id(),
            ObjectOrLink::Link(l) => l.id(),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ObjectOrLink {
    fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;

        match &value {
            serde_json::Value::Object(o) => {
                match o.get_key_value("type") {
                    Some((_, serde_json::Value::String(s))) => {
                        if s == "Link" {
                            serde_json::from_value(value).map(ObjectOrLink::Link).map_err(serde::de::Error::custom)
                        } else {
                            serde_json::from_value(value).map(ObjectOrLink::Object).map_err(serde::de::Error::custom)
                        }
                    },
                    _ => Err(serde::de::Error::missing_field("type"))
                }
            }
            _ => Err(serde::de::Error::invalid_type(serde::de::Unexpected::Other("not an object"), &"object")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ImageOrLink {
    Link(Link),
    Image(ObjectCommon),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum CollectionPageOrLink {
    Link(Link),
    CollectionPage(CollectionPage),
    OrderedCollectionPage(CollectionPage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum CollectionOrLink {
    Link(Link),
    Collection(Collection),
    OrderedCollection(Collection),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum URLOrLink {
    URL(String),
    Link(Link),
}

pub type LanguageMap<T> = std::collections::HashMap<String, T>;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ObjectCommon {
    #[serde(rename = "id", default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "attachment", default, skip_serializing_if = "Pluralisable::is_none")]
    pub attachment: Pluralisable<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "attributedTo", default, skip_serializing_if = "Option::is_none")]
    pub attributed_to: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "audience", default, skip_serializing_if = "Pluralisable::is_none")]
    pub audience: Pluralisable<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "content", default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(rename = "contentMap", default, skip_serializing_if = "Option::is_none")]
    pub content_map: Option<LanguageMap<String>>,
    #[serde(rename = "context", default, skip_serializing_if = "Option::is_none")]
    pub context: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "nameMap", default, skip_serializing_if = "Option::is_none")]
    pub name_map: Option<LanguageMap<String>>,
    #[serde(rename = "endTime", default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    #[serde(rename = "generator", default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "icon", default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ReferenceOrObject<ImageOrLink>>,
    #[serde(rename = "image", default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ReferenceOrObject<ImageOrLink>>,
    #[serde(rename = "inReplyTo", default, skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "location", default, skip_serializing_if = "Option::is_none")]
    pub location: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "preview", default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "published", default, skip_serializing_if = "Option::is_none")]
    pub published: Option<DateTime<Utc>>,
    #[serde(rename = "replies", default, skip_serializing_if = "Option::is_none")]
    pub replies: Option<ReferenceOrObject<Collection>>,
    #[serde(rename = "startTime", default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(rename = "summary", default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(rename = "summaryMap", default, skip_serializing_if = "Option::is_none")]
    pub summary_map: Option<LanguageMap<String>>,
    #[serde(rename = "tag", default, skip_serializing_if = "Pluralisable::is_none")]
    pub tag: Pluralisable<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "updated", default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<DateTime<Utc>>,
    #[serde(rename = "url", default, skip_serializing_if = "Option::is_none")]
    pub url: Option<URLOrLink>,
    #[serde(rename = "to", default, skip_serializing_if = "Pluralisable::is_none")]
    pub to: Pluralisable<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "bto", default, skip_serializing_if = "Pluralisable::is_none")]
    pub bto: Pluralisable<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "cc", default, skip_serializing_if = "Pluralisable::is_none")]
    pub cc: Pluralisable<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "bcc", default, skip_serializing_if = "Pluralisable::is_none")]
    pub bcc: Pluralisable<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "mediaType", skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(rename = "duration", skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(rename = "likes", default, skip_serializing_if = "Option::is_none")]
    pub likes: Option<ReferenceOrObject<Collection>>,
    #[serde(rename = "sensitive", default, skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
    #[serde(rename = "blurhash", default, skip_serializing_if = "Option::is_none")]
    pub blurhash: Option<String>,
    #[serde(rename = "height", default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    #[serde(rename = "width", default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
    #[serde(rename = "focalPoints", default, skip_serializing_if = "Option::is_none")]
    pub focal_points: Option<(f64, f64)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivityCommon {
    #[serde(flatten)]
    pub common: ObjectCommon,

    #[serde(rename = "actor", default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "object", default, skip_serializing_if = "Option::is_none")]
    pub object: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "target", default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "result", default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "origin", default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "instrument", default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<ReferenceOrObject<ObjectOrLink>>,
}

impl ObjectID for ActivityCommon {
    fn id(&self) -> Option<&str> {
        self.common.id.as_deref()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Collection {
    #[serde(flatten)]
    pub common: ObjectCommon,

    #[serde(rename = "totalItems", default, skip_serializing_if = "Option::is_none")]
    pub total_items: Option<u64>,
    #[serde(rename = "current", default, skip_serializing_if = "Option::is_none")]
    pub current: Option<ReferenceOrObject<CollectionPageOrLink>>,
    #[serde(rename = "first", default, skip_serializing_if = "Option::is_none")]
    pub first: Option<ReferenceOrObject<CollectionPageOrLink>>,
    #[serde(rename = "last", default, skip_serializing_if = "Option::is_none")]
    pub last: Option<ReferenceOrObject<CollectionPageOrLink>>,
    #[serde(rename = "items", alias = "orderedItems", default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ReferenceOrObject<ObjectOrLink>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CollectionPage {
    #[serde(flatten)]
    pub common: Collection,

    #[serde(rename = "partOf", default, skip_serializing_if = "Option::is_none")]
    pub part_of: Option<ReferenceOrObject<CollectionOrLink>>,
    #[serde(rename = "next", default, skip_serializing_if = "Option::is_none")]
    pub next: Option<ReferenceOrObject<CollectionPageOrLink>>,
    #[serde(rename = "prev", default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<ReferenceOrObject<CollectionPageOrLink>>,
    #[serde(rename = "startIndex", default, skip_serializing_if = "Option::is_none")]
    pub start_index: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Question {
    #[serde(flatten)]
    pub common: ActivityCommon,

    #[serde(rename = "oneOf", default, skip_serializing_if = "Option::is_none")]
    pub one_of: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "anyOf", default, skip_serializing_if = "Option::is_none")]
    pub any_of: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "closed", default, skip_serializing_if = "Option::is_none")]
    pub closed: Option<ReferenceOrObject<QuestionClosed>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum QuestionClosed {
    Link(Link),
    Object(Object),
    DateTime(DateTime<Utc>),
    Boolean(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Relationship {
    #[serde(flatten)]
    pub common: ObjectCommon,

    #[serde(rename = "anyOf", default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "anyOf", default, skip_serializing_if = "Option::is_none")]
    pub object: Option<ReferenceOrObject<ObjectOrLink>>,
    #[serde(rename = "anyOf", default, skip_serializing_if = "Option::is_none")]
    pub relationship: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Place {
    #[serde(flatten)]
    pub common: ObjectCommon,

    #[serde(rename = "accuracy", default, skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
    #[serde(rename = "altitude", default, skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>,
    #[serde(rename = "latitude", default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(rename = "longitude", default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(rename = "radius", default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    #[serde(rename = "units", default, skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    #[serde(flatten)]
    pub common: ObjectCommon,

    #[serde(rename = "describes", default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<ReferenceOrObject<ObjectOrLink>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tombstone {
    #[serde(flatten)]
    pub common: ObjectCommon,

    #[serde(rename = "formerType", default, skip_serializing_if = "Option::is_none")]
    pub former_type: Option<String>,
    #[serde(rename = "deleted", default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Actor {
    #[serde(flatten)]
    pub common: ObjectCommon,

    #[serde(rename = "preferredUsername", default, skip_serializing_if = "Option::is_none")]
    pub preferred_username: Option<String>,
    #[serde(rename = "inbox")]
    pub inbox: String,
    #[serde(rename = "outbox")]
    pub outbox: String,
    #[serde(rename = "following", default, skip_serializing_if = "Option::is_none")]
    pub following: Option<String>,
    #[serde(rename = "followers", default, skip_serializing_if = "Option::is_none")]
    pub followers: Option<String>,
    #[serde(rename = "liked", default, skip_serializing_if = "Option::is_none")]
    pub liked: Option<String>,
    #[serde(
        rename = "manuallyApprovesFollowers", alias = "as:manuallyApprovesFollowers",
        default, skip_serializing_if = "Option::is_none"
    )]
    pub manually_approves_followers: Option<bool>,
    #[serde(rename = "endpoints", default, skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<ReferenceOrObject<Endpoints>>,
    #[serde(rename = "publicKey", default, skip_serializing_if = "Pluralisable::is_none")]
    pub public_key: Pluralisable<ReferenceOrObject<PublicKey>>,
    #[serde(rename = "discoverable", default, skip_serializing_if = "Option::is_none")]
    pub discoverable: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Endpoints {
    #[serde(rename = "proxyUrl", default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    #[serde(rename = "oauthAuthorizationEndpoint", default, skip_serializing_if = "Option::is_none")]
    pub oauth_authorization_endpoint: Option<String>,
    #[serde(rename = "oauthTokenEndpoint", default, skip_serializing_if = "Option::is_none")]
    pub oauth_token_ndpoint: Option<String>,
    #[serde(rename = "provideClientKey", default, skip_serializing_if = "Option::is_none")]
    pub provide_client_key: Option<String>,
    #[serde(rename = "signClientKey", default, skip_serializing_if = "Option::is_none")]
    pub sign_client_key: Option<String>,
    #[serde(rename = "sharedInbox", default, skip_serializing_if = "Option::is_none")]
    pub shared_inbox: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Link {
    #[serde(rename = "href", default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(rename = "rel", default, skip_serializing_if = "Vec::is_empty")]
    pub rel: Vec<String>,
    #[serde(rename = "mediaType", default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(rename = "name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "nameMap", default, skip_serializing_if = "Option::is_none")]
    pub name_map: Option<LanguageMap<String>>,
    #[serde(rename = "hreflang", default, skip_serializing_if = "Option::is_none")]
    pub href_lang: Option<String>,
    #[serde(rename = "height", default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    #[serde(rename = "width", default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
    #[serde(rename = "preview", default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<ReferenceOrObject<ObjectOrLink>>,
}

impl ObjectID for Link {
    fn id(&self) -> Option<&str> {
        self.href.as_deref()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicKey {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<ReferenceOrObject<Object>>,
    #[serde(rename = "publicKeyPem", default, skip_serializing_if = "Option::is_none")]
    pub public_key_pem: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropertyValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    pub key_id: String,
    pub algorithm: SignatureAlgorithm,
    pub signature: Vec<u8>,
    pub signed_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum SignatureAlgorithm {
    RsaSha1,
    RsaSha256,
    RsaSha512,
    DsaSha1,
    HmacSha1,
    HmacSha256,
    HmacSha512,
}

impl Signature {
    pub fn verify(&self, pkey: &openssl::pkey::PKeyRef<openssl::pkey::Public>) -> bool {
        match self.algorithm {
            SignatureAlgorithm::RsaSha1 |
            SignatureAlgorithm::RsaSha256 |
            SignatureAlgorithm::RsaSha512 => {
                if !pkey.rsa().is_ok() {
                    return false;
                }
            }
            SignatureAlgorithm::DsaSha1 => {
                if !pkey.dsa().is_ok() {
                    return false;
                }
            }
            SignatureAlgorithm::HmacSha1 |
            SignatureAlgorithm::HmacSha256 |
            SignatureAlgorithm::HmacSha512 => return false,
        }
        let mut verifier = match openssl::sign::Verifier::new(match self.algorithm {
            SignatureAlgorithm::RsaSha1 => openssl::hash::MessageDigest::sha1(),
            SignatureAlgorithm::RsaSha256 => openssl::hash::MessageDigest::sha256(),
            SignatureAlgorithm::RsaSha512 => openssl::hash::MessageDigest::sha512(),
            SignatureAlgorithm::DsaSha1 => openssl::hash::MessageDigest::sha1(),
            SignatureAlgorithm::HmacSha1 |
            SignatureAlgorithm::HmacSha256 |
            SignatureAlgorithm::HmacSha512 => unreachable!(),
        }, pkey) {
            Ok(v) => v,
            Err(_) => return false,
        };
        match verifier.update(&self.signed_data) {
            Ok(_) => (),
            Err(_) => return false,
        }
        match verifier.verify(&self.signature) {
            Ok(v) => v,
            Err(_) => false,
        }
    }
}

enum SignatureParserState {
    Name,
    Quote,
    Value,
    Comma,
    Number,
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Signature {
    type Error = &'static str;

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = request.headers();
        if let Some(signature) = headers.get_one("Signature") {
            let mut params = std::collections::HashMap::new();

            let mut state = SignatureParserState::Name;
            let mut tmp_name = String::new();
            let mut tmp_value = String::new();
            for c in signature.chars() {
                match &state {
                    SignatureParserState::Name => {
                        if (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') {
                            tmp_name.push(c);
                        } else if c == '=' {
                            state = SignatureParserState::Quote;
                        } else {
                            return rocket::request::Outcome::Failure((rocket::http::Status::BadRequest, "Invalid field name"));
                        }
                    }
                    SignatureParserState::Quote => {
                        if c == '"' {
                            state = SignatureParserState::Value;
                        } else {
                            state = SignatureParserState::Number;
                            if c < '0' || c > '9' {
                                return rocket::request::Outcome::Failure((rocket::http::Status::BadRequest, "Invalid number"));
                            }
                            tmp_value.push(c)
                        }
                    }
                    SignatureParserState::Value => {
                        if c == '"' {
                            params.insert(tmp_name, tmp_value);
                            tmp_name = String::new();
                            tmp_value = String::new();
                            state = SignatureParserState::Comma;
                        } else {
                            tmp_value.push(c);
                        }
                    }
                    SignatureParserState::Number => {
                        if c == ',' {
                            params.insert(tmp_name, tmp_value);
                            tmp_name = String::new();
                            tmp_value = String::new();
                            state = SignatureParserState::Name;
                        } else if c < '0' || c > '9' {
                            return rocket::request::Outcome::Failure((rocket::http::Status::BadRequest, "Invalid number"));
                        } else {
                            tmp_value.push(c);
                        }
                    }
                    SignatureParserState::Comma => {
                        if c == ',' {
                            state = SignatureParserState::Name;
                        } else {
                            return rocket::request::Outcome::Failure((rocket::http::Status::BadRequest, "Invalid structure"));
                        }
                    }
                }
            }

            if !params.contains_key("signature") || !params.contains_key("algorithm")
                || !params.contains_key("keyId") {
                return rocket::request::Outcome::Failure((rocket::http::Status::BadRequest, "Missing required fields"));
            }

            let algorithm = match params.get("algorithm").unwrap().as_str() {
                "rsa-sha1" => SignatureAlgorithm::RsaSha1,
                "rsa-sha256" => SignatureAlgorithm::RsaSha256,
                "rsa-sha512" => SignatureAlgorithm::RsaSha512,
                "dsa-sha1" => SignatureAlgorithm::DsaSha1,
                "hmac-sha1" => SignatureAlgorithm::HmacSha1,
                "hmac-sha256" => SignatureAlgorithm::HmacSha256,
                "hmac-sha512" => SignatureAlgorithm::HmacSha512,
                _ => return rocket::request::Outcome::Failure((rocket::http::Status::BadRequest, "Invalid algorithm")),
            };

            let signature = match base64::decode(params.get("signature").unwrap()) {
                Ok(signature) => signature,
                Err(_) => return rocket::request::Outcome::Failure((rocket::http::Status::BadRequest, "Invalid signature encoding")),
            };

            let signed_headers = params.get("headers").map(|s| s.as_str()).unwrap_or("date").split(" ");
            let mut signed_data = vec![];

            for header in signed_headers {
                if header == "(request-target)" {
                    signed_data.push(format!("(request-target): {} {}", request.method().as_str().to_lowercase(), request.uri()));
                }
                for v in headers.get(header) {
                    signed_data.push(format!("{}: {}", header, v));
                }
            }

            rocket::request::Outcome::Success(Signature {
                key_id: params.get("keyId").unwrap().to_string(),
                algorithm,
                signature,
                signed_data: signed_data.join("\n").into_bytes(),
            })
        } else {
            rocket::request::Outcome::Forward(())
        }
    }
}

#[get("/as/transient/<_id>")]
pub async fn transient(_id: &str) -> rocket::http::Status {
    rocket::http::Status::Gone
}

#[get("/as/system")]
pub async fn system_actor(
    config: &rocket::State<AppConfig>
) -> Result<Object, rocket::http::Status> {
    Ok(Object::Application(Actor {
        common: ObjectCommon {
            id: Some(format!("https://{}/as/system", config.uri)),
            url: Some(URLOrLink::URL(format!("https://{}", config.uri))),
            ..Default::default()
        },
        preferred_username: Some(config.uri.clone()),
        inbox: format!("https://{}/as/system/inbox", config.uri),
        outbox: format!("https://{}/as/system/outbox", config.uri),
        following: None,
        followers: None,
        liked: None,
        manually_approves_followers: Some(true),
        endpoints: Some(ReferenceOrObject::Object(Box::new(Endpoints {
            shared_inbox: Some(format!("https://{}/as/inbox", config.uri)),
            ..Default::default()
        }))),
        public_key: Pluralisable::Object(ReferenceOrObject::Object(Box::new(PublicKey {
            id: Some(format!("https://{}/as/system#key", config.uri)),
            owner: Some(ReferenceOrObject::Reference(format!("https://{}/as/system", config.uri))),
            public_key_pem: Some(String::from_utf8(config.as_key.public_key_to_pem().unwrap()).unwrap()),
        }))),
        discoverable: Some(false)
    }))
}

async fn get_account(db: &crate::DbConn, id: &str) -> Result<crate::models::Account, rocket::http::Status> {
    let account_id = match uuid::Uuid::parse_str(id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let account: crate::models::Account = crate::db_run(db, move |c| -> diesel::result::QueryResult<_> {
        crate::schema::accounts::dsl::accounts.find(account_id).get_result(c)
    }).await?;

    if !account.local {
        return Err(rocket::http::Status::NotFound);
    }

    Ok(account)
}

#[get("/as/users/<id>")]
pub async fn user(
    db: crate::DbConn, config: &rocket::State<AppConfig>, id: &str,
) -> Result<Object, rocket::http::Status> {
    let account = get_account(&db, id).await?;

    let account = match crate::tasks::accounts::render_account(&account) {
        Ok(a) => a,
        Err(_) => return Err(rocket::http::Status::InternalServerError),
    };

    Ok(account)
}

#[get("/as/users/<_id>/inbox")]
pub async fn get_inbox(_id: &str) -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[post("/as/users/<id>/inbox", data = "<data>")]
pub async fn post_inbox(
    db: crate::DbConn, id: &str, data: Object, signature: Signature,
    celery: &rocket::State<crate::CeleryApp>,
) -> Result<(), rocket::http::Status> {
    get_account(&db, id).await?;

    match celery.send_task(
        super::super::tasks::inbox::process_activity::new(data, signature)
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    Ok(())
}

#[get("/as/users/<id>/outbox")]
pub async fn get_outbox(
    db: crate::DbConn, config: &rocket::State<AppConfig>, id: &str,
) -> Result<Object, rocket::http::Status> {
    let account = get_account(&db, id).await?;

    Ok(Object::OrderedCollection(Collection {
        common: ObjectCommon {
            id: Some(format!("https://{}/as/users/{}/outbox", config.uri, account.id)),
            ..Default::default()
        },
        total_items: Some(account.statuses_count as u64),
        current: None,
        first: Some(ReferenceOrObject::Reference(format!("https://{}/as/users/{}/outbox/page", config.uri, account.id))),
        last: None,
        items: None,
    }))
}

#[get("/as/users/<id>/outbox/page?<before>")]
pub async fn get_outbox_page(
    db: crate::DbConn, config: &rocket::State<AppConfig>, id: &str, before: Option<i64>,
) -> Result<Object, rocket::http::Status> {
    let account = get_account(&db, id).await?;

    Ok(Object::OrderedCollectionPage(CollectionPage {
        common: Collection {
            common: ObjectCommon {
                id: Some(match before {
                    Some(b) => format!("https://{}/as/users/{}/outbox/page?before={}", config.uri, account.id, b),
                    None => format!("https://{}/as/users/{}/outbox/page", config.uri, account.id)
                }),
                ..Default::default()
            },
            total_items: None,
            current: None,
            first: None,
            last: None,
            items: Some(vec![]),
        },
        part_of: Some(ReferenceOrObject::Reference(format!("https://{}/as/users/{}/outbox", config.uri, account.id))),
        next: None,
        prev: None,
        start_index: None,
    }))
}

#[post("/as/users/<_id>/outbox")]
pub async fn post_outbox(_id: &str) -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[get("/as/inbox")]
pub async fn get_shared_inbox() -> rocket::http::Status {
    rocket::http::Status::MethodNotAllowed
}

#[post("/as/inbox", data = "<data>")]
pub async fn post_shared_inbox(
    data: Object, signature: Signature, celery: &rocket::State<crate::CeleryApp>,
) -> Result<(), rocket::http::Status> {
    match celery.send_task(
        super::super::tasks::inbox::process_activity::new(data, signature)
    ).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to submit celery task: {:?}", err);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    Ok(())
}

#[get("/as/status/<id>")]
pub async fn status(
    db: crate::DbConn, id: &str,
) -> Result<Object, rocket::http::Status> {
    let status_id = match uuid::Uuid::parse_str(id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let (status, account): (crate::models::Status, crate::models::Account) =
        crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::statuses::dsl::statuses.find(status_id).inner_join(
                crate::schema::accounts::table.on(
                    crate::schema::statuses::dsl::account_id.eq(crate::schema::accounts::dsl::id)
                )
            ).get_result(c)
        }).await?;

    if !status.local {
        return Err(rocket::http::Status::NotFound);
    }

    let aud = match crate::tasks::statuses::make_audiences(&status, false).await {
        Ok(aud) => aud,
        Err(_) => return Err(rocket::http::Status::InternalServerError)
    };

    if !aud.is_visible() {
        return Err(rocket::http::Status::NotFound);
    }

    if status.boost_of_id.is_none() {
        crate::tasks::statuses::as_render_status(&status, &account, &aud)
            .map_err(|_| rocket::http::Status::InternalServerError)
    } else {
        Err(rocket::http::Status::NotFound)
    }
}

#[get("/as/status/<id>/activity")]
pub async fn status_activity(
    db: crate::DbConn, id: &str,
) -> Result<Object, rocket::http::Status> {
    let status_id = match uuid::Uuid::parse_str(id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let (status, account): (crate::models::Status, crate::models::Account) =
        crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::statuses::dsl::statuses.find(status_id).inner_join(
                crate::schema::accounts::table.on(
                    crate::schema::statuses::dsl::account_id.eq(crate::schema::accounts::dsl::id)
                )
            ).get_result(c)
        }).await?;

    if !status.local {
        return Err(rocket::http::Status::NotFound);
    }

    let aud = match crate::tasks::statuses::make_audiences(&status, false).await {
        Ok(aud) => aud,
        Err(_) => return Err(rocket::http::Status::InternalServerError)
    };

    if !aud.is_visible() {
        return Err(rocket::http::Status::NotFound);
    }

    if let Some(boost_of_id) = status.boost_of_id {
        if status.deleted_at.is_some() {
            return Err(rocket::http::Status::Gone);
        }

        let boosted_status: crate::models::Status = crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::statuses::dsl::statuses.find(boost_of_id).get_result(c)
        }).await?;

        let activity = crate::tasks::statuses::as_render_boost(&status, &boosted_status, &account, &aud);

        Ok(activity)
    } else {
        crate::tasks::statuses::as_render_status_activity(&status, &account, &aud)
            .map_err(|_| rocket::http::Status::InternalServerError)
    }
}

#[get("/as/like/<id>")]
pub async fn like(
    db: crate::DbConn, id: &str,
) -> Result<Object, rocket::http::Status> {
    let like_id = match uuid::Uuid::parse_str(id) {
        Ok(id) => id,
        Err(_) => return Err(rocket::http::Status::NotFound)
    };

    let (like, account, liked_status): (crate::models::Like, crate::models::Account, crate::models::Status) =
        crate::db_run(&db, move |c| -> QueryResult<_> {
            crate::schema::likes::dsl::likes.find(like_id).inner_join(
                crate::schema::accounts::table.on(
                    crate::schema::likes::dsl::account.eq(crate::schema::accounts::dsl::id)
                )
            ).inner_join(
                crate::schema::statuses::table.on(
                    crate::schema::likes::dsl::status.eq(crate::schema::statuses::dsl::id.nullable())
                )
            ).get_result(c)
        }).await?;

    if !like.local || like.status_url.is_some() {
        return Err(rocket::http::Status::NotFound);
    }

    let aud = match crate::tasks::statuses::make_like_audiences(&like, &liked_status, &account, false).await {
        Ok(aud) => aud,
        Err(_) => return Err(rocket::http::Status::InternalServerError)
    };

    if !aud.is_visible() {
        return Err(rocket::http::Status::NotFound);
    }

    let activity = match crate::tasks::statuses::as_render_like(&like, &liked_status, &account, &aud).await {
        Ok(act) => act,
        Err(_) => return Err(rocket::http::Status::InternalServerError)
    };

    Ok(activity)
}