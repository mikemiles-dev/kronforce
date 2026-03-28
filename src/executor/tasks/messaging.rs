use tokio::sync::oneshot;

use super::super::{CommandResult, run_command, shell_escape};

#[allow(clippy::too_many_arguments)]
pub async fn run_kafka_task(
    broker: &str,
    topic: &str,
    message: &str,
    key: Option<&str>,
    properties: Option<&str>,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let mut cmd = format!(
        "echo {} | kafka-console-producer --broker-list {} --topic {}",
        shell_escape(message),
        shell_escape(broker),
        shell_escape(topic)
    );
    if let Some(k) = key {
        cmd = format!(
            "echo {}:{} | kafka-console-producer --broker-list {} --topic {} --property parse.key=true --property key.separator=:",
            shell_escape(k),
            shell_escape(message),
            shell_escape(broker),
            shell_escape(topic)
        );
    }
    if let Some(props) = properties {
        cmd.push(' ');
        cmd.push_str(props);
    }
    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_rabbitmq_task(
    url: &str,
    exchange: &str,
    routing_key: &str,
    message: &str,
    content_type: Option<&str>,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let mut cmd = format!(
        "amqp-publish --url {} --exchange {} --routing-key {} --body {}",
        shell_escape(url),
        shell_escape(exchange),
        shell_escape(routing_key),
        shell_escape(message)
    );
    if let Some(ct) = content_type {
        cmd.push_str(&format!(" --content-type {}", shell_escape(ct)));
    }
    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_mqtt_task(
    broker: &str,
    topic: &str,
    message: &str,
    port: Option<u16>,
    qos: Option<u8>,
    username: Option<&str>,
    password: Option<&str>,
    client_id: Option<&str>,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let p = port.unwrap_or(1883);
    let mut cmd = format!(
        "mosquitto_pub -h {} -p {} -t {} -m {}",
        shell_escape(broker),
        p,
        shell_escape(topic),
        shell_escape(message)
    );
    if let Some(q) = qos {
        cmd.push_str(&format!(" -q {}", q));
    }
    if let Some(u) = username {
        cmd.push_str(&format!(" -u {}", shell_escape(u)));
    }
    if let Some(pw) = password {
        cmd.push_str(&format!(" -P {}", shell_escape(pw)));
    }
    if let Some(cid) = client_id {
        cmd.push_str(&format!(" -i {}", shell_escape(cid)));
    }
    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
}

pub async fn run_redis_task(
    url: &str,
    channel: &str,
    message: &str,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let cmd = format!(
        "redis-cli -u {} PUBLISH {} {}",
        shell_escape(url),
        shell_escape(channel),
        shell_escape(message)
    );
    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
}
