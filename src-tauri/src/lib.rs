mod audio;
mod llm;
mod model;
mod permissions;
mod speaker;
mod state;
mod storage;
#[cfg(target_os = "macos")]
mod system_audio;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;
use uuid::Uuid;

use crate::llm::{generate_minutes as generate_minutes_llm, list_models};
use crate::model::{
    format_timestamp, MinutesDocument, MinutesOptions, RecordingOptions, TranscriptSegment,
    TranscriptionEngine,
};
use crate::state::{AppState, GLOBAL_STATE};
use crate::storage::{
    clear_google_oauth_token, export_minutes as export_minutes_to_disk,
    export_transcript_markdown as export_transcript_markdown_to_disk, load_google_oauth_token,
    load_snapshot as load_snapshot_from_disk, save_google_oauth_token,
    save_snapshot as save_snapshot_to_disk, GoogleOAuthToken,
};

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .expect("Failed to build HTTP client")
}); // 1時間

static GOOGLE_TOKEN_STATE: Lazy<Mutex<Option<GoogleOAuthToken>>> = Lazy::new(|| Mutex::new(None));

#[tauri::command]
async fn start_session(app: AppHandle, options: RecordingOptions) -> Result<String, String> {
    start_session_internal(app, GLOBAL_STATE.clone(), options)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn stop_session(app: AppHandle) -> Result<(), String> {
    stop_session_internal(app, GLOBAL_STATE.clone())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_segment(app: AppHandle, segment: TranscriptSegment) -> Result<(), String> {
    GLOBAL_STATE
        .update_segment(segment.clone())
        .ok_or_else(|| "Segment not found".to_string())
        .map(|segment| {
            let _ = app.emit("transcription:final", &segment);
        })
}

#[tauri::command]
fn finalize_segment(_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn assign_speaker(app: AppHandle, id: String, speaker: String) -> Result<(), String> {
    GLOBAL_STATE
        .assign_speaker(&id, &speaker)
        .ok_or_else(|| "Segment not found".to_string())
        .map(|segment| {
            let _ = app.emit("transcription:final", &segment);
        })
}

#[tauri::command]
async fn list_ollama_models() -> Result<Vec<String>, String> {
    list_models(&HTTP_CLIENT)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn generate_minutes(
    options: MinutesOptions,
    segments: Vec<TranscriptSegment>,
) -> Result<MinutesDocument, String> {
    eprintln!("=== generate_minutes START ===");
    eprintln!("Segments count: {}", segments.len());
    eprintln!("Options: {:?}", options);

    for (i, segment) in segments.iter().enumerate() {
        eprintln!(
            "Segment {}: id={}, speaker={}, text={}",
            i, segment.id, segment.speaker, segment.text
        );
    }

    // Skip setting minutes options in GLOBAL_STATE to avoid deadlock
    // GLOBAL_STATE.set_minutes_options(options.clone());
    eprintln!("Skipping GLOBAL_STATE update (avoiding deadlock)");

    eprintln!("Creating segment_records...");
    let segment_records: Vec<crate::state::SegmentRecord> = segments
        .into_iter()
        .map(|segment| crate::state::SegmentRecord {
            segment,
            embedding: None,
        })
        .collect();
    eprintln!("Segment records created: {} records", segment_records.len());

    eprintln!("Calling generate_minutes_llm...");
    let result = generate_minutes_llm(&HTTP_CLIENT, options, &segment_records).await;
    eprintln!("generate_minutes_llm returned: {:?}", result.is_ok());

    result.map_err(|error| {
        eprintln!("Error: {}", error);
        error.to_string()
    })
}

#[tauri::command]
fn export_minutes(document: MinutesDocument, directory: Option<String>) -> Result<String, String> {
    let dir = directory.map(PathBuf::from);
    export_minutes_to_disk(&document, dir)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn export_transcript_markdown(
    segments: Vec<TranscriptSegment>,
    path: Option<String>,
) -> Result<String, String> {
    let file_path = path.map(PathBuf::from);
    export_transcript_markdown_to_disk(&segments, file_path)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_snapshot(app: AppHandle, path: Option<String>) -> Result<String, String> {
    let snapshot = GLOBAL_STATE
        .snapshot()
        .ok_or_else(|| "No active session".to_string())?;
    let path = save_snapshot_to_disk(&app, snapshot, path.map(PathBuf::from))
        .map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn load_snapshot(path: String) -> Result<serde_json::Value, String> {
    load_snapshot_from_disk(PathBuf::from(path).as_path())
        .map(|snapshot| {
            json!({
                "segments": snapshot.segments,
                "id": snapshot.id,
                "startedAt": snapshot.started_at,
            })
        })
        .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleDocAppendPayload {
    segment_id: String,
    speaker: String,
    text: String,
    start: f32,
    end: f32,
    timestamp: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleAuthStatus {
    connected: bool,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

fn google_oauth_client_id() -> Result<String, String> {
    let client_id = option_env!("GOOGLE_OAUTH_CLIENT_ID")
        .unwrap_or("")
        .trim()
        .to_string();

    if client_id.is_empty() {
        return Err(
            "GOOGLE_OAUTH_CLIENT_ID is not configured. Set it in your build environment."
                .to_string(),
        );
    }

    Ok(client_id)
}

fn google_oauth_client_secret() -> Option<String> {
    let client_secret = option_env!("GOOGLE_OAUTH_CLIENT_SECRET")
        .unwrap_or("")
        .trim()
        .to_string();
    if client_secret.is_empty() {
        None
    } else {
        Some(client_secret)
    }
}

fn build_pkce_code_verifier() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn build_pkce_code_challenge(code_verifier: &str) -> String {
    let hash = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

#[tauri::command]
fn google_auth_status() -> Result<GoogleAuthStatus, String> {
    if GOOGLE_TOKEN_STATE.lock().is_none() {
        if let Some(token) = load_google_oauth_token().map_err(|error| error.to_string())? {
            *GOOGLE_TOKEN_STATE.lock() = Some(token);
        }
    }

    Ok(GoogleAuthStatus {
        connected: GOOGLE_TOKEN_STATE.lock().is_some(),
    })
}

#[tauri::command]
fn google_auth_disconnect() -> Result<(), String> {
    *GOOGLE_TOKEN_STATE.lock() = None;
    clear_google_oauth_token().map_err(|error| error.to_string())
}

#[tauri::command]
async fn google_auth_sign_in(app: AppHandle) -> Result<GoogleAuthStatus, String> {
    let client_id = google_oauth_client_id()?;
    let client_secret = google_oauth_client_secret();
    let state = Uuid::new_v4().to_string();
    let code_verifier = build_pkce_code_verifier();
    let code_challenge = build_pkce_code_challenge(&code_verifier);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| error.to_string())?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth2callback");

    let mut auth_url =
        Url::parse("https://accounts.google.com/o/oauth2/v2/auth").map_err(|e| e.to_string())?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "https://www.googleapis.com/auth/documents")
        .append_pair("access_type", "offline")
        .append_pair("include_granted_scopes", "true")
        .append_pair("prompt", "consent")
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256");

    app.opener()
        .open_url(auth_url.as_str(), None::<&str>)
        .map_err(|error| error.to_string())?;

    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(180), listener.accept())
        .await
        .map_err(|_| "Google sign-in timed out".to_string())?
        .map_err(|error| error.to_string())?;

    let mut buffer = [0u8; 4096];
    let read_len = stream
        .read(&mut buffer)
        .await
        .map_err(|error| error.to_string())?;
    if read_len == 0 {
        return Err("No OAuth callback payload received".to_string());
    }

    let request = String::from_utf8_lossy(&buffer[..read_len]).to_string();
    let request_line = request
        .lines()
        .next()
        .ok_or_else(|| "Invalid OAuth callback request".to_string())?;
    let mut parts = request_line.split_whitespace();
    let _method = parts.next();
    let path_and_query = parts
        .next()
        .ok_or_else(|| "Invalid OAuth callback path".to_string())?;

    let callback_url =
        Url::parse(&format!("http://localhost{path_and_query}")).map_err(|e| e.to_string())?;
    let code = callback_url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| "OAuth authorization code is missing".to_string())?;
    let returned_state = callback_url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| "OAuth state is missing".to_string())?;

    if returned_state != state {
        return Err("OAuth state validation failed".to_string());
    }

    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("client_id", client_id.clone()),
        ("redirect_uri", redirect_uri.clone()),
        ("code", code),
        ("code_verifier", code_verifier.clone()),
    ];
    if let Some(secret) = client_secret {
        form.push(("client_secret", secret));
    }

    let token_response = HTTP_CLIENT
        .post("https://oauth2.googleapis.com/token")
        .form(&form)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let body = token_response.text().await.unwrap_or_default();
        let _ = stream
            .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/html\r\n\r\nGoogle authentication failed. You can close this window.")
            .await;
        return Err(format!("Google token exchange failed ({status}): {body}"));
    }

    let token: GoogleTokenResponse = token_response
        .json()
        .await
        .map_err(|error| error.to_string())?;

    let refresh_token = token
        .refresh_token
        .ok_or_else(|| "Google refresh token was not returned".to_string())?;

    let stored_token = GoogleOAuthToken {
        access_token: token.access_token,
        refresh_token,
        expires_at: chrono::Utc::now().timestamp() + token.expires_in - 60,
    };

    save_google_oauth_token(&stored_token).map_err(|error| error.to_string())?;
    *GOOGLE_TOKEN_STATE.lock() = Some(stored_token);

    let _ = stream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\nGoogle authentication succeeded. You can close this window.",
        )
        .await;

    Ok(GoogleAuthStatus { connected: true })
}

async fn get_valid_google_access_token() -> Result<String, String> {
    if GOOGLE_TOKEN_STATE.lock().is_none() {
        if let Some(token) = load_google_oauth_token().map_err(|error| error.to_string())? {
            *GOOGLE_TOKEN_STATE.lock() = Some(token);
        }
    }

    let token = GOOGLE_TOKEN_STATE
        .lock()
        .clone()
        .ok_or_else(|| "Google account is not connected".to_string())?;

    if token.expires_at > chrono::Utc::now().timestamp() {
        return Ok(token.access_token);
    }

    let client_id = google_oauth_client_id()?;
    let client_secret = google_oauth_client_secret();
    let mut form = vec![
        ("grant_type", "refresh_token".to_string()),
        ("client_id", client_id),
        ("refresh_token", token.refresh_token.clone()),
    ];
    if let Some(secret) = client_secret {
        form.push(("client_secret", secret));
    }

    let response = HTTP_CLIENT
        .post("https://oauth2.googleapis.com/token")
        .form(&form)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        *GOOGLE_TOKEN_STATE.lock() = None;
        let _ = clear_google_oauth_token();
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Google token refresh failed ({status}): {body}"));
    }

    let refreshed: GoogleTokenResponse = response.json().await.map_err(|e| e.to_string())?;
    let updated = GoogleOAuthToken {
        access_token: refreshed.access_token,
        refresh_token: token.refresh_token,
        expires_at: chrono::Utc::now().timestamp() + refreshed.expires_in - 60,
    };

    save_google_oauth_token(&updated).map_err(|error| error.to_string())?;
    *GOOGLE_TOKEN_STATE.lock() = Some(updated.clone());
    Ok(updated.access_token)
}

#[tauri::command]
async fn append_google_doc_transcript(
    document_id: String,
    payload: GoogleDocAppendPayload,
) -> Result<(), String> {
    let access_token = get_valid_google_access_token().await?;
    let line = format!(
        "[{} - {}] {}: {}\n",
        format_timestamp(payload.start),
        format_timestamp(payload.end),
        payload.speaker,
        payload.text
    );

    let response = HTTP_CLIENT
        .post(format!(
            "https://docs.googleapis.com/v1/documents/{document_id}:batchUpdate"
        ))
        .bearer_auth(access_token)
        .json(&json!({
            "requests": [
                {
                    "insertText": {
                        "endOfSegmentLocation": {},
                        "text": line,
                    }
                }
            ]
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Google Docs append failed ({status}): {body}"));
    }

    Ok(())
}

async fn start_session_internal(
    app: AppHandle,
    state: AppState,
    mut options: RecordingOptions,
) -> anyhow::Result<String> {
    normalize_recording_paths(&mut options)?;
    let session_id = state.start_session(options.clone());
    match audio::start_transcription(app.clone(), state.clone(), options) {
        Ok(runtime) => {
            state.set_runtime(runtime);
            Ok(session_id)
        }
        Err(error) => {
            state.stop_session();
            Err(error)
        }
    }
}

async fn stop_session_internal(app: AppHandle, state: AppState) -> anyhow::Result<()> {
    state.stop_session();
    let _ = app.emit("transcription:ended", &serde_json::json!({}));
    Ok(())
}

fn normalize_recording_paths(options: &mut RecordingOptions) -> anyhow::Result<()> {
    match options.engine {
        TranscriptionEngine::Vosk => {
            options.model_path = normalize_path(&options.model_path).with_context(|| {
                format!("Failed to resolve Vosk model path: {}", options.model_path)
            })?;

            if let Some(path) = options.speaker_model_path.as_mut() {
                *path = normalize_path(path)
                    .with_context(|| format!("Failed to resolve speaker model path: {path}"))?;
            }
        }
        TranscriptionEngine::Whisper => {
            let requested_model_path = options.whisper_model_path.trim().to_string();
            if requested_model_path.is_empty() {
                return Err(anyhow!("Whisper model path must not be empty"));
            }

            let model_name = PathBuf::from(&requested_model_path)
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string);

            options.whisper_model_path = match normalize_path(&requested_model_path) {
                Ok(path) => path,
                Err(resolve_error) => {
                    if let Some(found) = resolve_known_whisper_model_path(model_name.as_deref()) {
                        found
                    } else {
                        return Err(resolve_error).with_context(|| {
                            format!("Failed to resolve Whisper model path: {requested_model_path}")
                        });
                    }
                }
            };

            if options.whisper_command.trim().is_empty() {
                return Err(anyhow!("Whisper command must not be empty"));
            }

            if options.whisper_command.contains('/') || options.whisper_command.starts_with('~') {
                options.whisper_command =
                    normalize_path(&options.whisper_command).with_context(|| {
                        format!(
                            "Failed to resolve Whisper command path: {}",
                            options.whisper_command
                        )
                    })?;
            } else if let Some(found) = resolve_command_path(&options.whisper_command) {
                options.whisper_command = found;
            } else if let Some(found) = resolve_known_whisper_command_path(&options.whisper_command)
            {
                options.whisper_command = found;
            } else {
                return Err(anyhow!(
                    "Whisper command '{}' was not found. Set an absolute path in settings.",
                    options.whisper_command
                ));
            }
        }
    }

    Ok(())
}

fn normalize_path(input: &str) -> anyhow::Result<String> {
    if input.trim().is_empty() {
        return Err(anyhow!("path must not be empty"));
    }

    let expanded = if let Some(stripped) = input.strip_prefix("~/") {
        let home = home_dir().ok_or_else(|| anyhow!("unable to determine home directory"))?;
        home.join(stripped)
    } else if input == "~" {
        home_dir().ok_or_else(|| anyhow!("unable to determine home directory"))?
    } else {
        PathBuf::from(input)
    };

    if expanded.exists() {
        let canonical = std::fs::canonicalize(&expanded).unwrap_or_else(|_| expanded.clone());
        Ok(canonical.to_string_lossy().into())
    } else {
        Err(anyhow!("path does not exist: {}", expanded.display()))
    }
}

fn home_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf())
}

fn resolve_command_path(command: &str) -> Option<String> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn resolve_known_whisper_command_path(command_name: &str) -> Option<String> {
    let mut names = Vec::new();
    let trimmed = command_name.trim();
    if !trimmed.is_empty() {
        names.push(trimmed.to_string());
    }
    if !names.iter().any(|name| name == "whisper-cli") {
        names.push("whisper-cli".to_string());
    }
    if !names.iter().any(|name| name == "main") {
        names.push("main".to_string());
    }

    let mut candidates = Vec::new();
    for name in &names {
        candidates.push(PathBuf::from("/opt/homebrew/bin").join(name));
        candidates.push(PathBuf::from("/usr/local/bin").join(name));
        candidates.push(PathBuf::from("/usr/bin").join(name));
    }

    if let Ok(cwd) = std::env::current_dir() {
        for name in &names {
            candidates.push(cwd.join("tools/whisper.cpp/build/bin").join(name));
        }
    }

    if let Some(home) = home_dir() {
        for name in &names {
            candidates.push(home.join("whisper.cpp/build/bin").join(name));
            candidates.push(
                home.join("MemoBreeze/tools/whisper.cpp/build/bin")
                    .join(name),
            );
            candidates.push(
                home.join("Desktop/MemoBreeze/tools/whisper.cpp/build/bin")
                    .join(name),
            );
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        for ancestor in exe_path.ancestors() {
            for name in &names {
                candidates.push(ancestor.join("tools/whisper.cpp/build/bin").join(name));
            }
        }
    }

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

fn resolve_known_whisper_model_path(model_name: Option<&str>) -> Option<String> {
    let mut names = Vec::new();
    if let Some(name) = model_name.filter(|name| !name.trim().is_empty()) {
        names.push(name.to_string());
    }
    if !names.iter().any(|name| name == "ggml-base.bin") {
        names.push("ggml-base.bin".to_string());
    }

    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        for name in &names {
            candidates.push(cwd.join("tools/whisper.cpp/models").join(name));
        }
    }

    if let Some(home) = home_dir() {
        for name in &names {
            candidates.push(home.join("whisper.cpp/models").join(name));
            candidates.push(home.join("MemoBreeze/tools/whisper.cpp/models").join(name));
            candidates.push(
                home.join("Desktop/MemoBreeze/tools/whisper.cpp/models")
                    .join(name),
            );
        }
    }

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

fn register_shortcuts(app: &tauri::App) -> tauri::Result<()> {
    let shared_state = GLOBAL_STATE.clone();
    app.global_shortcut()
        .on_shortcut("ctrl+space", move |app_handle, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }

            let state = shared_state.clone();
            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if state.is_active() {
                    if let Err(error) = stop_session_internal(app.clone(), state.clone()).await {
                        tracing::error!(?error, "Failed to stop session via shortcut");
                    }
                } else if let Some(options) = state.recording_options() {
                    if let Err(error) =
                        start_session_internal(app.clone(), state.clone(), options).await
                    {
                        tracing::error!(?error, "Failed to start session via shortcut");
                    }
                }
            });
        })
        .map_err(|error| {
            tauri::Error::PluginInitialization("global-shortcut".to_string(), error.to_string())
        })?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .setup(|app| {
            permissions::initialize(app);
            register_shortcuts(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            stop_session,
            update_segment,
            finalize_segment,
            assign_speaker,
            list_ollama_models,
            generate_minutes,
            export_minutes,
            export_transcript_markdown,
            google_auth_sign_in,
            google_auth_status,
            google_auth_disconnect,
            append_google_doc_transcript,
            save_snapshot,
            load_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
