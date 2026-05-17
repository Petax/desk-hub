use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Window};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub openai_api_key: Option<String>,
    pub claude_session_key: Option<String>,
    pub claude_daily_limit: Option<u32>,
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

    // Try the claude.ai API if a session key is configured
    if let Some(session_key) = config.claude_session_key.as_deref() {
        if !session_key.is_empty() {
            if let Ok(usage) = fetch_claude_api(session_key, local_limit).await {
                return Ok(usage);
            }
        }
    }

    // Fallback: count today's user messages from local JSONL files
    let local_messages = count_local_messages();
    Ok(ClaudeUsage {
        plan: String::new(),
        local_messages: Some(local_messages),
        local_limit,
        ..Default::default()
    })
}

async fn powershell_get_json(url: &str, session_key: &str) -> Result<serde_json::Value, String> {
    let cmd = format!(
        "$r = Invoke-WebRequest -Uri '{url}' \
         -Headers @{{'Cookie'='sessionKey={session_key}';'anthropic-client-type'='web'}} \
         -UseBasicParsing; $r.Content"
    );
    let output = tokio::process::Command::new("powershell.exe")
        .args(["-NonInteractive", "-NoProfile", "-Command", &cmd])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).map_err(|e| format!("JSON parse: {e}\n{}", stdout.trim()))
}

async fn fetch_claude_api(session_key: &str, local_limit: u32) -> Result<ClaudeUsage, String> {
    // Step 1: get account info and org UUID
    let me = powershell_get_json("https://api.claude.ai/api/accounts/me", session_key).await?;

    let org_uuid = me["memberships"]
        .as_array()
        .and_then(|m| m.first())
        .and_then(|m| m["organization"]["uuid"].as_str())
        .unwrap_or("")
        .to_string();

    let plan = me["memberships"]
        .as_array()
        .and_then(|m| m.first())
        .and_then(|m| m["organization"]["active_flags"].as_array())
        .and_then(|flags| {
            if flags.iter().any(|f| f.as_str() == Some("max_plan")) {
                Some("Max")
            } else if flags.iter().any(|f| f.as_str() == Some("pro_plan")) {
                Some("Pro")
            } else {
                None
            }
        })
        .unwrap_or("Free")
        .to_string();

    if org_uuid.is_empty() {
        return Err("Could not find org UUID".to_string());
    }

    // Step 2: get rate limits for the org
    let limits = powershell_get_json(
        &format!("https://api.claude.ai/api/organizations/{}/rate_limit_status", org_uuid),
        session_key,
    ).await?;

    // Parse session usage
    let session_pct = limits["current_session"]
        .as_object()
        .and_then(|s| {
            let used = s["messages_used"].as_f64()?;
            let limit = s["messages_limit"].as_f64()?;
            if limit > 0.0 {
                Some((used / limit) * 100.0)
            } else {
                s["percent_used"].as_f64()
            }
        })
        .or_else(|| limits["current_session"]["percent_used"].as_f64());

    let session_resets_in = limits["current_session"]["resets_at"]
        .as_str()
        .map(fmt_resets_in)
        .or_else(|| {
            limits["current_session"]["resets_in_seconds"]
                .as_f64()
                .map(fmt_seconds)
        });

    let weekly_pct = limits["weekly"]
        .as_object()
        .and_then(|w| {
            let used = w["messages_used"].as_f64()?;
            let limit = w["messages_limit"].as_f64()?;
            if limit > 0.0 {
                Some((used / limit) * 100.0)
            } else {
                w["percent_used"].as_f64()
            }
        })
        .or_else(|| limits["weekly"]["percent_used"].as_f64());

    let weekly_resets_at = limits["weekly"]["resets_at"]
        .as_str()
        .map(fmt_resets_at);

    Ok(ClaudeUsage {
        plan,
        session_pct,
        session_resets_in,
        weekly_pct,
        weekly_resets_at,
        local_messages: None,
        local_limit,
    })
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

fn fmt_resets_in(iso: &str) -> String {
    // iso is an ISO8601 datetime; compute duration from now
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) {
        let now = chrono::Utc::now();
        let secs = (dt.with_timezone(&chrono::Utc) - now).num_seconds().max(0);
        return fmt_seconds(secs as f64);
    }
    iso.to_string()
}

fn fmt_resets_at(iso: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) {
        return dt
            .with_timezone(&chrono::Local)
            .format("%a %H:%M")
            .to_string();
    }
    iso.to_string()
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

// Returns raw API JSON for debugging — lets us see actual field names
#[tauri::command]
async fn debug_claude_api(session_key: String) -> Result<String, String> {
    let me = powershell_get_json("https://api.claude.ai/api/accounts/me", &session_key).await?;

    let org_uuid = me["memberships"]
        .as_array()
        .and_then(|m| m.first())
        .and_then(|m| m["organization"]["uuid"].as_str())
        .unwrap_or("")
        .to_string();

    if org_uuid.is_empty() {
        return Ok(format!("accounts/me response:\n{}", serde_json::to_string_pretty(&me).unwrap_or_default()));
    }

    let limits = powershell_get_json(
        &format!("https://api.claude.ai/api/organizations/{}/rate_limit_status", org_uuid),
        &session_key,
    ).await?;

    Ok(serde_json::to_string_pretty(&limits).unwrap_or_default())
}

// ── OpenAI usage ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct OpenAIUsage {
    pub tokens_used: u64,
    pub requests: u64,
}

#[tauri::command]
async fn get_openai_usage(api_key: String) -> Result<OpenAIUsage, String> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("https://api.openai.com/v1/usage?date={}", today))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("OpenAI API {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let (tokens, requests) = data["data"]
        .as_array()
        .map(|arr| {
            arr.iter().fold((0u64, 0u64), |(t, r), item| {
                (
                    t + item["n_context_tokens_total"].as_u64().unwrap_or(0)
                        + item["n_generated_tokens_total"].as_u64().unwrap_or(0),
                    r + item["n_requests"].as_u64().unwrap_or(0),
                )
            })
        })
        .unwrap_or((0, 0));

    Ok(OpenAIUsage { tokens_used: tokens, requests })
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
    let position_secs = timeline
        .as_ref()
        .and_then(|t| t.Position().ok())
        .map(|p: TimeSpan| p.Duration as f64 / 10_000_000.0)
        .unwrap_or(0.0);
    let duration_secs = timeline
        .as_ref()
        .and_then(|t| t.EndTime().ok())
        .map(|d: TimeSpan| d.Duration as f64 / 10_000_000.0)
        .unwrap_or(0.0);

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
            get_openai_usage,
            get_media_info,
            media_control,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
