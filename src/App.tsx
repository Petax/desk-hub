import { useState, useEffect, useCallback } from "react";
import { getConfig, saveConfig, toggleAlwaysOnTop, startDragging, minimizeWindow, closeWindow } from "./api";
import type { AppConfig } from "./types";
import { ClaudeWidget } from "./components/ClaudeWidget";
import { CodexWidget } from "./components/CodexWidget";
import { MediaWidget } from "./components/MediaWidget";
import { Settings } from "./components/Settings";
import "./App.css";

export default function App() {
  const [pinned, setPinned] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [config, setConfig] = useState<AppConfig>({});
  const [configLoaded, setConfigLoaded] = useState(false);
  // Live opacity preview while slider is dragged — null means use saved value
  const [previewOpacity, setPreviewOpacity] = useState<number | null>(null);

  useEffect(() => {
    getConfig().then((c) => { setConfig(c); setConfigLoaded(true); });
  }, []);

  const handlePin = useCallback(async () => {
    const next = !pinned;
    setPinned(next);
    await toggleAlwaysOnTop(next);
  }, [pinned]);

  const handleSaveConfig = useCallback(async (updated: AppConfig) => {
    await saveConfig(updated);
    setConfig(updated);
    setPreviewOpacity(null); // clear preview — saved value takes over
    setShowSettings(false);
  }, []);

  const handleDrag = useCallback((e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button") === null) {
      startDragging();
    }
  }, []);

  if (!configLoaded) return null;

  const opacity = previewOpacity ?? config.opacity ?? 0.93;

  return (
    <div
      className="hud"
      style={{ background: `rgba(14, 14, 16, ${opacity})` }}
    >
      <div className="titlebar" onMouseDown={handleDrag} data-tauri-drag-region>
        <span className="titlebar-drag">DESK·HUD</span>
        <button
          className={`btn-icon ${pinned ? "pin-active" : ""}`}
          title={pinned ? "Unpin" : "Pin on top"}
          onClick={handlePin}
        >
          📌
        </button>
        <button
          className={`btn-icon ${showSettings ? "active" : ""}`}
          title="Settings"
          onClick={() => setShowSettings((s) => !s)}
        >
          ⚙
        </button>
        <div className="titlebar-sep" />
        <button className="btn-icon btn-min" title="Minimise" onClick={minimizeWindow}>−</button>
        <button className="btn-icon btn-close" title="Close" onClick={closeWindow}>×</button>
      </div>

      <div className="content">
        {showSettings ? (
          <Settings
            config={config}
            onSave={handleSaveConfig}
            onOpacityPreview={setPreviewOpacity}
          />
        ) : (
          <>
            <ClaudeWidget config={config} />
            <CodexWidget />
            {(config.show_media ?? true) && <MediaWidget />}
          </>
        )}
      </div>
    </div>
  );
}
