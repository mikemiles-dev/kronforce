## 1. Backend: TaskType Variants

- [x] 1.1 Add `Kafka { broker, topic, message, key, properties }` variant to `TaskType` enum
- [x] 1.2 Add `Rabbitmq { url, exchange, routing_key, message, content_type }` variant
- [x] 1.3 Add `Mqtt { broker, topic, message, qos, username, password, client_id }` variant
- [x] 1.4 Add `Redis { url, channel, message }` variant

## 2. Backend: run_task Handlers

- [x] 2.1 Add Kafka handler in `run_task`: build `echo MSG | kafka-console-producer --broker-list BROKER --topic TOPIC` command, handle optional key with `--property parse.key=true`
- [x] 2.2 Add RabbitMQ handler: build `amqp-publish --url URL --exchange EX --routing-key KEY --body MSG` command, handle optional content type
- [x] 2.3 Add MQTT handler: build `mosquitto_pub -h HOST -p PORT -t TOPIC -m MSG` command, handle optional auth (`-u`/`-P`), QoS (`-q`), client ID (`-i`)
- [x] 2.4 Add Redis handler: build `redis-cli -u URL PUBLISH CHANNEL MSG` command
- [x] 2.5 All handlers use existing `run_command` with proper `shell_escape` for message content

## 3. Frontend: Task Type Radio Buttons

- [x] 3.1 Add "Message Queues" label separator and Kafka, RabbitMQ, MQTT, Redis radio buttons to the task type group
- [x] 3.2 Add `task-kafka-fields` div: broker URL input, topic input, message textarea, optional key input, optional properties input
- [x] 3.3 Add `task-rabbitmq-fields` div: AMQP URL input, exchange input, routing key input, message textarea, optional content type input
- [x] 3.4 Add `task-mqtt-fields` div: broker host input, port input (default 1883), topic input, message textarea, QoS dropdown (0/1/2), optional username/password inputs, optional client ID
- [x] 3.5 Add `task-redis-fields` div: Redis URL input, channel input, message textarea
- [x] 3.6 Update `updateTaskFields()` to show/hide MQ field divs

## 4. Frontend: Build and Populate

- [x] 4.1 Update `buildTaskFromForm()` to handle kafka, rabbitmq, mqtt, redis task types
- [x] 4.2 Update `populateTaskForm()` to populate MQ fields when editing existing MQ jobs

## 5. Frontend: Display

- [x] 5.1 Update `fmtTaskBadge()` with badges for kafka, rabbitmq, mqtt, redis
- [x] 5.2 Update `fmtTaskDetail()` to show broker + topic/channel for each MQ type

## 6. Docker

- [x] 6.1 Add MQ CLI tools to Dockerfile: `mosquitto-clients`, `amqp-tools`, `redis-tools`

## 7. Python Example

- [x] 7.1 Update `examples/custom_agent.py` to handle MQ task types (construct and run the same CLI commands)
