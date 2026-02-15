use std::collections::HashSet;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::model::{MinutesDocument, MinutesOptions, MinutesSection, TimelineMarker, TopicSummary};
use crate::state::SegmentRecord;

const OLLAMA_URL: &str = "http://127.0.0.1:11434";

#[derive(Deserialize)]
struct OllamaTagResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

pub async fn list_models(client: &Client) -> Result<Vec<String>> {
    let url = format!("{OLLAMA_URL}/api/tags");
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to contact Ollama daemon")?;

    if !response.status().is_success() {
        bail!("Failed to list models: {}", response.status());
    }

    let payload = response.json::<OllamaTagResponse>().await?;
    Ok(payload.models.into_iter().map(|model| model.name).collect())
}

pub async fn generate_minutes(
    client: &Client,
    options: MinutesOptions,
    segments: &[SegmentRecord],
) -> Result<MinutesDocument> {
    if segments.is_empty() {
        bail!("No transcript segments are available. Record audio before generating minutes.");
    }

    let prompt = build_prompt(&options, segments)?;

    let url = format!("{OLLAMA_URL}/api/generate");
    let body = json!({
        "model": options.model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": options.temperature,
        }
    });

    info!(
        model = %options.model,
        segments = segments.len(),
        "Requesting minutes generation from Ollama"
    );

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            if error.is_connect() {
                anyhow!("Failed to connect to Ollama at {OLLAMA_URL}. Is `ollama serve` running?")
            } else if error.is_timeout() {
                anyhow!("Request to Ollama timed out after 600 seconds. The model may be too slow or stuck.")
            } else {
                anyhow!("Request failed: {}", error)
            }
        })
        .context("Failed to invoke Ollama generate endpoint")?;

    let status = response.status();

    if !status.is_success() {
        if status == StatusCode::NOT_FOUND {
            bail!(
                "Model `{}` was not found on the Ollama server. Run `ollama pull {}` before generating minutes.",
                options.model,
                options.model
            );
        }

        let detail = response.text().await.unwrap_or_default();
        let snippet = summarize_body(&detail);
        bail!(
            "Minutes generation failed with status {}{}",
            status,
            if snippet.is_empty() {
                String::new()
            } else {
                format!(": {snippet}")
            }
        );
    }

    let payload = response.json::<OllamaGenerateResponse>().await?;
    let response_text = payload.response;

    match extract_json_value(&response_text) {
        Ok(json) => map_minutes(options, json, segments),
        Err(error) => {
            warn!(
                ?error,
                "LLM response was not valid JSON; using fallback minutes"
            );
            Ok(fallback_minutes(options, response_text))
        }
    }
}

fn build_prompt(options: &MinutesOptions, segments: &[SegmentRecord]) -> Result<String> {
    let template = match options.preset.as_str() {
        "detailed" => include_str!("../templates/detailed_minutes_prompt.txt"),
        _ => include_str!("../templates/default_minutes_prompt.txt"),
    };

    let mut transcript_lines = Vec::with_capacity(segments.len());
    for record in segments {
        let segment = &record.segment;
        transcript_lines.push(format!(
            "[{start}-{end}] {speaker}: {text}",
            start = crate::model::format_timestamp(segment.start),
            end = crate::model::format_timestamp(segment.end),
            speaker = segment.speaker,
            text = segment.text.trim()
        ));
    }

    let transcript_block = transcript_lines.join("\n");

    Ok(format!(
        "{template}\n\n<transcript>\n{transcript}\n</transcript>\n\nReturn JSON only.",
        template = template,
        transcript = transcript_block
    ))
}

fn extract_json_value(response: &str) -> anyhow::Result<Value> {
    let trimmed = response.trim();

    if let Some(value) = deserialize_first_json_value(trimmed) {
        return Ok(value);
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }

    if let Some(value) = extract_from_code_blocks(trimmed) {
        return Ok(value);
    }

    if let Some(value) = extract_from_enclosed(trimmed, '{', '}') {
        return Ok(value);
    }

    if let Some(value) = extract_from_enclosed(trimmed, '[', ']') {
        return Ok(value);
    }

    let snippet = trimmed.lines().take(4).collect::<Vec<_>>().join("\n");
    bail!("Unable to parse JSON from response: {snippet}");
}

fn deserialize_first_json_value(text: &str) -> Option<Value> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    Value::deserialize(&mut deserializer).ok()
}

