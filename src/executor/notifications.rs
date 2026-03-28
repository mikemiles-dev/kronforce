use crate::db::Db;
use crate::models::{EventSeverity, ExecutionStatus, JobNotificationConfig};
use serde::{Deserialize, Serialize};

/// SMTP email delivery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    #[serde(default = "default_true")]
    pub tls: bool,
}

/// SMS delivery configuration via an HTTP webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    pub enabled: bool,
    pub webhook_url: String,
    pub auth_user: Option<String>,
    pub auth_pass: Option<String>,
    pub from_number: Option<String>,
}

/// Email addresses and phone numbers that receive notifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationRecipients {
    #[serde(default)]
    pub emails: Vec<String>,
    #[serde(default)]
    pub phones: Vec<String>,
}

/// Toggles for system-level alert notifications (e.g., agent going offline).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemAlerts {
    #[serde(default)]
    pub agent_offline: bool,
}

fn default_true() -> bool {
    true
}

/// Loads the email configuration from the database, returning `None` if disabled.
pub fn load_email_config(db: &Db) -> Option<EmailConfig> {
    db.get_setting("notification_email")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .filter(|c: &EmailConfig| c.enabled)
}

/// Loads the SMS configuration from the database, returning `None` if disabled.
pub fn load_sms_config(db: &Db) -> Option<SmsConfig> {
    db.get_setting("notification_sms")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .filter(|c: &SmsConfig| c.enabled)
}

