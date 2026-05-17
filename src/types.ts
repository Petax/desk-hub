export interface AppConfig {
  openai_api_key?: string;
  claude_session_key?: string;
  claude_daily_limit?: number;
  opacity?: number; // 0.0–1.0, default 0.93
}

export interface ClaudeUsage {
  plan: string;
  session_pct?: number;
  session_resets_in?: string;
  weekly_pct?: number;
  weekly_resets_at?: string;
  local_messages?: number;
  local_limit: number;
}

export interface OpenAIUsage {
  tokens_used: number;
  requests: number;
}

export interface MediaInfo {
  title: string;
  artist: string;
  album: string;
  source_app: string;
  is_playing: boolean;
  position_secs: number;
  duration_secs: number;
  thumbnail_b64?: string;
}
