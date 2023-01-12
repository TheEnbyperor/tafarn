#[macro_use]
extern crate log;

use rocket_sync_db_pools::Poolable;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Tafarn task runner starting...");

    let app = tafarn::setup().await;
    let db_pool = diesel::PgConnection::pool("db", &app.rocket).unwrap();
    let celery_app = std::sync::Arc::new(app.celery_app);

    tafarn::tasks::CONFIG.write().unwrap().replace(tafarn::tasks::Config {
        db: std::sync::Arc::new(db_pool),
        celery: celery_app.clone(),
        uri: app.uri,
        vapid_key: app.vapid_key,
        web_push_client: std::sync::Arc::new(web_push_old::WebPushClient::new()),
        as_key: std::sync::Arc::new(app.as_key),
    });

    celery_app.consume().await.unwrap();
}