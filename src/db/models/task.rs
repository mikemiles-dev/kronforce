use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A selectable option for a task field (e.g., dropdown value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

/// Schema definition for a single field within a task type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFieldDefinition {
    pub name: String,
    pub label: String,
    pub field_type: String,
    #[serde(default)]
    pub required: Option<bool>,
    pub placeholder: Option<String>,
    pub options: Option<Vec<FieldOption>>,
}

/// Describes a custom task type with its name, description, and required fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTypeDefinition {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<TaskFieldDefinition>,
}

/// The work a job performs. Each variant represents a different execution backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskType {
    /// Execute a shell command on the target host.
    Shell {
        command: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_dir: Option<String>,
    },
    /// Run a SQL query against a database.
    Sql {
        driver: SqlDriver,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection_string: Option<String>,
        query: String,
        /// Named connection to use for credentials.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Transfer a file via FTP, FTPS, or SFTP.
    Ftp {
        protocol: FtpProtocol,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        host: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        port: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        username: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        password: Option<String>,
        direction: TransferDirection,
        remote_path: String,
        local_path: String,
        /// Named connection to use for credentials.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Make an HTTP request.
    Http {
        method: HttpMethod,
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        expect_status: Option<u16>,
        /// Named connection to use for auth.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Run a stored Rhai script by name.
    Script { script_name: String },
    /// Build a Docker image from a stored Dockerfile script and optionally run it.
    DockerBuild {
        script_name: String,
        image_tag: Option<String>,
        /// If true, run the built image after building.
        #[serde(default)]
        run_after_build: bool,
        /// Extra docker build args (e.g., "--build-arg FOO=bar").
        build_args: Option<String>,
    },
    /// Agent-defined custom task type with arbitrary JSON data.
    Custom {
        agent_task_type: String,
        data: serde_json::Value,
    },
    /// Push a file (base64-encoded) to the target host.
    FilePush {
        filename: String,
        destination: String,
        content_base64: String,
        permissions: Option<String>,
        #[serde(default)]
        overwrite: bool,
    },
    /// Publish a message to a Kafka topic.
    Kafka {
        broker: String,
        topic: String,
        message: String,
        key: Option<String>,
        properties: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Publish a message to a RabbitMQ exchange.
    Rabbitmq {
        url: String,
        exchange: String,
        routing_key: String,
        message: String,
        content_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Publish a message to an MQTT topic.
    Mqtt {
        broker: String,
        topic: String,
        message: String,
        port: Option<u16>,
        qos: Option<u8>,
        username: Option<String>,
        password: Option<String>,
        client_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Publish a message to a Redis channel.
    Redis {
        url: String,
        channel: String,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Call a tool on an MCP (Model Context Protocol) server via HTTP.
    Mcp {
        /// MCP server URL (e.g., http://localhost:8000/mcp)
        server_url: String,
        tool: String,
        arguments: Option<serde_json::Value>,
    },
    /// Consume messages from a Kafka topic.
    KafkaConsume {
        broker: String,
        topic: String,
        group_id: Option<String>,
        /// Max number of messages to consume (default 1)
        max_messages: Option<u32>,
        /// Start from: "earliest" or "latest" (default "latest")
        offset: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Subscribe to an MQTT topic and receive messages.
    MqttSubscribe {
        broker: String,
        topic: String,
        port: Option<u16>,
        /// Max number of messages to receive (default 1)
        max_messages: Option<u32>,
        username: Option<String>,
        password: Option<String>,
        client_id: Option<String>,
        qos: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Consume messages from a RabbitMQ queue.
    RabbitmqConsume {
        url: String,
        queue: String,
        /// Max number of messages (default 1)
        max_messages: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// Read from a Redis list, stream, or subscribe to a channel.
    RedisRead {
        url: String,
        /// Key or channel name
        key: String,
        /// "lpop", "rpop", "subscribe", "xread" (default "lpop")
        mode: Option<String>,
        /// Max number of items (default 1)
        count: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
}

/// Supported SQL database drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqlDriver {
    Postgres,
    Mysql,
    Sqlite,
}

/// File transfer protocol variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FtpProtocol {
    Ftp,
    Ftps,
    Sftp,
}

/// Direction of a file transfer operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Upload,
    Download,
}

/// HTTP method for HTTP task requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}
