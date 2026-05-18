use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Window};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub claude_session_key: Option<String>,
    pub claude_daily_limit: Option<u32>,
    pub show_media: Option<bool>,
    pub opacity: Option<f64>,
}

fn config_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("config.json")
}

#[tauri::command]
fn get_config(app: AppHandle) -> AppConfig {
    let p = config_path(&app);
    if p.exists() {
        serde_json::from_str(&fs::read_to_string(p).unwrap_or_default()).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

#[tauri::command]
fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let p = config_path(&app);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(p, serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

// ── Window ────────────────────────────────────────────────────────────────────

#[tauri::command]
fn toggle_always_on_top(window: Window, on_top: bool) -> Result<(), String> {
    window.set_always_on_top(on_top).map_err(|e| e.to_string())
}

fn cli_org_uuid() -> Option<String> {
    let creds_path = dirs::home_dir()?.join(".claude").join(".credentials.json");
    let creds: serde_json::Value = serde_json::from_str(&fs::read_to_string(creds_path).ok()?).ok()?;
    creds["organizationUuid"].as_str().map(str::to_string)
}

// ── Claude.ai session usage ───────────────────────────────────────────────────

#[derive(Serialize, Default)]
pub struct ClaudeUsage {
    pub plan: String,
    pub session_pct: Option<f64>,
    pub session_resets_in: Option<String>,
    pub weekly_pct: Option<f64>,
    pub weekly_resets_at: Option<String>,
    // Fallback: local message count
    pub local_messages: Option<u32>,
    pub local_limit: u32,
}

#[tauri::command]
async fn get_claude_usage(app: AppHandle) -> Result<ClaudeUsage, String> {
    let config = get_config(app);
    let local_limit = config.claude_daily_limit.unwrap_or(500);

    if let Some(session_key) = config.claude_session_key.as_deref() {
        if !session_key.trim().is_empty() {
            return fetch_claude_api(session_key.trim(), local_limit).await;
        }
    }

    let local_messages = count_local_messages();
    Ok(ClaudeUsage {
        plan: String::new(),
        local_messages: Some(local_messages),
        local_limit,
        ..Default::default()
    })
}

async fn fetch_claude_api(session_key: &str, local_limit: u32) -> Result<ClaudeUsage, String> {
    let org_uuid = match cli_org_uuid() {
        Some(id) if !id.is_empty() => id,
        _ => {
            let me = claude_get_json("https://claude.ai/api/account", session_key).await?;
            claude_org_uuid(&me).ok_or_else(|| "Could not find org UUID".to_string())?
        }
    };

    let subscription = claude_get_json(
        &format!("https://claude.ai/api/organizations/{}/subscription_details", org_uuid),
        session_key,
    ).await.ok();

    let limits = claude_get_json(
        &format!("https://claude.ai/api/organizations/{}/usage", org_uuid),
        session_key,
    ).await?;

    let session_pct = usage_pct(&limits, "five_hour");
    let session_resets_in = usage_resets_at(&limits, "five_hour").map(fmt_resets_in);
    let weekly_pct = usage_pct(&limits, "seven_day");
    let weekly_resets_at = usage_resets_at(&limits, "seven_day").map(fmt_resets_at);

    let me = serde_json::Value::Null;
    let plan = claude_plan_label(&me, subscription.as_ref(), &limits).to_string();

    Ok(ClaudeUsage { plan, session_pct, session_resets_in, weekly_pct, weekly_resets_at, local_messages: None, local_limit })
}

async fn claude_get_json(url: &str, session_key: &str) -> Result<serde_json::Value, String> {
    let resp = reqwest::Client::new()
        .get(url)
        .header("Cookie", format!("sessionKey={}", session_key))
        .header("anthropic-client-type", "web")
        .header("Origin", "https://claude.ai")
        .header("Referer", "https://claude.ai/settings/usage")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("Claude API {}: {}", status, compact_error_body(&body)));
    }
    serde_json::from_str(body.trim()).map_err(|e| format!("Claude JSON parse: {e}"))
}

fn usage_pct(data: &serde_json::Value, key: &str) -> Option<f64> {
    data[key]["utilization"].as_f64()
        .or_else(|| data[key]["utilization_pct"].as_f64())
        .or_else(|| data[key]["percent_used"].as_f64())
        .map(|v| if v <= 1.0 { v * 100.0 } else { v })
}

fn usage_resets_at<'a>(data: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    data[key]["resets_at"].as_str().or_else(|| data[key]["reset_at"].as_str())
}

