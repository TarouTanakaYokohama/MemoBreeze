use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use uuid::Uuid;

use crate::audio::TranscriptionRuntime;
use crate::model::{MinutesOptions, RecordingOptions, SessionSnapshot, TranscriptSegment};

#[derive(Clone)]
pub struct SegmentRecord {
    pub segment: TranscriptSegment,
    pub embedding: Option<Vec<f32>>,
}

pub struct RecordingSession {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub options: RecordingOptions,
    pub runtime: Option<TranscriptionRuntime>,
    pub segments: Vec<SegmentRecord>,
    pub minutes_options: Option<MinutesOptions>,
}

impl RecordingSession {
    pub fn new(options: RecordingOptions) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            options,
            runtime: None,
            segments: Vec::new(),
            minutes_options: None,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<Option<RecordingSession>>>,
    last_options: Arc<RwLock<Option<RecordingOptions>>>,
    completed_segments: Arc<RwLock<Vec<SegmentRecord>>>,
    completed_minutes_options: Arc<RwLock<Option<MinutesOptions>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
            last_options: Arc::new(RwLock::new(None)),
            completed_segments: Arc::new(RwLock::new(Vec::new())),
            completed_minutes_options: Arc::new(RwLock::new(None)),
        }
    }
}

impl AppState {
    pub fn start_session(&self, options: RecordingOptions) -> String {
        let session = RecordingSession::new(options);
        let session_id = session.id.clone();
        *self.last_options.write() = Some(session.options.clone());
        *self.inner.write() = Some(session);
        self.completed_segments.write().clear();
        *self.completed_minutes_options.write() = None;
        session_id
    }

    pub fn set_runtime(&self, runtime: TranscriptionRuntime) {
        if let Some(session) = self.inner.write().as_mut() {
            session.runtime = Some(runtime);
        }
    }

    pub fn stop_session(&self) {
        // Take the session out first, then drop the lock before blocking stop/join.
        // Otherwise runtime shutdown can deadlock with worker threads touching AppState.
        let session = self.inner.write().take();

        if let Some(mut session) = session {
            if let Some(mut runtime) = session.runtime.take() {
                let _ = runtime.stop();
            }

            // Move data instead of cloning
            *self.completed_segments.write() = session.segments;
            *self.completed_minutes_options.write() = session.minutes_options;
            *self.last_options.write() = Some(session.options);
        }
    }

    #[cfg(target_os = "macos")]
    pub fn push_partial(
        &self,
        mut segment: TranscriptSegment,
        embedding: Option<Vec<f32>>,
    ) -> TranscriptSegment {
        segment.is_final = false;
        let mut guard = self.inner.write();
        let session = guard.as_mut().expect("session must exist");
        if let Some(existing) = session
            .segments
            .iter_mut()
            .find(|record| record.segment.id == segment.id)
        {
            existing.segment = segment.clone();
            existing.embedding = embedding;
        } else {
            session.segments.push(SegmentRecord {
                segment: segment.clone(),
                embedding,
            });
        }
        segment
    }

