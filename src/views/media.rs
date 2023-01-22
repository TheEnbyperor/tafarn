use diesel::prelude::*;
use image::GenericImageView;
use std::os::unix::fs::PermissionsExt;

pub struct Focus(f64, f64);

impl<'a> rocket::form::FromFormField<'a> for Focus {
    fn from_value(field: rocket::form::ValueField) -> rocket::form::Result<'a, Self> {
        let (x, y) = field.value.split_once(',')
            .ok_or(rocket::form::Error::validation("Invalid focus"))?;
        let x = x.parse().map_err(|_| rocket::form::Error::validation("Invalid X"))?;
        let y = y.parse().map_err(|_| rocket::form::Error::validation("Invalid Y"))?;
        Ok(Focus(x, y))
    }
}

#[derive(FromForm)]
pub struct MediaForm<'a> {
    file: rocket::fs::TempFile<'a>,
    thumbnail: Option<rocket::fs::TempFile<'a>>,
    description: Option<String>,
    focus: Option<Focus>,
}

#[post("/api/v1/media", data = "<form>")]
pub async fn upload_media(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    mut form: rocket::form::Form<MediaForm<'_>>, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::MediaAttachment>, super::Error> {
    if !user.has_scope("write:media") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let format = match form.file.content_type() {
        Some(f) => match image::ImageFormat::from_mime_type(f.to_string()) {
            Some(f) => f,
            None => return Err(super::Error {
                code: rocket::http::Status::UnprocessableEntity,
                error: fl!(localizer, "unsupported-media-type")
            })
        },
        None => return Err(super::Error {
            code: rocket::http::Status::BadRequest,
            error: fl!(localizer, "invalid-request")
        })
    };

    let attachment_id = uuid::Uuid::new_v4();
    let (image_name, image_path) = crate::gen_media_path(&config.media_path, "png");
    let (preview_image_name, preview_image_path) = crate::gen_media_path(&config.media_path, "png");

    let mut image_r = image::io::Reader::open(match form.file.path() {
        Some(p) => p,
        None => return Err(super::Error {
            code: rocket::http::Status::InternalServerError,
            error: fl!(localizer, "internal-server-error")
        })
    }).map_err(|_| super::Error {
        code: rocket::http::Status::InternalServerError,
        error: fl!(localizer, "internal-server-error")
    })?;
    image_r.set_format(format);
    let image = image_r.decode().map_err(|e| {
        warn!("Failed to decode image: {}", e);
        super::Error {
            code: rocket::http::Status::UnprocessableEntity,
            error: fl!(localizer, "failed-to-decode-image")
        }
    })?;
    let (width, height) = image.dimensions();
    let blurhash = blurhash::encode(4, 3, width, height, &image.to_rgba8().into_vec());

    let (preview_content_type, (preview_width, preview_height)) = match &mut form.thumbnail {
        Some(thumbnail) => {
            let preview_format = match thumbnail.content_type() {
                Some(f) => match image::ImageFormat::from_mime_type(f.to_string()) {
                    Some(f) => f,
                    None => return Err(super::Error {
                        code: rocket::http::Status::UnprocessableEntity,
                        error: fl!(localizer, "unsupported-media-type")
                    })
                },
                None => return Err(super::Error {
                    code: rocket::http::Status::BadRequest,
                    error: fl!(localizer, "invalid-request")
                })
            };
            let mut preview_image_r = image::io::Reader::open(match thumbnail.path() {
                Some(p) => p,
                None => return Err(super::Error {
                    code: rocket::http::Status::InternalServerError,
                    error: fl!(localizer, "internal-server-error")
                })
            }).map_err(|_| super::Error {
                code: rocket::http::Status::InternalServerError,
                error: fl!(localizer, "internal-server-error")
            })?;
            preview_image_r.set_format(preview_format);
            let preview_image = preview_image_r.decode().map_err(|e| {
                warn!("Failed to decode image: {}", e);
                super::Error {
                    code: rocket::http::Status::UnprocessableEntity,
                    error: fl!(localizer, "failed-to-decode-image")
                }
            })?;

            thumbnail.move_copy_to(&preview_image_path).await.map_err(|_| super::Error {
                code: rocket::http::Status::InternalServerError,
                error: fl!(localizer, "internal-server-error")
            })?;
            let perms = std::fs::Permissions::from_mode(0o644);
            std::fs::set_permissions(&preview_image_path, perms).map_err(|_| super::Error {
                code: rocket::http::Status::InternalServerError,
                error: fl!(localizer, "internal-server-error")
            })?;

            (thumbnail.content_type().unwrap().to_string(), preview_image.dimensions())
        },
        None => {
            let preview_image = image.thumbnail(crate::PREVIEW_DIMENSION, crate::PREVIEW_DIMENSION);

            let mut out_image_bytes: Vec<u8> = Vec::new();
            preview_image.write_to(&mut std::io::Cursor::new(&mut out_image_bytes), image::ImageOutputFormat::Jpeg(80))
                .map_err(|_| super::Error {
                    code: rocket::http::Status::InternalServerError,
                    error: fl!(localizer, "internal-server-error")
                })?;
            std::fs::write(&preview_image_path, &out_image_bytes).map_err(|_| super::Error {
                code: rocket::http::Status::InternalServerError,
                error: fl!(localizer, "internal-server-error")
            })?;

            ("image/jpeg".to_string(), preview_image.dimensions())
        }
    };

    form.file.move_copy_to(&image_path).await.map_err(|_| super::Error {
        code: rocket::http::Status::InternalServerError,
        error: fl!(localizer, "internal-server-error")
    })?;
    let perms = std::fs::Permissions::from_mode(0o644);
    std::fs::set_permissions(&image_path, perms).map_err(|_| super::Error {
        code: rocket::http::Status::InternalServerError,
        error: fl!(localizer, "internal-server-error")
    })?;

    let media = crate::models::Media {
        id: attachment_id,
        media_type: "image".to_string(),
        file: Some(image_name.clone()),
        content_type: Some(form.file.content_type().unwrap().to_string()),
        remote_url: None,
        preview_file: Some(preview_image_name.clone()),
        preview_content_type: Some(preview_content_type),
        blurhash: Some(blurhash.clone()),
        focus_x: form.focus.as_ref().map(|f| f.0),
        focus_y: form.focus.as_ref().map(|f| f.1),
        original_width: Some(width as i32),
        original_height: Some(height as i32),
        preview_width: Some(preview_width as i32),
        preview_height: Some(preview_height as i32),
        created_at: chrono::Utc::now().naive_utc(),
        description: form.description.clone(),
        owned_by: Some(user.subject),
    };
    crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
        diesel::insert_into(crate::schema::media::dsl::media)
            .values(media).execute(c)
    }).await?;

    Ok(rocket::serde::json::Json(super::objs::MediaAttachment {
        id: attachment_id.to_string(),
        media_type: super::objs::MediaAttachmentType::Image,
        url: Some(format!("https://{}/media/{}", config.uri, image_name)),
        preview_url: Some(format!("https://{}/media/{}", config.uri, preview_image_name)),
        blurhash: Some(blurhash),
        description: form.description.clone(),
        meta: super::objs::MediaAttachmentMeta {
            focus: form.focus.as_ref().map(|f| super::objs::MediaAttachmentMetaFocus {
                x: f.0,
                y: f.1
            })
        },
        remote_url: None
    }))
}

