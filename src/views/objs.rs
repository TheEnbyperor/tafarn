use chrono::prelude::*;

#[derive(Serialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub acct: String,
    pub url: Option<String>,
    pub display_name: String,
    pub note: String,
    pub avatar: String,
    pub avatar_static: String,
    pub header: String,
    pub header_static: String,
    pub locked: bool,
    pub fields: Vec<Field>,
    pub emojis: Vec<Emoji>,
    pub bot: bool,
    pub group: bool,
    pub discoverable: Option<bool>,
    pub noindex: Option<bool>,
    pub moved: Option<Box<Account>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limited: Option<bool>,
    #[serde(serialize_with = "serialize_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_date_opt")]
    pub last_status_at: Option<NaiveDate>,
    pub statuses_count: u64,
    pub followers_count: u64,
    pub following_count: u64,
}

#[derive(Serialize)]
pub struct CredentialAccount {
    #[serde(flatten)]
    pub base: Account,
    pub source: AccountSource,
}

#[derive(Serialize)]
pub struct AccountSource {
    pub note: String,
    pub fields: Vec<Field>,
    pub privacy: String,
    pub sensitive: bool,
    pub language: String,
    pub follow_requests_count: u64,
}

#[derive(Serialize)]
pub struct Field {
    pub name: String,
    pub value: String,
    #[serde(serialize_with = "serialize_timestamp_opt")]
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct App {
    pub id: uuid::Uuid,
    pub name: String,
    pub website: Option<String>,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vapid_key: Option<String>,
}

#[derive(Serialize)]
pub struct Emoji {
    pub shortcode: String,
    pub url: String,
    pub static_url: String,
    pub visible_in_picker: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Serialize)]
pub struct Filter {}

#[derive(Serialize)]
pub struct Instance {
    pub uri: String,
    pub title: String,
    pub short_description: String,
    pub description: String,
    pub email: String,
    pub version: String,
    pub urls: InstanceURLs,
    pub stats: InstanceStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_account: Option<Account>,
    pub languages: Vec<String>,
    pub registrations: bool,
    pub approval_required: bool,
    pub invites_enabled: bool,
}

#[derive(Serialize)]
pub struct InstanceV2 {
    pub domain: String,
    pub title: String,
    pub version: String,
    pub source_url: String,
    pub description: String,
    pub usage: InstanceV2Usage,
    pub thumbnail: InstanceV2Thumbnail,
    pub languages: Vec<String>,
    pub configuration: InstanceV2Configuration,
    pub registrations: InstanceV2Registrations,
    pub contact: InstanceV2Contact,
    pub rules: Vec<Rule>
}

#[derive(Serialize)]
pub struct InstanceV2Usage {
    pub users: InstanceV2UsageUsers,
}

#[derive(Serialize)]
pub struct InstanceV2UsageUsers {
    pub active_month: u64,
}

#[derive(Serialize)]
pub struct InstanceV2Thumbnail {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<InstanceV2ThumbnailVersions>,
}

#[derive(Serialize)]
pub struct InstanceV2Configuration {
    pub urls: InstanceV2URLs,
    pub accounts: InstanceV2Accounts,
    pub statuses: InstanceV2Statuses,
    pub media_attachments: InstanceV2MediaAttachments,
    pub polls: InstanceV2Polls,
    pub translation: InstanceV2Translation,
}

#[derive(Serialize)]
pub struct InstanceV2URLs {
    pub streaming_api: String,
}

#[derive(Serialize)]
pub struct InstanceV2Accounts {
    pub max_featured_tags: u64,
}

#[derive(Serialize)]
pub struct InstanceV2Statuses {
    pub max_characters: u64,
    pub max_media_attachments: u64,
    pub characters_reserved_per_url: u64,
}

#[derive(Serialize)]
pub struct InstanceV2MediaAttachments {
    pub supported_mime_types: Vec<String>,
    pub image_size_limit: u64,
    pub image_matrix_limit: u64,
    pub video_size_limit: u64,
    pub video_frame_rate_limit: u64,
    pub video_matrix_limit: u64,
}

