// ============================================================================
// File: preprocessor.rs
// Description: Data preprocessing including CSV loading, z-score normalization, splitting, and sequence creation
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Data preprocessing for the training pipeline.
//!
//! Handles CSV loading, z-score normalization, train/val/test splitting,
//! and sequence creation for temporal models (LSTM, GRU).

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use crate::{Result, TrainingError};

// ---------------------------------------------------------------------------
// Dataset type
// ---------------------------------------------------------------------------

/// A dataset consisting of N samples, each with F features.
///
/// Stored as row-major: `data[sample_index][feature_index]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    /// Row-major feature matrix.
    pub data: Vec<Vec<f32>>,
    /// Optional labels (one per sample).
    pub labels: Option<Vec<f32>>,
    /// Feature column names from the CSV header.
    pub feature_names: Vec<String>,
    /// Number of features per sample.
    pub num_features: usize,
    /// Number of samples.
    pub num_samples: usize,
}

impl Dataset {
    /// Create a new dataset from raw data.
    pub fn new(
        data: Vec<Vec<f32>>,
        labels: Option<Vec<f32>>,
        feature_names: Vec<String>,
    ) -> Result<Self> {
        if data.is_empty() {
            return Err(TrainingError::Preprocessing(
                "dataset must contain at least one sample".into(),
            ));
        }
        let num_features = data[0].len();
        if num_features == 0 {
            return Err(TrainingError::Preprocessing(
                "dataset must contain at least one feature".into(),
            ));
        }
        // Verify all rows have the same number of features.
        for (i, row) in data.iter().enumerate() {
            if row.len() != num_features {
                return Err(TrainingError::Preprocessing(format!(
                    "row {i} has {} features, expected {num_features}",
                    row.len()
                )));
            }
        }
        if let Some(ref l) = labels {
            if l.len() != data.len() {
                return Err(TrainingError::Preprocessing(format!(
                    "labels length {} does not match data length {}",
                    l.len(),
                    data.len()
                )));
            }
        }
        let num_samples = data.len();
        Ok(Self {
            data,
            labels,
            feature_names,
            num_features,
            num_samples,
        })
    }

    /// Return the number of samples.
    pub fn len(&self) -> usize {
        self.num_samples
    }

    /// Check if the dataset is empty.
    pub fn is_empty(&self) -> bool {
        self.num_samples == 0
    }
}

// ---------------------------------------------------------------------------
// Normalization statistics
// ---------------------------------------------------------------------------

/// Per-feature mean and standard deviation used for z-score normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationStats {
    /// Per-feature mean values.
    pub means: Vec<f32>,
    /// Per-feature standard deviations.
    pub stds: Vec<f32>,
}

impl NormalizationStats {
    /// Compute normalization statistics from a dataset.
    pub fn from_data(data: &[Vec<f32>]) -> Result<Self> {
        if data.is_empty() {
            return Err(TrainingError::Preprocessing(
                "cannot compute stats from empty data".into(),
            ));
        }
        let num_features = data[0].len();
        let n = data.len() as f32;

        let mut means = vec![0.0f32; num_features];
        for row in data {
            for (j, val) in row.iter().enumerate() {
                means[j] += val;
            }
        }
        for m in &mut means {
            *m /= n;
        }

        let mut stds = vec![0.0f32; num_features];
        for row in data {
            for (j, val) in row.iter().enumerate() {
                let diff = val - means[j];
                stds[j] += diff * diff;
            }
        }
        for s in &mut stds {
            *s = (*s / n).sqrt();
            // Prevent division by zero for constant features.
            if *s < 1e-8 {
                *s = 1.0;
            }
        }

        Ok(Self { means, stds })
    }

    /// Apply z-score normalization: (x - mean) / std.
    pub fn normalize(&self, data: &mut [Vec<f32>]) {
        for row in data.iter_mut() {
            for (j, val) in row.iter_mut().enumerate() {
                *val = (*val - self.means[j]) / self.stds[j];
            }
        }
    }

