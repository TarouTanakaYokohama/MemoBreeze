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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MinutesSection, TopicSummary, TranscriptToken};
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    fn sample_minutes_document() -> MinutesDocument {
        MinutesDocument {
            preset: "default".to_string(),
            format: "meeting".to_string(),
            model: "llama3".to_string(),
            generated_at: Utc.with_ymd_and_hms(2026, 2, 1, 10, 0, 0).unwrap(),
            summary: MinutesSection {
                title: "Summary".to_string(),
                content: "Summary content".to_string(),
            },
            decisions: MinutesSection {
                title: "Decisions".to_string(),
                content: "Decision content".to_string(),
            },
            actions: MinutesSection {
                title: "Actions".to_string(),
                content: "Action content".to_string(),
            },
            timeline: vec![TopicSummary {
                id: "topic-1".to_string(),
                title: "Topic".to_string(),
                description: "Description".to_string(),
                start: 0.0,
                end: 60.0,
                markers: Vec::new(),
            }],
            highlights: None,
            blockers: None,
        }
    }

    fn segment(id: &str, start: f32, end: f32, speaker: &str, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            speaker: speaker.to_string(),
            text: text.to_string(),
            start,
            end,
            tokens: vec![TranscriptToken {
                text: text.to_string(),
                start,
                end,
                confidence: 0.8,
            }],
            is_final: true,
        }
    }

    #[test]
    fn export_minutes_writes_markdown_to_explicit_path() {
        let tmp = tempdir().unwrap();
        let out_path = tmp.path().join("nested/minutes.md");
        let doc = sample_minutes_document();

        let written = export_minutes(&doc, Some(out_path.clone())).unwrap();
        let content = fs::read_to_string(&written).unwrap();

        assert_eq!(written, out_path);
        assert!(content.contains("# Meeting Minutes"));
        assert!(content.contains("## Summary"));
        assert!(content.contains("Summary content"));
    }

    #[test]
    fn export_transcript_markdown_sorts_segments_by_start() {
        let tmp = tempdir().unwrap();
        let out_path = tmp.path().join("transcript.md");
        let segments = vec![
            segment("b", 30.0, 31.0, "Speaker 2", "  second  "),
            segment("a", 10.0, 11.0, "Speaker 1", " first "),
        ];

        export_transcript_markdown(&segments, Some(out_path.clone())).unwrap();
        let content = fs::read_to_string(out_path).unwrap();

        let first_idx = content.find("Speaker 1").unwrap();
        let second_idx = content.find("Speaker 2").unwrap();
        assert!(first_idx < second_idx);
        assert!(content.contains("**Speaker 1**: first"));
        assert!(content.contains("**Speaker 2**: second"));
    }

    #[test]
    fn load_snapshot_parses_serialized_snapshot() {
        let tmp = tempdir().unwrap();
        let snapshot_path = tmp.path().join("snapshot.json");
        let snapshot = SessionSnapshot {
            id: "session-1".to_string(),
            started_at: Utc.with_ymd_and_hms(2026, 2, 1, 11, 0, 0).unwrap(),
            segments: vec![segment("a", 0.0, 1.0, "Speaker 1", "hello")],
        };

        fs::write(
            &snapshot_path,
            serde_json::to_string_pretty(&snapshot).unwrap(),
        )
        .unwrap();

        let loaded = load_snapshot(&snapshot_path).unwrap();
        assert_eq!(loaded.id, "session-1");
        assert_eq!(loaded.segments.len(), 1);
        assert_eq!(loaded.segments[0].text, "hello");
    }
}
