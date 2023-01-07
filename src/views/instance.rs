use crate::AppConfig;
use crate::views::objs::InstanceV2Configuration;

#[get("/api/v1/instance")]
pub async fn instance(config: &rocket::State<AppConfig>) -> rocket::serde::json::Json<super::objs::Instance> {
    rocket::serde::json::Json(super::objs::Instance {
        uri: config.uri.clone(),
        title: "Tafarn Test".to_string(),
        short_description: "".to_string(),
        description: "".to_string(),
        email: "test@example.com".to_string(),
        version: "4.0.2".to_string(),
        urls: super::objs::InstanceURLs {
            streaming_api: None,
        },
        stats: super::objs::InstanceStats {
            user_count: 0,
            status_count: 0,
            domain_count: 0,
        },
        thumbnail: None,
        languages: vec!["en".to_string()],
        registrations: true,
        approval_required: false,
        contact_account: None,
        invites_enabled: false,
    })
}

#[get("/api/v2/instance")]
pub async fn instance_v2(config: &rocket::State<AppConfig>) -> rocket::serde::json::Json<super::objs::InstanceV2> {
    rocket::serde::json::Json(super::objs::InstanceV2 {
        domain: config.uri.clone(),
        title: "Tafarn Test".to_string(),
        description: "".to_string(),
        version: "4.0.2".to_string(),
        source_url: "".to_string(),
        usage: super::objs::InstanceV2Usage {
            users: super::objs::InstanceV2UsageUsers {
                active_month: 0,
            },
        },
        configuration: InstanceV2Configuration {
            urls: super::objs::InstanceV2URLs {
                streaming_api: "".to_string(),
            },
            accounts: super::objs::InstanceV2Accounts {
                max_featured_tags: 0
            },
            statuses: super::objs::InstanceV2Statuses {
                max_characters: 500,
                max_media_attachments: 0,
                characters_reserved_per_url: 0
            },
            media_attachments: super::objs::InstanceV2MediaAttachments {
                supported_mime_types: vec![],
                image_size_limit: 0,
                image_matrix_limit: 0,
                video_size_limit: 0,
                video_frame_rate_limit: 0,
                video_matrix_limit: 0
            },
            polls: super::objs::InstanceV2Polls {
                max_options: 0,
                max_characters_per_option: 0,
                min_expiration: 0,
                max_expiration: 0
            },
            translation: super::objs::InstanceV2Translation {
                enabled: false
            }
        },
        thumbnail: super::objs::InstanceV2Thumbnail {
            url: "".to_string(),
            blurhash: None,
            versions: None,
        },
        languages: vec!["en".to_string()],
        registrations: super::objs::InstanceV2Registrations {
            enabled: true,
            approval_required: false,
            message: None,
        },
        contact: super::objs::InstanceV2Contact {
            email: "test@example.com".to_string(),
            account: None,
        },
        rules: vec![]
    })
}

#[get("/api/v1/instance/peers")]
pub async fn instance_peers() -> rocket::serde::json::Json<Vec<String>> {
    rocket::serde::json::Json(vec![])
}

#[derive(Serialize)]
pub struct Activity {
    week: i64,
    statuses: i64,
    logins: i64,
    registrations: i64
}

#[get("/api/v1/instance/activity")]
pub async fn instance_activity() -> rocket::serde::json::Json<Vec<Activity>> {
    rocket::serde::json::Json(vec![])
}

#[get("/api/v1/custom_emojis")]
pub async fn custom_emoji() -> rocket::serde::json::Json<Vec<super::objs::Emoji>> {
    rocket::serde::json::Json(vec![])
}