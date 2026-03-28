use std::collections::HashMap;

use tokio::sync::oneshot;

use crate::db::models::{ExecutionStatus, HttpMethod};

use super::super::{CapturedOutput, CommandResult};

/// Validates a URL is not targeting internal/private networks (SSRF protection).
fn validate_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid URL: {e}"))?;
    let host = parsed.host_str().unwrap_or_default();

    // Block common internal addresses
    if host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host == "0.0.0.0"
        || host.ends_with(".local")
    {
        return Err(format!("URL targets a local address: {host}"));
    }
    // Block AWS/cloud metadata endpoints
    if host == "169.254.169.254" || host == "metadata.google.internal" {
        return Err(format!("URL targets a cloud metadata endpoint: {host}"));
    }
    // Block common private ranges
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        if ip.is_loopback() || ip.is_private() || ip.is_link_local() {
            return Err(format!("URL targets a private IP: {ip}"));
        }
    }
    Ok(())
}

pub async fn run_http_task(
    method: &HttpMethod,
    url: &str,
    headers: Option<&HashMap<String, String>>,
    body: Option<&str>,
    expect_status: Option<u16>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    // SSRF protection: block internal/private URLs
    if let Err(e) = validate_url(url) {
        return CommandResult {
            status: ExecutionStatus::Failed,
            exit_code: None,
            stdout: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
            stderr: CapturedOutput {
                text: e,
                truncated: false,
            },
        };
    }

    let client = reqwest::Client::builder()
        .timeout(
            timeout_secs
                .map(std::time::Duration::from_secs)
                .unwrap_or(std::time::Duration::from_secs(30)),
        )
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut req = match method {
        HttpMethod::Get => client.get(url),
        HttpMethod::Post => client.post(url),
        HttpMethod::Put => client.put(url),
        HttpMethod::Delete => client.delete(url),
    };

    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req = req.header(k.as_str(), v.as_str());
        }
    }

    if let Some(b) = body {
        req = req.body(b.to_string());
    }

    let http_future = async {
        match req.send().await {
            Ok(resp) => {
                let status_code = resp.status().as_u16();
                let resp_body = resp.text().await.unwrap_or_default();

                if let Some(expected) = expect_status
                    && status_code != expected
                {
                    return CommandResult {
                        status: ExecutionStatus::Failed,
                        exit_code: Some(status_code as i32),
                        stdout: CapturedOutput {
                            text: resp_body,
                            truncated: false,
                        },
                        stderr: CapturedOutput {
                            text: format!("expected status {}, got {}", expected, status_code),
                            truncated: false,
                        },
                    };
                }

                CommandResult {
                    status: if (200..300).contains(&status_code) {
                        ExecutionStatus::Succeeded
                    } else {
                        ExecutionStatus::Failed
                    },
                    exit_code: Some(status_code as i32),
                    stdout: CapturedOutput {
                        text: resp_body,
                        truncated: false,
                    },
                    stderr: CapturedOutput {
                        text: String::new(),
                        truncated: false,
                    },
                }
            }
            Err(e) => CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: format!("HTTP request failed: {e}"),
                    truncated: false,
                },
            },
        }
    };

    tokio::select! {
        result = http_future => result,
        _ = cancel_rx => {
            CommandResult {
                status: ExecutionStatus::Cancelled,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: "job cancelled by user".to_string(), truncated: false },
            }
        }
    }
}
