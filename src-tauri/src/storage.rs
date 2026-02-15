use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::model::{format_timestamp, MinutesDocument, SessionSnapshot, TranscriptSegment};

fn ensure_app_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "MemoBreeze", "MemoBreeze")
        .context("Unable to determine project directory")?;
    let data_dir = dirs.data_dir();
    fs::create_dir_all(data_dir)?;
    Ok(data_dir.to_path_buf())
}

pub fn save_snapshot(
    app: &AppHandle,
    snapshot: SessionSnapshot,
    path: Option<PathBuf>,
) -> Result<PathBuf> {
    let dir = ensure_app_dir()?;
    let file_path = path.unwrap_or_else(|| dir.join(format!("session-{}.json", snapshot.id)));
    let json = serde_json::to_string_pretty(&snapshot)?;
    fs::write(&file_path, json)?;
    let _ = app.emit("storage:snapshot", json!({ "path": file_path }));
    Ok(file_path)
}

pub fn load_snapshot(path: &Path) -> Result<SessionSnapshot> {
    let raw = fs::read_to_string(path)?;
    let snapshot: SessionSnapshot = serde_json::from_str(&raw)?;
    Ok(snapshot)
}

pub fn export_minutes(document: &MinutesDocument, file_path: Option<PathBuf>) -> Result<PathBuf> {
    let path = if let Some(path) = file_path {
        // ユーザーが指定したパスを使用
        path
    } else {
        // デフォルトパスを使用
        let dir = ensure_app_dir()?;
        let filename = format!(
            "minutes-{}.md",
            document.generated_at.format("%Y%m%d-%H%M%S")
        );
        dir.join(filename)
    };

    // 親ディレクトリが存在することを確認
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, document.as_markdown())?;
    Ok(path)
}

pub fn export_transcript_markdown(
    segments: &[TranscriptSegment],
    file_path: Option<PathBuf>,
) -> Result<PathBuf> {
    let path = if let Some(path) = file_path {
        path
    } else {
        let dir = ensure_app_dir()?;
        let filename = format!("transcript-{}.md", Utc::now().format("%Y%m%d-%H%M%S"));
        dir.join(filename)
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut sorted_segments = segments.to_vec();
    sorted_segments.sort_by(|a, b| a.start.total_cmp(&b.start));

    let mut lines = vec![
        format!("# Transcript ({})", Utc::now().to_rfc3339()),
        String::new(),
    ];

    for segment in sorted_segments {
        lines.push(format!(
            "- [{} - {}] **{}**: {}",
            format_timestamp(segment.start),
            format_timestamp(segment.end),
            segment.speaker,
            segment.text.trim()
        ));
    }

    lines.push(String::new());
    fs::write(&path, lines.join("\n"))?;
    Ok(path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleOAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

pub fn save_google_oauth_token(token: &GoogleOAuthToken) -> Result<()> {
    let dir = ensure_app_dir()?;
    let path = dir.join("google-oauth-token.json");
    let raw = serde_json::to_string_pretty(token)?;
    fs::write(path, raw)?;
    Ok(())
}

pub fn load_google_oauth_token() -> Result<Option<GoogleOAuthToken>> {
    let dir = ensure_app_dir()?;
    let path = dir.join("google-oauth-token.json");

    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(path)?;
    let token: GoogleOAuthToken = serde_json::from_str(&raw)?;
    Ok(Some(token))
}

pub fn clear_google_oauth_token() -> Result<()> {
    let dir = ensure_app_dir()?;
    let path = dir.join("google-oauth-token.json");
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
