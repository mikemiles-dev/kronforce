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
        // Escape each property individually to prevent command injection
        for prop in props.split_whitespace() {
            cmd.push(' ');
            cmd.push_str(&shell_escape(prop));
        }
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

// --- Consume / Subscribe tasks ---

#[allow(clippy::too_many_arguments)]
pub async fn run_kafka_consume_task(
    broker: &str,
    topic: &str,
    group_id: Option<&str>,
    max_messages: Option<u32>,
    offset: Option<&str>,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let n = max_messages.unwrap_or(1);
    let from = offset.unwrap_or("latest");
    let mut cmd = format!(
        "kafka-console-consumer --bootstrap-server {} --topic {} --max-messages {} --from-beginning",
        shell_escape(broker),
        shell_escape(topic),
        n
    );
    // Only add --from-beginning for "earliest"; for "latest" we omit it
    if from != "earliest" {
        cmd = format!(
            "kafka-console-consumer --bootstrap-server {} --topic {} --max-messages {}",
            shell_escape(broker),
            shell_escape(topic),
            n
        );
    }
    if let Some(gid) = group_id {
        cmd.push_str(&format!(" --group {}", shell_escape(gid)));
    }
    run_command(&cmd, run_as, timeout_secs.or(Some(30)), cancel_rx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_mqtt_subscribe_task(
    broker: &str,
    topic: &str,
    port: Option<u16>,
    max_messages: Option<u32>,
    username: Option<&str>,
    password: Option<&str>,
    client_id: Option<&str>,
    qos: Option<u8>,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let p = port.unwrap_or(1883);
    let n = max_messages.unwrap_or(1);
    let mut cmd = format!(
        "mosquitto_sub -h {} -p {} -t {} -C {}",
        shell_escape(broker),
        p,
        shell_escape(topic),
        n
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
    run_command(&cmd, run_as, timeout_secs.or(Some(30)), cancel_rx).await
}

pub async fn run_rabbitmq_consume_task(
    url: &str,
    queue: &str,
    max_messages: Option<u32>,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let n = max_messages.unwrap_or(1);
    let cmd = format!(
        "amqp-consume --url {} --queue {} -c {} cat",
        shell_escape(url),
        shell_escape(queue),
        n
    );
    run_command(&cmd, run_as, timeout_secs.or(Some(30)), cancel_rx).await
}

pub async fn run_redis_read_task(
    url: &str,
    key: &str,
    mode: Option<&str>,
    count: Option<u32>,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let m = mode.unwrap_or("lpop");
    let n = count.unwrap_or(1);
    let cmd = match m {
        "rpop" => format!(
            "redis-cli -u {} RPOP {} {}",
            shell_escape(url),
            shell_escape(key),
            n
        ),
        "subscribe" => format!(
            "redis-cli -u {} SUBSCRIBE {} &\nSUB_PID=$!; sleep {}; kill $SUB_PID 2>/dev/null",
            shell_escape(url),
            shell_escape(key),
            timeout_secs.unwrap_or(10)
        ),
        "xread" => format!(
            "redis-cli -u {} XREAD COUNT {} STREAMS {} 0",
            shell_escape(url),
            n,
            shell_escape(key)
        ),
        _ => format!(
            "redis-cli -u {} LPOP {} {}",
            shell_escape(url),
            shell_escape(key),
            n
        ),
    };
    run_command(&cmd, run_as, timeout_secs.or(Some(30)), cancel_rx).await
}
