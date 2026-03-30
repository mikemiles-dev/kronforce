#!/usr/bin/env python3
"""
Kronforce Custom Agent that bridges to MCP servers.

This agent registers with the Kronforce controller as a custom agent,
polls for work, and executes MCP tool calls on behalf of the controller.
Use this when you want agents (not the controller) to run MCP tasks.

Setup:
    pip install requests mcp

Run:
    KRONFORCE_URL=http://localhost:8080 \
    KRONFORCE_AGENT_KEY=kf_your_agent_key \
    MCP_SERVER_URL=http://localhost:8000/mcp \
    python3 examples/mcp_agent.py

Then in the Kronforce dashboard:
    1. Go to Agents page, click on this agent's card
    2. Add a task type named "mcp_tool" with fields:
       - tool (text, required) — MCP tool name
       - arguments (textarea, optional) — JSON arguments
    3. Create a job with Custom Agent execution mode
    4. Select this agent, pick "mcp_tool", fill in tool name and arguments
    5. Trigger the job — the agent calls the MCP tool and reports the result
"""

import json
import os
import sys
import time

import requests

# --- Configuration ---

CONTROLLER_URL = os.environ.get("KRONFORCE_URL", "http://localhost:8080")
AGENT_KEY = os.environ.get("KRONFORCE_AGENT_KEY", "")
AGENT_NAME = os.environ.get("AGENT_NAME", "mcp-agent")
AGENT_TAGS = os.environ.get("AGENT_TAGS", "mcp,python").split(",")
MCP_SERVER_URL = os.environ.get("MCP_SERVER_URL", "http://localhost:8000/mcp")
POLL_INTERVAL = int(os.environ.get("POLL_INTERVAL", "2"))


def api_headers():
    headers = {"Content-Type": "application/json"}
    if AGENT_KEY:
        headers["Authorization"] = f"Bearer {AGENT_KEY}"
    return headers


# --- MCP Client (HTTP) ---

class McpClient:
    def __init__(self, server_url):
        self.url = server_url
        self.session = requests.Session()
        self.session.headers["Content-Type"] = "application/json"

    def _send(self, msg):
        resp = self.session.post(self.url, json=msg)
        resp.raise_for_status()
        data = resp.json()
        # Handle single response or find response with matching id
        if isinstance(data, dict):
            return data
        if isinstance(data, list):
            for item in data:
                if "id" in item:
                    return item
        return data

    def _notify(self, msg):
        self.session.post(self.url, json=msg)

    def handshake(self):
        init = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "kronforce-mcp-agent", "version": "0.1.0"},
            },
        }
        resp = self._send(init)
        if "error" in resp and resp["error"]:
            raise Exception(f"Handshake error: {resp['error']['message']}")

        self._notify({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        })

    def call_tool(self, tool_name, arguments=None):
        msg = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments or {},
            },
        }
        resp = self._send(msg)
        if "error" in resp and resp["error"]:
            raise Exception(f"Tool error: {resp['error']['message']}")
        result = resp.get("result", {})
        return result


# --- Agent Logic ---

def register():
    """Register with the controller as a custom agent."""
    resp = requests.post(
        f"{CONTROLLER_URL}/api/agents/register",
        headers=api_headers(),
        json={
            "name": AGENT_NAME,
            "tags": AGENT_TAGS,
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


def report_result(callback_url, execution_id, job_id, agent_id, status, stdout, stderr=""):
    """Report execution result back to the controller."""
    from datetime import datetime, timezone

    now = datetime.now(timezone.utc).isoformat()
    payload = {
        "execution_id": execution_id,
        "job_id": job_id,
        "agent_id": agent_id,
        "status": status,
        "exit_code": 0 if status == "succeeded" else 1,
        "stdout": stdout,
        "stderr": stderr,
        "stdout_truncated": False,
        "stderr_truncated": False,
        "started_at": now,
        "finished_at": now,
    }
    resp = requests.post(callback_url, headers=api_headers(), json=payload)
    resp.raise_for_status()


def execute_task(task_data):
    """Execute an MCP tool call via the configured MCP server."""
    tool_name = task_data.get("tool", "")
    args_json = task_data.get("arguments", "{}")

    if not tool_name:
        return "failed", "", "No tool name provided"

    try:
        arguments = json.loads(args_json) if isinstance(args_json, str) else args_json
    except json.JSONDecodeError as e:
        return "failed", "", f"Invalid arguments JSON: {e}"

    try:
        client = McpClient(MCP_SERVER_URL)
        client.handshake()
        result = client.call_tool(tool_name, arguments)

        # Extract text content
        content = result.get("content", [])
        texts = [c["text"] for c in content if c.get("type") == "text"]
        stdout = "\n".join(texts)

        is_error = result.get("isError", False)
        if is_error:
            return "failed", "", stdout
        return "succeeded", stdout, ""

    except Exception as e:
        return "failed", "", str(e)


def main():
    print(f"Kronforce MCP Agent")
    print(f"  Controller: {CONTROLLER_URL}")
    print(f"  MCP Server: {MCP_SERVER_URL}")
    print(f"  Agent Name: {AGENT_NAME}")
    print()

    agent_id = register()

    print(f"Polling for work every {POLL_INTERVAL}s...")
    print("Configure task types in the dashboard: Agents > {agent_name} > Add task type")
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
            task_data = task.get("data", {})

            print(f"Received job {exec_id[:8]}...")

            status, stdout, stderr = execute_task(task_data)

            print(f"  Result: {status}")
            if stdout:
                print(f"  Output: {stdout[:100]}")

            report_result(callback_url, exec_id, job_id, agent_id, status, stdout, stderr)

        except KeyboardInterrupt:
            print("\nShutting down...")
            sys.exit(0)
        except Exception as e:
            print(f"Error: {e}")
            time.sleep(POLL_INTERVAL)


if __name__ == "__main__":
    main()