    pub fn push_final(
        &self,
        mut segment: TranscriptSegment,
        embedding: Option<Vec<f32>>,
    ) -> TranscriptSegment {
        segment.is_final = true;
        let mut guard = self.inner.write();
        if let Some(session) = guard.as_mut() {
            if let Some(existing) = session
                .segments
                .iter_mut()
                .find(|record| record.segment.id == segment.id)
            {
                existing.segment = segment.clone();
                existing.embedding = embedding;
            } else {
                let new_start = segment.start;
                let new_record = SegmentRecord {
                    segment: segment.clone(),
                    embedding,
                };

                // Insert in sorted position instead of push + sort
                let insert_pos = session
                    .segments
                    .binary_search_by(|record| {
                        record
                            .segment
                            .start
                            .partial_cmp(&new_start)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or_else(|pos| pos);

                session.segments.insert(insert_pos, new_record);
            }
        }
        segment
    }

    pub fn update_segment(&self, segment: TranscriptSegment) -> Option<TranscriptSegment> {
        let mut guard = self.inner.write();
        guard.as_mut().and_then(|session| {
            session
                .segments
                .iter_mut()
                .find(|record| record.segment.id == segment.id)
                .map(|record| {
                    record.segment = segment.clone();
                    segment
                })
        })
    }

    pub fn assign_speaker(&self, id: &str, speaker: &str) -> Option<TranscriptSegment> {
        let mut guard = self.inner.write();
        guard.as_mut().and_then(|session| {
            session
                .segments
                .iter_mut()
                .find(|record| record.segment.id == id)
                .map(|record| {
                    record.segment.speaker = speaker.to_string();
                    record.segment.clone()
                })
        })
    }

    pub fn assign_speaker_if_changed(&self, id: &str, speaker: &str) -> Option<TranscriptSegment> {
        let mut guard = self.inner.write();
        guard.as_mut().and_then(|session| {
            session
                .segments
                .iter_mut()
                .find(|record| record.segment.id == id)
                .and_then(|record| {
                    if record.segment.speaker == speaker {
                        return None;
                    }
                    record.segment.speaker = speaker.to_string();
                    Some(record.segment.clone())
                })
        })
    }

    #[allow(dead_code)]
    pub fn set_minutes_options(&self, options: MinutesOptions) {
        if let Some(session) = self.inner.write().as_mut() {
            session.minutes_options = Some(options.clone());
        }
        *self.completed_minutes_options.write() = Some(options);
    }

    pub fn snapshot(&self) -> Option<SessionSnapshot> {
        self.inner.read().as_ref().map(|session| SessionSnapshot {
            id: session.id.clone(),
            started_at: session.started_at,
            segments: session
                .segments
                .iter()
                .map(|record| record.segment.clone())
                .collect(),
        })
    }

    pub fn speaker_embeddings(&self) -> Vec<(String, Vec<f32>)> {
        if let Some(embeddings) = {
            let guard = self.inner.read();
            guard
                .as_ref()
                .map(|session| collect_speaker_embeddings(&session.segments))
        } {
            embeddings
        } else {
            collect_speaker_embeddings(&self.completed_segments.read())
        }
    }

    pub fn recording_options(&self) -> Option<RecordingOptions> {
        self.inner
            .read()
            .as_ref()
            .map(|session| session.options.clone())
            .or_else(|| self.last_options.read().as_ref().cloned())
    }

    pub fn is_active(&self) -> bool {
        self.inner
            .read()
            .as_ref()
            .map(|session| session.runtime.is_some())
            .unwrap_or(false)
    }
}

pub static GLOBAL_STATE: once_cell::sync::Lazy<AppState> =
    once_cell::sync::Lazy::new(AppState::default);

fn collect_speaker_embeddings(records: &[SegmentRecord]) -> Vec<(String, Vec<f32>)> {
    let mut embeddings = Vec::new();
    for record in records {
        if let Some(vector) = &record.embedding {
            embeddings.push((record.segment.id.clone(), vector.clone()));
        }
    }
    embeddings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{TranscriptToken, TranscriptionEngine};
    use pretty_assertions::assert_eq;

    fn sample_options() -> RecordingOptions {
        RecordingOptions {
            engine: TranscriptionEngine::Whisper,
            model_path: "/tmp/vosk-model".to_string(),
            speaker_model_path: None,
            whisper_model_path: "/tmp/ggml-base.bin".to_string(),
            whisper_language: Some("ja".to_string()),
            whisper_command: "whisper-cli".to_string(),
            enable_input: true,
            enable_output: false,
            energy_threshold: 0.0,
        }
    }

    fn sample_segment(id: &str, start: f32, speaker: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            speaker: speaker.to_string(),
            text: format!("text-{id}"),
            start,
            end: start + 1.0,
            tokens: vec![TranscriptToken {
                text: "hello".to_string(),
                start,
                end: start + 0.2,
                confidence: 0.9,
            }],
            is_final: false,
        }
    }

    #[test]
    fn start_session_sets_options_and_clears_completed() {
        let state = AppState::default();
        let session_id = state.start_session(sample_options());
        assert!(!session_id.is_empty());
        assert!(state.recording_options().is_some());
        assert_eq!(state.speaker_embeddings(), Vec::<(String, Vec<f32>)>::new());
    }

    #[test]
    fn push_final_inserts_in_start_time_order() {
        let state = AppState::default();
        state.start_session(sample_options());

        state.push_final(sample_segment("b", 20.0, "S2"), None);
        state.push_final(sample_segment("a", 10.0, "S1"), None);
        state.push_final(sample_segment("c", 30.0, "S3"), None);

        let snapshot = state.snapshot().expect("snapshot should exist");
        let ids: Vec<String> = snapshot.segments.iter().map(|s| s.id.clone()).collect();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn push_final_updates_existing_segment_instead_of_duplicate() {
        let state = AppState::default();
        state.start_session(sample_options());

        state.push_final(sample_segment("x", 10.0, "S1"), Some(vec![0.1, 0.2]));
        let mut updated = sample_segment("x", 10.0, "S1");
        updated.text = "updated".to_string();
        state.push_final(updated, Some(vec![0.3, 0.4]));

        let snapshot = state.snapshot().expect("snapshot should exist");
        assert_eq!(snapshot.segments.len(), 1);
        assert_eq!(snapshot.segments[0].text, "updated");

        let embeddings = state.speaker_embeddings();
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].1, vec![0.3, 0.4]);
    }

