use crate::models;
use crate::views::activity_streams;
use celery::prelude::*;
use diesel::prelude::*;
use crate::tasks::config;

const SIGNED_HEADERS: [&str; 4] = ["host", "date", "digest", "content-type"];

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
        let mut signed_data = vec![
            format!("(request-target): post {}", req.url().path()),
        ];
        for (header_name, header_value) in req.headers().iter() {
            if SIGNED_HEADERS.iter().any(|h| header_name == h) {
                signed_data.push(format!(
                    "{}: {}",
                    header_name.as_str().to_lowercase(),
                    header_value.to_str().with_unexpected_err(|| "Unable to convert header to string")?
                ));
            }
        }

        let signed_data = signed_data.join("\n").into_bytes();
        let mut signer = openssl::sign::Signer::new(openssl::hash::MessageDigest::sha256(), &pkey)
            .with_unexpected_err(|| "Unable to create signer")?;
        let signature = signer.sign_oneshot_to_vec(&signed_data)
            .with_unexpected_err(|| "Unable to sign request")?;
        req.headers_mut().insert("Signature", format!(
            "keyId=\"{}\",algorithm=\"rsa-sha256\",headers=\"(request-target) {}\",signature=\"{}\"",
            account.key_id(&config.uri), SIGNED_HEADERS.join(" "),
            base64::encode(signature)
        ).parse().with_unexpected_err(|| "Unable to parse signature header")?);
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