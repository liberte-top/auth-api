use reqwest::StatusCode;
use serde::Deserialize;
use std::{env, time::Duration};
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Deserialize)]
struct RegisterResponse {
    account_uid: String,
    email: String,
    verification_required: bool,
    verification_expires_at: String,
}

#[derive(Deserialize)]
struct LoginResponse {
    account_uid: String,
    email: Option<String>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    code: Option<String>,
}

#[derive(Deserialize)]
struct VerifyResponse {
    status: String,
}

#[derive(Deserialize)]
struct MeResponse {
    account_uid: String,
    email: Option<String>,
}

#[tokio::test]
async fn smoke_auth_flow() {
    dotenvy::dotenv().ok();

    // This test expects the full local stack to be up (auth-api reachable + Mailpit reachable).
    // To keep `cargo test` fast and reliable by default, only run when explicitly enabled.
    let run_smoke = env::var("RUN_SMOKE_AUTH")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !run_smoke {
        eprintln!("skipping smoke_auth_flow (set RUN_SMOKE_AUTH=1 to enable)");
        return;
    }

    let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3333".to_string());
    let mailpit_base_url =
        env::var("MAILPIT_BASE_URL").unwrap_or_else(|_| "http://localhost:8025".to_string());
    let resend_api_base =
        env::var("RESEND_API_BASE").unwrap_or_else(|_| "https://api.resend.com".to_string());
    let smoke_email_source = env::var("SMOKE_EMAIL_SOURCE")
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            let provider = env::var("EMAIL_PROVIDER").unwrap_or_else(|_| "auto".to_string());
            if provider.eq_ignore_ascii_case("resend") {
                "resend".to_string()
            } else {
                "mailpit".to_string()
            }
        });
    let retries: usize = env::var("SMOKE_AUTH_RETRIES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    let retry_delay_ms: u64 = env::var("SMOKE_AUTH_RETRY_DELAY_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(300);

    let client = reqwest::Client::new();
    wait_for_health(&client, &base_url, retries, retry_delay_ms).await;

    let email = build_test_email(&smoke_email_source);

    let register = client
        .post(format!("{}/api/v1/auth/register", base_url))
        .json(&serde_json::json!({
            "email": email,
            "password": "Abcdef1!",
        }))
        .send()
        .await
        .expect("register request failed");
    assert_eq!(register.status(), StatusCode::CREATED);
    let register_json: serde_json::Value = register.json().await.expect("register json");
    assert!(
        register_json.get("verification_token").is_none(),
        "register response must not expose verification_token: {}",
        register_json
    );
    let register_body: RegisterResponse =
        serde_json::from_value(register_json).expect("register response parse");
    assert!(!register_body.account_uid.is_empty());
    assert!(!register_body.email.is_empty());
    assert!(register_body.verification_required);
    assert!(!register_body.verification_expires_at.is_empty());

    let login_before = client
        .post(format!("{}/api/v1/auth/login", base_url))
        .json(&serde_json::json!({
            "identifier": register_body.email,
            "password": "Abcdef1!",
        }))
        .send()
        .await
        .expect("login before verify failed");
    assert_eq!(login_before.status(), StatusCode::FORBIDDEN);
    let login_before_body: ErrorResponse = login_before.json().await.expect("login error json");
    assert_eq!(
        login_before_body.code.as_deref(),
        Some("email_not_verified")
    );

    let verification_token = match smoke_email_source.as_str() {
        "mailpit" => {
            // Fetch verification token from Mailpit inbox (black-box end-to-end).
            wait_for_verification_token_from_mailpit(
                &client,
                &mailpit_base_url,
                &register_body.email,
                retries,
                retry_delay_ms,
            )
            .await
        }
        "resend" => {
            // Fetch verification token via Resend email APIs.
            let api_key = env::var("RESEND_API_KEY")
                .expect("RESEND_API_KEY is required when SMOKE_EMAIL_SOURCE=resend");
            wait_for_verification_token_from_resend(
                &client,
                &resend_api_base,
                &api_key,
                &register_body.email,
                retries,
                retry_delay_ms,
            )
            .await
        }
        other => panic!(
            "unsupported SMOKE_EMAIL_SOURCE={}, expected mailpit|resend",
            other
        ),
    };

    let verify = client
        .post(format!("{}/api/v1/auth/verify-email", base_url))
        .json(&serde_json::json!({
            "token": verification_token,
        }))
        .send()
        .await
        .expect("verify request failed");
    assert_eq!(verify.status(), StatusCode::OK);
    let verify_body: VerifyResponse = verify.json().await.expect("verify json");
    assert_eq!(verify_body.status, "ok");

    let login_after = client
        .post(format!("{}/api/v1/auth/login", base_url))
        .json(&serde_json::json!({
            "identifier": register_body.email,
            "password": "Abcdef1!",
        }))
        .send()
        .await
        .expect("login after verify failed");
    assert_eq!(login_after.status(), StatusCode::OK);

    let sid_cookie = extract_sid_cookie(&login_after);
    let login_body: LoginResponse = login_after.json().await.expect("login json");
    assert_eq!(login_body.account_uid, register_body.account_uid);
    assert_eq!(
        login_body.email.as_deref(),
        Some(register_body.email.as_str())
    );

    let me = client
        .get(format!("{}/api/v1/me", base_url))
        .header(reqwest::header::COOKIE, sid_cookie.clone())
        .send()
        .await
        .expect("me request failed");
    assert_eq!(me.status(), StatusCode::OK);
    let me_body: MeResponse = me.json().await.expect("me json");
    assert_eq!(me_body.account_uid, register_body.account_uid);
    assert_eq!(me_body.email.as_deref(), Some(register_body.email.as_str()));

    let logout = client
        .post(format!("{}/api/v1/auth/logout", base_url))
        .header(reqwest::header::COOKIE, sid_cookie.clone())
        .send()
        .await
        .expect("logout request failed");
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let me_after = client
        .get(format!("{}/api/v1/me", base_url))
        .header(reqwest::header::COOKIE, sid_cookie)
        .send()
        .await
        .expect("me after logout request failed");
    assert_eq!(me_after.status(), StatusCode::UNAUTHORIZED);

    let delete_result = client
        .delete(format!(
            "{}/api/v1/accounts/{}",
            base_url, register_body.account_uid
        ))
        .send()
        .await;
    let _ = delete_result;
}