    #[test]
    fn assign_speaker_if_changed_returns_none_when_same() {
        let state = AppState::default();
        state.start_session(sample_options());
        state.push_final(sample_segment("x", 0.0, "Speaker 1"), None);

        let result = state.assign_speaker_if_changed("x", "Speaker 1");
        assert!(result.is_none());
    }

    #[test]
    fn assign_speaker_if_changed_updates_when_different() {
        let state = AppState::default();
        state.start_session(sample_options());
        state.push_final(sample_segment("x", 0.0, "Speaker 1"), None);

        let result = state
            .assign_speaker_if_changed("x", "Speaker 2")
            .expect("segment should be updated");
        assert_eq!(result.speaker, "Speaker 2");
    }

    #[test]
    fn update_segment_returns_none_for_missing_segment() {
        let state = AppState::default();
        state.start_session(sample_options());

        let result = state.update_segment(sample_segment("missing", 0.0, "S1"));
        assert!(result.is_none());
    }

    #[test]
    fn stop_session_preserves_last_options() {
        let state = AppState::default();
        let options = sample_options();
        state.start_session(options.clone());
        state.push_final(sample_segment("x", 0.0, "S1"), None);

        state.stop_session();

        let restored = state.recording_options().expect("options should persist");
        assert_eq!(restored.whisper_model_path, options.whisper_model_path);
        assert!(state.snapshot().is_none());
    }

    #[test]
    fn speaker_embeddings_read_from_completed_after_stop() {
        let state = AppState::default();
        state.start_session(sample_options());
        state.push_final(sample_segment("x", 0.0, "S1"), Some(vec![1.0, 2.0, 3.0]));

        state.stop_session();

        let embeddings = state.speaker_embeddings();
        assert_eq!(embeddings, vec![("x".to_string(), vec![1.0, 2.0, 3.0])]);
    }

    #[test]
    fn snapshot_contains_segments_while_session_is_active() {
        let state = AppState::default();
        state.start_session(sample_options());
        state.push_final(sample_segment("x", 5.0, "S1"), None);

        let snapshot = state.snapshot().expect("active snapshot");
        assert_eq!(snapshot.segments.len(), 1);
        assert_eq!(snapshot.segments[0].id, "x");
    }
}
