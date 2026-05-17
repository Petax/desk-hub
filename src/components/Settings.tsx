import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig } from "../types";

interface Props {
  config: AppConfig;
  onSave: (config: AppConfig) => void;
  onOpacityPreview: (v: number) => void;
}

export function Settings({ config, onSave, onOpacityPreview }: Props) {
  const [form, setForm] = useState<AppConfig>({ ...config });
  const [debugOutput, setDebugOutput] = useState<string | null>(null);
  const [debugging, setDebugging] = useState(false);

  function set<K extends keyof AppConfig>(key: K, value: AppConfig[K]) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  function handleOpacity(raw: string) {
    const v = parseFloat(raw);
    set("opacity", v);
    onOpacityPreview(v);
  }

  async function runDebug() {
    const key = form.claude_session_key;
    if (!key) {
      setDebugOutput("No session key set.");
      return;
    }
    setDebugging(true);
    setDebugOutput(null);
    try {
      const raw = await invoke<string>("debug_claude_api", { sessionKey: key });
      setDebugOutput(raw);
    } catch (e) {
      setDebugOutput(String(e));
    } finally {
      setDebugging(false);
    }
  }

  const opacity = form.opacity ?? 0.93;

  return (
    <div className="settings">
      <div className="settings-group">
        <label className="settings-label">Claude session key</label>
        <input
          className="settings-input"
          type="password"
          placeholder="sk-ant-sid01-..."
          value={form.claude_session_key ?? ""}
          onChange={(e) => set("claude_session_key", e.target.value || undefined)}
        />
        <span className="settings-hint">
          DevTools - Application - Cookies - claude.ai - sessionKey
        </span>
      </div>

      <div className="settings-row">
        <button className="btn-secondary" onClick={runDebug} disabled={debugging}>
          {debugging ? "fetching..." : "Debug API response"}
        </button>
      </div>

      {debugOutput !== null && <pre className="debug-output">{debugOutput}</pre>}

      <div className="settings-group">
        <label className="settings-label">Claude daily limit (msg fallback)</label>
        <input
          className="settings-input"
          type="number"
          value={form.claude_daily_limit ?? 500}
          onChange={(e) => set("claude_daily_limit", Number(e.target.value) || undefined)}
        />
      </div>

      <div className="settings-hint">
        Codex: uses your local Codex CLI sign-in from ~/.codex/auth.json.
      </div>

      <div className="settings-group">
        <label className="settings-label">
          Background opacity - {Math.round(opacity * 100)}%
        </label>
        <input
          type="range"
          className="settings-input"
          min="0.15"
          max="1"
          step="0.01"
          value={opacity}
          onChange={(e) => handleOpacity(e.target.value)}
        />
      </div>

      <div className="settings-hint">
        Media: Windows SMTC - auto-detects Spotify, browser, etc.
      </div>

      <div className="settings-row">
        <button className="btn-primary" onClick={() => onSave(form)}>
          Save
        </button>
      </div>
    </div>
  );
}
