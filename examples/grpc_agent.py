#!/usr/bin/env python3
"""
Kronforce Custom Agent for gRPC services.

A pull-based agent that calls gRPC services using grpcurl. Supports
both reflection-based and proto-file-based service discovery.

Requirements:
    pip install requests
    brew install grpcurl  (or: go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest)

Usage:
    KRONFORCE_AGENT_KEY=kf_your_agent_key python3 examples/grpc_agent.py

Then configure task types in the dashboard (Agents page > grpc-agent):

    Task type: "grpc_call"
    Fields:
        - address   (text, required)     — gRPC server address, e.g. localhost:50051
        - service   (text, required)     — Full service name, e.g. helloworld.Greeter
        - method    (text, required)     — Method name, e.g. SayHello
        - data      (textarea, optional) — JSON request body, e.g. {"name": "World"}
        - proto     (text, optional)     — Path to .proto file (omit to use reflection)
        - plaintext (select, optional)   — Use plaintext connection
            options: [{"value": "true", "label": "Yes"}, {"value": "false", "label": "No (TLS)"}]
        - metadata  (textarea, optional) — Request metadata as key: value lines

    Task type: "grpc_list"
    Fields:
        - address   (text, required)     — gRPC server address
        - service   (text, optional)     — Service to list methods for (omit for all services)
        - plaintext (select, optional)   — Use plaintext connection
            options: [{"value": "true", "label": "Yes"}, {"value": "false", "label": "No (TLS)"}]

Environment variables:
    KRONFORCE_URL        — Controller URL (default: http://localhost:8080)
    KRONFORCE_AGENT_KEY  — API key with agent role (required)
    AGENT_NAME           — Agent name (default: grpc-agent)
    AGENT_TAGS           — Comma-separated tags (default: grpc,python)
    POLL_INTERVAL        — Seconds between polls (default: 5)
"""

import datetime
import json
import os
import subprocess
import sys
import time

import requests

CONTROLLER_URL = os.environ.get("KRONFORCE_URL", "http://localhost:8080")
AGENT_KEY = os.environ.get("KRONFORCE_AGENT_KEY", "")
AGENT_NAME = os.environ.get("AGENT_NAME", "grpc-agent")
AGENT_TAGS = os.environ.get("AGENT_TAGS", "grpc,python").split(",")
POLL_INTERVAL = int(os.environ.get("POLL_INTERVAL", "5"))


def api_headers():
    headers = {"Content-Type": "application/json"}
    if AGENT_KEY:
        headers["Authorization"] = f"Bearer {AGENT_KEY}"
    return headers


def register():
    """Register with the controller as a custom agent."""
    resp = requests.post(
        f"{CONTROLLER_URL}/api/agents/register",
        headers=api_headers(),
        json={
            "name": AGENT_NAME,
            "tags": [t.strip() for t in AGENT_TAGS],
            "hostname": os.uname().nodename if hasattr(os, "uname") else "unknown",
            "address": "0.0.0.0",
            "port": 0,
            "agent_type": "custom",
        },
    )
    resp.raise_for_status()
    data = resp.json()
    print(f"Registered as agent: {data['agent_id']}")
    return data["agent_id"]


def poll(agent_id):
    """Poll for work from the controller."""
    resp = requests.get(
        f"{CONTROLLER_URL}/api/agent-queue/{agent_id}/next",
        headers=api_headers(),
    )
    if resp.status_code == 204:
        return None
    resp.raise_for_status()
    return resp.json()


def report_result(callback_url, execution_id, job_id, agent_id, status, exit_code, stdout, stderr, started_at):
    """Report execution result back to the controller."""
    finished_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
    payload = {
        "execution_id": execution_id,
        "job_id": job_id,
        "agent_id": agent_id,
        "status": status,
        "exit_code": exit_code,
        "stdout": stdout[:256000],
        "stderr": stderr[:256000],
        "stdout_truncated": len(stdout) > 256000,
        "stderr_truncated": len(stderr) > 256000,
        "started_at": started_at,
        "finished_at": finished_at,
    }
    resp = requests.post(callback_url, headers=api_headers(), json=payload)
    resp.raise_for_status()


