use lettre::{
    message::{header, Mailbox, Message},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use std::time::Duration;

use reqwest::StatusCode;
use serde::Serialize;

use crate::config::Config;

#[derive(Serialize)]
struct ResendEmailRequest<'a> {
    from: &'a str,
    to: Vec<&'a str>,
    subject: &'a str,
    html: &'a str,
}

fn build_verification_email_html(verify_url: &str) -> String {
    format!(
        concat!(
            "<div style=\"font-family:ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Helvetica,Arial;line-height:1.5\">",
            "<h2 style=\"margin:0 0 12px\">Verify your email</h2>",
            "<p style=\"margin:0 0 12px\">Click this link to verify your email:</p>",
            "<p style=\"margin:0 0 12px\"><a href=\"{url}\">{url}</a></p>",
            "<p style=\"margin:18px 0 0;color:#666;font-size:12px\">If you did not request this, you can ignore this email.</p>",
            "</div>"
        ),
        url = verify_url
    )
}

pub async fn try_send_verification_email(
    cfg: &Config,
    to: &str,
    verify_token: &str,
) -> Result<(), String> {
    let (Some(from), Some(url_base)) = (
        cfg.email_from.as_deref(),
        cfg.verify_email_url_base.as_deref(),
    ) else {
        return Ok(());
    };
    let verify_url = format!(
        "{}?token={}",
        url_base.trim_end_matches('/'),
        urlencoding::encode(verify_token)
    );

    let provider = cfg.email_provider.as_deref().unwrap_or("auto");
    match provider {
        "smtp" => {
            let (Some(host), Some(port)) = (cfg.smtp_host.as_deref(), cfg.smtp_port) else {
                return Err("EMAIL_PROVIDER=smtp but SMTP_HOST/SMTP_PORT are missing".to_string());
            };
            send_verification_email_smtp(
                host,
                port,
                cfg.smtp_starttls,
                cfg.smtp_username.as_deref(),
                cfg.smtp_password.as_deref(),
                from,
                to,
                &verify_url,
            )
            .await
        }
        "resend" => {
            let Some(api_key) = cfg.resend_api_key.as_deref() else {
                return Err("EMAIL_PROVIDER=resend but RESEND_API_KEY is missing".to_string());
            };
            send_verification_email_resend(api_key, from, to, &verify_url).await
        }
        "auto" => {
            if let (Some(host), Some(port)) = (cfg.smtp_host.as_deref(), cfg.smtp_port) {
                return send_verification_email_smtp(
                    host,
                    port,
                    cfg.smtp_starttls,
                    cfg.smtp_username.as_deref(),
                    cfg.smtp_password.as_deref(),
                    from,
                    to,
                    &verify_url,
                )
                .await;
            }
            if let Some(api_key) = cfg.resend_api_key.as_deref() {
                return send_verification_email_resend(api_key, from, to, &verify_url).await;
            }
            Ok(())
        }
        other => Err(format!(
            "unsupported EMAIL_PROVIDER={}, expected smtp|resend|auto",
            other
        )),
    }
}

pub async fn send_verification_email_resend(
    api_key: &str,
    from: &str,
    to: &str,
    verify_url: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let subject = "Verify your email";
    let html = build_verification_email_html(verify_url);

    let payload = ResendEmailRequest {
        from,
        to: vec![to],
        subject,
        html: &html,
    };

    let res = client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("resend request failed: {}", err))?;

    if res.status() == StatusCode::OK || res.status() == StatusCode::CREATED {
        return Ok(());
    }

    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    Err(format!("resend returned {}: {}", status, body))
}

pub async fn send_verification_email_smtp(
    host: &str,
    port: u16,
    starttls: bool,
    username: Option<&str>,
    password: Option<&str>,
    from: &str,
    to: &str,
    verify_url: &str,
) -> Result<(), String> {
    let subject = "Verify your email";
    let html = build_verification_email_html(verify_url);

    let from: Mailbox = from
        .parse()
        .map_err(|err| format!("invalid EMAIL_FROM: {}", err))?;
    let to: Mailbox = to
        .parse()
        .map_err(|err| format!("invalid recipient email: {}", err))?;

    let msg = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .header(header::ContentType::TEXT_HTML)
        .body(html)
        .map_err(|err| format!("build message failed: {}", err))?;

    let mut builder = if starttls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(|err| format!("smtp transport init failed: {}", err))?
            .port(port)
            .timeout(Some(Duration::from_secs(10)))
    } else {
        // Mailpit (local/CI) uses plain SMTP by default.
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
            .timeout(Some(Duration::from_secs(10)))
    };

    if let (Some(username), Some(password)) = (username, password) {
        builder = builder.credentials(lettre::transport::smtp::authentication::Credentials::new(
            username.to_string(),
            password.to_string(),
        ));
    }

    let transport = builder.build();
    transport
        .send(msg)
        .await
        .map_err(|err| format!("smtp send failed: {}", err))?;

    Ok(())
}
