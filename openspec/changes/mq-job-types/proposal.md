## Why

Kronforce has task types for shell, HTTP, SQL, FTP, and file push, but no native support for message queues. Publishing messages to Kafka, RabbitMQ, MQTT, and Redis is a common automation need — triggering downstream systems, sending notifications to event buses, pushing metrics, or orchestrating microservices. Currently users must shell out to CLI tools (`kafka-console-producer`, `mosquitto_pub`) or write Rhai scripts, which is clunky and error-prone. Native MQ task types with proper form fields make this a first-class capability.

## What Changes

- **Kafka task type**: Publish a message to a Kafka topic. Fields: broker URL, topic, message body (supports templating), optional message key, optional headers. Uses `rdkafka` or shells out to `kafka-console-producer`.
- **RabbitMQ task type**: Publish a message to a RabbitMQ exchange. Fields: AMQP URL, exchange, routing key, message body, optional properties (content type, headers). Uses `lapin` or shells out to `rabbitmqadmin`.
- **MQTT task type**: Publish a message to an MQTT topic. Fields: broker URL (tcp/ssl), topic, message body, QoS level (0/1/2), optional client ID, optional username/password. Uses `rumqttc` or shells out to `mosquitto_pub`.
- **Redis Pub/Sub task type**: Publish a message to a Redis channel. Fields: Redis URL, channel, message body. Uses `redis` crate or shells out to `redis-cli`.
- **Job form**: New task type radio buttons for each MQ type with appropriate fields.
- **Output capture**: Each MQ publish captures confirmation/response as stdout (offset for Kafka, delivery confirmation for RabbitMQ, etc.).

## Capabilities

### New Capabilities
- `mq-publish-tasks`: Kafka, RabbitMQ, MQTT, and Redis publish task types with form fields and execution handlers

### Modified Capabilities

## Impact

- **Models**: 4 new variants in `TaskType` enum (Kafka, Rabbitmq, Mqtt, Redis)
- **Executor**: `run_task` handlers for each MQ type — shell out to CLI tools for simplicity and zero native dependencies
- **Frontend**: New task type radio buttons and form fields for each MQ type
- **Dependencies**: None — all MQ publishing shells out to CLI tools (`kafka-console-producer`, `rabbitmqadmin`/`amqp-publish`, `mosquitto_pub`, `redis-cli`) which must be installed on the controller/agent
- **Docker**: MQ CLI tools added to Dockerfile for out-of-box support
