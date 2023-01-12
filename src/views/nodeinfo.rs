use std::collections::HashMap;
use crate::AppConfig;

#[derive(Serialize, Clone)]
pub struct NodeInfo2_1 {
    pub version: String,
    pub software: Software2_1,
    pub protocols: Vec<Protocols2_0>,
    pub services: Services2_0,
    #[serde(rename = "openRegistrations")]
    pub open_registrations: bool,
    pub usage: Usage2_0,
    pub metadata: HashMap<String, ()>
}

#[derive(Serialize, Clone)]
pub struct Software2_1 {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>
}

#[derive(Serialize, Clone)]
pub struct NodeInfo2_0 {
    pub version: String,
    pub software: Software2_0,
    pub protocols: Vec<Protocols2_0>,
    pub services: Services2_0,
    #[serde(rename = "openRegistrations")]
    pub open_registrations: bool,
    pub usage: Usage2_0,
    pub metadata: HashMap<String, ()>
}

#[derive(Serialize, Clone)]
pub struct Software2_0 {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Copy, Clone)]
pub enum Protocols2_0 {
    #[serde(rename = "activitypub")]
    ActivityPub,
    #[serde(rename = "buddycloud")]
    BuddyCloud,
    #[serde(rename = "dfrn")]
    Dfrn,
    #[serde(rename = "diaspora")]
    Diaospora,
    #[serde(rename = "libertree")]
    Libertree,
    #[serde(rename = "ostatus")]
    OStatus,
    #[serde(rename = "pumpio")]
    PumpIo,
    #[serde(rename = "tent")]
    Tent,
    #[serde(rename = "xmpp")]
    Xmpp,
    #[serde(rename = "zot")]
    Zot,
}

#[derive(Serialize, Clone)]
pub struct Services2_0 {
    pub inbound: Vec<NodeInfoServicesInbound2_0>,
    pub outbound: Vec<NodeInfoServicesOutbound2_0>,
}

#[derive(Serialize, Copy, Clone)]
pub enum NodeInfoServicesInbound2_0 {
    #[serde(rename = "atom1.0")]
    Atom1_0,
    #[serde(rename = "gnusocial")]
    GnuSocial,
    #[serde(rename = "imap")]
    Imap,
    #[serde(rename = "pnut")]
    Pnut,
    #[serde(rename = "pop3")]
    Pop3,
    #[serde(rename = "pumpio")]
    PumpIo,
    #[serde(rename = "rss2.0")]
    Rss2_0,
    #[serde(rename = "twitter")]
    Twitter
}

#[derive(Serialize, Copy, Clone)]
pub enum NodeInfoServicesOutbound2_0 {
    #[serde(rename = "atom1.0")]
    Atom1_0,
    #[serde(rename = "blogger")]
    Blogger,
    #[serde(rename = "buddycloud")]
    Buddycloud,
    #[serde(rename = "diaspora")]
    Diaspora,
    #[serde(rename = "dreamwidth")]
    Dreamwidth,
    #[serde(rename = "drupal")]
    Drupal,
    #[serde(rename = "facebook")]
    Facebook,
    #[serde(rename = "frendica")]
    Friendica,
    #[serde(rename = "gnusocial")]
    GnuSocial,
    #[serde(rename = "google")]
    Google,
    #[serde(rename = "insanejournal")]
    InsaneJournal,
    #[serde(rename = "libertreaa")]
    Libertree,
    #[serde(rename = "linkedin")]
    LinkedIn,
    #[serde(rename = "livejournal")]
    LiveJournal,
    #[serde(rename = "mediagoblin")]
    Mediagoblin,
    #[serde(rename = "myspace")]
    MySpace,
    #[serde(rename = "pinterest")]
    Pinterest,
    #[serde(rename = "pnut")]
    Pnut,
    #[serde(rename = "posterus")]
    Posterous,
    #[serde(rename = "pumpio")]
    PumpIo,
    #[serde(rename = "redmatrix")]
    RedMatrix,
    #[serde(rename = "rss2.0")]
    Rss2_0,
    #[serde(rename = "smtp")]
    Smtp,
    #[serde(rename = "tent")]
    Tent,
    #[serde(rename = "tumblr")]
    Tumblr,
    #[serde(rename = "twitter")]
    Twitter,
    #[serde(rename = "wordpress")]
    Wordpress,
    #[serde(rename = "xmpp")]
    Xmpp
}

#[derive(Serialize, Copy, Clone)]
pub struct Usage2_0 {
    pub users: Users2_0,
    #[serde(rename = "localPosts")]
    pub local_posts: Option<u64>,
    #[serde(rename = "localComments")]
    pub local_comments: Option<u64>
}

#[derive(Serialize, Copy, Clone)]
pub struct Users2_0 {
    pub total: Option<u64>,
    #[serde(rename = "activeHalfyear")]
    pub active_half_year: Option<u64>,
    #[serde(rename = "activeMonth")]
    pub active_month: Option<u64>
}

pub struct NodeInfo<T> {
    inner: T,
    profile: &'static str
}

impl<'r, T: serde::ser::Serialize> rocket::response::Responder<'r, 'static> for NodeInfo<T> {
    fn respond_to(self, _req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let body = serde_json::to_string(&self.inner).unwrap();

        let mut res = rocket::Response::new();
        res.set_status(rocket::http::Status::Ok);
        res.adjoin_raw_header(
            "Content-Type",
            format!("application/json; profile=\"{}\"", self.profile));
        res.adjoin_raw_header("Access-Control-Allow-Methods", "GET, OPTIONS");
        res.adjoin_raw_header("Access-Control-Allow-Origin", "*");
        res.adjoin_raw_header("Access-Control-Allow-Credentials", "false");
        res.set_sized_body(body.len(), std::io::Cursor::new(body));
        Ok(res)
    }
}

#[get("/nodeinfo/2.1")]
pub fn node_info_2_1() -> NodeInfo<NodeInfo2_1> {
    let repository = env!("CARGO_PKG_REPOSITORY").to_string();
    let homepage = env!("CARGO_PKG_HOMEPAGE").to_string();
    NodeInfo {
        inner: NodeInfo2_1 {
            version: "2.1".to_string(),
            software: Software2_1 {
                name: "Tafarn".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                repository: if repository.is_empty() {
                    None
                } else {
                    Some(repository)
                },
                homepage: if homepage.is_empty() {
                    None
                } else {
                    Some(homepage)
                }
            },
            protocols: vec![Protocols2_0::ActivityPub],
            services: Services2_0 { inbound: vec![], outbound: vec![] },
            open_registrations: true,
            usage: Usage2_0 {
                users: Users2_0 {
                    total: None,
                    active_half_year: None,
                    active_month: None
                },
                local_posts: None,
                local_comments: None
            },
            metadata: Default::default()
        },
        profile: "http://nodeinfo.diaspora.software/ns/schema/2.1#"
    }
}

#[get("/nodeinfo/2.0")]
pub fn node_info_2_0() -> NodeInfo<NodeInfo2_0> {
    NodeInfo {
        inner: NodeInfo2_0 {
            version: "2.1".to_string(),
            software: Software2_0 {
                name: "Tafarn".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            protocols: vec![Protocols2_0::ActivityPub],
            services: Services2_0 { inbound: vec![], outbound: vec![] },
            open_registrations: true,
            usage: Usage2_0 {
                users: Users2_0 {
                    total: None,
                    active_half_year: None,
                    active_month: None
                },
                local_posts: None,
                local_comments: None
            },
            metadata: Default::default()
        },
        profile: "http://nodeinfo.diaspora.software/ns/schema/2.0#"
    }
}