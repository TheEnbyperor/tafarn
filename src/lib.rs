#![crate_type = "rlib"]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate i18n_embed_fl;

use std::path::PathBuf;
use rocket_sync_db_pools::database;
use rocket_sync_db_pools::Poolable;
use celery::prelude::*;

mod models;
mod schema;
pub mod i18n;
pub mod views;
pub mod csrf;
pub mod tasks;

#[database("db")]
pub struct DbConn(diesel::PgConnection);

embed_migrations!("./migrations");

pub async fn db_run<
    T: 'static + Send,
    F: 'static + FnOnce(&mut diesel::PgConnection) -> diesel::result::QueryResult<T> + Send
>(db: &DbConn, localizer: &i18n::Localizer, func: F) -> Result<T, views::Error> {
    Ok(match db.run(func).await {
        Ok(r) => r,
        Err(e) => {
            warn!("DB error: {}", e);
            return Err(views::Error {
                code: rocket::http::Status::InternalServerError,
                error: fl!(localizer, "error-db"),
            });
        }
    })
}

#[derive(rust_embed::RustEmbed)]
#[folder = "i18n"]
struct Localizations;

lazy_static! {
    pub static ref AS_CLIENT: reqwest::Client = {
        let mut headers = reqwest::header::HeaderMap::new();

        headers.insert("User-Agent", format!("Tafarn/{}", env!("CARGO_PKG_VERSION")).parse().unwrap());
        headers.insert("Accept", "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"".parse().unwrap());

        reqwest::ClientBuilder::new()
            .default_headers(headers)
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(15))
            .http1_title_case_headers()
            .https_only(true)
            .build().unwrap()
    };

    pub static ref HTML_RULES: sanitize_html::rules::Rules = {
        let class_re = regex::Regex::new("^((((h|p|u|dt|e)-[^ ]+)|mention|hashtag|ellipsis|invisible) ?)*$").unwrap();

        sanitize_html::rules::Rules::new()
            .allow_comments(true)
            .element(sanitize_html::rules::Element::new("abbr")
                .attribute("title", sanitize_html::rules::pattern::Pattern::any())
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("acronym")
                .attribute("title", sanitize_html::rules::pattern::Pattern::any())
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("a")
                .attribute("href", sanitize_html::rules::pattern::Pattern::any())
                .attribute("rel", sanitize_html::rules::pattern::Pattern::any())
                .attribute("target", sanitize_html::rules::pattern::Pattern::any())
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("b")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("blockquote")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("br")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("code")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("dd")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("del")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("div")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("dl")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("dt")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("em")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("i")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("li")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("ol")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("p")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("section")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("span")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("sub")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("sup")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("strong")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("table")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("td")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("tr")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("th")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
            .element(sanitize_html::rules::Element::new("ul")
                .attribute("class", sanitize_html::rules::pattern::Pattern::regex(class_re.clone())))
    };

    pub static ref COMRAK_OPTIONS: comrak::ComrakOptions = comrak::ComrakOptions {
        extension: comrak::ComrakExtensionOptions {
            strikethrough: true,
            tagfilter: true,
            table: true,
            autolink: true,
            tasklist: true,
            superscript: true,
            header_ids: None,
            footnotes: true,
            description_lists: true,
            front_matter_delimiter: None,
        },
        parse: comrak::ComrakParseOptions {
            smart: true,
            default_info_string: None,
        },
        render: comrak::ComrakRenderOptions {
            hardbreaks: true,
            github_pre_lang: false,
            escape: true,
            ..Default::default()
        }
    };

    pub static ref WEBFINGER_RE: regex::Regex = regex::Regex::new("@?(?P<acct>(?P<user>.+)@(?P<domain>.+))").unwrap();

    pub static ref LANGUAGE_LOADER: i18n_embed::fluent::FluentLanguageLoader = {
        use i18n_embed::LanguageLoader;

        let l = i18n_embed::fluent::fluent_language_loader!();
        l.load_available_languages(&crate::Localizations).unwrap();
        l.set_use_isolating(true);
        l
    };
}

pub type CeleryApp = std::sync::Arc<celery::Celery<AMQPBroker>>;

pub const AVATAR_SIZE: u32 = 400;
pub const HEADER_WIDTH: u32 = 1500;
pub const HEADER_HEIGHT: u32 = 500;
pub const PREVIEW_DIMENSION: u32 = 640;

#[derive(Deserialize)]
pub struct Config {
    jwt_secret: String,
    celery: CeleryConfig,
    oidc: OIDCConfig,
    uri: String,
    vapid_key: std::path::PathBuf,
    as_key: std::path::PathBuf,
    media_path: std::path::PathBuf,
}

#[derive(Deserialize)]
pub struct OIDCConfig {
    issuer_url: String,
    client_id: String,
    client_secret: String,
    #[serde(default)]
    required_role: Option<String>,
}

#[derive(Deserialize)]
pub struct CeleryConfig {
    amqp_url: String,
}

pub struct AppConfig {
    pub jwt_secret: jwt_simple::algorithms::HS512Key,
    pub uri: String,
    pub web_push_signature: web_push::PartialVapidSignatureBuilder,
    pub as_key: openssl::pkey::PKey<openssl::pkey::Private>,
    pub media_path: PathBuf,
}

pub struct App {
    pub rocket: rocket::Rocket<rocket::Build>,
    pub celery_app: CeleryApp,
    pub uri: String,
    pub vapid_key: Vec<u8>,
    pub as_key: openssl::pkey::PKey<openssl::pkey::Private>,
    pub media_path: PathBuf,
}

pub fn gen_media_path(root: &std::path::Path, ext: &str) -> (String, PathBuf) {
    let media_id = uuid::Uuid::new_v4();
    let media_name = format!("{}.{}", media_id.to_string(), ext);
    let media_path = root.join(&media_name);
    (media_name, media_path)
}

pub async fn setup() -> App {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let config = figment.extract::<Config>().expect("Unable to read config");

    let vapid_key_bytes = std::fs::read(config.vapid_key).expect("Unable to read VAPID key");
    let web_push_signature = web_push::VapidSignatureBuilder::from_pem_no_sub(vapid_key_bytes.as_slice()).expect("Unable to parse VAPID key");

    let as_key_bytes = std::fs::read(config.as_key).expect("Unable to read ActivityStreams key");
    let as_key = openssl::pkey::PKey::private_key_from_pem(as_key_bytes.as_slice()).expect("Unable to parse ActivityStreams key");

    let celery_app = celery::app!(
        broker = AMQPBroker { config.celery.amqp_url.clone() },
        tasks = [
            tasks::accounts::update_account,
            tasks::accounts::update_account_from_object,
            tasks::accounts::update_accounts,
            tasks::accounts::update_account_relations,
            tasks::accounts::deliver_account_update,
            tasks::accounts::delete_account,
            tasks::accounts::delete_account_by_id,

            tasks::inbox::process_activity,
            tasks::delivery::deliver_object,

            tasks::relationships::process_follow,
            tasks::relationships::process_undo_follow,
            tasks::relationships::follow_account,
            tasks::relationships::unfollow_account,
            tasks::relationships::process_accept_follow,
            tasks::relationships::process_reject_follow,

            tasks::notifications::notify,
            tasks::notifications::deliver_notification,

            tasks::statuses::create_status,
            tasks::statuses::create_announce,
            tasks::statuses::create_like,
            tasks::statuses::delete_status,
            tasks::statuses::delete_status_by_id,
            tasks::statuses::undo_announce,
            tasks::statuses::undo_like,
            tasks::statuses::insert_into_timelines,
            tasks::statuses::deliver_status,
            tasks::statuses::deliver_status_delete,
            tasks::statuses::deliver_boost,
            tasks::statuses::deliver_undo_boost,
            tasks::statuses::deliver_like,
            tasks::statuses::deliver_undo_like,
            tasks::statuses::get_replies,
        ],
        task_routes = [],
        prefetch_count = 5,
        acks_late = false,
        task_max_retries = 25,
        task_min_retry_delay = 30,
        task_retry_for_unexpected = false,
        broker_connection_retry = true,
        broker_connection_timeout = 10,
        heartbeat = Some(60),
    ).await.expect("Unable to setup Celery app");

    info!("Configuring OIDC");

    let oidc_app = views::oidc::OIDCApplication::new(
        &config.oidc.issuer_url,
        &config.oidc.client_id,
        &config.oidc.client_secret,
        config.oidc.required_role.as_deref(),
    ).await.expect("Unable to setup OIDC app");

    info!("Applying database migrations");

    let db_pool = diesel::PgConnection::pool("db", &rocket).unwrap();
    embedded_migrations::run_with_output(&db_pool.get().unwrap(), &mut std::io::stdout()).unwrap();

    App {
        uri: config.uri.clone(),
        rocket: rocket.manage(AppConfig {
            uri: config.uri.clone(),
            jwt_secret: jwt_simple::algorithms::HS512Key::from_bytes(config.jwt_secret.as_bytes()),
            web_push_signature,
            as_key: as_key.clone(),
            media_path: config.media_path.clone(),
        }).manage(oidc_app),
        celery_app,
        vapid_key: vapid_key_bytes,
        as_key,
        media_path: config.media_path,
    }
}