def build_grpcurl_cmd(data, method_path=None):
    """Build a grpcurl command from task data."""
    address = data.get("address", "")
    plaintext = data.get("plaintext", "true").lower() == "true"
    proto = data.get("proto", "")
    metadata = data.get("metadata", "")

    cmd = ["grpcurl"]

    if plaintext:
        cmd.append("-plaintext")

    if proto:
        cmd.extend(["-proto", proto])

    # Add metadata headers
    if metadata:
        for line in metadata.strip().splitlines():
            line = line.strip()
            if ":" in line:
                cmd.extend(["-H", line])

    # Add request data
    request_data = data.get("data", "")
    if request_data:
        if isinstance(request_data, dict):
            request_data = json.dumps(request_data)
        cmd.extend(["-d", request_data])

    cmd.append(address)

    if method_path:
        cmd.append(method_path)

    return cmd


def execute_grpc_call(data):
    """Execute a gRPC method call via grpcurl."""
    address = data.get("address", "")
    service = data.get("service", "")
    method = data.get("method", "")

    if not address:
        return "failed", 1, "", "address is required"
    if not service or not method:
        return "failed", 1, "", "service and method are required"

    method_path = f"{service}/{method}"
    cmd = build_grpcurl_cmd(data, method_path)

    print(f"  Calling {method_path} on {address}")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
        status = "succeeded" if result.returncode == 0 else "failed"
        return status, result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return "timed_out", -1, "", "gRPC call timed out after 300s"
    except FileNotFoundError:
        return "failed", -1, "", "grpcurl not found. Install: brew install grpcurl"
    except Exception as e:
        return "failed", -1, "", str(e)


def execute_grpc_list(data):
    """List gRPC services or methods via grpcurl reflection."""
    address = data.get("address", "")
    service = data.get("service", "")

    if not address:
        return "failed", 1, "", "address is required"

    cmd = build_grpcurl_cmd(data, service if service else "list")
    if not service:
        # List all services
        cmd_idx = cmd.index(address)
        cmd.insert(cmd_idx + 1, "list")

    print(f"  Listing {'methods for ' + service if service else 'services'} on {address}")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        status = "succeeded" if result.returncode == 0 else "failed"
        return status, result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return "timed_out", -1, "", "gRPC list timed out"
    except FileNotFoundError:
        return "failed", -1, "", "grpcurl not found. Install: brew install grpcurl"
    except Exception as e:
        return "failed", -1, "", str(e)


def execute_task(task):
    """Dispatch task by agent_task_type."""
    task_type = task.get("agent_task_type", "")
    data = task.get("data", {})

    if task_type == "grpc_call":
        return execute_grpc_call(data)
    elif task_type == "grpc_list":
        return execute_grpc_list(data)
    else:
        return "failed", -1, "", f"unsupported task type: {task_type}"


def main():
    if not AGENT_KEY:
        print("ERROR: KRONFORCE_AGENT_KEY is required.")
        print()
        print("Usage:")
        print("  KRONFORCE_AGENT_KEY=kf_your_agent_key python3 examples/grpc_agent.py")
        print()
        print("Get your agent key from the Kronforce dashboard: Settings > API Keys")
        sys.exit(1)

    # Check for grpcurl
    try:
        result = subprocess.run(["grpcurl", "--version"], capture_output=True, text=True)
        version = result.stderr.strip() or result.stdout.strip()
        print(f"grpcurl: {version}")
    except FileNotFoundError:
        print("WARNING: grpcurl not found. Install it before running gRPC tasks.")
        print("  macOS:   brew install grpcurl")
        print("  Linux:   go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest")
        print()

    print(f"Kronforce gRPC Agent")
    print(f"  Controller:  {CONTROLLER_URL}")
    print(f"  Agent Name:  {AGENT_NAME}")
    print(f"  Agent Key:   {AGENT_KEY[:11]}...")
    print()

    agent_id = register()

    print(f"Polling for work every {POLL_INTERVAL}s...")
    print("Configure task types in the dashboard: Agents > grpc-agent > Add task type")
    print()

    while True:
        try:
            job = poll(agent_id)
            if job is None:
                time.sleep(POLL_INTERVAL)
                continue

            exec_id = job["execution_id"]
            job_id = job.get("job_id", "")
            callback_url = job["callback_url"]
            task = job.get("task", {})
            started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()

            print(f"Received job {exec_id[:8]}...")

            status, exit_code, stdout, stderr = execute_task(task)

            print(f"  Result: {status}")
            if stdout:
                print(f"  Output: {stdout[:200]}")
            if stderr and status == "failed":
                print(f"  Error: {stderr[:200]}")

            report_result(callback_url, exec_id, job_id, agent_id, status, exit_code, stdout, stderr, started_at)

        except KeyboardInterrupt:
            print("\nShutting down...")
            sys.exit(0)
        except Exception as e:
            print(f"Error: {e}")
            time.sleep(POLL_INTERVAL)


if __name__ == "__main__":
    main()