fn claude_org_uuid(data: &serde_json::Value) -> Option<String> {
    data["memberships"].as_array()?.iter().find_map(|m| {
        m["organization"]["uuid"].as_str()
            .or_else(|| m["organization"]["organization_uuid"].as_str())
            .or_else(|| m["organization_uuid"].as_str())
            .or_else(|| m["uuid"].as_str())
            .map(str::to_string)
    })
}

fn claude_plan_label(account: &serde_json::Value, subscription: Option<&serde_json::Value>, usage: &serde_json::Value) -> &'static str {
    if json_has_plan(account, &["max_plan", "claude_max", "max"]) || subscription.is_some_and(|s| json_has_plan(s, &["max_plan", "claude_max", "max"])) { return "Max"; }
    if json_has_plan(account, &["pro_plan", "claude_pro", "pro"]) || subscription.is_some_and(|s| json_has_plan(s, &["pro_plan", "claude_pro", "pro"])) { return "Pro"; }
    if json_has_plan(account, &["team_plan", "claude_team", "team"]) || subscription.is_some_and(|s| json_has_plan(s, &["team_plan", "claude_team", "team"])) { return "Team"; }
    if json_has_plan(account, &["enterprise_plan", "claude_enterprise", "enterprise"]) || subscription.is_some_and(|s| json_has_plan(s, &["enterprise_plan", "claude_enterprise", "enterprise"])) { return "Enterprise"; }
    if usage["seven_day"].is_object() || usage["seven_day_sonnet"].is_object() { return "Pro"; }
    "Free"
}

fn json_has_plan(value: &serde_json::Value, needles: &[&str]) -> bool {
    match value {
        serde_json::Value::String(s) => { let n = s.to_ascii_lowercase(); needles.iter().any(|needle| n == *needle || n == format!("{}_plan", needle) || n.contains(&format!("{} plan", needle)) || n.contains(&format!("claude {}", needle))) }
        serde_json::Value::Array(items) => items.iter().any(|i| json_has_plan(i, needles)),
        serde_json::Value::Object(map) => map.iter().any(|(k, v)| { let k = k.to_ascii_lowercase(); (k.contains("plan") || k.contains("tier") || k.contains("subscription") || k.contains("flag")) && needles.iter().any(|n| k.contains(n)) || json_has_plan(v, needles) }),
        _ => false,
    }
}

fn fmt_resets_in(iso: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) {
        let secs = (dt.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_seconds().max(0);
        return fmt_seconds(secs as f64);
    }
    iso.to_string()
}

fn fmt_resets_at(iso: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) {
        return dt.with_timezone(&chrono::Local).format("%a %H:%M").to_string();
    }
    iso.to_string()
}


