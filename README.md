# Configonaut

A desktop app for managing Claude Desktop and Claude Code configuration files. Built with Tauri 2, React, and Rust.

## Features

- **MCP Server Management** -- Add, remove, and toggle MCP servers between Active and Inactive. Drag-and-drop to reorder. Inline JSON editor with validation.
- **Marketplace** -- Browse and install MCP servers from a curated catalog. Runtime prerequisite detection warns when Node.js, Python, or Docker are missing.
- **Desktop/CLI Mode Toggle** -- Switch between managing Claude Desktop (`claude_desktop_config.json`) and Claude Code (`~/.claude/settings.json`) configurations.
- **Hooks** -- View and edit Claude Code automation hooks with a toggle and JSON editor.
- **Agents** -- Manage personal and plugin agents with a full markdown editor.
- **Skills** -- Manage slash commands and skills, enable/disable plugin skills.
- **Backups** -- Automatic config backups with diff preview and one-click restore.
- **Cross-platform** -- macOS and Windows. Linux support planned.

## Project Structure

```
tauri-app/           Tauri 2 + React + Rust application
marketplace-catalog/ Catalog tooling and schema
docs/                Design specs, plans, and guides
```

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+ and [bun](https://bun.sh/)
- [Tauri CLI](https://tauri.app/start/): `cargo install tauri-cli`

### Run locally

```bash
cd tauri-app
bun install
cargo tauri dev
```

### Build for release

```bash
cd tauri-app
cargo tauri build
```

The bundled `.app` (macOS) or `.msi` / `.exe` (Windows) will be in `tauri-app/src-tauri/target/release/bundle/`.

## License

MIT
