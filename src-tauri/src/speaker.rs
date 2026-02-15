use linfa::dataset::{AsTargets, DatasetBase};
use linfa::traits::{Fit, Predict};
use linfa_clustering::KMeans;
use ndarray::Array2;
use rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use tauri::{AppHandle, Emitter};
use tracing::warn;

use crate::state::AppState;

pub fn update_labels(app: &AppHandle, state: &AppState) {
    let embeddings = state.speaker_embeddings();
    if let Some(assignments) = cluster_segments(&embeddings) {
        for (segment_id, label) in assignments {
            if let Some(segment) = state.assign_speaker_if_changed(&segment_id, &label) {
                if let Err(error) = app.emit("transcription:final", &segment) {
                    warn!(?error, "failed to emit updated segment after clustering");
                }
            }
        }
    }
}

fn cluster_segments(embeddings: &[(String, Vec<f32>)]) -> Option<Vec<(String, String)>> {
    let count = embeddings.len();
    if count < 2 {
        return None;
    }

    let dimension = embeddings[0].1.len();
    if dimension == 0 {
        return None;
    }
    if embeddings
        .iter()
        .any(|(_, vector)| vector.len() != dimension)
    {
        warn!("speaker embedding dimensions are inconsistent; skipping clustering");
        return None;
    }

    let mut flattened = Vec::with_capacity(count * dimension);
    for (_, vector) in embeddings {
        flattened.extend(vector.iter().copied());
    }

    let data = Array2::from_shape_vec((count, dimension), flattened).ok()?;
    let dataset = DatasetBase::from(data);

    let cluster_count = count.clamp(2, 6);
    let rng = Xoshiro256Plus::seed_from_u64(0);
    let params = KMeans::params_with_rng(cluster_count, rng).max_n_iterations(50);
    let model = match params.fit(&dataset) {
        Ok(m) => m,
        Err(error) => {
            warn!(?error, "Failed to fit speaker clustering");
            return None;
        }
    };

    let predicted = model.predict(&dataset);
    let labels: Vec<usize> = predicted.as_targets().iter().copied().collect();
    let mut assignments = Vec::with_capacity(count);
    for (index, (segment_id, _)) in embeddings.iter().enumerate() {
        let label_index = labels
            .get(index)
            .copied()
            .unwrap_or_default()
            .min(cluster_count.saturating_sub(1));
        assignments.push((segment_id.clone(), format!("Speaker {}", label_index + 1)));
    }
    Some(assignments)
}
