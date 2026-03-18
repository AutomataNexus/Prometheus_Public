# =============================================================================
# File: evaluator.py
# Description: Specialist agent for model evaluation against domain benchmarks
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 8, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Evaluator Agent
===============

Specialist agent that interprets model-evaluation metrics, compares them
against domain benchmarks, suggests improvements, and makes a deploy/no-deploy
recommendation.
"""

from __future__ import annotations

import logging
from typing import Any

from gradient_adk import RequestContext

from tools.model_evaluator import fetch_evaluation_metrics

logger = logging.getLogger(__name__)


# ── Benchmark thresholds per equipment type ───────────────────────────────

_BENCHMARKS: dict[str, dict[str, float]] = {
    "default": {
        "precision_min": 0.85,
        "recall_min": 0.80,
        "f1_min": 0.82,
        "auc_roc_min": 0.90,
    },
    "chiller": {
        "precision_min": 0.88,
        "recall_min": 0.85,
        "f1_min": 0.86,
        "auc_roc_min": 0.92,
    },
    "ahu": {
        "precision_min": 0.85,
        "recall_min": 0.82,
        "f1_min": 0.83,
        "auc_roc_min": 0.91,
    },
    "boiler": {
        "precision_min": 0.90,
        "recall_min": 0.88,
        "f1_min": 0.89,
        "auc_roc_min": 0.93,
    },
    "pump": {
        "precision_min": 0.84,
        "recall_min": 0.80,
        "f1_min": 0.82,
        "auc_roc_min": 0.90,
    },
    "fan_coil": {
        "precision_min": 0.82,
        "recall_min": 0.78,
        "f1_min": 0.80,
        "auc_roc_min": 0.88,
    },
    "steam": {
        "precision_min": 0.87,
        "recall_min": 0.84,
        "f1_min": 0.85,
        "auc_roc_min": 0.91,
    },
}


class EvaluatorAgent:
    """Evaluate a trained Prometheus model and decide on deployment readiness."""

    # ── public API ────────────────────────────────────────────────────────

    async def evaluate(
        self,
        message: str,
        model_id: str | None,
        metrics_payload: dict[str, float] | None,
        conversation_context: Any,
        request_context: RequestContext,
    ) -> dict[str, Any]:
        """Run a full evaluation pipeline.

        Parameters
        ----------
        message:
            Natural-language request.
        model_id:
            If provided, metrics are fetched from the Prometheus API.
        metrics_payload:
            Pre-computed metrics dict (used when caller already has them).
        conversation_context:
            Shared orchestrator context.
        request_context:
            ADK request context.

        Returns
        -------
        dict
            ``{"status", "metrics", "benchmark_comparison",
            "deploy_ready", "improvements", "summary"}``
        """
        # Resolve metrics ─────────────────────────────────────────────────
        metrics: dict[str, float] | None = metrics_payload
        if metrics is None and model_id is not None:
            fetched = await fetch_evaluation_metrics(model_id)
            if fetched.get("status") == "success":
                metrics = fetched.get("metrics")

        if not metrics:
            return {
                "status": "error",
                "error": (
                    "No metrics available.  Provide a model_id to fetch them "
                    "or pass a metrics dict directly."
                ),
            }

        # Determine equipment type from context ───────────────────────────
        equipment_type: str = "default"
        if conversation_context and hasattr(conversation_context, "last_architecture"):
            arch = conversation_context.last_architecture or {}
            equipment_type = arch.get("equipment_type", "default")

        # Compare against benchmarks ──────────────────────────────────────
        comparison = self._compare_benchmarks(metrics, equipment_type)

        # Decide deploy-readiness ─────────────────────────────────────────
        deploy_ready = self._is_deploy_ready(comparison)

        # Generate improvement suggestions ────────────────────────────────
        improvements = self._suggest_improvements(metrics, comparison, equipment_type)

        # Build human-readable summary ────────────────────────────────────
        summary = self._build_summary(metrics, comparison, deploy_ready, improvements)

        return {
            "status": "success",
            "metrics": metrics,
            "equipment_type": equipment_type,
            "benchmark_comparison": comparison,
            "deploy_ready": deploy_ready,
            "improvements": improvements,
            "summary": summary,
        }

    # ── benchmarking ──────────────────────────────────────────────────────

    def _compare_benchmarks(
        self,
        metrics: dict[str, float],
        equipment_type: str,
    ) -> dict[str, dict[str, Any]]:
        """Compare each metric against the benchmark for *equipment_type*."""
        bench = _BENCHMARKS.get(equipment_type, _BENCHMARKS["default"])
        result: dict[str, dict[str, Any]] = {}

        metric_bench_map = {
            "precision": "precision_min",
            "recall": "recall_min",
            "f1": "f1_min",
            "f1_score": "f1_min",
            "auc_roc": "auc_roc_min",
            "auc": "auc_roc_min",
        }

        for metric_key, bench_key in metric_bench_map.items():
            value = metrics.get(metric_key)
            if value is None:
                continue
            threshold = bench[bench_key]
            passed = value >= threshold
            result[metric_key] = {
                "value": round(value, 4),
                "threshold": threshold,
                "passed": passed,
                "delta": round(value - threshold, 4),
            }

        return result

    @staticmethod
    def _is_deploy_ready(comparison: dict[str, dict[str, Any]]) -> bool:
        """Model is deploy-ready only if every benchmarked metric passes."""
        if not comparison:
            return False
        return all(entry["passed"] for entry in comparison.values())

    # ── improvement suggestions ───────────────────────────────────────────

    @staticmethod
    def _suggest_improvements(
        metrics: dict[str, float],
        comparison: dict[str, dict[str, Any]],
        equipment_type: str,
    ) -> list[str]:
        """Generate actionable improvement suggestions."""
        suggestions: list[str] = []

        precision = metrics.get("precision", 1.0)
        recall = metrics.get("recall", 1.0)
        f1 = metrics.get("f1", metrics.get("f1_score", 1.0))
        auc = metrics.get("auc_roc", metrics.get("auc", 1.0))

        # Precision-recall trade-off hints
        if precision < recall - 0.10:
            suggestions.append(
                "Precision is significantly lower than recall.  Consider "
                "increasing the anomaly detection threshold to reduce false "
                "positives, or add hard-negative mining to the training loop."
            )
        elif recall < precision - 0.10:
            suggestions.append(
                "Recall is significantly lower than precision.  The model is "
                "missing anomalies.  Try lowering the detection threshold, "
                "augmenting the training set with more anomaly examples, or "
                "extending the sequence length."
            )

        # Low F1
        if f1 < 0.75:
            suggestions.append(
                "F1 score is below 0.75.  Review the data quality report — "
                "missing values, outliers or class imbalance may be degrading "
                "performance.  Focal loss or oversampling could help."
            )

        # Low AUC
        if auc < 0.85:
            suggestions.append(
                "AUC-ROC is below 0.85.  The model has poor discriminative "
                "ability.  Consider switching to a more expressive architecture "
                "(e.g. Sentinel) or increasing the hidden dimension."
            )

        # Equipment-specific tips
        if equipment_type == "chiller" and recall < 0.85:
            suggestions.append(
                "For chillers, recall above 0.85 is critical because missed "
                "faults can cause compressor damage.  Prioritise recall "
                "improvements."
            )
        if equipment_type == "boiler" and precision < 0.88:
            suggestions.append(
                "For boilers, high precision avoids unnecessary shutdowns.  "
                "Consider tuning the anomaly threshold upward."
            )

        # General fallback
        if not suggestions:
            suggestions.append(
                "All metrics look solid.  For marginal gains, try ensemble "
                "methods or hyperparameter search (learning rate, dropout)."
            )

        # Specific metric-level notes for failures
        for metric_key, entry in comparison.items():
            if not entry["passed"]:
                suggestions.append(
                    f"{metric_key} is {entry['delta']:+.4f} below the "
                    f"benchmark threshold of {entry['threshold']:.2f}."
                )

        return suggestions

    # ── summary ───────────────────────────────────────────────────────────

    @staticmethod
    def _build_summary(
        metrics: dict[str, float],
        comparison: dict[str, dict[str, Any]],
        deploy_ready: bool,
        improvements: list[str],
    ) -> str:
        """Human-readable evaluation summary."""
        lines: list[str] = ["## Model Evaluation Summary", ""]

        # Metrics table
        lines.append("| Metric | Value | Benchmark | Status |")
        lines.append("|--------|-------|-----------|--------|")
        for key, entry in comparison.items():
            status = "PASS" if entry["passed"] else "FAIL"
            lines.append(
                f"| {key} | {entry['value']:.4f} | "
                f">= {entry['threshold']:.2f} | {status} |"
            )

        lines.append("")
        lines.append(
            f"**Deploy ready:** {'Yes' if deploy_ready else 'No'}"
        )

        if improvements:
            lines.append("")
            lines.append("### Suggested improvements")
            for imp in improvements:
                lines.append(f"- {imp}")

        return "\n".join(lines)
