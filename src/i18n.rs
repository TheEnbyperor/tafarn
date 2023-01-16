use std::ops::Deref;
use diesel::RunQueryDsl;

pub struct Languages(pub Vec<i18n_embed::unic_langid::LanguageIdentifier>);
pub struct Localizer {
    pub localizer: i18n_embed::fluent::FluentLanguageLoader,
    pub languages: Vec<i18n_embed::unic_langid::LanguageIdentifier>
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Localizer {
    type Error = &'static str;

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let langs = match request.guard::<Languages>().await {
            rocket::request::Outcome::Success(l) => l,
            rocket::request::Outcome::Failure(e) => return rocket::request::Outcome::Failure(e),
            rocket::request::Outcome::Forward(()) => return rocket::request::Outcome::Forward(()),
        };

        rocket::request::Outcome::Success(Localizer {
            localizer: crate::LANGUAGE_LOADER.select_languages(&langs.0),
            languages: langs.0
        })
    }
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Languages {
    type Error = &'static str;

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = request.headers();
        let langs = accept_language::parse(headers.get_one("Accept-Language").unwrap_or("en-GB"))
            .into_iter().map(|l| l.parse::<i18n_embed::unic_langid::LanguageIdentifier>())
            .collect::<Result<Vec<_>, _>>().unwrap_or_default();

        rocket::request::Outcome::Success(Languages(langs))
    }
}

impl Localizer {
    pub fn get_lang(id: &str) -> Self {
        let langs = match id.parse::<i18n_embed::unic_langid::LanguageIdentifier>() {
            Ok(lang) => vec![lang],
            Err(_) => vec![],
        };

        Localizer {
            localizer: crate::LANGUAGE_LOADER.select_languages(&langs),
            languages: langs
        }
    }

    pub fn get_lang_opt(id: Option<&str>) -> Self {
        let langs = match id {
            Some(id) => match id.parse::<i18n_embed::unic_langid::LanguageIdentifier>() {
                Ok(lang) => vec![lang],
                Err(_) => vec![]
            },
            None => vec![]
        };

        Localizer {
            localizer: crate::LANGUAGE_LOADER.select_languages(&langs),
            languages: langs
        }
    }
}

impl Deref for Localizer {
    type Target = i18n_embed::fluent::FluentLanguageLoader;

    fn deref(&self) -> &Self::Target {
        &self.localizer
    }
}

impl serde::Serialize for Localizer {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.languages.first() {
            Some(lang) => serializer.serialize_str(&lang.to_string()),
            None => serializer.serialize_none()
        }
    }
}

pub struct TeraLocalizer {
    cache: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<i18n_embed::unic_langid::LanguageIdentifier, i18n_embed::fluent::FluentLanguageLoader>>>
}

impl TeraLocalizer {
    pub fn new() -> Self {
        Self {
            cache: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()))
        }
    }
}

impl rocket_dyn_templates::tera::Function for TeraLocalizer {
    fn call(
        &self, args: &std::collections::HashMap<String, rocket_dyn_templates::tera::Value>
    ) -> rocket_dyn_templates::tera::Result<rocket_dyn_templates::tera::Value> {
        let mut args = args.clone();
        let lang = match args.remove("lang").ok_or_else(|| rocket_dyn_templates::tera::Error::msg("lang parameter is required"))? {
            rocket_dyn_templates::tera::Value::String(s) => Some(s),
            rocket_dyn_templates::tera::Value::Null => None,
            _ => return Err(rocket_dyn_templates::tera::Error::msg("lang parameter must be a string"))
        };
        let message_id = args.remove("id").ok_or_else(|| rocket_dyn_templates::tera::Error::msg("id parameter is required"))?
            .as_str().ok_or_else(|| rocket_dyn_templates::tera::Error::msg("id parameter must be a string"))?.to_string();

        let args = args.into_iter().map(|(k, v)| {
            let v = match v {
                rocket_dyn_templates::tera::Value::String(s) => fluent_bundle::types::FluentValue::String(std::borrow::Cow::Owned(s)),
                rocket_dyn_templates::tera::Value::Number(n) => fluent_bundle::types::FluentValue::Number(
                    fluent_bundle::types::FluentNumber::new(n.as_f64().unwrap_or(0.0), Default::default())
                ),
                _ => fluent_bundle::types::FluentValue::Error
            };
            (k, v)
        }).collect::<std::collections::HashMap<String, _>>();

        if let Some(lang) = lang {
            match lang.parse::<i18n_embed::unic_langid::LanguageIdentifier>() {
                Ok(lang) => {
                    {
                        let cache = self.cache.read().unwrap();
                        if let Some(localizer) = cache.get(&lang) {
                            return Ok(rocket_dyn_templates::tera::Value::String(localizer.get_args(&message_id, args)));
                        }
                    }
                    let l = crate::LANGUAGE_LOADER.select_languages(&[&lang]);
                    let s = l.get_args(&message_id, args);
                    self.cache.write().unwrap().insert(lang, l);
                    Ok(rocket_dyn_templates::tera::Value::String(s))
                },
                Err(_) => Ok(rocket_dyn_templates::tera::Value::String(crate::LANGUAGE_LOADER.get_args(&message_id, args)))
            }
        } else {
            Ok(rocket_dyn_templates::tera::Value::String(crate::LANGUAGE_LOADER.get_args(&message_id, args)))
        }
    }
}