#[derive(Serialize)]
pub struct InstanceV2Polls {
    pub max_options: u64,
    pub max_characters_per_option: u64,
    pub min_expiration: u64,
    pub max_expiration: u64,
}

#[derive(Serialize)]
pub struct InstanceV2Translation {
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct InstanceV2Registrations {
    pub enabled: bool,
    pub approval_required: bool,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct InstanceV2Contact {
    pub email: String,
    pub account: Option<Account>,
}

#[derive(Serialize)]
pub struct InstanceV2ThumbnailVersions {
    #[serde(rename = "@1x", skip_serializing_if = "Option::is_none")]
    pub x1: Option<String>,
    #[serde(rename = "@2x", skip_serializing_if = "Option::is_none")]
    pub x2: Option<String>,
}

#[derive(Serialize)]
pub struct InstanceURLs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_api: Option<String>,
}

#[derive(Serialize)]
pub struct InstanceStats {
    pub user_count: u64,
    pub status_count: u64,
    pub domain_count: u64,
}

#[derive(Serialize)]
pub struct List {}

#[derive(Serialize, Deserialize, FromFormField, Debug)]
pub enum ListRepliesPolicy {
    #[serde(rename = "followed")]
    Followed,
    #[serde(rename = "list")]
    List,
    #[serde(rename = "none")]
    None
}

impl Default for ListRepliesPolicy {
    fn default() -> Self {
        Self::List
    }
}

#[derive(Serialize)]
pub struct Notification {
    pub id: String,
    #[serde(rename = "type")]
    pub notification_type: String,
    #[serde(serialize_with = "serialize_timestamp")]
    pub created_at: DateTime<Utc>,
    pub account: Account,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<Report>
}

#[derive(Serialize)]
pub struct Status {
    pub id: String,
    pub uri: String,
    #[serde(serialize_with = "serialize_timestamp")]
    pub created_at: DateTime<Utc>,
    pub account: Account,
    pub content: String,
    pub visibility: StatusVisibility,
    pub sensitive: bool,
    pub spoiler_text: String,
    pub media_attachments: Vec<MediaAttachment>,
    pub mentions: Vec<StatusMention>,
    pub tags: Vec<StatusTag>,
    pub emojis: Vec<Emoji>,
    pub reblogs_count: u64,
    pub favourites_count: u64,
    pub replies_count: u64,
    pub url: Option<String>,
    pub in_reply_to_id: Option<String>,
    pub in_reply_to_account_id: Option<String>,
    pub reblog: Option<Box<Status>>,
    pub poll: Option<Poll>,
    pub card: Option<PreviewCard>,
    pub language: Option<String>,
    #[serde(serialize_with = "serialize_timestamp_opt")]
    pub edited_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favourited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmarked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub enum StatusVisibility {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "unlisted")]
    Unlisted,
    #[serde(rename = "private")]
    Private,
    #[serde(rename = "direct")]
    Direct,
}

#[derive(Serialize)]
pub struct StatusMention {
    pub id: String,
    pub username: String,
    pub url: String,
    pub acct: String,
}

