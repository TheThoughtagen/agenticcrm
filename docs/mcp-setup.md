# MCP Setup Guide

Connect AgenticCRM to AI assistants via the Model Context Protocol.

## Prerequisites

- `acrm` installed and on your PATH (verify with `acrm --help`)
- An MCP-compatible client (Claude Desktop, Claude Code, or any MCP client)

## Quick Start (stdio)

The fastest way to connect is stdio transport -- add a single config block and restart your client.

## Claude Desktop Setup

Add AgenticCRM to your Claude Desktop config file:

**Config file location:**
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

**Basic configuration:**

```json
{
  "mcpServers": {
    "agenticcrm": {
      "command": "acrm",
      "args": ["serve"]
    }
  }
}
```

**With sync enabled** (allows CardDAV sync via MCP):

```json
{
  "mcpServers": {
    "agenticcrm": {
      "command": "acrm",
      "args": ["serve", "--allow-sync"]
    }
  }
}
```

> **Note:** Restart Claude Desktop after making config changes. The MCP connection initializes at startup.

## Claude Code Setup

**Option A -- CLI command:**

```bash
claude mcp add agenticcrm -- acrm serve
```

**Option B -- Project-level `.mcp.json`:**

Create a `.mcp.json` file in your project root:

```json
{
  "mcpServers": {
    "agenticcrm": {
      "type": "stdio",
      "command": "acrm",
      "args": ["serve"]
    }
  }
}
```

This makes the CRM available whenever you open the project in Claude Code.

## HTTP Transport

Use HTTP transport when you need remote access, multiple simultaneous clients, or integration with non-stdio MCP clients.

**Start the server:**

```bash
acrm serve --http --port 3000
```

**Endpoint:** `http://localhost:3000/mcp`

This uses the Streamable HTTP transport (per the MCP spec 2025-03-26), not the deprecated HTTP+SSE transport.

## Available Tools

| Tool | Description |
|------|-------------|
| `search_contacts` | Search contacts by name, company, tag, email, or free text |
| `show_contact` | Show full details for a contact by name or partial match |
| `add_contact` | Add a new contact by full name |
| `edit_contact` | Edit a contact's fields using key=value pairs |
| `log_interaction` | Log an interaction (coffee, call, email, message, etc.) |
| `delete_contact` | Delete a contact permanently by name |
| `archive_contact` | Archive a contact (sets status to archived) |
| `due_followups` | List contacts due for follow-up, sorted by most overdue first |
| `sync_contacts` | Sync contacts with iCloud via CardDAV (requires `--allow-sync`) |

## Resources

AgenticCRM exposes contacts as MCP resources using the `contact://` URI scheme. This allows MCP clients to browse and read contacts directly, without invoking a tool.

- **List contacts:** The server provides a resource list of all contacts
- **Read a contact:** Access individual contacts via `contact://{filename}` URIs
- **Use case:** Useful for clients that support resource browsing or need to read contact data without a search query

## Troubleshooting

### "Server not found" or "command not found"

Ensure `acrm` is on your PATH:

```bash
which acrm
```

If not found, either add the directory containing `acrm` to your PATH, or use the full path in your MCP config:

```json
{
  "mcpServers": {
    "agenticcrm": {
      "command": "/path/to/acrm",
      "args": ["serve"]
    }
  }
}
```

### "Sync tool returns error"

The sync tool requires two things:

1. The server must be started with `--allow-sync`
2. Sync credentials must be configured first: `acrm sync setup`

### "Connection refused" (HTTP transport)

- Verify the server is running: `acrm serve --http --port 3000`
- Check that the port matches your client config
- Ensure no other process is using the same port

### "Tools not appearing" in your MCP client

- Restart the MCP client after config changes
- Check the client's MCP logs for connection errors
- Verify the config JSON is valid (no trailing commas, correct structure)
