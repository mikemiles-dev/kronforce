#!/usr/bin/env python3
"""
Kronforce Custom Agent Example (Python)

A minimal pull-based agent that registers with the controller,
polls for work, executes tasks, and reports results back.

Usage:
    python3 examples/custom_agent.py

Environment variables:
    KRONFORCE_URL    - Controller URL (default: http://localhost:8080)
    AGENT_NAME       - Agent name (default: python-agent)
    AGENT_TAGS       - Comma-separated tags (default: python,custom)
    POLL_INTERVAL    - Seconds between polls (default: 5)
"""

import os
import sys
import time
import json
import subprocess
import datetime
import requests

CONTROLLER_URL = os.environ.get("KRONFORCE_URL", "http://localhost:8080")
AGENT_NAME = os.environ.get("AGENT_NAME", "python-agent")
AGENT_TAGS = os.environ.get("AGENT_TAGS", "python,custom").split(",")
POLL_INTERVAL = int(os.environ.get("POLL_INTERVAL", "5"))
AGENT_KEY = os.environ.get("KRONFORCE_AGENT_KEY", "")

def auth_headers():
    """Return auth headers if agent key is configured."""
    if AGENT_KEY:
        return {"Authorization": f"Bearer {AGENT_KEY}"}
    return {}


def register():
    """Register this agent with the controller."""
    print(f"Registering with {CONTROLLER_URL} as '{AGENT_NAME}'...")
    # Task types are configured in the UI, not in registration
    resp = requests.post(f"{CONTROLLER_URL}/api/agents/register", headers=auth_headers(), json={
        "name": AGENT_NAME,
        "tags": [t.strip() for t in AGENT_TAGS],
        "hostname": os.uname().nodename,
        "address": "0.0.0.0",
        "port": 0,
        "agent_type": "custom"
    })
    resp.raise_for_status()
    data = resp.json()
    agent_id = data["agent_id"]
    print(f"Registered! Agent ID: {agent_id}")
    return agent_id


def discover_task_types(agent_id):
    """Fetch configured task types from the controller."""
    try:
        resp = requests.get(f"{CONTROLLER_URL}/api/agents/{agent_id}/task-types", headers=auth_headers())
        resp.raise_for_status()
        task_types = resp.json()
        if task_types:
            print(f"Configured task types: {', '.join(tt['name'] for tt in task_types)}")
        else:
            print("No task types configured yet (configure via the dashboard)")
        return task_types
    except Exception as e:
        print(f"Could not fetch task types: {e}")
        return []


def poll_for_work(agent_id):
    """Poll the controller for a job to execute."""
    resp = requests.get(f"{CONTROLLER_URL}/api/agent-queue/{agent_id}/next", headers=auth_headers())
    if resp.status_code == 204:
        return None  # No work available
    resp.raise_for_status()
    return resp.json()


