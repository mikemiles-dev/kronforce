## ADDED Requirements

### Requirement: Kafka publish task type
The `TaskType` enum SHALL include a `Kafka` variant that publishes a message to a Kafka topic by shelling out to `kafka-console-producer`.

#### Scenario: Publishing a message to Kafka
- **WHEN** a job with Kafka task type runs with broker `localhost:9092`, topic `events`, and message `{"event":"test"}`
- **THEN** the system runs `echo '{"event":"test"}' | kafka-console-producer --broker-list localhost:9092 --topic events` and captures the output

#### Scenario: Publishing with a message key
- **WHEN** a Kafka task includes a key `user-123`
- **THEN** the command includes `--property parse.key=true --property key.separator=:` and prepends the key to the message

#### Scenario: Kafka CLI tool not installed
- **WHEN** `kafka-console-producer` is not found on the system
- **THEN** the execution fails with a clear error message indicating the tool is not installed

### Requirement: RabbitMQ publish task type
The `TaskType` enum SHALL include a `Rabbitmq` variant that publishes a message to a RabbitMQ exchange by shelling out to `amqp-publish`.

#### Scenario: Publishing a message to RabbitMQ
- **WHEN** a job with RabbitMQ task type runs with URL `amqp://localhost:5672`, exchange `events`, routing key `user.created`, and message `{"user":"alice"}`
- **THEN** the system runs `amqp-publish` with the appropriate flags and captures the output

#### Scenario: Publishing with content type
- **WHEN** a RabbitMQ task includes content type `application/json`
- **THEN** the command includes the content type flag

### Requirement: MQTT publish task type
The `TaskType` enum SHALL include an `Mqtt` variant that publishes a message to an MQTT topic by shelling out to `mosquitto_pub`.

#### Scenario: Publishing a message to MQTT
- **WHEN** a job with MQTT task type runs with broker `localhost`, port `1883`, topic `sensors/temp`, and message `22.5`
- **THEN** the system runs `mosquitto_pub -h localhost -p 1883 -t sensors/temp -m '22.5'`

#### Scenario: Publishing with authentication
- **WHEN** an MQTT task includes username `device1` and password `secret`
- **THEN** the command includes `-u device1 -P secret`

#### Scenario: Publishing with QoS
- **WHEN** an MQTT task includes QoS level 2
- **THEN** the command includes `-q 2`

### Requirement: Redis publish task type
The `TaskType` enum SHALL include a `Redis` variant that publishes a message to a Redis Pub/Sub channel by shelling out to `redis-cli`.

#### Scenario: Publishing a message to Redis
- **WHEN** a job with Redis task type runs with URL `redis://localhost:6379`, channel `notifications`, and message `{"type":"alert"}`
- **THEN** the system runs `redis-cli -u redis://localhost:6379 PUBLISH notifications '{"type":"alert"}'`

### Requirement: MQ task types have form fields in job modal
The job creation modal SHALL include radio buttons for Kafka, RabbitMQ, MQTT, and Redis task types with appropriate form fields for each.

#### Scenario: Selecting Kafka task type
- **WHEN** the user selects "Kafka" from the task type options
- **THEN** the form shows fields for broker URL, topic, message body (textarea), optional key, and optional additional properties

#### Scenario: Selecting MQTT task type
- **WHEN** the user selects "MQTT" from the task type options
- **THEN** the form shows fields for broker host, port, topic, message body, QoS dropdown, and optional username/password

#### Scenario: MQ types grouped visually
- **WHEN** the task type radio buttons are displayed
- **THEN** MQ types appear in a distinct group labeled "Message Queues" separate from the built-in types

### Requirement: MQ task badges and detail display
Each MQ task type SHALL have a distinct badge color and display the broker/topic info in the job detail.

#### Scenario: Kafka badge
- **WHEN** a job has a Kafka task type
- **THEN** it displays a "kafka" badge and shows broker + topic in the detail

#### Scenario: MQTT badge
- **WHEN** a job has an MQTT task type
- **THEN** it displays an "mqtt" badge and shows broker + topic in the detail
