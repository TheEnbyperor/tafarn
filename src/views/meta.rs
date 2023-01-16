use crate::AppConfig;
use rocket_dyn_templates::{Template, context};
use diesel::prelude::*;

#[get("/.well-known/host-meta")]
pub async fn host_meta(config: &rocket::State<AppConfig>) -> Template {
    Template::render("host-meta", context! { uri: config.uri.clone() })
}

#[derive(Serialize, Deserialize)]
pub struct JRD {
    pub subject: String,
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<JRDProperties>,
    pub links: Vec<JRDLink>
}

impl<'r> rocket::response::Responder<'r, 'static> for JRD {
    fn respond_to(self, _req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let body = serde_json::to_string(&self).unwrap();

        let mut res = rocket::Response::new();
        res.set_status(rocket::http::Status::Ok);
        res.adjoin_raw_header("Content-Type", "application/jrd+json");
        res.adjoin_raw_header("Access-Control-Allow-Methods", "GET, OPTIONS");
        res.adjoin_raw_header("Access-Control-Allow-Origin", "*");
        res.adjoin_raw_header("Access-Control-Allow-Credentials", "false");
        res.set_sized_body(body.len(), std::io::Cursor::new(body));
        Ok(res)
    }
}

#[derive(Serialize, Deserialize)]
pub struct JRDProperties {

}

#[derive(Serialize, Deserialize)]
pub struct JRDLink {
    pub rel: String,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub titles: Option<std::collections::HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<JRDProperties>
}

#[get("/.well-known/nodeinfo")]
pub fn well_known_node_info(
    config: &rocket::State<AppConfig>
) -> JRD {
    JRD {
        subject: config.uri.to_string(),
        aliases: vec![],
        properties: None,
        links: vec![JRDLink {
            rel: "http://nodeinfo.diaspora.software/ns/schema/2.1".to_string(),
            type_: Some("application/json".to_string()),
            href: Some(format!("https://{}/nodeinfo/2.1", config.uri)),
            titles: None,
            properties: None
        }, JRDLink {
            rel: "http://nodeinfo.diaspora.software/ns/schema/2.0".to_string(),
            type_: Some("application/json".to_string()),
            href: Some(format!("https://{}/nodeinfo/2.0", config.uri)),
            titles: None,
            properties: None
        }]
    }
}

#[get("/.well-known/webfinger?<resource>")]
pub async fn web_finger(
    db: crate::DbConn, config: &rocket::State<AppConfig>, resource: String, localizer: crate::i18n::Localizer
) -> Result<JRD, rocket::http::Status> {
    let (scheme, acct) = match resource.split_once(':') {
        Some((scheme, acct)) => (scheme, acct),
        None => return Err(rocket::http::Status::NotFound)
    };

    if scheme != "acct" {
        return Err(rocket::http::Status::NotFound);
    }

    let (username, domain) = match acct.split_once('@') {
        Some((username, domain)) => (username.to_string(), domain),
        None => return Err(rocket::http::Status::NotFound)
    };

    if domain != config.uri {
        return Err(rocket::http::Status::NotFound);
    }

    if username == config.uri {
        return Ok(JRD {
            subject: format!("acct:{}@{}", config.uri, config.uri),
            aliases: vec![],
            properties: None,
            links: vec![JRDLink {
                rel: "hhttp://webfinger.net/rel/profile-page".to_string(),
                type_: Some("text/html".to_string()),
                href: Some(config.uri.to_string()),
                titles: None,
                properties: None
            }, JRDLink {
                rel: "self".to_string(),
                type_: Some("application/activity+json".to_string()),
                href: Some(format!("https://{}/as/system", config.uri)),
                titles: None,
                properties: None
            }]
        })
    }

    let account: Option<crate::models::Account> = crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
        crate::schema::accounts::dsl::accounts.filter(
            crate::schema::accounts::dsl::username.eq(username)
        ).first(c).optional()
    }).await?;
    
    let account = match account {
        Some(account) => account,
        None => return Err(rocket::http::Status::NotFound)
    };

    Ok(JRD {
        subject: format!("acct:{}@{}", account.username, config.uri),
        aliases: vec![],
        properties: None,
        links: vec![JRDLink {
            rel: "hhttp://webfinger.net/rel/profile-page".to_string(),
            type_: Some("text/html".to_string()),
            href: Some(format!("https://{}/users/{}", config.uri, account.id)),
            titles: None,
            properties: None
        }, JRDLink {
            rel: "self".to_string(),
            type_: Some("application/activity+json".to_string()),
            href: Some(format!("https://{}/as/users/{}", config.uri, account.id)),
            titles: None,
            properties: None
        }]
    })
}

