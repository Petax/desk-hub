# Desk HUD

Desk HUD is a small Windows desktop overlay for keeping useful developer signals in view while working. It is built with Tauri, React, TypeScript, and Rust.

## Current Features

- Compact draggable HUD window with minimize, close, and always-on-top controls.
- Claude usage widget:
  - Uses a configured Claude `sessionKey` for live session and weekly usage when available.
  - Reads `organizationUuid` from `~/.claude/.credentials.json` automatically (no extra API call needed).
  - Falls back to counting today's local Claude messages from `~/.claude/projects/**/*.jsonl` when no session key is set.
- Codex usage widget:
  - Reads live Codex agentic usage limits from your local Codex CLI sign-in.
  - Shows 5-hour remaining limit, weekly remaining limit, and reset times.
- Usage windows show a live countdown during the final 24 hours before reset.
- Each live usage window estimates your current pace and shows whether you are on track to stay within the limit or at risk of exceeding it.
- Media widget:
  - Uses Windows System Media Transport Controls.
  - Shows current track metadata, artwork, playback progress, and previous/play-pause/next controls.
- Settings panel for Claude auth, Claude fallback limit, API debug output, and HUD opacity.

## Requirements

- Windows
- Node.js and npm
- Rust toolchain
- Tauri system prerequisites
- Codex CLI sign-in for Codex usage status

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

## Package a Windows test build

Create an installable release build:

```bash
npm run tauri build
```

The installers are written to:

```text
src-tauri/target/release/bundle/nsis/desk-hud_0.1.1_x64-setup.exe
src-tauri/target/release/bundle/msi/desk-hud_0.1.1_x64_en-US.msi
```

Use the NSIS `.exe` for normal testing and sharing. The MSI is useful for managed Windows installation. The app still needs the tester's own Claude session key and Codex CLI sign-in to show live usage data.

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

- `Claude session key`: optional `sessionKey` cookie from `claude.ai` for live usage data. Find it in DevTools → Application → Cookies → claude.ai → `sessionKey`.
- `Claude daily limit`: local fallback limit used when no Claude session key is configured.
- Codex usage uses `~/.codex/auth.json` from your local Codex CLI sign-in. No OpenAI Admin API key is needed.
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
- Pace projections are linear estimates based on usage so far in the current 5-hour or 7-day window; they are guidance rather than provider guarantees.
