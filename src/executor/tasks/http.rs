use std::collections::HashMap;

use tokio::sync::oneshot;

use crate::db::models::{ExecutionStatus, HttpMethod};

use super::super::{CapturedOutput, CommandResult};

pub async fn run_http_task(
    method: &HttpMethod,
    url: &str,
    headers: Option<&HashMap<String, String>>,
    body: Option<&str>,
    expect_status: Option<u16>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let client = reqwest::Client::builder()
        .timeout(
            timeout_secs
                .map(std::time::Duration::from_secs)
                .unwrap_or(std::time::Duration::from_secs(30)),
        )
        .build()
        .unwrap();

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
