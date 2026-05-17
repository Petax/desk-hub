# Desk HUD

Desk HUD is a small Windows desktop overlay for keeping useful developer signals in view while working. It is built with Tauri, React, TypeScript, and Rust.

## Current Features

- Compact draggable HUD window with minimize, close, and always-on-top controls.
- Claude usage widget:
  - Uses a configured Claude `sessionKey` for live session and weekly usage when available.
  - Falls back to counting today's local Claude messages from `~/.claude/projects/**/*.jsonl`.
- OpenAI / Codex usage widget:
  - Reads today's token and request usage from the configured OpenAI API key.
- Media widget:
  - Uses Windows System Media Transport Controls.
  - Shows current track metadata, artwork, playback progress, and previous/play-pause/next controls.
- Settings panel for API keys, Claude fallback limit, API debug output, and HUD opacity.

## Requirements

- Windows
- Node.js and npm
- Rust toolchain
- Tauri system prerequisites

## Setup

Install dependencies:

```bash
npm install
```

Run the Vite frontend:

```bash
npm run dev
```

Run the full Tauri desktop app:

```bash
npm run tauri dev
```

Build the frontend:

```bash
npm run build
```

Build the desktop app:

```bash
npm run tauri build
```

## Configuration

Configuration is saved in the Tauri app data directory as `config.json`.

The settings panel supports:

- `Claude session key`: optional `sessionKey` cookie from `claude.ai` for live usage data.
- `Claude daily limit`: local fallback limit used when no Claude session key is configured.
- `OpenAI API key`: optional API key for fetching today's OpenAI usage.
- `Background opacity`: live-previewed HUD background opacity.

## Project Structure

```text
src/                 React frontend
src/components/      HUD widgets and settings UI
src-tauri/           Tauri/Rust backend
public/              Static frontend assets
```

## Notes

- The media widget is Windows-only because it uses SMTC APIs.
- API keys and session cookies are stored locally by the app and should not be committed.
- This is an early personal utility, so some usage APIs may need adjustment as upstream services change.
