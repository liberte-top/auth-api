use sea_orm::{Database, DatabaseConnection, DbErr};
use std::env;
use tokio::net::lookup_host;

fn redact_db_url(url: &str) -> String {
    let mut result = String::with_capacity(url.len());
    let mut chars = url.chars().peekable();
    let mut in_authority = false;
    let mut redacting = false;

    while let Some(ch) = chars.next() {
        if !in_authority {
            result.push(ch);
            if ch == '/' && chars.peek() == Some(&'/') {
                // keep the second slash
                if let Some(next) = chars.next() {
                    result.push(next);
                }
                in_authority = true;
            }
            continue;
        }

        if redacting {
            if ch == '@' {
                redacting = false;
                result.push(ch);
            }
            continue;
        }

        if ch == ':' {
            if let Some(next) = chars.peek() {
                if *next != '/' {
                    result.push(ch);
                    result.push_str("***");
                    // consume until '@' handled by redacting state
                    redacting = true;
                    continue;
                }
            }
        }

        result.push(ch);

        if ch == '/' {
            // end of authority section
            break;
        }
    }

    for ch in chars {
        result.push(ch);
    }

    result
}

fn extract_host_port(url: &str) -> Option<(String, u16)> {
    let after_scheme = url.split("://").nth(1)?;
    let authority = after_scheme.split('/').next().unwrap_or(after_scheme);
    let hostport = authority.split('@').last().unwrap_or(authority);
    let mut parts = hostport.split(':');
    let host = parts.next()?.to_string();
    let port = parts
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(5432);
    Some((host, port))
}

pub async fn connect() -> Result<DatabaseConnection, DbErr> {
    let url = env::var("DATABASE_URL")
        .map_err(|_| DbErr::Custom("DATABASE_URL is not set".to_string()))?;
    let redacted = redact_db_url(&url);
    eprintln!("db: using DATABASE_URL={}", redacted);

    if let Some((host, port)) = extract_host_port(&url) {
        match lookup_host((host.as_str(), port)).await {
            Ok(addrs) => {
                let list: Vec<String> = addrs.map(|addr| addr.to_string()).collect();
                eprintln!("db: lookup_host {}:{} => {:?}", host, port, list);
            }
            Err(err) => {
                eprintln!("db: lookup_host {}:{} failed: {}", host, port, err);
            }
        }
    }
    Database::connect(url).await
}
