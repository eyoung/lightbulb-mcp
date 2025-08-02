# Lightbulb MCP Server

A Model Context Protocol (MCP) server implementation in Rust for managing a virtual lightbulb. This server provides tools to control and monitor a simulated lightbulb, with all actions logged to a file.

## Features

- **Get Status**: Check if the lightbulb is currently on or off
- **Turn On**: Turn the lightbulb on (with logging)
- **Turn Off**: Turn the lightbulb off (with logging)
- **Event Logging**: All state changes are logged to `lightbulb.log` with timestamps

## Available Tools

### `get_lightbulb_status`
- **Description**: Get the current status of the lightbulb
- **Parameters**: None
- **Returns**: String indicating whether the lightbulb is on or off

### `turn_on_lightbulb`
- **Description**: Turn on the lightbulb
- **Parameters**: None
- **Returns**: Success message or error if already on
- **Side Effect**: Logs the action to `lightbulb.log`

### `turn_off_lightbulb`
- **Description**: Turn off the lightbulb
- **Parameters**: None
- **Returns**: Success message or error if already off
- **Side Effect**: Logs the action to `lightbulb.log`

## Building and Running

### Prerequisites
- Rust (latest stable version)
- Cargo

### Build
```bash
cargo build
```

### Run
```bash
cargo run
```

## Log Format

The server logs all lightbulb actions to `lightbulb.log` in the following format:
```
[2025-08-02T14:24:27.652821025+00:00] Lightbulb turned ON
[2025-08-02T15:48:03.599625808+00:00] Lightbulb turned OFF
```

## Technical Details

- Built using the `rmcp` crate for MCP protocol implementation
- Uses `Arc<Mutex<bool>>` for thread-safe state management
- Implements async tool handlers
- Uses `chrono` for RFC3339 timestamp formatting
- File I/O for persistent logging

## Dependencies

- `rmcp` - MCP protocol implementation
- `tokio` - Async runtime
- `chrono` - Date/time handling
- `serde` & `serde_json` - Serialization

## Testing

You can test the server manually by sending JSON-RPC requests via stdin:

```json
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}
{"jsonrpc": "2.0", "id": 2, "method": "tools/list"}
{"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "turn_on_lightbulb"}}
```
