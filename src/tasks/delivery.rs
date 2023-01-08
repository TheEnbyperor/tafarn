use crate::models;
use crate::views::activity_streams;
use celery::prelude::*;
use diesel::prelude::*;
use crate::tasks::config;

#[celery::task]
pub async fn deliver_object(object: activity_streams::Object, inbox: String, account: models::Account) -> TaskResult<()> {
    let config = config();
    let url = reqwest::Url::parse(&inbox).with_unexpected_err(|| "Invalid inbox URL")?;
    let host = url.host_str().map(|h| h.to_string()).ok_or(TaskError::UnexpectedError("Invalid inbox URL".to_string()))?;

    let body = object.to_json();
    let body_hash = openssl::hash::hash(openssl::hash::MessageDigest::sha256(), body.as_bytes())
        .with_unexpected_err(|| "Unable to hash body")?;
    let date = chrono::Utc::now().naive_utc().format("%a, %d %h %Y %H:%M:%S GMT").to_string();
    let mut req = crate::AS_CLIENT.post(url)
        .body(body)
        .header("Host", host)
        .header("Date", date)
        .header("Digest", format!("SHA-256={}", base64::encode(body_hash)))
        .header("Content-Type", "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"")
        .build().with_unexpected_err(|| "Unable to build request")?;

    if let Some(pkey) = account.private_key.as_ref().map(|k| match openssl::pkey::PKey::private_key_from_pem(k.as_bytes()) {
        Ok(pkey) => Ok(pkey),
        Err(_) => Err(TaskError::UnexpectedError("Invalid private key".to_string())),
    }).transpose()? {
        super::sign_request(&mut req, &pkey,  account.key_id(&config.uri))
            .map_err(|e| TaskError::UnexpectedError(e))?;
    }

    let r = crate::AS_CLIENT.execute(req).await
        .with_expected_err(|| "Unable to send request")?;

    let status = r.status();
    if !status.is_success() {
        let text = r.text().await.with_expected_err(|| "Unable to read response")?;
        return Err(TaskError::UnexpectedError(format!("Delivery failed ({}): {}", status, text)));
    }

    Ok(())
}