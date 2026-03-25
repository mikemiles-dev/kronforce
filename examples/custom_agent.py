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


def register():
    """Register this agent with the controller."""
    print(f"Registering with {CONTROLLER_URL} as '{AGENT_NAME}'...")
    resp = requests.post(f"{CONTROLLER_URL}/api/agents/register", json={
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


def poll_for_work(agent_id):
    """Poll the controller for a job to execute."""
    resp = requests.get(f"{CONTROLLER_URL}/api/agent-queue/{agent_id}/next")
    if resp.status_code == 204:
        return None  # No work available
    resp.raise_for_status()
    return resp.json()


def execute_task(task):
    """Execute a task and return (status, exit_code, stdout, stderr)."""
    task_type = task.get("type", "unknown")

    if task_type == "shell":
        # Run shell command
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

    else:
        # Unknown task type — you can add your own handlers here!
        print(f"  Unknown task type: {task_type}")
        return "failed", -1, "", f"unsupported task type: {task_type}"


def report_result(job, status, exit_code, stdout, stderr, started_at):
    """Report execution result back to the controller."""
    finished_at = datetime.datetime.utcnow().isoformat() + "Z"
    payload = {
        "execution_id": job["execution_id"],
        "job_id": job.get("job_id", ""),
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
        resp = requests.post(job["callback_url"], json=payload)
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