def execute_task(task):
    """Execute a task and return (status, exit_code, stdout, stderr)."""
    task_type = task.get("type", "unknown")

    if task_type == "custom":
        # Custom task type — dispatch by agent_task_type
        agent_task_type = task.get("agent_task_type", "")
        data = task.get("data", {})
        print(f"  Custom task: {agent_task_type}")

        if agent_task_type == "python":
            script = data.get("script", "print('no script')")
            args = data.get("args", "")
            print(f"  Running Python script")
            try:
                cmd = ["python3", "-c", script]
                if args:
                    cmd.extend(args.split())
                result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
                status = "succeeded" if result.returncode == 0 else "failed"
                return status, result.returncode, result.stdout, result.stderr
            except subprocess.TimeoutExpired:
                return "timed_out", -1, "", "script timed out"
            except Exception as e:
                return "failed", -1, "", str(e)

        elif agent_task_type == "shell":
            command = data.get("command", "echo 'no command'")
            print(f"  Running shell: {command}")
            try:
                result = subprocess.run(
                    ["sh", "-c", command],
                    capture_output=True, text=True, timeout=300
                )
                status = "succeeded" if result.returncode == 0 else "failed"
                return status, result.returncode, result.stdout, result.stderr
            except subprocess.TimeoutExpired:
                return "timed_out", -1, "", "command timed out"
            except Exception as e:
                return "failed", -1, "", str(e)

        else:
            return "failed", -1, "", f"unsupported custom task type: {agent_task_type}"

    elif task_type == "shell":
        # Run shell command (legacy built-in type)
        command = task.get("command", "echo 'no command'")
        print(f"  Running shell: {command}")
        try:
            result = subprocess.run(
                ["sh", "-c", command],
                capture_output=True, text=True, timeout=300
            )
            status = "succeeded" if result.returncode == 0 else "failed"
            return status, result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return "timed_out", -1, "", "command timed out"
        except Exception as e:
            return "failed", -1, "", str(e)

    elif task_type == "http":
        # Make HTTP request
        method = task.get("method", "get").upper()
        url = task.get("url", "")
        headers = task.get("headers", {})
        body = task.get("body")
        expect_status = task.get("expect_status")

        print(f"  HTTP {method} {url}")
        try:
            resp = requests.request(method, url, headers=headers, data=body, timeout=30)
            stdout = resp.text
            if expect_status and resp.status_code != expect_status:
                return "failed", resp.status_code, stdout, f"expected {expect_status}, got {resp.status_code}"
            status = "succeeded" if 200 <= resp.status_code < 300 else "failed"
            return status, resp.status_code, stdout, ""
        except Exception as e:
            return "failed", -1, "", str(e)

    elif task_type == "file_push":
        # Write file to destination
        import base64
        destination = task.get("destination", "")
        filename = task.get("filename", "unknown")
        content_b64 = task.get("content_base64", "")
        overwrite = task.get("overwrite", True)
        print(f"  File push: {filename} -> {destination}")
        try:
            content = base64.b64decode(content_b64)
            dest = os.path.abspath(destination)
            if not overwrite and os.path.exists(dest):
                return "failed", 1, "", f"file already exists: {dest} (overwrite=false)"
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, "wb") as f:
                f.write(content)
            permissions = task.get("permissions")
            if permissions:
                os.chmod(dest, int(permissions, 8))
            return "succeeded", 0, f"File '{filename}' written to {dest} ({len(content)} bytes)", ""
        except Exception as e:
            return "failed", 1, "", str(e)

    elif task_type in ("kafka", "rabbitmq", "mqtt", "redis"):
        # MQ types — construct and run the CLI command
        try:
            if task_type == "kafka":
                cmd = f"echo {task.get('message', '')} | kafka-console-producer --broker-list {task.get('broker', '')} --topic {task.get('topic', '')}"
            elif task_type == "rabbitmq":
                cmd = f"amqp-publish --url {task.get('url', '')} --exchange {task.get('exchange', '')} --routing-key {task.get('routing_key', '')} --body {task.get('message', '')}"
            elif task_type == "mqtt":
                port = task.get("port", 1883)
                cmd = f"mosquitto_pub -h {task.get('broker', '')} -p {port} -t {task.get('topic', '')} -m '{task.get('message', '')}'"
                if task.get("username"):
                    cmd += f" -u {task['username']}"
                if task.get("password"):
                    cmd += f" -P {task['password']}"
                if task.get("qos") is not None:
                    cmd += f" -q {task['qos']}"
            elif task_type == "redis":
                cmd = f"redis-cli -u {task.get('url', '')} PUBLISH {task.get('channel', '')} '{task.get('message', '')}'"
            print(f"  MQ publish: {cmd[:80]}...")
            result = subprocess.run(["sh", "-c", cmd], capture_output=True, text=True, timeout=30)
            status = "succeeded" if result.returncode == 0 else "failed"
            return status, result.returncode, result.stdout, result.stderr
        except Exception as e:
            return "failed", -1, "", str(e)

    else:
        # Unknown task type
        print(f"  Unknown task type: {task_type}")
        return "failed", -1, "", f"unsupported task type: {task_type}"


def report_result(job, status, exit_code, stdout, stderr, started_at):
    """Report execution result back to the controller."""
    finished_at = datetime.datetime.utcnow().isoformat() + "Z"
    payload = {
        "execution_id": job["execution_id"],
        "job_id": job.get("job_id") or "",
        "agent_id": job["agent_id"],
        "status": status,
        "exit_code": exit_code,
        "stdout": stdout[:256000],  # Truncate to 256KB
        "stderr": stderr[:256000],
        "stdout_truncated": len(stdout) > 256000,
        "stderr_truncated": len(stderr) > 256000,
        "started_at": started_at,
        "finished_at": finished_at
    }
    try:
        resp = requests.post(job["callback_url"], headers=auth_headers(), json=payload)
        resp.raise_for_status()
        print(f"  Result reported: {status}")
    except Exception as e:
        print(f"  WARNING: Failed to report result: {e}")


def main():
    print("=" * 50)
    print("Kronforce Custom Agent (Python)")
    print("=" * 50)

    # Register
    try:
        agent_id = register()
    except Exception as e:
        print(f"ERROR: Failed to register: {e}")
        sys.exit(1)

    # Discover configured task types
    discover_task_types(agent_id)

    # Poll loop
    print(f"Polling for work every {POLL_INTERVAL}s...")
    while True:
        try:
            job = poll_for_work(agent_id)
            if job is None:
                time.sleep(POLL_INTERVAL)
                continue

            print(f"Got job: execution={job['execution_id'][:8]}")
            started_at = datetime.datetime.utcnow().isoformat() + "Z"

            # Execute
            task = job.get("task", {})
            status, exit_code, stdout, stderr = execute_task(task)

            # Report
            report_result(job, status, exit_code, stdout, stderr, started_at)

        except KeyboardInterrupt:
            print("\nShutting down...")
            break
        except Exception as e:
            print(f"Error: {e}")
            time.sleep(POLL_INTERVAL)


if __name__ == "__main__":
    main()