fn count_local_messages() -> u32 {
    use glob::glob;
    use std::io::{BufRead, BufReader};

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return 0,
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let pattern = home
        .join(".claude/projects/**/*.jsonl")
        .to_string_lossy()
        .to_string();
    let mut count = 0u32;
    if let Ok(paths) = glob(&pattern) {
        for path in paths.flatten() {
            if let Ok(file) = fs::File::open(path) {
                for line in BufReader::new(file).lines().flatten() {
                    if line.contains(&today)
                        && line.contains("\"type\":\"user\"")
                        && !line.contains("\"isSidechain\":true")
                    {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

fn fmt_seconds(secs: f64) -> String {
    let s = secs as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}

#[tauri::command]
async fn debug_claude_api(session_key: String) -> Result<String, String> {
    let session_key = session_key.trim();
    let org_uuid = match cli_org_uuid() {
        Some(id) if !id.is_empty() => id,
        _ => {
            let me = claude_get_json("https://claude.ai/api/account", session_key).await?;
            claude_org_uuid(&me).unwrap_or_default()
        }
    };
    if org_uuid.is_empty() {
        return Err("Could not find org UUID".to_string());
    }
    let limits = claude_get_json(
        &format!("https://claude.ai/api/organizations/{}/usage", org_uuid),
        session_key,
    ).await?;
    Ok(serde_json::to_string_pretty(&limits).unwrap_or_default())
}

// ── Codex usage ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CodexUsage {
    pub plan: String,
    pub five_hour_remaining_pct: Option<f64>,
    pub five_hour_resets_at: Option<String>,
    pub weekly_remaining_pct: Option<f64>,
    pub weekly_resets_at: Option<String>,
}

#[tauri::command]
async fn get_codex_usage() -> Result<CodexUsage, String> {
    let auth_path = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?
        .join(".codex")
        .join("auth.json");

    let auth: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&auth_path)
            .map_err(|_| "Sign in with Codex CLI first".to_string())?,
    )
    .map_err(|e| format!("Codex auth JSON parse: {e}"))?;

    let access_token = auth["tokens"]["access_token"]
        .as_str()
        .ok_or_else(|| "Codex access token not found; sign in with Codex CLI again".to_string())?;
    let account_id = auth["tokens"]["account_id"]
        .as_str()
        .or_else(|| auth["account_id"].as_str())
        .unwrap_or_default();

    let mut req = reqwest::Client::new()
        .get("https://chatgpt.com/backend-api/wham/usage")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "codex-cli")
        .header("Accept", "application/json");

    if !account_id.is_empty() {
        req = req.header("ChatGPT-Account-Id", account_id);
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "Codex usage API {}: {}",
            status,
            compact_error_body(&body)
        ));
    }

    let usage: serde_json::Value =
        serde_json::from_str(body.trim()).map_err(|e| format!("Codex usage JSON parse: {e}"))?;

    let primary = &usage["rate_limit"]["primary_window"];
    let secondary = &usage["rate_limit"]["secondary_window"];

    Ok(CodexUsage {
        plan: codex_plan_label(usage["plan_type"].as_str().unwrap_or_default()),
        five_hour_remaining_pct: codex_remaining_pct(primary),
        five_hour_resets_at: codex_reset_at(primary, false),
        weekly_remaining_pct: codex_remaining_pct(secondary),
        weekly_resets_at: codex_reset_at(secondary, true),
    })
}

fn codex_remaining_pct(window: &serde_json::Value) -> Option<f64> {
    let used = window["used_percent"].as_f64()?;
    Some((100.0 - used).clamp(0.0, 100.0))
}

fn codex_reset_at(window: &serde_json::Value, include_date: bool) -> Option<String> {
    let reset_at = window["reset_at"].as_i64()?;
    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(reset_at, 0)?;
    let local = dt.with_timezone(&chrono::Local);
    Some(
        if include_date {
            local.format("%b %d, %Y %H:%M")
        } else {
            local.format("%H:%M")
        }
        .to_string(),
    )
}

fn codex_plan_label(plan_type: &str) -> String {
    match plan_type.to_ascii_lowercase().as_str() {
        "free" => "Free".to_string(),
        "plus" => "Plus".to_string(),
        "pro" => "Pro".to_string(),
        "team" => "Team".to_string(),
        "enterprise" => "Enterprise".to_string(),
        "" => String::new(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        }
    }
}

fn compact_error_body(body: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(message) = json["error"]["message"].as_str() {
            return message.to_string();
        }
        if let Some(message) = json["message"].as_str() {
            return message.to_string();
        }
    }

    let body = body.trim().replace('\r', " ").replace('\n', " ");
    let preview: String = body.chars().take(220).collect();
    if body.chars().count() > 220 {
        format!("{}...", preview)
    } else {
        body
    }
}

// ── Windows SMTC (desktop media controls) ────────────────────────────────────

#[derive(Serialize, Default)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source_app: String,
    pub is_playing: bool,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub thumbnail_b64: Option<String>,
}