    /// Reverse z-score normalization: x * std + mean.
    pub fn denormalize(&self, data: &mut [Vec<f32>]) {
        for row in data.iter_mut() {
            for (j, val) in row.iter_mut().enumerate() {
                *val = *val * self.stds[j] + self.means[j];
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CSV loading
// ---------------------------------------------------------------------------

/// Load a dataset from a CSV file.
///
/// The CSV file is expected to have a header row. If a column named `label`,
/// `target`, or `y` exists, it will be separated into the labels vector.
/// All other numeric columns become features.
#[instrument(skip_all, fields(path = %path))]
pub fn load_csv(path: &str) -> Result<Dataset> {
    let csv_path = Path::new(path);
    if !csv_path.exists() {
        return Err(TrainingError::Preprocessing(format!(
            "dataset file not found: {path}"
        )));
    }

    info!("Loading CSV dataset from {}", path);

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(csv_path)?;

    let headers: Vec<String> = reader
        .headers()?
        .iter()
        .map(|h| h.to_string())
        .collect();

    // Identify the label column (if any).
    let label_col_names = ["label", "target", "y", "class"];
    let label_col_idx = headers.iter().position(|h| {
        label_col_names.contains(&h.to_lowercase().as_str())
    });

    let feature_names: Vec<String> = headers
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != label_col_idx)
        .map(|(_, name)| name.clone())
        .collect();

    let mut data: Vec<Vec<f32>> = Vec::new();
    let mut labels: Vec<f32> = Vec::new();
    let mut row_count = 0usize;

    for result in reader.records() {
        let record = result?;
        let mut row = Vec::with_capacity(feature_names.len());
        for (i, field) in record.iter().enumerate() {
            let val: f32 = field.trim().parse().unwrap_or_else(|_| {
                debug!("Non-numeric value '{}' at row {}, col {} — using 0.0", field, row_count, i);
                0.0
            });
            if Some(i) == label_col_idx {
                labels.push(val);
            } else {
                row.push(val);
            }
        }
        data.push(row);
        row_count += 1;
    }

    info!(
        "Loaded {} samples with {} features (label column: {})",
        row_count,
        feature_names.len(),
        label_col_idx.map(|i| headers[i].as_str()).unwrap_or("none")
    );

    let labels_opt = if labels.is_empty() {
        None
    } else {
        Some(labels)
    };

    Dataset::new(data, labels_opt, feature_names)
}

// ---------------------------------------------------------------------------
// Train/validation/test split with normalization
// ---------------------------------------------------------------------------

/// Split a dataset into train, validation, and test sets, then apply z-score
/// normalization. Normalization statistics are computed from the training set
/// only (to prevent data leakage) and applied to all three sets.
///
/// Returns `(train_set, val_set, test_set)`.
#[instrument(skip(dataset), fields(n = dataset.num_samples))]
pub fn split_and_normalize(
    dataset: Dataset,
    train_ratio: f64,
    val_ratio: f64,
    test_ratio: f64,
) -> Result<(Dataset, Dataset, Dataset)> {
    let sum = train_ratio + val_ratio + test_ratio;
    if (sum - 1.0).abs() > 1e-6 {
        return Err(TrainingError::Preprocessing(format!(
            "split ratios must sum to 1.0, got {sum}"
        )));
    }
    if dataset.num_samples < 3 {
        return Err(TrainingError::Preprocessing(
            "need at least 3 samples for train/val/test split".into(),
        ));
    }

    let n = dataset.num_samples;
    let train_end = (n as f64 * train_ratio).round() as usize;
    let val_end = train_end + (n as f64 * val_ratio).round() as usize;

    // Ensure we have at least 1 sample per split.
    let train_end = train_end.max(1).min(n - 2);
    let val_end = val_end.max(train_end + 1).min(n - 1);

    let mut train_data: Vec<Vec<f32>> = dataset.data[..train_end].to_vec();
    let mut val_data: Vec<Vec<f32>> = dataset.data[train_end..val_end].to_vec();
    let mut test_data: Vec<Vec<f32>> = dataset.data[val_end..].to_vec();

    let train_labels = dataset.labels.as_ref().map(|l| l[..train_end].to_vec());
    let val_labels = dataset.labels.as_ref().map(|l| l[train_end..val_end].to_vec());
    let test_labels = dataset.labels.as_ref().map(|l| l[val_end..].to_vec());

    // Compute normalization stats from training data only.
    let stats = NormalizationStats::from_data(&train_data)?;

    // Apply normalization to all sets.
    stats.normalize(&mut train_data);
    stats.normalize(&mut val_data);
    stats.normalize(&mut test_data);

    info!(
        "Split: train={}, val={}, test={}",
        train_data.len(),
        val_data.len(),
        test_data.len()
    );

    let feature_names = dataset.feature_names.clone();

    let train_set = Dataset::new(train_data, train_labels, feature_names.clone())?;
    let val_set = Dataset::new(val_data, val_labels, feature_names.clone())?;
    let test_set = Dataset::new(test_data, test_labels, feature_names)?;

    Ok((train_set, val_set, test_set))
}

// ---------------------------------------------------------------------------
// Sequence creation for temporal models
// ---------------------------------------------------------------------------

/// Create input/target sequence pairs from time-series data for LSTM/GRU models.
///
/// Given a multi-variate time series, creates sliding windows of length `seq_len`.
/// Each window becomes an input sequence and the next time step's values become
/// the target vector.
///
/// Returns a vector of `(input_sequence, target_vector)` pairs where:
/// - `input_sequence` is `seq_len` rows of `num_features` columns
/// - `target_vector` is the feature values at time `t + seq_len`
pub fn create_sequences(
    data: &[Vec<f32>],
    seq_len: usize,
) -> Vec<(Vec<Vec<f32>>, Vec<f32>)> {
    if data.len() <= seq_len {
        return Vec::new();
    }

    let num_sequences = data.len() - seq_len;
    let mut sequences = Vec::with_capacity(num_sequences);

    for i in 0..num_sequences {
        let input: Vec<Vec<f32>> = data[i..i + seq_len].to_vec();
        let target: Vec<f32> = data[i + seq_len].clone();
        sequences.push((input, target));
    }

    sequences
}

/// Create sequences where labels serve as targets instead of the next timestep.
///
/// Each window of `seq_len` rows becomes an input, paired with the label
/// at the last position of the window.
pub fn create_labeled_sequences(
    data: &[Vec<f32>],
    labels: &[f32],
    seq_len: usize,
) -> Vec<(Vec<Vec<f32>>, f32)> {
    if data.len() < seq_len || labels.len() < seq_len {
        return Vec::new();
    }

    let num_sequences = data.len() - seq_len + 1;
    let mut sequences = Vec::with_capacity(num_sequences);

    for i in 0..num_sequences {
        let input: Vec<Vec<f32>> = data[i..i + seq_len].to_vec();
        let label = labels[i + seq_len - 1];
        sequences.push((input, label));
    }

    sequences
}

/// Create mini-batches from a dataset.
///
/// Chunks the data into groups of `batch_size` samples. The last batch may
/// be smaller if the dataset size is not evenly divisible.
pub fn create_batches(data: &[Vec<f32>], batch_size: usize) -> Vec<Vec<Vec<f32>>> {
    data.chunks(batch_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> Vec<Vec<f32>> {
        vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
            vec![10.0, 11.0, 12.0],
            vec![13.0, 14.0, 15.0],
        ]
    }

    #[test]
    fn test_normalization_stats() {
        let data = sample_data();
        let stats = NormalizationStats::from_data(&data).unwrap();

        // Mean of [1,4,7,10,13] = 7.0
        assert!((stats.means[0] - 7.0).abs() < 1e-4);
        // Mean of [2,5,8,11,14] = 8.0
        assert!((stats.means[1] - 8.0).abs() < 1e-4);
    }

    #[test]
    fn test_normalize_denormalize_roundtrip() {
        let data = sample_data();
        let stats = NormalizationStats::from_data(&data).unwrap();

        let mut normalized = data.clone();
        stats.normalize(&mut normalized);

        // After normalization, mean should be ~0 and std ~1.
        let norm_stats = NormalizationStats::from_data(&normalized).unwrap();
        for m in &norm_stats.means {
            assert!(m.abs() < 1e-4);
        }

        stats.denormalize(&mut normalized);

        // Should be back to original.
        for (orig, restored) in data.iter().zip(normalized.iter()) {
            for (a, b) in orig.iter().zip(restored.iter()) {
                assert!((a - b).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn test_create_sequences() {
        let data = sample_data();
        let seqs = create_sequences(&data, 2);

        assert_eq!(seqs.len(), 3); // 5 - 2 = 3 sequences
        assert_eq!(seqs[0].0.len(), 2); // seq_len = 2
        assert_eq!(seqs[0].1, vec![7.0, 8.0, 9.0]); // target is third row
    }

    #[test]
    fn test_create_sequences_too_short() {
        let data = vec![vec![1.0], vec![2.0]];
        let seqs = create_sequences(&data, 5);
        assert!(seqs.is_empty());
    }

    #[test]
    fn test_create_batches() {
        let data = sample_data();
        let batches = create_batches(&data, 2);
        assert_eq!(batches.len(), 3); // 5 / 2 = 2 full + 1 partial
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[2].len(), 1);
    }

    #[test]
    fn test_dataset_validation() {
        // Mismatched row lengths should fail.
        let bad_data = vec![vec![1.0, 2.0], vec![3.0]];
        assert!(Dataset::new(bad_data, None, vec!["a".into(), "b".into()]).is_err());
    }

    // -----------------------------------------------------------------------
    // Additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_csv_valid_file() {
        // Create a temporary CSV file and load it.
        let dir = std::env::temp_dir().join("prometheus_test_load_csv_valid");
        std::fs::create_dir_all(&dir).unwrap();
        let csv_path = dir.join("valid.csv");
        std::fs::write(
            &csv_path,
            "feature_a,feature_b,label\n1.0,2.0,0.0\n3.0,4.0,1.0\n5.0,6.0,0.0\n",
        )
        .unwrap();

        let dataset = load_csv(csv_path.to_str().unwrap()).unwrap();
        assert_eq!(dataset.num_samples, 3);
        assert_eq!(dataset.num_features, 2); // label column is separated
        assert_eq!(dataset.feature_names, vec!["feature_a", "feature_b"]);
        assert!(dataset.labels.is_some());
        let labels = dataset.labels.unwrap();
        assert_eq!(labels, vec![0.0, 1.0, 0.0]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_csv_missing_file() {
        let result = load_csv("/tmp/nonexistent_prometheus_test_file_12345.csv");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_csv_no_label_column() {
        let dir = std::env::temp_dir().join("prometheus_test_load_csv_nolabel");
        std::fs::create_dir_all(&dir).unwrap();
        let csv_path = dir.join("no_label.csv");
        std::fs::write(
            &csv_path,
            "sensor_a,sensor_b,sensor_c\n1.0,2.0,3.0\n4.0,5.0,6.0\n",
        )
        .unwrap();

        let dataset = load_csv(csv_path.to_str().unwrap()).unwrap();
        assert_eq!(dataset.num_features, 3);
        assert!(dataset.labels.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_csv_empty_file_has_header_only() {
        let dir = std::env::temp_dir().join("prometheus_test_load_csv_empty");
        std::fs::create_dir_all(&dir).unwrap();
        let csv_path = dir.join("empty.csv");
        std::fs::write(&csv_path, "a,b,c\n").unwrap();

        let result = load_csv(csv_path.to_str().unwrap());
        // Should fail because there are no data rows.
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_split_and_normalize_proportions() {
        // 10 samples, 80/10/10 split
        let data: Vec<Vec<f32>> = (0..10).map(|i| vec![i as f32, (i * 2) as f32]).collect();
        let ds = Dataset::new(data, None, vec!["a".into(), "b".into()]).unwrap();

        let (train, val, test) = split_and_normalize(ds, 0.8, 0.1, 0.1).unwrap();

        // Total samples must be preserved.
        assert_eq!(train.num_samples + val.num_samples + test.num_samples, 10);

        // Training set should be the largest.
        assert!(train.num_samples >= val.num_samples);
        assert!(train.num_samples >= test.num_samples);

        // Each split must have at least 1 sample.
        assert!(train.num_samples >= 1);
        assert!(val.num_samples >= 1);
        assert!(test.num_samples >= 1);
    }

    #[test]
    fn test_split_and_normalize_invalid_ratios() {
        let data = vec![vec![1.0], vec![2.0], vec![3.0]];
        let ds = Dataset::new(data, None, vec!["a".into()]).unwrap();

        let result = split_and_normalize(ds, 0.5, 0.3, 0.3); // sums to 1.1
        assert!(result.is_err());
    }

    #[test]
    fn test_split_and_normalize_too_few_samples() {
        let data = vec![vec![1.0], vec![2.0]];
        let ds = Dataset::new(data, None, vec!["a".into()]).unwrap();

        let result = split_and_normalize(ds, 0.7, 0.15, 0.15);
        assert!(result.is_err()); // need at least 3
    }

    #[test]
    fn test_normalization_stats_zero_variance() {
        // All values the same — std should be clamped to 1.0.
        let data = vec![vec![5.0, 5.0], vec![5.0, 5.0], vec![5.0, 5.0]];
        let stats = NormalizationStats::from_data(&data).unwrap();

        assert!((stats.means[0] - 5.0).abs() < 1e-6);
        assert!((stats.means[1] - 5.0).abs() < 1e-6);
        // Std should be clamped to 1.0 for constant features.
        assert!((stats.stds[0] - 1.0).abs() < 1e-6);
        assert!((stats.stds[1] - 1.0).abs() < 1e-6);

        // Normalization should produce (5 - 5) / 1 = 0.
        let mut normalized = data.clone();
        stats.normalize(&mut normalized);
        for row in &normalized {
            for val in row {
                assert!(val.abs() < 1e-6);
            }
        }
    }

    #[test]
    fn test_normalization_stats_from_empty_data() {
        let data: Vec<Vec<f32>> = Vec::new();
        let result = NormalizationStats::from_data(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_zscore_normalization_known_values() {
        // Data: [1, 3], [3, 7] => means = [2, 5], stds = [1, 2]
        let data = vec![vec![1.0, 3.0], vec![3.0, 7.0]];
        let stats = NormalizationStats::from_data(&data).unwrap();

        assert!((stats.means[0] - 2.0).abs() < 1e-4);
        assert!((stats.means[1] - 5.0).abs() < 1e-4);
        assert!((stats.stds[0] - 1.0).abs() < 1e-4);
        assert!((stats.stds[1] - 2.0).abs() < 1e-4);

        let mut norm = data.clone();
        stats.normalize(&mut norm);
        // (1 - 2) / 1 = -1, (3 - 5) / 2 = -1
        assert!((norm[0][0] - (-1.0)).abs() < 1e-4);
        assert!((norm[0][1] - (-1.0)).abs() < 1e-4);
        // (3 - 2) / 1 = 1, (7 - 5) / 2 = 1
        assert!((norm[1][0] - 1.0).abs() < 1e-4);
        assert!((norm[1][1] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_create_sequences_seq_len_5() {
        let data: Vec<Vec<f32>> = (0..10).map(|i| vec![i as f32]).collect();
        let seqs = create_sequences(&data, 5);

        assert_eq!(seqs.len(), 5); // 10 - 5 = 5
        // First sequence: input is rows 0..5, target is row 5.
        assert_eq!(seqs[0].0.len(), 5);
        assert_eq!(seqs[0].0[0], vec![0.0]);
        assert_eq!(seqs[0].0[4], vec![4.0]);
        assert_eq!(seqs[0].1, vec![5.0]);
    }

    #[test]
    fn test_create_sequences_exact_length() {
        // data.len() == seq_len => should return empty (need at least seq_len + 1).
        let data = vec![vec![1.0], vec![2.0], vec![3.0]];
        let seqs = create_sequences(&data, 3);
        assert!(seqs.is_empty());
    }

    #[test]
    fn test_create_labeled_sequences_shapes() {
        let data: Vec<Vec<f32>> = (0..8).map(|i| vec![i as f32, (i * 10) as f32]).collect();
        let labels: Vec<f32> = (0..8).map(|i| if i >= 4 { 1.0 } else { 0.0 }).collect();
        let seqs = create_labeled_sequences(&data, &labels, 3);

        // num_sequences = 8 - 3 + 1 = 6
        assert_eq!(seqs.len(), 6);
        // Each input should have seq_len=3 rows, each with 2 features.
        for (input, _label) in &seqs {
            assert_eq!(input.len(), 3);
            assert_eq!(input[0].len(), 2);
        }
        // The label should correspond to the last element in each window.
        assert_eq!(seqs[0].1, labels[2]); // window [0,1,2] -> label at index 2
        assert_eq!(seqs[5].1, labels[7]); // window [5,6,7] -> label at index 7
    }

    #[test]
    fn test_create_labeled_sequences_too_short() {
        let data = vec![vec![1.0], vec![2.0]];
        let labels = vec![0.0, 1.0];
        let seqs = create_labeled_sequences(&data, &labels, 5);
        assert!(seqs.is_empty());
    }

    #[test]
    fn test_dataset_new_empty_data() {
        let result = Dataset::new(Vec::new(), None, vec!["a".into()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_dataset_new_zero_features() {
        let data = vec![vec![]];
        let result = Dataset::new(data, None, Vec::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_dataset_labels_length_mismatch() {
        let data = vec![vec![1.0], vec![2.0]];
        let labels = vec![0.0, 1.0, 2.0]; // 3 labels for 2 rows
        let result = Dataset::new(data, Some(labels), vec!["a".into()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_dataset_len_and_is_empty() {
        let ds = Dataset::new(vec![vec![1.0]], None, vec!["a".into()]).unwrap();
        assert_eq!(ds.len(), 1);
        assert!(!ds.is_empty());
    }

    #[test]
    fn test_create_batches_single_sample() {
        let data = vec![vec![1.0, 2.0]];
        let batches = create_batches(&data, 10);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
    }

    #[test]
    fn test_create_batches_exact_division() {
        let data: Vec<Vec<f32>> = (0..6).map(|i| vec![i as f32]).collect();
        let batches = create_batches(&data, 3);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 3);
        assert_eq!(batches[1].len(), 3);
    }
}
