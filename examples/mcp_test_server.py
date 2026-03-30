#!/usr/bin/env python3
"""
Simple MCP test server for Kronforce.

This server exposes a few basic tools via the Model Context Protocol (MCP)
over stdio transport. Use it to test the MCP task type in Kronforce.

Setup:
    pip install mcp

Usage in Kronforce:
    1. Go to Jobs > Create Job
    2. Select task type: MCP Tool
    3. Transport: Stdio
    4. Server: python3 examples/mcp_test_server.py
    5. Click "Discover Tools" to see available tools
    6. Select a tool and fill in arguments

Available tools:
    - greet(name) — Returns a greeting message
    - add(a, b) — Adds two numbers
    - system_info() — Returns OS and Python version info
    - word_count(text) — Counts words in text
    - reverse(text) — Reverses a string
"""

from mcp.server.fastmcp import FastMCP

mcp = FastMCP("kronforce-test-server")


@mcp.tool()
def greet(name: str) -> str:
    """Greet someone by name"""
    return f"Hello, {name}! Welcome to Kronforce MCP."


@mcp.tool()
def add(a: int, b: int) -> str:
    """Add two numbers together"""
    return f"{a} + {b} = {a + b}"


@mcp.tool()
def system_info() -> str:
    """Get basic system information"""
    import platform
    import sys
    return (
        f"OS: {platform.system()} {platform.release()}\n"
        f"Python: {sys.version}\n"
        f"Machine: {platform.machine()}\n"
        f"Hostname: {platform.node()}"
    )


@mcp.tool()
def word_count(text: str) -> str:
    """Count the number of words in a text string"""
    words = text.split()
    return f"{len(words)} words"


@mcp.tool()
def reverse(text: str) -> str:
    """Reverse a string"""
    return text[::-1]


if __name__ == "__main__":
    mcp.run(transport="stdio")
