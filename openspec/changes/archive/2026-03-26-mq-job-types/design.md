## Context

The `TaskType` enum currently has 8 variants (Shell, Sql, Ftp, Http, Script, Custom, FilePush). `run_task` in `executor/local.rs` handles each variant. Shell-based task types (SQL, FTP) already use the pattern of constructing a CLI command and running it via `run_command`. The HTTP task type uses in-process `reqwest`.

## Goals / Non-Goals

**Goals:**
- 4 new MQ publish task types: Kafka, RabbitMQ, MQTT, Redis
- Each shells out to a CLI tool for maximum compatibility and zero native dependencies
- Proper form fields in the job modal for each type
- Confirmation/response captured as stdout
- Works on controller and standard agents (CLI tools must be installed)

**Non-Goals:**
- Message consumption / subscribing
- Native Rust MQ client libraries (too many dependencies, complex connection management)
- Message templating with variables from previous executions
- Connection pooling or persistent connections

## Decisions

### 1. Shell out to CLI tools

**Decision**: All 4 MQ types construct a shell command and run it via the existing `run_command` function, just like SQL and FTP task types.

| Type | CLI Tool | Command Pattern |
|---|---|---|
| Kafka | `echo MSG \| kafka-console-producer --broker-list HOST --topic TOPIC` | Pipes message via stdin |
| RabbitMQ | `rabbitmqadmin publish exchange=EX routing_key=KEY payload="MSG"` | Or `amqp-publish` |
| MQTT | `mosquitto_pub -h HOST -p PORT -t TOPIC -m "MSG"` | Direct publish |
| Redis | `redis-cli -u URL PUBLISH CHANNEL "MSG"` | Pub/sub publish |

**Rationale**: No new Rust dependencies. CLI tools are well-tested, widely available, and handle connection negotiation, TLS, and authentication. The existing `run_command` infrastructure handles timeouts, output capture, and cancellation.

### 2. Message body supports multiline and JSON

**Decision**: The message body field is a `textarea` in the form. The body is passed to CLI tools via stdin pipe (Kafka) or as a command argument with proper escaping (MQTT, Redis, RabbitMQ). For tools that don't handle multiline well as arguments, use stdin or temp file.

**Rationale**: JSON payloads are the most common MQ message format. Textarea + stdin piping handles them naturally.

### 3. Each MQ type is a separate TaskType variant

**Decision**: Add 4 variants to the `TaskType` enum:

```rust
Kafka {
    broker: String,
    topic: String,
    message: String,
    key: Option<String>,
    properties: Option<String>, // additional kafka-console-producer flags
}

Rabbitmq {
    url: String,         // amqp://user:pass@host:5672/vhost
    exchange: String,
    routing_key: String,
    message: String,
    content_type: Option<String>,
}

Mqtt {
    broker: String,      // tcp://host:1883 or ssl://host:8883
    topic: String,
    message: String,
    qos: Option<u8>,     // 0, 1, or 2
    username: Option<String>,
    password: Option<String>,
    client_id: Option<String>,
}

Redis {
    url: String,         // redis://host:6379
    channel: String,
    message: String,
}
```

**Rationale**: Separate variants give strong typing and clear form fields per MQ type. Matches the pattern of existing task types.

### 4. Form uses the existing tab structure

**Decision**: Add Kafka, RabbitMQ, MQTT, and Redis as radio buttons in the Task tab. Each has its own `task-*-fields` div that shows/hides based on selection, matching the existing Shell/HTTP/SQL/FTP/Script pattern.

Group them visually: the radio group gets two rows — built-in types on the first row, MQ types on the second with a subtle separator or label.

**Rationale**: Consistent with existing UI. No new components needed.

### 5. Docker image includes MQ CLI tools

**Decision**: Add MQ CLI tools to the Dockerfile:
- `kafka` tools (via confluent-kafka package or download)
- `rabbitmq-server` (for `rabbitmqadmin`)
- `mosquitto-clients` (for `mosquitto_pub`)
- `redis-tools` (for `redis-cli`)

These are small packages. For the slim runtime image, only install the client tools.

**Rationale**: Docker users get MQ support out of the box. Binary users install the tools they need.

## Risks / Trade-offs

- **CLI tools must be installed** — users deploying from binary need to install the appropriate MQ CLI tools. Docker image includes them. Clear error message if tool is not found.
- **Shell escaping for messages** — complex JSON messages with quotes need proper escaping. Using stdin piping (Kafka) or the existing `shell_escape` helper mitigates this.
- **No TLS configuration UI** — Kafka and MQTT TLS is configured via CLI tool flags in the "properties" / "additional flags" field. Acceptable for power users.
- **`rabbitmqadmin` requires management plugin** — alternative is `amqp-publish` from `amqp-tools` package. We'll try `amqp-publish` first, fall back to `rabbitmqadmin`.