pub fn render_media_attachment(
    media: crate::models::Media, config: &crate::AppConfig, localizer: &crate::i18n::Localizer
) -> Result<super::objs::MediaAttachment, super::Error> {
    Ok(super::objs::MediaAttachment {
        id: media.id.to_string(),
        media_type: match media.media_type.as_str() {
            "image" => super::objs::MediaAttachmentType::Image,
            "video" => super::objs::MediaAttachmentType::Video,
            "gifv" => super::objs::MediaAttachmentType::Gifv,
            _ => return Err(super::Error {
                code: rocket::http::Status::InternalServerError,
                error: fl!(localizer, "internal-server-error")
            })
        },
        url: media.file.map(|f| format!("https://{}/media/{}", config.uri, f)),
        preview_url: media.preview_file.map(|f| format!("https://{}/media/{}", config.uri, f)),
        blurhash: media.blurhash,
        description: media.description,
        meta: super::objs::MediaAttachmentMeta {
            focus: match (media.focus_x, media.focus_y) {
                (Some(x), Some(y)) => Some(super::objs::MediaAttachmentMetaFocus {
                    x,
                    y,
                }),
                _ => None
            }
        },
        remote_url: media.remote_url
    })
}

#[get("/api/v1/media/<media_id>")]
pub async fn get_media(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    media_id: String, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::MediaAttachment>, super::Error> {
    if !user.has_scope("write:media") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let media_id = match uuid::Uuid::parse_str(&media_id) {
        Ok(id) => id,
        Err(_) => return Err(super::Error {
            code: rocket::http::Status::NotFound,
            error: fl!(localizer, "error-media-not-found")
        })
    };

    let media = match crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
        crate::schema::media::dsl::media
            .filter(crate::schema::media::dsl::id.eq(media_id))
            .first::<crate::models::Media>(c).optional()
    }).await? {
        Some(m) => m,
        None => return Err(super::Error {
            code: rocket::http::Status::NotFound,
            error: fl!(localizer, "error-media-not-found")
        })
    };

    if media.owned_by != Some(user.subject) {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        })
    }

    Ok(rocket::serde::json::Json(render_media_attachment(media, config, &localizer)?))
}

