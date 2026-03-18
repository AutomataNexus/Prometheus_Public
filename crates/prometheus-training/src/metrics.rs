// ============================================================================
// File: metrics.rs
// Description: Training evaluation metrics including accuracy, precision, recall, F1, AUC-ROC, and confusion matrix
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! Training metrics computation.
//!
//! Provides accuracy, precision, recall, F1 score, AUC-ROC, and confusion
//! matrix calculations for evaluating model performance.

use serde::{Deserialize, Serialize};
use tracing::debug;



// ---------------------------------------------------------------------------
// Metrics struct
// ---------------------------------------------------------------------------

/// Comprehensive evaluation metrics for a trained model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    /// Average loss on the evaluation set.
    pub loss: f32,
    /// Classification accuracy (correct / total).
    pub accuracy: f32,
    /// Precision = TP / (TP + FP).
    pub precision: f32,
    /// Recall = TP / (TP + FN).
    pub recall: f32,
    /// F1 score = 2 * (precision * recall) / (precision + recall).
    pub f1: f32,
    /// Area under the ROC curve (approximated via trapezoidal rule).
    pub auc_roc: f32,
    /// Mean squared error (primarily for regression / autoencoder tasks).
    pub mse: f32,
    /// Mean absolute error.
    pub mae: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            loss: f32::MAX,
            accuracy: 0.0,
            precision: 0.0,
            recall: 0.0,
            f1: 0.0,
            auc_roc: 0.0,
            mse: f32::MAX,
            mae: f32::MAX,
        }
    }
}

impl std::fmt::Display for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "loss={:.4} acc={:.4} prec={:.4} rec={:.4} f1={:.4} auc={:.4} mse={:.6} mae={:.6}",
            self.loss, self.accuracy, self.precision, self.recall, self.f1, self.auc_roc,
            self.mse, self.mae,
        )
    }
}

// ---------------------------------------------------------------------------
// Confusion matrix
// ---------------------------------------------------------------------------

/// Binary confusion matrix.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConfusionMatrix {
    /// True positives.
    pub tp: usize,
    /// True negatives.
    pub tn: usize,
    /// False positives.
    pub fp: usize,
    /// False negatives.
    pub fn_: usize,
}

impl ConfusionMatrix {
    /// Compute a binary confusion matrix from predictions and labels.
    ///
    /// Values >= `threshold` are considered positive.
    pub fn from_predictions(predictions: &[f32], labels: &[f32], threshold: f32) -> Self {
        let mut tp = 0usize;
        let mut tn = 0usize;
        let mut fp = 0usize;
        let mut fn_ = 0usize;

        for (pred, label) in predictions.iter().zip(labels.iter()) {
            let pred_pos = *pred >= threshold;
            let label_pos = *label >= threshold;

            match (pred_pos, label_pos) {
                (true, true) => tp += 1,
                (true, false) => fp += 1,
                (false, true) => fn_ += 1,
                (false, false) => tn += 1,
            }
        }

        Self { tp, tn, fp, fn_ }
    }

    /// Total number of samples.
    pub fn total(&self) -> usize {
        self.tp + self.tn + self.fp + self.fn_
    }

    /// Accuracy = (TP + TN) / total.
    pub fn accuracy(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        (self.tp + self.tn) as f32 / total as f32
    }

    /// Precision = TP / (TP + FP).
    pub fn precision(&self) -> f32 {
        let denom = self.tp + self.fp;
        if denom == 0 {
            return 0.0;
        }
        self.tp as f32 / denom as f32
    }

    /// Recall = TP / (TP + FN).
    pub fn recall(&self) -> f32 {
        let denom = self.tp + self.fn_;
        if denom == 0 {
            return 0.0;
        }
        self.tp as f32 / denom as f32
    }

    /// F1 score = 2 * (precision * recall) / (precision + recall).
    pub fn f1(&self) -> f32 {
        let p = self.precision();
        let r = self.recall();
        let denom = p + r;
        if denom < 1e-8 {
            return 0.0;
        }
        2.0 * p * r / denom
    }
}