#[derive(Serialize)]
pub struct StatusTag {
    pub name: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct Context {
    pub ancestors: Vec<Status>,
    pub descendants: Vec<Status>,
}

#[derive(Serialize)]
pub struct Report {}

#[derive(Serialize)]
pub struct WebPushSubscription {
    pub id: String,
    pub endpoint: String,
    pub alerts: WebPushAlerts,
    pub server_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct WebPushAlerts {
    #[serde(default)]
    pub follow: bool,
    #[serde(default)]
    pub favourite: bool,
    #[serde(default)]
    pub reblog: bool,
    #[serde(default)]
    pub mention: bool,
    #[serde(default)]
    pub poll: bool,
    #[serde(default)]
    pub status: bool,
    #[serde(default)]
    pub follow_request: bool,
    #[serde(default)]
    pub update: bool,
    #[serde(default, rename = "admin.sign_up")]
    pub admin_sign_up: bool,
    #[serde(default, rename = "admin.report")]
    pub admin_report: bool,
}

#[derive(Serialize)]
pub struct Conversation {
    pub id: String,
    pub unread: bool,
    pub accounts: Vec<Account>,
    pub last_status: Option<Status>,
}

#[derive(Serialize)]
pub struct Rule {
    pub id: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct Relationship {
    pub id: String,
    pub following: bool,
    pub showing_reblogs: bool,
    pub notifying: bool,
    pub languages: Vec<String>,
    pub followed_by: bool,
    pub blocking: bool,
    pub blocked_by: bool,
    pub muting: bool,
    pub muting_notifications: bool,
    pub requested: bool,
    pub domain_blocking: bool,
    pub endorsed: bool,
    pub note: Option<String>,
}

#[derive(Serialize)]
pub struct FamiliarFollowers {
    pub id: String,
    pub accounts: Vec<Account>,
}

#[derive(Serialize)]
pub struct Tag {
    pub name: String,
    pub url: String,
    pub history: Vec<TagHistory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<bool>
}

#[derive(Serialize)]
pub struct TagHistory {
    pub day: String,
    pub uses: String,
    pub accounts: String,
}

#[derive(Serialize)]
pub struct Search {
    pub accounts: Vec<Account>,
    pub statuses: Vec<Status>,
    pub hashtags: Vec<Tag>,
}

#[derive(Serialize)]
pub struct MediaAttachment {
    pub id: String,
    #[serde(rename = "type")]
    pub media_type: MediaAttachmentType,
    pub url: Option<String>,
    pub preview_url: Option<String>,
    pub remote_url: Option<String>,
    pub meta: MediaAttachmentMeta,
    pub description: Option<String>,
    pub blurhash: Option<String>,
}

#[derive(Serialize)]
pub enum MediaAttachmentType {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "gifv")]
    Gifv,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "audio")]
    Audio,
}

#[derive(Serialize)]
pub struct MediaAttachmentMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<MediaAttachmentMetaFocus>,
}

#[derive(Serialize)]
pub struct MediaAttachmentMetaFocus {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize)]
pub struct Poll {
    pub id: String,
    #[serde(serialize_with = "serialize_timestamp_opt")]
    pub expires_at: Option<DateTime<Utc>>,
    pub expired: bool,
    pub multiple: bool,
    pub votes_count: u64,
    pub voters_count: Option<u64>,
    pub options: Vec<PollOption>,
    pub emojis: Vec<Emoji>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_votes: Option<Vec<u64>>,
}

#[derive(Serialize)]
pub struct PollOption {
    pub title: String,
    pub votes_count: Option<u64>,
}

#[derive(Serialize)]
pub struct PreviewCard {
    pub url: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub card_type: PreviewCardType,
    pub author_name: String,
    pub author_url: String,
    pub provider_name: String,
    pub provider_url: String,
    pub html: String,
    pub width: u64,
    pub height: u64,
    pub image: Option<String>,
    pub embed_url: String,
    pub blurhash: Option<String>,
}

#[derive(Serialize)]
pub enum PreviewCardType {
    #[serde(rename = "link")]
    Link,
    #[serde(rename = "photo")]
    Photo,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "rich")]
    Rich,
}

fn serialize_timestamp<S: serde::Serializer,>(timestamp: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(timestamp.to_rfc3339_opts(SecondsFormat::Millis, true).as_str())
}

fn serialize_timestamp_opt<S: serde::Serializer,>(timestamp: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
    match timestamp {
        Some(timestamp) => s.serialize_str(
            timestamp.to_rfc3339_opts(SecondsFormat::Millis, true).as_str()
        ),
        None => s.serialize_none(),
    }
}

fn serialize_date_opt<S: serde::Serializer,>(date: &Option<NaiveDate>, s: S) -> Result<S::Ok, S::Error> {
    match date {
        Some(date) => s.serialize_str(
            date.format("%Y-%m-%d").to_string().as_str()
        ),
        None => s.serialize_none(),
    }
}