async fn wait_for_health(client: &reqwest::Client, base_url: &str, retries: usize, delay_ms: u64) {
    let url = format!("{}/api/v1/health", base_url);
    for attempt in 0..retries {
        match client.get(&url).send().await {
            Ok(response) if response.status() == StatusCode::OK => return,
            _ => {
                if attempt + 1 >= retries {
                    panic!(
                        "service not ready after {} attempts (base_url={}); 建议检查本地容器是否未启动",
                        retries, base_url
                    );
                }
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

async fn wait_for_verification_token_from_mailpit(
    client: &reqwest::Client,
    mailpit_base_url: &str,
    to_email: &str,
    retries: usize,
    delay_ms: u64,
) -> String {
    for attempt in 0..retries {
        match fetch_latest_mailpit_token(client, mailpit_base_url, to_email).await {
            Ok(Some(token)) => return token,
            Ok(None) => {}
            Err(err) => {
                eprintln!("mailpit poll error (attempt {}): {}", attempt + 1, err);
            }
        }

        if attempt + 1 >= retries {
            panic!(
                "verification email not found in mailpit after {} attempts (mailpit_base_url={})",
                retries, mailpit_base_url
            );
        }
        sleep(Duration::from_millis(delay_ms)).await;
    }
    unreachable!()
}

async fn fetch_latest_mailpit_token(
    client: &reqwest::Client,
    mailpit_base_url: &str,
    to_email: &str,
) -> Result<Option<String>, String> {
    let url = format!("{}/api/v1/messages", mailpit_base_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("mailpit messages request failed: {}", err))?;
    if !res.status().is_success() {
        return Err(format!("mailpit messages returned {}", res.status()));
    }
    let value: serde_json::Value = res
        .json()
        .await
        .map_err(|err| format!("mailpit messages json parse failed: {}", err))?;

    let list = value
        .get("messages")
        .or_else(|| value.get("Messages"))
        .or_else(|| value.get("items"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| "mailpit messages json missing list".to_string())?;

    let matching = list
        .iter()
        .find(|item| mailpit_message_matches_to(item, to_email));
    let Some(first) = matching else {
        return Ok(None);
    };

    let id = first
        .get("ID")
        .or_else(|| first.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "mailpit message missing ID".to_string())?;

    let detail_url = format!(
        "{}/api/v1/message/{}",
        mailpit_base_url.trim_end_matches('/'),
        id
    );
    let detail = client
        .get(&detail_url)
        .send()
        .await
        .map_err(|err| format!("mailpit message detail request failed: {}", err))?;
    if !detail.status().is_success() {
        return Err(format!(
            "mailpit message detail returned {}",
            detail.status()
        ));
    }
    let detail_json: serde_json::Value = detail
        .json()
        .await
        .map_err(|err| format!("mailpit detail json parse failed: {}", err))?;
    let s = detail_json.to_string();
    Ok(extract_token_from_text(&s))
}

fn extract_token_from_text(text: &str) -> Option<String> {
    let idx = text.find("token=")?;
    let rest = &text[idx + "token=".len()..];
    let mut end = rest.len();
    for (i, ch) in rest.char_indices() {
        if ch.is_whitespace()
            || ch == '&'
            || ch == '"'
            || ch == '\''
            || ch == '<'
            || ch == '>'
            || ch == '\\'
        {
            end = i;
            break;
        }
    }
    let token = &rest[..end];
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn build_test_email(source: &str) -> String {
    if source == "resend" {
        // For real delivery E2E, use a fixed mailbox from env and add a run-unique plus alias.
        let base = env::var("SMOKE_TEST_EMAIL_BASE")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .expect("SMOKE_TEST_EMAIL_BASE is required when SMOKE_EMAIL_SOURCE=resend");
        return plus_alias_email(&base, &format!("smoke{}", Uuid::new_v4().simple()));
    }
    format!("smoke+{}@example.com", Uuid::new_v4().simple())
}

fn plus_alias_email(base: &str, alias: &str) -> String {
    if let Some((local, domain)) = base.split_once('@') {
        return format!("{}+{}@{}", local, alias, domain);
    }
    panic!("invalid SMOKE_TEST_EMAIL_BASE={}", base);
}

fn mailpit_message_matches_to(message: &serde_json::Value, to_email: &str) -> bool {
    let Some(to_list) = message.get("To").and_then(|v| v.as_array()) else {
        return false;
    };
    to_list.iter().any(|entry| {
        entry
            .get("Address")
            .and_then(|v| v.as_str())
            .map(|addr| addr.eq_ignore_ascii_case(to_email))
            .unwrap_or(false)
    })
}

async fn wait_for_verification_token_from_resend(
    client: &reqwest::Client,
    resend_api_base: &str,
    api_key: &str,
    to_email: &str,
    retries: usize,
    delay_ms: u64,
) -> String {
    for attempt in 0..retries {
        match fetch_latest_resend_token(client, resend_api_base, api_key, to_email).await {
            Ok(Some(token)) => return token,
            Ok(None) => {}
            Err(err) => {
                eprintln!("resend poll error (attempt {}): {}", attempt + 1, err);
            }
        }

        if attempt + 1 >= retries {
            panic!(
                "verification email not found in resend after {} attempts (resend_api_base={}, to={})",
                retries, resend_api_base, to_email
            );
        }
        sleep(Duration::from_millis(delay_ms)).await;
    }
    unreachable!()
}

async fn fetch_latest_resend_token(
    client: &reqwest::Client,
    resend_api_base: &str,
    api_key: &str,
    to_email: &str,
) -> Result<Option<String>, String> {
    let base = resend_api_base.trim_end_matches('/');
    let list_url = format!("{}/emails", base);
    let list_res = client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|err| format!("resend list emails request failed: {}", err))?;
    if !list_res.status().is_success() {
        let status = list_res.status();
        let body = list_res.text().await.unwrap_or_default();
        return Err(format!("resend list emails returned {}: {}", status, body));
    }
    let list_json: serde_json::Value = list_res
        .json()
        .await
        .map_err(|err| format!("resend list emails json parse failed: {}", err))?;

    let list = list_json
        .get("data")
        .or_else(|| list_json.get("emails"))
        .or_else(|| list_json.get("messages"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| "resend list emails json missing list".to_string())?;

    let message = list
        .iter()
        .find(|item| resend_message_matches_to(item, to_email));
    let Some(message) = message else {
        return Ok(None);
    };

    let id = message
        .get("id")
        .or_else(|| message.get("Id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "resend message missing id".to_string())?;

    let detail_url = format!("{}/emails/{}", base, id);
    let detail_res = client
        .get(&detail_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|err| format!("resend retrieve email request failed: {}", err))?;
    if !detail_res.status().is_success() {
        let status = detail_res.status();
        let body = detail_res.text().await.unwrap_or_default();
        return Err(format!(
            "resend retrieve email returned {}: {}",
            status, body
        ));
    }
    let detail_json: serde_json::Value = detail_res
        .json()
        .await
        .map_err(|err| format!("resend retrieve email json parse failed: {}", err))?;

    // Try structured fields first, then fallback to stringified JSON scan.
    if let Some(text) = find_resend_email_body_text(&detail_json) {
        if let Some(token) = extract_token_from_text(&text) {
            return Ok(Some(token));
        }
    }
    Ok(extract_token_from_text(&detail_json.to_string()))
}

fn resend_message_matches_to(message: &serde_json::Value, to_email: &str) -> bool {
    let direct = message
        .get("to")
        .or_else(|| message.get("To"))
        .or_else(|| message.get("recipient"))
        .or_else(|| message.get("Recipient"));

    match direct {
        Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case(to_email),
        Some(serde_json::Value::Array(arr)) => arr.iter().any(|x| match x {
            serde_json::Value::String(s) => s.eq_ignore_ascii_case(to_email),
            serde_json::Value::Object(obj) => obj
                .get("email")
                .or_else(|| obj.get("address"))
                .and_then(|v| v.as_str())
                .map(|s| s.eq_ignore_ascii_case(to_email))
                .unwrap_or(false),
            _ => false,
        }),
        _ => message
            .to_string()
            .to_ascii_lowercase()
            .contains(&to_email.to_ascii_lowercase()),
    }
}

fn find_resend_email_body_text(detail_json: &serde_json::Value) -> Option<String> {
    let root_data = detail_json.get("data").unwrap_or(detail_json);

    let mut parts: Vec<&str> = Vec::new();
    for key in ["html", "Html", "text", "Text"] {
        if let Some(s) = root_data.get(key).and_then(|v| v.as_str()) {
            parts.push(s);
        }
    }
    if !parts.is_empty() {
        return Some(parts.join("\n"));
    }
    None
}

fn extract_sid_cookie(response: &reqwest::Response) -> String {
    let header = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let mut parts = header.split(';');
    let first = parts.next().unwrap_or("");
    if !first.starts_with("sid=") {
        panic!("missing sid cookie in response: {}", header);
    }
    first.to_string()
}