// ---------------------------------------------------------------------------
// Metric computation
// ---------------------------------------------------------------------------

/// Compute comprehensive metrics from model predictions and ground truth labels.
///
/// For classification tasks, a threshold of 0.5 is used for the confusion matrix.
/// For regression/autoencoder tasks, MSE and MAE are the primary metrics.
pub fn compute_metrics(predictions: &[f32], labels: &[f32]) -> Metrics {
    if predictions.is_empty() || labels.is_empty() {
        return Metrics::default();
    }

    let n = predictions.len().min(labels.len());
    let predictions = &predictions[..n];
    let labels = &labels[..n];

    // MSE and MAE (useful for all tasks).
    let (mse, mae) = compute_mse_mae(predictions, labels);

    // Confusion matrix at threshold 0.5.
    let cm = ConfusionMatrix::from_predictions(predictions, labels, 0.5);

    let accuracy = cm.accuracy();
    let precision = cm.precision();
    let recall = cm.recall();
    let f1 = cm.f1();

    // AUC-ROC via trapezoidal approximation.
    let auc_roc = compute_auc_roc(predictions, labels);

    let loss = mse; // Default loss is MSE; callers can override.

    debug!(
        "Metrics computed: acc={:.4}, prec={:.4}, rec={:.4}, f1={:.4}, auc={:.4}, mse={:.6}",
        accuracy, precision, recall, f1, auc_roc, mse
    );

    Metrics {
        loss,
        accuracy,
        precision,
        recall,
        f1,
        auc_roc,
        mse,
        mae,
    }
}

/// Compute mean squared error and mean absolute error.
fn compute_mse_mae(predictions: &[f32], labels: &[f32]) -> (f32, f32) {
    let n = predictions.len() as f32;
    if n == 0.0 {
        return (0.0, 0.0);
    }

    let mut sum_sq = 0.0f32;
    let mut sum_abs = 0.0f32;

    for (p, l) in predictions.iter().zip(labels.iter()) {
        let diff = p - l;
        sum_sq += diff * diff;
        sum_abs += diff.abs();
    }

    (sum_sq / n, sum_abs / n)
}

/// Compute AUC-ROC using the trapezoidal rule.
///
/// Sorts predictions by score and sweeps the threshold to compute the
/// true-positive rate and false-positive rate at each point.
fn compute_auc_roc(predictions: &[f32], labels: &[f32]) -> f32 {
    let n = predictions.len();
    if n == 0 {
        return 0.0;
    }

    // Create (prediction, label) pairs sorted by prediction descending.
    let mut pairs: Vec<(f32, f32)> = predictions
        .iter()
        .zip(labels.iter())
        .map(|(&p, &l)| (p, l))
        .collect();
    pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let total_pos = labels.iter().filter(|&&l| l >= 0.5).count() as f32;
    let total_neg = n as f32 - total_pos;

    if total_pos == 0.0 || total_neg == 0.0 {
        // AUC is undefined when only one class is present; return 0.5.
        return 0.5;
    }

    let mut auc = 0.0f32;
    let mut tp = 0.0f32;
    let mut fp = 0.0f32;
    let mut prev_tpr = 0.0f32;
    let mut prev_fpr = 0.0f32;

    for &(_, label) in &pairs {
        if label >= 0.5 {
            tp += 1.0;
        } else {
            fp += 1.0;
        }

        let tpr = tp / total_pos;
        let fpr = fp / total_neg;

        // Trapezoidal area.
        auc += (fpr - prev_fpr) * (tpr + prev_tpr) / 2.0;

        prev_tpr = tpr;
        prev_fpr = fpr;
    }

    auc.clamp(0.0, 1.0)
}

/// Compute the reconstruction error (MSE) for autoencoder models.
///
/// Returns a per-sample error vector that can be thresholded for anomaly detection.
pub fn reconstruction_errors(inputs: &[Vec<f32>], reconstructions: &[Vec<f32>]) -> Vec<f32> {
    inputs
        .iter()
        .zip(reconstructions.iter())
        .map(|(input, recon)| {
            let n = input.len() as f32;
            if n == 0.0 {
                return 0.0;
            }
            input
                .iter()
                .zip(recon.iter())
                .map(|(a, b)| {
                    let d = a - b;
                    d * d
                })
                .sum::<f32>()
                / n
        })
        .collect()
}