/// Loads the global notification recipients from the database.
pub fn load_recipients(db: &Db) -> NotificationRecipients {
    db.get_setting("notification_recipients")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Loads the system alert toggle settings from the database.
pub fn load_system_alerts(db: &Db) -> SystemAlerts {
    db.get_setting("notification_system_alerts")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Sends a notification via all enabled channels (email, SMS) to the given or global recipients.
pub async fn send_notification(
    db: &Db,
    subject: &str,
    body: &str,
    recipient_override: Option<&NotificationRecipients>,
) {
    let recipients = match recipient_override {
        Some(r) if !r.emails.is_empty() || !r.phones.is_empty() => r.clone(),
        _ => load_recipients(db),
    };

    if let Some(email_config) = load_email_config(db)
        && !recipients.emails.is_empty()
    {
        let to = recipients.emails.clone();
        let subj = subject.to_string();
        let bod = bod_clone(body);
        let db_clone = db.clone();
        tokio::spawn(async move {
            match send_email(&email_config, &to, &subj, &bod).await {
                Ok(_) => {
                    let _ = db_clone.log_event(
                        "notification.sent",
                        EventSeverity::Info,
                        &format!("Email sent to {} recipient(s): {}", to.len(), subj),
                        None,
                        None,
                    );
                }
                Err(e) => {
                    let _ = db_clone.log_event(
                        "notification.failed",
                        EventSeverity::Error,
                        &format!("Email failed: {} — {}", subj, e),
                        None,
                        None,
                    );
                }
            }
        });
    }

    if let Some(sms_config) = load_sms_config(db)
        && !recipients.phones.is_empty()
    {
        let to = recipients.phones.clone();
        let bod = bod_clone(body);
        let subj = subject.to_string();
        let db_clone = db.clone();
        tokio::spawn(async move {
            match send_sms(&sms_config, &to, &bod).await {
                Ok(_) => {
                    let _ = db_clone.log_event(
                        "notification.sent",
                        EventSeverity::Info,
                        &format!("SMS sent to {} recipient(s): {}", to.len(), subj),
                        None,
                        None,
                    );
                }
                Err(e) => {
                    let _ = db_clone.log_event(
                        "notification.failed",
                        EventSeverity::Error,
                        &format!("SMS failed: {} — {}", subj, e),
                        None,
                        None,
                    );
                }
            }
        });
    }
}

fn bod_clone(s: &str) -> String {
    s.to_string()
}

/// Check if notification should be sent for a completed execution, and send it if so.
pub async fn notify_execution_complete(
    db: &Db,
    notif: &JobNotificationConfig,
    job_name: &str,
    exec_id_short: &str,
    exec_status: ExecutionStatus,
    stderr_excerpt: &str,
) {
    let should_notify = match exec_status {
        ExecutionStatus::Failed | ExecutionStatus::TimedOut => {
            notif.on_failure || notif.on_assertion_failure
        }
        ExecutionStatus::Succeeded => notif.on_success,
        _ => false,
    };
    if !should_notify {
        return;
    }
    let subject = format!(
        "[Kronforce] Job '{}' {}",
        job_name,
        match exec_status {
            ExecutionStatus::Succeeded => "succeeded",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::TimedOut => "timed out",
            _ => "completed",
        }
    );
    let body = format!(
        "Job: {}\nStatus: {:?}\nExecution: {}\nTime: {}\n{}",
        job_name,
        exec_status,
        exec_id_short,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        if !stderr_excerpt.is_empty() {
            format!("\nError output:\n{}", stderr_excerpt)
        } else {
            String::new()
        }
    );
    let recipients = notif.recipients.as_ref().map(|r| NotificationRecipients {
        emails: r.emails.clone(),
        phones: r.phones.clone(),
    });
    send_notification(db, &subject, &body, recipients.as_ref()).await;
}

/// Sends an email to one or more recipients via SMTP.
pub async fn send_email(
    config: &EmailConfig,
    to: &[String],
    subject: &str,
    body: &str,
) -> Result<(), String> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    let creds = Credentials::new(config.username.clone(), config.password.clone());

    let mailer = if config.tls {
        SmtpTransport::starttls_relay(&config.smtp_host)
            .map_err(|e| format!("SMTP relay error: {e}"))?
            .port(config.smtp_port)
            .credentials(creds)
            .build()
    } else {
        SmtpTransport::builder_dangerous(&config.smtp_host)
            .port(config.smtp_port)
            .credentials(creds)
            .build()
    };

    for recipient in to {
        let email = Message::builder()
            .from(
                config
                    .from
                    .parse()
                    .map_err(|e| format!("bad from address: {e}"))?,
            )
            .to(recipient
                .parse()
                .map_err(|e| format!("bad to address '{}': {e}", recipient))?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| format!("email build error: {e}"))?;

        mailer
            .send(&email)
            .map_err(|e| format!("SMTP send error: {e}"))?;
    }

    Ok(())
}

/// Sends an SMS to one or more phone numbers via the configured webhook.
pub async fn send_sms(config: &SmsConfig, to: &[String], body: &str) -> Result<(), String> {
    let client = reqwest::Client::new();

    for phone in to {
        let mut req = client.post(&config.webhook_url).json(&serde_json::json!({
            "To": phone,
            "From": config.from_number.as_deref().unwrap_or(""),
            "Body": body,
        }));

        if let (Some(user), Some(pass)) = (&config.auth_user, &config.auth_pass) {
            req = req.basic_auth(user, Some(pass));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("SMS webhook error: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("SMS webhook returned {}: {}", status, text));
        }
    }

    Ok(())
}

/// Send a test notification to verify channel configuration
pub async fn send_test(db: &Db) -> Result<String, String> {
    let recipients = load_recipients(db);
    let mut results = Vec::new();

    if let Some(email_config) = load_email_config(db) {
        if let Some(first) = recipients.emails.first() {
            match send_email(
                &email_config,
                std::slice::from_ref(first),
                "[Kronforce] Test Notification",
                "This is a test notification from Kronforce.",
            )
            .await
            {
                Ok(_) => results.push(format!("Email sent to {}", first)),
                Err(e) => results.push(format!("Email failed: {}", e)),
            }
        } else {
            results.push("Email enabled but no recipients configured".to_string());
        }
    } else {
        results.push("Email channel not enabled".to_string());
    }

    if let Some(sms_config) = load_sms_config(db) {
        if let Some(first) = recipients.phones.first() {
            match send_sms(
                &sms_config,
                std::slice::from_ref(first),
                "[Kronforce] Test notification",
            )
            .await
            {
                Ok(_) => results.push(format!("SMS sent to {}", first)),
                Err(e) => results.push(format!("SMS failed: {}", e)),
            }
        } else {
            results.push("SMS enabled but no recipients configured".to_string());
        }
    } else {
        results.push("SMS channel not enabled".to_string());
    }

    Ok(results.join("; "))
}