#[derive(FromForm)]
pub struct MediaUpdateForm<'a> {
    thumbnail: Option<rocket::fs::TempFile<'a>>,
    description: Option<String>,
    focus: Option<Focus>,
}

#[put("/api/v1/media/<media_id>", data = "<form>")]
pub async fn update_media(
    db: crate::DbConn, config: &rocket::State<crate::AppConfig>, user: super::oauth::TokenClaims,
    media_id: String, mut form: rocket::form::Form<MediaUpdateForm<'_>>, localizer: crate::i18n::Localizer
) -> Result<rocket::serde::json::Json<super::objs::MediaAttachment>, super::Error> {
    if !user.has_scope("write:media") {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    let media_id = match uuid::Uuid::parse_str(&media_id) {
        Ok(id) => id,
        Err(_) => return Err(super::Error {
            code: rocket::http::Status::NotFound,
            error: fl!(localizer, "error-media-not-found")
        })
    };

    let mut media = match crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
        crate::schema::media::dsl::media
            .filter(crate::schema::media::dsl::id.eq(media_id))
            .first::<crate::models::Media>(c).optional()
    }).await? {
        Some(m) => m,
        None => return Err(super::Error {
            code: rocket::http::Status::NotFound,
            error: fl!(localizer, "error-media-not-found")
        })
    };

    if media.owned_by != Some(user.subject) {
        return Err(super::Error {
            code: rocket::http::Status::Forbidden,
            error: fl!(localizer, "error-no-permission")
        });
    }

    if let Some(focus) = &form.focus {
        media.focus_x = Some(focus.0);
        media.focus_y = Some(focus.1);
    }

    if let Some(description) = &form.description {
        media.description = Some(description.clone());
    }

    if let Some(thumbnail) = &mut form.thumbnail {
        let format = match thumbnail.content_type() {
            Some(f) => match image::ImageFormat::from_mime_type(f.to_string()) {
                Some(f) => f,
                None => return Err(super::Error {
                    code: rocket::http::Status::UnprocessableEntity,
                    error: fl!(localizer, "unsupported-media-type")
                })
            },
            None => return Err(super::Error {
                code: rocket::http::Status::BadRequest,
                error: fl!(localizer, "invalid-request")
            })
        };
        let mut image_r = image::io::Reader::open(match thumbnail.path() {
            Some(p) => p,
            None => return Err(super::Error {
                code: rocket::http::Status::InternalServerError,
                error: fl!(localizer, "internal-server-error")
            })
        }).map_err(|_| super::Error {
            code: rocket::http::Status::InternalServerError,
            error: fl!(localizer, "internal-server-error")
        })?;
        image_r.set_format(format);
        let image = image_r.decode().map_err(|e| {
            warn!("Failed to decode image: {}", e);
            super::Error {
                code: rocket::http::Status::UnprocessableEntity,
                error: fl!(localizer, "failed-to-decode-image")
            }
        })?;

        let (preview_image_name, preview_image_path) = crate::gen_media_path(&config.media_path, "png");
        thumbnail.move_copy_to(&preview_image_path).await.map_err(|_| super::Error {
            code: rocket::http::Status::InternalServerError,
            error: fl!(localizer, "internal-server-error")
        })?;
        let perms = std::fs::Permissions::from_mode(0o644);
        std::fs::set_permissions(&preview_image_path, perms).map_err(|_| super::Error {
            code: rocket::http::Status::InternalServerError,
            error: fl!(localizer, "internal-server-error")
        })?;

        media.preview_file = Some(preview_image_name);
        media.preview_content_type = Some(thumbnail.content_type().unwrap().to_string());
        let (width, height) = image.dimensions();
        media.preview_width = Some(width as i32);
        media.preview_height = Some(height as i32);
    }

    let media = crate::db_run(&db, &localizer, move |c| -> QueryResult<_> {
        diesel::update(crate::schema::media::dsl::media
            .filter(crate::schema::media::dsl::id.eq(media_id)))
            .set(media).get_result::<crate::models::Media>(c)
    }).await?;

    Ok(rocket::serde::json::Json(render_media_attachment(media, config, &localizer)?))
}