/// Determine an anomaly threshold from reconstruction errors using a percentile.
///
/// For example, `percentile = 0.95` means the top 5% of reconstruction errors
/// are considered anomalous.
pub fn anomaly_threshold(errors: &[f32], percentile: f32) -> f32 {
    if errors.is_empty() {
        return 0.0;
    }
    let mut sorted = errors.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() as f32 * percentile) as usize).min(sorted.len() - 1);
    sorted[idx]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confusion_matrix_perfect() {
        let preds = vec![0.9, 0.8, 0.1, 0.05];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let cm = ConfusionMatrix::from_predictions(&preds, &labels, 0.5);

        assert_eq!(cm.tp, 2);
        assert_eq!(cm.tn, 2);
        assert_eq!(cm.fp, 0);
        assert_eq!(cm.fn_, 0);
        assert!((cm.accuracy() - 1.0).abs() < 1e-6);
        assert!((cm.precision() - 1.0).abs() < 1e-6);
        assert!((cm.recall() - 1.0).abs() < 1e-6);
        assert!((cm.f1() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_confusion_matrix_all_wrong() {
        let preds = vec![0.1, 0.2, 0.9, 0.8];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let cm = ConfusionMatrix::from_predictions(&preds, &labels, 0.5);

        assert_eq!(cm.tp, 0);
        assert_eq!(cm.tn, 0);
        assert_eq!(cm.fp, 2);
        assert_eq!(cm.fn_, 2);
        assert!((cm.accuracy() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_metrics() {
        let preds = vec![0.9, 0.8, 0.3, 0.1];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let m = compute_metrics(&preds, &labels);

        assert!((m.accuracy - 1.0).abs() < 1e-6);
        assert!((m.f1 - 1.0).abs() < 1e-6);
        assert!(m.mse < 0.1);
    }

    #[test]
    fn test_auc_roc_perfect() {
        let preds = vec![0.9, 0.8, 0.2, 0.1];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let auc = compute_auc_roc(&preds, &labels);
        assert!((auc - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_auc_roc_random() {
        // When predictions are uncorrelated with labels, AUC should be ~0.5.
        let preds = vec![0.1, 0.9, 0.3, 0.7, 0.5, 0.5, 0.8, 0.2];
        let labels = vec![0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0];
        let auc = compute_auc_roc(&preds, &labels);
        // Random-ish predictions should give AUC near 0.5 (with some variance).
        assert!(auc > 0.0 && auc < 1.0);
    }

    #[test]
    fn test_reconstruction_errors() {
        let inputs = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let recons = vec![vec![1.1, 2.1, 3.1], vec![4.0, 5.0, 6.0]];
        let errors = reconstruction_errors(&inputs, &recons);

        assert_eq!(errors.len(), 2);
        assert!(errors[0] > 0.0); // Not perfect reconstruction
        assert!((errors[1] - 0.0).abs() < 1e-6); // Perfect reconstruction
    }

    #[test]
    fn test_anomaly_threshold() {
        let errors = vec![0.01, 0.02, 0.03, 0.04, 0.05, 0.1, 0.2, 0.5, 0.8, 1.0];
        let thresh = anomaly_threshold(&errors, 0.9);
        assert!(thresh >= 0.5); // 90th percentile should be high
    }

    // -----------------------------------------------------------------------
    // Additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_confusion_matrix_counts_mixed() {
        // 2 TP, 1 FP, 1 TN, 1 FN
        let preds = vec![0.9, 0.7, 0.8, 0.2, 0.3];
        let labels = vec![1.0, 1.0, 0.0, 0.0, 1.0];
        let cm = ConfusionMatrix::from_predictions(&preds, &labels, 0.5);

        assert_eq!(cm.tp, 2);
        assert_eq!(cm.fp, 1);
        assert_eq!(cm.tn, 1);
        assert_eq!(cm.fn_, 1);
        assert_eq!(cm.total(), 5);
    }

    #[test]
    fn test_precision_calculation() {
        // TP=3, FP=1 => precision = 3/4 = 0.75
        let cm = ConfusionMatrix {
            tp: 3,
            tn: 2,
            fp: 1,
            fn_: 0,
        };
        assert!((cm.precision() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_recall_calculation() {
        // TP=3, FN=2 => recall = 3/5 = 0.6
        let cm = ConfusionMatrix {
            tp: 3,
            tn: 5,
            fp: 0,
            fn_: 2,
        };
        assert!((cm.recall() - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_f1_score_calculation() {
        // precision = 4/5 = 0.8, recall = 4/6 = 0.667
        // f1 = 2 * 0.8 * 0.667 / (0.8 + 0.667) = 1.0667 / 1.467 ≈ 0.727
        let cm = ConfusionMatrix {
            tp: 4,
            tn: 3,
            fp: 1,
            fn_: 2,
        };
        let p = cm.precision();
        let r = cm.recall();
        let expected_f1 = 2.0 * p * r / (p + r);
        assert!((cm.f1() - expected_f1).abs() < 1e-6);
    }

    #[test]
    fn test_accuracy_calculation() {
        // 7 correct out of 10
        let cm = ConfusionMatrix {
            tp: 4,
            tn: 3,
            fp: 2,
            fn_: 1,
        };
        assert!((cm.accuracy() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_confusion_matrix_all_positive_predictions() {
        // Every prediction is positive.
        let preds = vec![0.9, 0.8, 0.7, 0.6];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let cm = ConfusionMatrix::from_predictions(&preds, &labels, 0.5);

        assert_eq!(cm.tp, 2);
        assert_eq!(cm.fp, 2);
        assert_eq!(cm.tn, 0);
        assert_eq!(cm.fn_, 0);
        // Recall should be 1.0 (no false negatives).
        assert!((cm.recall() - 1.0).abs() < 1e-6);
        // Precision = 2/4 = 0.5
        assert!((cm.precision() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_confusion_matrix_all_negative_predictions() {
        // Every prediction is negative.
        let preds = vec![0.1, 0.2, 0.3, 0.4];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let cm = ConfusionMatrix::from_predictions(&preds, &labels, 0.5);

        assert_eq!(cm.tp, 0);
        assert_eq!(cm.fp, 0);
        assert_eq!(cm.tn, 2);
        assert_eq!(cm.fn_, 2);
        // Precision and recall should be 0.
        assert!((cm.precision() - 0.0).abs() < 1e-6);
        assert!((cm.recall() - 0.0).abs() < 1e-6);
        assert!((cm.f1() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_confusion_matrix_empty() {
        let cm = ConfusionMatrix::from_predictions(&[], &[], 0.5);
        assert_eq!(cm.total(), 0);
        assert!((cm.accuracy() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_auc_roc_worst_predictions() {
        // Completely reversed predictions: high scores for negatives, low for positives.
        let preds = vec![0.1, 0.2, 0.9, 0.8];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let auc = compute_auc_roc(&preds, &labels);
        assert!(auc < 0.1, "AUC for worst predictions should be near 0.0, got {}", auc);
    }

    #[test]
    fn test_auc_roc_single_class_only() {
        // All labels are positive — AUC is undefined, should return 0.5.
        let preds = vec![0.9, 0.8, 0.7];
        let labels = vec![1.0, 1.0, 1.0];
        let auc = compute_auc_roc(&preds, &labels);
        assert!((auc - 0.5).abs() < 1e-6);

        // All labels are negative.
        let preds2 = vec![0.1, 0.2, 0.3];
        let labels2 = vec![0.0, 0.0, 0.0];
        let auc2 = compute_auc_roc(&preds2, &labels2);
        assert!((auc2 - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_auc_roc_empty() {
        let auc = compute_auc_roc(&[], &[]);
        assert!((auc - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_metrics_returns_valid_struct() {
        let preds = vec![0.8, 0.7, 0.3, 0.2];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let m = compute_metrics(&preds, &labels);

        assert!(m.accuracy >= 0.0 && m.accuracy <= 1.0);
        assert!(m.precision >= 0.0 && m.precision <= 1.0);
        assert!(m.recall >= 0.0 && m.recall <= 1.0);
        assert!(m.f1 >= 0.0 && m.f1 <= 1.0);
        assert!(m.auc_roc >= 0.0 && m.auc_roc <= 1.0);
        assert!(m.mse >= 0.0);
        assert!(m.mae >= 0.0);
        assert!(m.loss >= 0.0); // loss == mse
    }

    #[test]
    fn test_compute_metrics_empty_inputs() {
        let m = compute_metrics(&[], &[]);
        assert_eq!(m.accuracy, 0.0);
        assert_eq!(m.loss, f32::MAX);
    }

    #[test]
    fn test_reconstruction_errors_perfect() {
        let inputs = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let recons = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let errors = reconstruction_errors(&inputs, &recons);
        assert_eq!(errors.len(), 2);
        assert!((errors[0] - 0.0).abs() < 1e-6);
        assert!((errors[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_reconstruction_errors_known_mse() {
        // Input: [1.0, 2.0], Recon: [2.0, 4.0]
        // Errors: (1.0)^2 + (2.0)^2 = 1 + 4 = 5, MSE = 5/2 = 2.5
        let inputs = vec![vec![1.0, 2.0]];
        let recons = vec![vec![2.0, 4.0]];
        let errors = reconstruction_errors(&inputs, &recons);
        assert!((errors[0] - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_reconstruction_errors_empty_features() {
        let inputs = vec![vec![]];
        let recons = vec![vec![]];
        let errors = reconstruction_errors(&inputs, &recons);
        assert_eq!(errors.len(), 1);
        assert!((errors[0] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_anomaly_threshold_empty() {
        let thresh = anomaly_threshold(&[], 0.95);
        assert!((thresh - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_anomaly_threshold_at_various_percentiles() {
        let errors: Vec<f32> = (0..100).map(|i| i as f32).collect();

        let t50 = anomaly_threshold(&errors, 0.5);
        let t90 = anomaly_threshold(&errors, 0.9);
        let t99 = anomaly_threshold(&errors, 0.99);

        // Higher percentiles should yield higher thresholds.
        assert!(t90 > t50);
        assert!(t99 > t90);
    }

    #[test]
    fn test_anomaly_threshold_single_element() {
        let errors = vec![42.0];
        let thresh = anomaly_threshold(&errors, 0.95);
        assert!((thresh - 42.0).abs() < 1e-6);
    }

    #[test]
    fn test_metrics_default() {
        let m = Metrics::default();
        assert_eq!(m.loss, f32::MAX);
        assert_eq!(m.accuracy, 0.0);
        assert_eq!(m.precision, 0.0);
        assert_eq!(m.recall, 0.0);
        assert_eq!(m.f1, 0.0);
        assert_eq!(m.auc_roc, 0.0);
        assert_eq!(m.mse, f32::MAX);
        assert_eq!(m.mae, f32::MAX);
    }

    #[test]
    fn test_metrics_display() {
        let m = Metrics {
            loss: 0.1234,
            accuracy: 0.95,
            precision: 0.9,
            recall: 0.85,
            f1: 0.875,
            auc_roc: 0.98,
            mse: 0.001,
            mae: 0.01,
        };
        let s = format!("{}", m);
        assert!(s.contains("loss="));
        assert!(s.contains("acc="));
        assert!(s.contains("prec="));
    }

    #[test]
    fn test_compute_metrics_mismatched_lengths() {
        // Shorter labels — should be truncated to min length.
        let preds = vec![0.9, 0.8, 0.7, 0.6, 0.5];
        let labels = vec![1.0, 1.0, 0.0];
        let m = compute_metrics(&preds, &labels);
        // Should not panic, should work on the first 3 elements.
        assert!(m.accuracy >= 0.0);
    }

    #[test]
    fn test_confusion_matrix_total() {
        let cm = ConfusionMatrix {
            tp: 10,
            tn: 20,
            fp: 5,
            fn_: 15,
        };
        assert_eq!(cm.total(), 50);
    }
}
