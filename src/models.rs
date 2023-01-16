use crate::schema::*;

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="session"]
pub struct Session {
    pub id: uuid::Uuid,
    pub access_token: String,
    pub expires_at: Option<chrono::NaiveDateTime>,
    pub refresh_token: Option<String>,
    pub claims: String,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="apps"]
pub struct Apps {
    pub id: uuid::Uuid,
    pub name: String,
    pub website: Option<String>,
    pub redirect_uri: String,
    pub client_secret: String,
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="app_scopes"]
pub struct AppScopes {
    pub app_id: uuid::Uuid,
    pub scope: String,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="oauth_consents"]
pub struct OAuthConsents {
    pub id: uuid::Uuid,
    pub app_id: uuid::Uuid,
    pub user_id: String,
    pub time: chrono::NaiveDateTime,
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="oauth_consent_scopes"]
pub struct OAuthConsentScopes {
    pub consent_id: uuid::Uuid,
    pub scope: String,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="oauth_codes"]
pub struct OAuthCodes {
    pub id: uuid::Uuid,
    pub time: chrono::NaiveDateTime,
    pub redirect_uri: String,
    pub client_id: uuid::Uuid,
    pub user_id: String,
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="oauth_code_scopes"]
pub struct OAuthCodeScopes {
    pub code_id: uuid::Uuid,
    pub scope: String,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="oauth_token"]
pub struct OAuthToken {
    pub id: uuid::Uuid,
    pub time: chrono::NaiveDateTime,
    pub client_id: uuid::Uuid,
    pub user_id: String,
    pub revoked: bool
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="oauth_token_scopes"]
pub struct OAuthTokenScopes {
    pub token_id: uuid::Uuid,
    pub scope: String,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="accounts"]
pub struct Account {
    pub id: uuid::Uuid,
    pub iid: i32,
    pub actor: Option<String>,
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub locked: bool,
    pub bot: bool,
    pub group: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub default_sensitive: Option<bool>,
    pub default_language: Option<String>,
    pub discoverable: Option<bool>,
    pub follower_count: i32,
    pub following_count: i32,
    pub statuses_count: i32,
    pub owned_by: Option<String>,
    pub private_key: Option<String>,
    pub local: bool,
    pub inbox_url: Option<String>,
    pub outbox_url: Option<String>,
    pub shared_inbox_url: Option<String>,
    pub url: Option<String>,
    pub avatar_file: Option<String>,
    pub avatar_content_type: Option<String>,
    pub avatar_remote_url: Option<String>,
    pub header_file: Option<String>,
    pub header_content_type: Option<String>,
    pub header_remote_url: Option<String>,
    pub follower_collection_url: Option<String>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable, Clone, Debug)]
#[table_name="accounts"]
pub struct NewAccount {
    pub id: uuid::Uuid,
    pub actor: Option<String>,
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub locked: bool,
    pub bot: bool,
    pub group: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub default_sensitive: Option<bool>,
    pub default_language: Option<String>,
    pub discoverable: Option<bool>,
    pub follower_count: i32,
    pub following_count: i32,
    pub statuses_count: i32,
    pub owned_by: Option<String>,
    pub private_key: Option<String>,
    pub local: bool,
    pub inbox_url: Option<String>,
    pub outbox_url: Option<String>,
    pub shared_inbox_url: Option<String>,
    pub url: Option<String>,
    pub avatar_file: Option<String>,
    pub avatar_content_type: Option<String>,
    pub avatar_remote_url: Option<String>,
    pub header_file: Option<String>,
    pub header_content_type: Option<String>,
    pub header_remote_url: Option<String>,
    pub follower_collection_url: Option<String>,
}

impl Account {
    pub fn actor_id(&self, uri: &str) -> String {
        if self.local {
            format!("https://{}/as/users/{}", uri, self.id)
        } else {
            self.actor.clone().unwrap_or_default()
        }
    }

    pub fn follower_collection(&self, uri: &str) -> String {
        if self.local {
            format!("https://{}/as/users/{}/followers", uri, self.id)
        } else {
            self.follower_collection_url.clone().unwrap_or_default()
        }
    }

