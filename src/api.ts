import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppConfig, ClaudeUsage, CodexUsage, MediaInfo } from "./types";

export const getConfig = (): Promise<AppConfig> => invoke("get_config");
export const saveConfig = (config: AppConfig): Promise<void> => invoke("save_config", { config });

export const toggleAlwaysOnTop = (onTop: boolean): Promise<void> =>
  invoke("toggle_always_on_top", { onTop });

export const startDragging = (): Promise<void> => getCurrentWindow().startDragging();
export const minimizeWindow = (): Promise<void> => getCurrentWindow().minimize();
export const closeWindow = (): Promise<void> => getCurrentWindow().close();

export const getClaudeUsage = (): Promise<ClaudeUsage> => invoke("get_claude_usage");

export const getCodexUsage = (): Promise<CodexUsage> => invoke("get_codex_usage");

export const getMediaInfo = (): Promise<MediaInfo> => invoke("get_media_info");
export const mediaControl = (action: string): Promise<void> =>
  invoke("media_control", { action });

export function fmtSecs(s: number): string {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
}
