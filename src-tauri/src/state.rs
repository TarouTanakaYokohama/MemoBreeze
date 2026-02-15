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