#[cfg(target_os = "windows")]
fn smtc_com_init() {
    unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        // S_FALSE = already initialised on this thread — that's fine
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

#[cfg(target_os = "windows")]
fn find_spotify_session(
    manager: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager,
) -> Option<windows::Media::Control::GlobalSystemMediaTransportControlsSession> {
    let sessions = manager.GetSessions().ok()?;
    let count = sessions.Size().ok()?;
    for i in 0..count {
        let s = sessions.GetAt(i).ok()?;
        if let Ok(id) = s.SourceAppUserModelId() {
            if id.to_string().to_lowercase().contains("spotify") {
                return Some(s);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn get_media_info_inner() -> Result<MediaInfo, String> {
    use base64::Engine;
    use windows::Foundation::TimeSpan;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Storage::Streams::DataReader;

    smtc_com_init();

    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;

    let session = find_spotify_session(&manager).ok_or_else(|| "no_playback".to_string())?;

    let props = session
        .TryGetMediaPropertiesAsync()
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;

    let playback = session.GetPlaybackInfo().map_err(|e| e.to_string())?;
    let is_playing = playback
        .PlaybackStatus()
        .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false);

    let timeline = session.GetTimelineProperties().ok();
    let mut position_secs = timeline
        .as_ref()
        .and_then(|t| t.Position().ok())
        .map(|p: TimeSpan| p.Duration as f64 / 10_000_000.0)
        .unwrap_or(0.0);
    let duration_secs = timeline
        .as_ref()
        .and_then(|t| t.EndTime().ok())
        .map(|d: TimeSpan| d.Duration as f64 / 10_000_000.0)
        .unwrap_or(0.0);

    if is_playing {
        if let Some(last_updated_secs) = timeline
            .as_ref()
            .and_then(|t| t.LastUpdatedTime().ok())
            .map(windows_datetime_to_unix_secs)
        {
            let elapsed = (chrono::Utc::now().timestamp_millis() as f64 / 1000.0
                - last_updated_secs)
                .clamp(0.0, 10.0);
            position_secs = (position_secs + elapsed).min(duration_secs.max(position_secs));
        }
    }

    let source_app = "Spotify".to_string();

    // Try to get album art thumbnail (best-effort; skip on any error)
    let thumbnail_b64 = (|| -> Option<String> {
        let thumb_ref = props.Thumbnail().ok()?;
        let stream = thumb_ref.OpenReadAsync().ok()?.get().ok()?;
        let size = stream.Size().ok()? as u32;
        if size == 0 || size > 2_000_000 {
            return None;
        }
        let reader = DataReader::CreateDataReader(&stream).ok()?;
        reader.LoadAsync(size).ok()?.get().ok()?;
        let mut buf = vec![0u8; size as usize];
        reader.ReadBytes(&mut buf).ok()?;
        Some(format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&buf)
        ))
    })();

    Ok(MediaInfo {
        title: props.Title().map(|s| s.to_string()).unwrap_or_default(),
        artist: props.Artist().map(|s| s.to_string()).unwrap_or_default(),
        album: props.AlbumTitle().map(|s| s.to_string()).unwrap_or_default(),
        source_app,
        is_playing,
        position_secs,
        duration_secs,
        thumbnail_b64,
    })
}

#[cfg(target_os = "windows")]
fn windows_datetime_to_unix_secs(dt: windows::Foundation::DateTime) -> f64 {
    const WINDOWS_TO_UNIX_EPOCH_100NS: i64 = 116_444_736_000_000_000;
    (dt.UniversalTime - WINDOWS_TO_UNIX_EPOCH_100NS) as f64 / 10_000_000.0
}

// Async wrapper — offloads the blocking .get() calls to a dedicated thread
#[cfg(target_os = "windows")]
#[tauri::command]
async fn get_media_info() -> Result<MediaInfo, String> {
    tokio::task::spawn_blocking(get_media_info_inner)
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
async fn get_media_info() -> Result<MediaInfo, String> {
    Err("SMTC is Windows-only".to_string())
}

#[cfg(target_os = "windows")]
fn media_control_inner(action: String) -> Result<(), String> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;

    smtc_com_init();

    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;

    let session = find_spotify_session(&manager).ok_or_else(|| "no_playback".to_string())?;

    match action.as_str() {
        "play" => { session.TryPlayAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?; }
        "pause" => { session.TryPauseAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?; }
        "play_pause" => { session.TryTogglePlayPauseAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?; }
        "next" => { session.TrySkipNextAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?; }
        "previous" => { session.TrySkipPreviousAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?; }
        _ => return Err("Unknown action".to_string()),
    }
    Ok(())
}

#[cfg(target_os = "windows")]
#[tauri::command]
async fn media_control(action: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || media_control_inner(action))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
async fn media_control(_action: String) -> Result<(), String> {
    Err("SMTC is Windows-only".to_string())
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            toggle_always_on_top,
            get_config,
            save_config,
            get_claude_usage,
            debug_claude_api,
            get_codex_usage,
            get_media_info,
            media_control,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
