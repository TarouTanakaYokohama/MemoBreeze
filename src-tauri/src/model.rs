use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptToken {
    pub text: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    pub speaker: String,
    pub text: String,
    pub start: f32,
    pub end: f32,
    #[serde(default)]
    pub tokens: Vec<TranscriptToken>,
    #[serde(default)]
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMarker {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub timestamp: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub start: f32,
    pub end: f32,
    #[serde(default)]
    pub markers: Vec<TimelineMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinutesSection {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinutesDocument {
    pub preset: String,
    pub format: String,
    pub model: String,
    pub generated_at: DateTime<Utc>,
    pub summary: MinutesSection,
    pub decisions: MinutesSection,
    pub actions: MinutesSection,
    #[serde(default)]
    pub timeline: Vec<TopicSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlights: Option<MinutesSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockers: Option<MinutesSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionEngine {
    Vosk,
    #[default]
    Whisper,
}

fn default_whisper_command() -> String {
    "whisper-cli".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingOptions {
    #[serde(default)]
    pub engine: TranscriptionEngine,
    pub model_path: String,
    #[serde(default)]
    pub speaker_model_path: Option<String>,
    #[serde(default)]
    pub whisper_model_path: String,
    #[serde(default)]
    pub whisper_language: Option<String>,
    #[serde(default = "default_whisper_command")]
    pub whisper_command: String,
    pub enable_input: bool,
    pub enable_output: bool,
    #[serde(default = "default_energy_threshold")]
    pub energy_threshold: f32,
}

const fn default_energy_threshold() -> f32 {
    0.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinutesOptions {
    pub preset: String,
    pub format: String,
    pub block_size_minutes: u32,
    pub model: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub segments: Vec<TranscriptSegment>,
}

impl MinutesDocument {
    pub fn as_markdown(&self) -> String {
        let summary_content = normalize_section_content(&self.summary.content);
        let decisions_content = normalize_section_content(&self.decisions.content);
        let actions_content = normalize_section_content(&self.actions.content);

        let mut lines = vec![
            format!("# Meeting Minutes ({})", self.generated_at),
            String::new(),
            format!("## Summary\n{}", summary_content),
            String::new(),
            format!("## 決定事項 / Decisions\n{}", decisions_content),
            String::new(),
            format!("## アクション / Action Items\n{}", actions_content),
            String::new(),
            "## Timeline".to_string(),
        ];

        for (index, topic) in self.timeline.iter().enumerate() {
            lines.push(format!(
                "### Block {} ({} - {})",
                index + 1,
                format_timestamp(topic.start),
                format_timestamp(topic.end)
            ));
            lines.push(topic.description.clone());
            if !topic.markers.is_empty() {
                lines.push("- Markers:".to_string());
                for marker in &topic.markers {
                    lines.push(format!(
                        "  - [{}] {} @ {}",
                        marker.kind,
                        marker.label,
                        format_timestamp(marker.timestamp)
                    ));
                }
            }
            lines.push(String::new());
        }

        if let Some(highlights) = &self.highlights {
            lines.push(format!("## Highlights\n{}", highlights.content));
        }

        if let Some(blockers) = &self.blockers {
            lines.push(format!("## Blockers\n{}", blockers.content));
        }

        lines.join("\n")
    }
}

fn normalize_section_content(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        "- (抽出なし)".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn format_timestamp(seconds: f32) -> String {
    let seconds = seconds.max(0.0);
    let minutes = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{:02}:{:02}", minutes, secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    fn sample_document() -> MinutesDocument {
        MinutesDocument {
            preset: "default".to_string(),
            format: "meeting".to_string(),
            model: "llama3".to_string(),
            generated_at: Utc.with_ymd_and_hms(2026, 2, 1, 10, 0, 0).unwrap(),
            summary: MinutesSection {
                title: "Summary".to_string(),
                content: "Summary body".to_string(),
            },
            decisions: MinutesSection {
                title: "Decisions".to_string(),
                content: "Decision A".to_string(),
            },
            actions: MinutesSection {
                title: "Actions".to_string(),
                content: "Action A".to_string(),
            },
            timeline: Vec::new(),
            highlights: None,
            blockers: None,
        }
    }

    #[test]
    fn format_timestamp_clamps_negative_values() {
        assert_eq!(format_timestamp(-9.2), "00:00");
    }

    #[test]
    fn format_timestamp_formats_minutes_and_seconds() {
        assert_eq!(format_timestamp(125.9), "02:05");
    }

    #[test]
    fn as_markdown_uses_fallback_for_empty_sections() {
        let mut doc = sample_document();
        doc.summary.content = "   ".to_string();
        doc.decisions.content = "\n".to_string();
        doc.actions.content = "".to_string();

        let markdown = doc.as_markdown();
        assert!(markdown.contains("## Summary\n- (抽出なし)"));
        assert!(markdown.contains("## 決定事項 / Decisions\n- (抽出なし)"));
        assert!(markdown.contains("## アクション / Action Items\n- (抽出なし)"));
    }

    #[test]
    fn as_markdown_renders_timeline_and_markers() {
        let mut doc = sample_document();
        doc.timeline = vec![TopicSummary {
            id: "topic-1".to_string(),
            title: "Topic".to_string(),
            description: "Discussed project status".to_string(),
            start: 30.0,
            end: 95.0,
            markers: vec![TimelineMarker {
                id: "marker-1".to_string(),
                label: "Decision".to_string(),
                kind: "decision".to_string(),
                timestamp: 63.0,
            }],
        }];

        let markdown = doc.as_markdown();
        assert!(markdown.contains("### Block 1 (00:30 - 01:35)"));
        assert!(markdown.contains("Discussed project status"));
        assert!(markdown.contains("- Markers:"));
        assert!(markdown.contains("[decision] Decision @ 01:03"));
    }

    #[test]
    fn as_markdown_includes_optional_sections() {
        let mut doc = sample_document();
        doc.highlights = Some(MinutesSection {
            title: "Highlights".to_string(),
            content: "Shipped feature X".to_string(),
        });
        doc.blockers = Some(MinutesSection {
            title: "Blockers".to_string(),
            content: "Need infra access".to_string(),
        });

        let markdown = doc.as_markdown();
        assert!(markdown.contains("## Highlights\nShipped feature X"));
        assert!(markdown.contains("## Blockers\nNeed infra access"));
    }

    proptest! {
        #[test]
        fn format_timestamp_returns_mm_ss_shape(seconds in -5000.0f32..5000.0f32) {
            let out = format_timestamp(seconds);
            prop_assert_eq!(out.len(), 5);
            prop_assert_eq!(&out[2..3], ":");
        }
    }
}
