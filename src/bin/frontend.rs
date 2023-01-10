#[macro_use]
extern crate log;

use rocket_sync_db_pools::Poolable;

pub struct CORS;

#[rocket::async_trait]
impl rocket::fairing::Fairing for CORS {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "CORS",
            kind: rocket::fairing::Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r rocket::Request<'_>, response: &mut rocket::Response<'r>) {
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[rocket::options("/<_..>")]
fn all_options() {}

#[tokio::main]
async fn main() -> Result<(), rocket::Error> {
    pretty_env_logger::init();

    let app = tafarn::setup().await;

    info!("Tafarn frontend starting...");

    tafarn::tasks::CONFIG.write().unwrap().replace(tafarn::tasks::Config {
        db: std::sync::Arc::new(diesel::PgConnection::pool("db", &app.rocket).unwrap()),
        celery: std::sync::Arc::new(app.celery_app.clone()),
        uri: app.uri,
        vapid_key: app.vapid_key,
        web_push_client: std::sync::Arc::new(web_push_old::WebPushClient::new()),
        as_key: std::sync::Arc::new(app.as_key),
    });

    let _ = app.rocket
        .attach(CORS)
        .attach(tafarn::DbConn::fairing())
        .attach(tafarn::csrf::CSRFFairing)
        .attach(rocket_dyn_templates::Template::fairing())
        .manage(app.celery_app)
        .mount("/static", rocket::fs::FileServer::from("./static"))
        .mount("/media", rocket::fs::FileServer::from("./media"))
        .mount("/", rocket::routes![
            all_options,

            tafarn::views::oidc::oidc_redirect,

            tafarn::views::meta::host_meta,
            tafarn::views::meta::web_finger,

            tafarn::views::oauth::api_apps_form,
            tafarn::views::oauth::api_apps_json,
            tafarn::views::oauth::oauth_authorize,
            tafarn::views::oauth::oauth_consent,
            tafarn::views::oauth::oauth_token_form,
            tafarn::views::oauth::oauth_token_json,
            tafarn::views::oauth::oauth_revoke,

            tafarn::views::instance::instance,
            tafarn::views::instance::instance_v2,
            tafarn::views::instance::instance_peers,
            tafarn::views::instance::instance_activity,
            tafarn::views::instance::custom_emoji,

            tafarn::views::accounts::verify_credentials,
            tafarn::views::accounts::update_credentials,
            tafarn::views::accounts::account,
            tafarn::views::accounts::account_statuses,
            tafarn::views::accounts::account_following,
            tafarn::views::accounts::account_followers,
            tafarn::views::accounts::lists,
            tafarn::views::accounts::relationships,
            tafarn::views::accounts::familiar_followers,
            tafarn::views::accounts::follow_account,
            tafarn::views::accounts::unfollow_account,
            tafarn::views::accounts::note,

            tafarn::views::timelines::timeline_home,
            tafarn::views::timelines::timeline_hashtag,
            tafarn::views::timelines::timeline_public,

            tafarn::views::conversations::conversations,
            tafarn::views::conversations::delete_conversation,
            tafarn::views::conversations::read_conversation,

            tafarn::views::lists::lists,
            tafarn::views::lists::list,
            tafarn::views::lists::create_list,
            tafarn::views::lists::update_list,
            tafarn::views::lists::delete_list,
            tafarn::views::lists::list_accounts,
            tafarn::views::lists::list_add_accounts,
            tafarn::views::lists::list_delete_accounts,

            tafarn::views::filters::filters,
            tafarn::views::filters::filter,
            tafarn::views::filters::create_filter,
            tafarn::views::filters::update_filter,
            tafarn::views::filters::delete_filter,

            tafarn::views::domain_blocks::domain_blocks,
            tafarn::views::domain_blocks::create_domain_block,
            tafarn::views::domain_blocks::delete_domain_block,

            tafarn::views::follow_requests::follow_requests,
            tafarn::views::follow_requests::accept_follow_request,
            tafarn::views::follow_requests::reject_follow_request,

            tafarn::views::suggestions::suggestions,
            tafarn::views::suggestions::delete_suggestion,

            tafarn::views::notifications::notifications,
            tafarn::views::notifications::notification,
            tafarn::views::notifications::clear_notifications,
            tafarn::views::notifications::dismiss_notification,

            tafarn::views::search::search,

            tafarn::views::mutes::mutes,
            tafarn::views::mutes::get_mute_account,
            tafarn::views::mutes::mute_account,
            tafarn::views::mutes::get_unmute_account,
            tafarn::views::mutes::unmute_account,

            tafarn::views::blocks::blocks,
            tafarn::views::blocks::get_block_account,
            tafarn::views::blocks::block_account,
            tafarn::views::blocks::get_unblock_account,
            tafarn::views::blocks::unblock_account,

            tafarn::views::media::upload_media,
            tafarn::views::media::get_media,
            tafarn::views::media::update_media,

            tafarn::views::statuses::get_status,
            tafarn::views::statuses::status_context,
            tafarn::views::statuses::status_boosted_by,
            tafarn::views::statuses::status_liked_by,
            tafarn::views::statuses::boost_status,
            tafarn::views::statuses::unboost_status,
            tafarn::views::statuses::like_status,
            tafarn::views::statuses::unlike_status,
            tafarn::views::statuses::bookmark_status,
            tafarn::views::statuses::unbookmark_status,
            tafarn::views::statuses::pin_status,
            tafarn::views::statuses::unpin_status,

            tafarn::views::web_push::create_subscription,
            tafarn::views::web_push::get_subscription,
            tafarn::views::web_push::update_subscription,
            tafarn::views::web_push::delete_subscription,

            tafarn::views::activity_streams::transient,
            tafarn::views::activity_streams::user,
            tafarn::views::activity_streams::get_inbox,
            tafarn::views::activity_streams::post_inbox,
            tafarn::views::activity_streams::get_outbox,
            tafarn::views::activity_streams::post_outbox,
            tafarn::views::activity_streams::get_shared_inbox,
            tafarn::views::activity_streams::post_shared_inbox,
            tafarn::views::activity_streams::system_actor,
            tafarn::views::activity_streams::status_activity,
            tafarn::views::activity_streams::like,
        ])
        .launch()
        .await?;
    Ok(())
}