    pub fn key_id(&self, uri: &str) -> String {
        format!("https://{}/as/users/{}#key", uri, self.id)
    }
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="account_fields"]
pub struct AccountField {
    pub id: uuid::Uuid,
    pub account_id: uuid::Uuid,
    pub name: String,
    pub value: String,
    pub sort_order: i32,
}


#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="web_push_subscriptions"]
pub struct WebPushSubscription {
    pub id: uuid::Uuid,
    pub token_id: uuid::Uuid,
    pub account_id: uuid::Uuid,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub follow: bool,
    pub favourite: bool,
    pub reblog: bool,
    pub mention: bool,
    pub poll: bool,
    pub status: bool,
    pub follow_request: bool,
    pub update: bool,
    pub admin_sign_up: bool,
    pub admin_report: bool,
    pub policy: String,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="public_keys"]
pub struct PublicKey {
    pub id: uuid::Uuid,
    pub key_id: String,
    pub user_id: uuid::Uuid,
    pub key: String,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="following"]
pub struct Following {
    pub id: uuid::Uuid,
    pub iid: i32,
    pub follower: uuid::Uuid,
    pub followee: uuid::Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub pending: bool,
    pub notify: bool,
    pub reblogs: bool,
}

#[derive(Insertable, Clone, Debug)]
#[table_name="following"]
pub struct NewFollowing {
    pub id: uuid::Uuid,
    pub follower: uuid::Uuid,
    pub followee: uuid::Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub pending: bool,
    pub notify: bool,
    pub reblogs: bool,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="notifications"]
pub struct Notification {
    pub id: uuid::Uuid,
    pub iid: i32,
    pub notification_type: String,
    pub account: uuid::Uuid,
    pub cause: uuid::Uuid,
    pub status: Option<uuid::Uuid>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Insertable, Clone, Debug)]
#[table_name="notifications"]
pub struct NewNotification {
    pub id: uuid::Uuid,
    pub notification_type: String,
    pub account: uuid::Uuid,
    pub cause: uuid::Uuid,
    pub status: Option<uuid::Uuid>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="media"]
pub struct Media {
    pub id: uuid::Uuid,
    pub media_type: String,
    pub file: Option<String>,
    pub content_type: Option<String>,
    pub remote_url: Option<String>,
    pub preview_file: Option<String>,
    pub preview_content_type: Option<String>,
    pub blurhash: Option<String>,
    pub focus_x: Option<f64>,
    pub focus_y: Option<f64>,
    pub original_width: Option<i32>,
    pub original_height: Option<i32>,
    pub preview_width: Option<i32>,
    pub preview_height: Option<i32>,
    pub created_at: chrono::NaiveDateTime,
    pub description: Option<String>,
    pub owned_by: Option<String>,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="statuses"]
pub struct Status {
    pub id: uuid::Uuid,
    pub iid: i32,
    pub url: String,
    pub uri: Option<String>,
    pub text: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub in_reply_to_id: Option<uuid::Uuid>,
    pub boost_of_id: Option<uuid::Uuid>,
    pub in_reply_to_url: Option<String>,
    pub boost_of_url: Option<String>,
    pub sensitive: bool,
    pub spoiler_text: String,
    pub language: Option<String>,
    pub local: bool,
    pub account_id: uuid::Uuid,
    pub deleted_at: Option<chrono::NaiveDateTime>,
    pub edited_at: Option<chrono::NaiveDateTime>,
    pub public: bool,
    pub visible: bool,
    pub text_source: Option<String>,
    pub spoiler_text_source: Option<String>,
}

impl Status {
    pub fn url(&self, uri: &str) -> String {
        if self.local {
            if self.boost_of_id.is_some() {
                format!("https://{}/as/status/{}/activity", uri, self.id)
            } else {
                format!("https://{}/as/status/{}", uri, self.id)
            }
        } else {
            self.url.clone()
        }
    }

    pub fn activity_url(&self, uri: &str) -> Option<String> {
        if self.local {
            Some(format!("https://{}/as/status/{}/activity", uri, self.id))
        } else {
            None
        }
    }
}