fn extract_from_code_blocks(text: &str) -> Option<Value> {
    let mut rest = text;

    while let Some(start) = rest.find("```") {
        let after_start = &rest[start + 3..];
        let Some(end) = after_start.find("```") else {
            break;
        };

        let block = &after_start[..end];
        let block_trimmed = block.trim();

        if let Some(value) = deserialize_first_json_value(block_trimmed) {
            return Some(value);
        }

        let mut lines = block_trimmed.lines();
        if let Some(first_line) = lines.next() {
            if first_line.trim().eq_ignore_ascii_case("json") {
                let content = lines.collect::<Vec<_>>().join("\n");
                let content_trimmed = content.trim();
                if let Some(value) = deserialize_first_json_value(content_trimmed) {
                    return Some(value);
                }
            }
        }

        rest = &after_start[end + 3..];
    }

    None
}

fn extract_from_enclosed(text: &str, open: char, close: char) -> Option<Value> {
    let mut search_start = 0;

    while search_start < text.len() {
        let tail = &text[search_start..];
        let Some(pos) = tail.find(open) else {
            break;
        };
        let start_idx = search_start + pos;
        let mut depth = 0;
        let mut in_string = false;
        let mut escape = false;
        let mut end_idx = None;

        for (offset, ch) in text[start_idx..].char_indices() {
            if in_string {
                if escape {
                    escape = false;
                } else if ch == '\\' {
                    escape = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }

            match ch {
                '"' => in_string = true,
                c if c == open => depth += 1,
                c if c == close => {
                    depth -= 1;
                    if depth == 0 {
                        end_idx = Some(start_idx + offset + ch.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end) = end_idx {
            let candidate = text[start_idx..end].trim();
            if let Some(value) = deserialize_first_json_value(candidate) {
                return Some(value);
            }
        } else {
            break;
        }

        search_start = start_idx + 1;
    }

    None
}

fn fallback_minutes(options: MinutesOptions, response: String) -> MinutesDocument {
    let summary_content = response.trim().to_string();

    MinutesDocument {
        preset: options.preset,
        format: options.format,
        model: options.model,
        generated_at: Utc::now(),
        summary: MinutesSection {
            title: "Summary".to_string(),
            content: if summary_content.is_empty() {
                "No summary available.".to_string()
            } else {
                summary_content
            },
        },
        decisions: MinutesSection {
            title: "Decisions".to_string(),
            content: String::new(),
        },
        actions: MinutesSection {
            title: "Action Items".to_string(),
            content: String::new(),
        },
        highlights: None,
        blockers: None,
        timeline: Vec::new(),
    }
}

fn summarize_body(body: &str) -> String {
    let snippet = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    snippet.trim().chars().take(160).collect::<String>()
}

fn map_minutes(
    options: MinutesOptions,
    payload: Value,
    segments: &[SegmentRecord],
) -> Result<MinutesDocument> {
    let summary = section_from_value(
        get_by_aliases(&payload, &["summary", "overview", "要約"]),
        "Summary",
    );
    let mut decisions = section_from_value(
        get_by_aliases(
            &payload,
            &["decisions", "decision", "decisionItems", "決定事項"],
        ),
        "Decisions",
    );
    let mut actions = section_from_value(
        get_by_aliases(
            &payload,
            &[
                "actions",
                "actionItems",
                "action_items",
                "tasks",
                "アクション",
            ],
        ),
        "Action Items",
    );
    let highlights = get_by_aliases(&payload, &["highlights"])
        .map(|value| section_from_value(Some(value), "Highlights"));
    let blockers = get_by_aliases(&payload, &["blockers", "risks"])
        .map(|value| section_from_value(Some(value), "Blockers"));

    let inferred_actions = infer_actions_from_segments(segments);
    let actions_low_quality =
        is_placeholder_or_empty(&actions.content) || is_json_object_list(&actions.content);
    if actions_low_quality && !inferred_actions.is_empty() {
        actions.content = inferred_actions
            .iter()
            .map(format_inferred_action_bullet)
            .collect::<Vec<_>>()
            .join("\n");
    } else {
        actions.content = normalize_json_bullets(&actions.content);
    }

    if is_placeholder_or_empty(&decisions.content) && !inferred_actions.is_empty() {
        decisions.content = inferred_actions
            .iter()
            .map(|item| format!("- {}", item.description))
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
    }

    let timeline = build_timeline(get_by_aliases(&payload, &["timeline", "topics"]))?;

    Ok(MinutesDocument {
        preset: options.preset,
        format: options.format,
        model: options.model,
        generated_at: Utc::now(),
        summary,
        decisions,
        actions,
        highlights,
        blockers,
        timeline,
    })
}

fn get_by_aliases<'a>(payload: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| payload.get(*key))
}

#[derive(Clone)]
struct InferredAction {
    description: String,
    owner: Option<String>,
    due: Option<String>,
}

fn infer_actions_from_segments(segments: &[SegmentRecord]) -> Vec<InferredAction> {
    let mut inferred = Vec::new();
    let mut seen = HashSet::new();

    for (index, record) in segments.iter().enumerate() {
        let current = sanitize_text(record.segment.text.trim());
        if current.is_empty() {
            continue;
        }

        let mut candidate = current.clone();
        if let Some(previous) = index
            .checked_sub(1)
            .and_then(|i| segments.get(i))
            .map(|segment| sanitize_text(segment.segment.text.trim()))
        {
            if looks_like_due_prefix(&previous) {
                candidate = format!("{previous} {candidate}");
            }
        }

        if !looks_like_action_statement(&candidate) {
            continue;
        }

        let normalized = candidate.trim_end_matches('。').trim().to_string();
        if !seen.insert(normalized.clone()) {
            continue;
        }

        let owner = normalize_owner(&record.segment.speaker);
        let due = detect_due_phrase(&normalized);
        inferred.push(InferredAction {
            description: normalized,
            owner,
            due,
        });
    }

    inferred
}

fn sanitize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn looks_like_due_prefix(text: &str) -> bool {
    let short = text.trim();
    if short.is_empty() || short.len() > 10 {
        return false;
    }
    ["今日", "明日", "明後日", "今週", "来週", "今月", "来月"]
        .iter()
        .any(|token| short.contains(token))
        || short.ends_with('は')
}

fn looks_like_action_statement(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.ends_with('?') || trimmed.ends_with('？') {
        return false;
    }

    let ja_patterns = [
        "します",
        "する",
        "行きます",
        "買います",
        "買いに行きます",
        "やります",
        "進めます",
        "予定",
        "ことにします",
        "対応します",
        "実施します",
    ];
    if ja_patterns.iter().any(|pattern| trimmed.contains(pattern)) {
        return true;
    }

    let lower = trimmed.to_lowercase();
    let en_patterns = [
        " will ",
        "going to",
        "plan to",
        "plans to",
        "decided to",
        "action item",
    ];
    en_patterns.iter().any(|pattern| lower.contains(pattern))
}

fn detect_due_phrase(text: &str) -> Option<String> {
    let due_tokens = ["明後日", "明日", "今日", "来週", "今週", "来月", "今月"];
    due_tokens
        .iter()
        .find(|token| text.contains(**token))
        .map(|token| (*token).to_string())
}

fn normalize_owner(owner: &str) -> Option<String> {
    let normalized = owner.trim();
    if normalized.is_empty()
        || normalized.eq_ignore_ascii_case("unknown")
        || normalized.eq_ignore_ascii_case("speaker unknown")
    {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn format_inferred_action_bullet(action: &InferredAction) -> String {
    let mut details = Vec::new();
    details.push(format!(
        "担当: {}",
        action.owner.clone().unwrap_or_else(|| "不明".to_string())
    ));
    if let Some(due) = &action.due {
        details.push(format!("期限: {due}"));
    }

    format!("- {}（{}）", action.description, details.join(" / "))
}

fn is_placeholder_or_empty(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return true;
    }

    let lowered = trimmed.to_lowercase();
    lowered.contains("none identified")
        || lowered.contains("not identified")
        || trimmed.contains("該当なし")
        || trimmed.contains("抽出できる項目はありませんでした")
}

fn is_json_object_list(content: &str) -> bool {
    let lines: Vec<&str> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return false;
    }

    lines.iter().all(|line| {
        if let Some(rest) = line.strip_prefix("- ") {
            rest.starts_with('{') && rest.ends_with('}')
        } else {
            false
        }
    })
}

fn normalize_json_bullets(content: &str) -> String {
    let mut normalized = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("- ") {
            if rest.starts_with('{') && rest.ends_with('}') {
                if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(rest) {
                    let description = [
                        "item",
                        "task",
                        "action",
                        "title",
                        "description",
                        "content",
                        "text",
                    ]
                    .iter()
                    .find_map(|key| map.get(*key).and_then(Value::as_str))
                    .unwrap_or("実施項目未特定")
                    .trim()
                    .to_string();

                    let owner = map
                        .get("owner")
                        .or_else(|| map.get("assignee"))
                        .and_then(Value::as_str)
                        .unwrap_or("不明")
                        .trim()
                        .to_string();
                    let due = map
                        .get("dueDate")
                        .or_else(|| map.get("due_date"))
                        .or_else(|| map.get("deadline"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string);

                    let mut details = vec![format!("担当: {owner}")];
                    if let Some(due) = due {
                        details.push(format!("期限: {due}"));
                    }
                    normalized.push(format!("- {}（{}）", description, details.join(" / ")));
                    continue;
                }
            }
        }

        normalized.push(line.to_string());
    }

    normalized.join("\n")
}

fn section_from_value(value: Option<&Value>, title: &str) -> MinutesSection {
    let content = match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => format!("- {text}"),
                other => format!("- {}", other),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(Value::Object(map)) => map
            .iter()
            .map(|(key, value)| format!("- {key}: {value}"))
            .collect::<Vec<_>>()
            .join("\n"),
        Some(other) => other.to_string(),
        None => String::new(),
    };

    MinutesSection {
        title: title.to_string(),
        content,
    }
}

fn build_timeline(value: Option<&Value>) -> Result<Vec<TopicSummary>> {
    let mut timeline = Vec::new();
    let entries = match value {
        Some(Value::Array(items)) => items,
        _ => return Ok(timeline),
    };

    for (index, entry) in entries.iter().enumerate() {
        let object = match entry {
            Value::Object(map) => map,
            _ => continue,
        };
        let title = object
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Untitled")
            .to_string();
        let description = object
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let start = parse_timestamp(object.get("start"));
        let end = parse_timestamp(object.get("end"));

        let markers = build_marker_array(object.get("markers"));

        timeline.push(TopicSummary {
            id: format!("topic-{index}"),
            title,
            description,
            start,
            end,
            markers,
        });
    }

    Ok(timeline)
}

fn build_marker_array(value: Option<&Value>) -> Vec<TimelineMarker> {
    let mut markers = Vec::new();
    let lines = match value {
        Some(Value::Array(items)) => items,
        Some(Value::String(text)) => return parse_marker_lines(text),
        _ => return markers,
    };

    for entry in lines {
        match entry {
            Value::Object(map) => {
                let label = map
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let kind = map
                    .get("type")
                    .or_else(|| map.get("kind"))
                    .and_then(Value::as_str)
                    .unwrap_or("note")
                    .to_string();
                let timestamp = parse_timestamp(map.get("timestamp"));
                markers.push(TimelineMarker {
                    id: format!("marker-{}", markers.len() + 1),
                    label,
                    kind,
                    timestamp,
                });
            }
            Value::String(text) => markers.extend(parse_marker_lines(text)),
            _ => continue,
        }
    }
    markers
}

fn parse_marker_lines(text: &str) -> Vec<TimelineMarker> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let parts: Vec<&str> = line.split('@').collect();
            if parts.len() == 2 {
                let label = parts[0].trim().trim_start_matches('-').trim().to_string();
                let timestamp = parse_timestamp(Some(&Value::String(parts[1].trim().to_string())));
                Some(TimelineMarker {
                    id: format!("marker-line-{index}"),
                    label,
                    kind: "note".to_string(),
                    timestamp,
                })
            } else {
                Some(TimelineMarker {
                    id: format!("marker-line-{index}"),
                    label: line.to_string(),
                    kind: "note".to_string(),
                    timestamp: 0.0,
                })
            }
        })
        .collect()
}

fn parse_timestamp(value: Option<&Value>) -> f32 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or_default() as f32,
        Some(Value::String(text)) => {
            if let Some((minutes, seconds)) = text.split_once(':') {
                let minutes = minutes.trim().parse::<f32>().unwrap_or_default();
                let seconds = seconds.trim().parse::<f32>().unwrap_or_default();
                minutes * 60.0 + seconds
            } else {
                text.trim().parse::<f32>().unwrap_or_default()
            }
        }
        _ => 0.0,
    }
}