#[derive(Insertable, Clone, Debug)]
#[table_name="statuses"]
pub struct NewStatus {
    pub id: uuid::Uuid,
    pub url: String,
    pub uri: Option<String>,
    pub text: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub in_reply_to_id: Option<uuid::Uuid>,
    pub boost_of_id: Option<uuid::Uuid>,
    pub in_reply_to_url: Option<String>,
    pub boost_of_url: Option<String>,
    pub sensitive: bool,
    pub spoiler_text: String,
    pub language: Option<String>,
    pub local: bool,
    pub account_id: uuid::Uuid,
    pub deleted_at: Option<chrono::NaiveDateTime>,
    pub edited_at: Option<chrono::NaiveDateTime>,
    pub public: bool,
    pub visible: bool,
    pub text_source: Option<String>,
    pub spoiler_text_source: Option<String>,
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="status_media_attachments"]
pub struct StatusMediaAttachment {
    pub status_id: uuid::Uuid,
    pub media_attachment_id: uuid::Uuid,
    pub attachment_order: i32,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="status_audiences"]
pub struct StatusAudience {
    pub id: uuid::Uuid,
    pub status_id: uuid::Uuid,
    pub mention: bool,
    pub account: Option<uuid::Uuid>,
    pub account_followers: Option<uuid::Uuid>,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="home_timeline"]
pub struct HomeTimelineEntry {
    pub id: i32,
    pub account_id: uuid::Uuid,
    pub status_id: uuid::Uuid,
}

#[derive(Insertable, Clone, Debug)]
#[table_name="home_timeline"]
pub struct NewHomeTimelineEntry {
    pub account_id: uuid::Uuid,
    pub status_id: uuid::Uuid,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="public_timeline"]
pub struct PublicTimelineEntry {
    pub id: i32,
    pub status_id: uuid::Uuid,
}

#[derive(Insertable, Clone, Debug)]
#[table_name="public_timeline"]
pub struct NewPublicTimelineEntry {
    pub status_id: uuid::Uuid,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="likes"]
pub struct Like {
    pub id: uuid::Uuid,
    pub iid: i32,
    pub status: Option<uuid::Uuid>,
    pub account: uuid::Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub url: Option<String>,
    pub local: bool,
    pub status_url: Option<String>,
}

impl Like {
    pub fn url(&self, uri: &str) -> String {
        if self.local {
            format!("https://{}/as/like/{}", uri, self.id)
        } else {
            self.url.clone().unwrap_or_default()
        }
    }
}

#[derive(Insertable, Clone, Debug)]
#[table_name="likes"]
pub struct NewLike {
    pub id: uuid::Uuid,
    pub status: Option<uuid::Uuid>,
    pub account: uuid::Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub url: Option<String>,
    pub local: bool,
    pub status_url: Option<String>,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="bookmarks"]
pub struct Bookmark {
    pub id: uuid::Uuid,
    pub iid: i32,
    pub status: uuid::Uuid,
    pub account: uuid::Uuid,
}

#[derive(Insertable, Clone, Debug)]
#[table_name="bookmarks"]
pub struct NewBookmark {
    pub id: uuid::Uuid,
    pub status: uuid::Uuid,
    pub account: uuid::Uuid,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="pins"]
pub struct Pin {
    pub id: uuid::Uuid,
    pub iid: i32,
    pub status: uuid::Uuid,
    pub account: uuid::Uuid,
}

#[derive(Insertable, Clone, Debug)]
#[table_name="pins"]
pub struct NewPin {
    pub id: uuid::Uuid,
    pub status: uuid::Uuid,
    pub account: uuid::Uuid,
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="account_notes"]
pub struct AccountNote {
    pub account: uuid::Uuid,
    pub owner: uuid::Uuid,
    pub note: String,
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="media_attachments"]
pub struct MediaAttachment {
    pub status: uuid::Uuid,
    pub media: uuid::Uuid,
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="status_mentions"]
pub struct StatusMention {
    pub id: uuid::Uuid,
    pub status: uuid::Uuid,
    pub account: uuid::Uuid,
}

#[derive(Insertable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Debug)]
#[table_name="tags"]
pub struct Tag {
    pub id: uuid::Uuid,
    pub name: String
}