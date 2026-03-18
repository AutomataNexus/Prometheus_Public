# =============================================================================
# File: architect.py
# Description: Specialist agent that recommends model architectures for equipment types
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 8, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Architect Agent
===============

Specialist agent that recommends a model architecture for a given equipment
type and data profile.  Three candidate architectures are supported:

* **LSTM Autoencoder** – best for dense, multi-variate time-series with long
  sequences and low anomaly ratios.  Learns a compressed normal-behaviour
  representation and flags reconstruction-error outliers.

* **GRU Predictor** – best for shorter sequences or when predicting a target
  variable (e.g. vibration in the next hour).  Lighter than LSTM AE.

* **Sentinel** – Prometheus' proprietary hybrid architecture combining
  convolutional feature extraction with attention-based sequence modelling.
  Recommended for complex, multi-modal equipment with high feature counts.
"""

from __future__ import annotations

import logging
from typing import Any

from gradient_adk import RequestContext

from tools.architecture_db import lookup_architecture

logger = logging.getLogger(__name__)


# ── Architecture rule engine ──────────────────────────────────────────────

_ARCHITECTURE_RULES: list[dict[str, Any]] = [
    {
        "name": "sentinel",
        "label": "Sentinel (Hybrid CNN-Attention)",
        "conditions": {
            "min_features": 12,
            "min_sequence_length": 96,
        },
        "priority": 3,
        "description": (
            "Prometheus Sentinel architecture.  Combines 1-D convolutional "
            "feature extraction with multi-head self-attention for long, "
            "high-dimensional sequences.  Best for complex equipment with "
            "many correlated sensor channels."
        ),
    },
    {
        "name": "lstm_autoencoder",
        "label": "LSTM Autoencoder",
        "conditions": {
            "min_features": 4,
            "min_sequence_length": 48,
            "max_anomaly_ratio": 0.05,
        },
        "priority": 2,
        "description": (
            "Encoder-decoder architecture using stacked LSTM layers.  "
            "Learns to reconstruct normal operating patterns; anomalies "
            "surface as high reconstruction error.  Works well when "
            "labelled anomalies are scarce (< 5 %)."
        ),
    },
    {
        "name": "gru_predictor",
        "label": "GRU Predictor",
        "conditions": {
            "max_features": 11,
            "max_sequence_length": 95,
        },
        "priority": 1,
        "description": (
            "Single-target GRU predictor.  Lightweight, fast to train, "
            "suitable for simpler equipment with fewer channels or "
            "shorter look-back windows."
        ),
    },
]

# Default hyperparameter suggestions per architecture
_DEFAULT_HYPERPARAMS: dict[str, dict[str, Any]] = {
    "sentinel": {
        "conv_filters": [64, 128],
        "attention_heads": 8,
        "hidden_dim": 256,
        "dropout": 0.2,
        "learning_rate": 5e-4,
        "sequence_length": 168,
        "batch_size": 32,
        "epochs": 150,
    },
    "lstm_autoencoder": {
        "encoder_layers": [128, 64],
        "decoder_layers": [64, 128],
        "latent_dim": 32,
        "dropout": 0.2,
        "learning_rate": 1e-3,
        "sequence_length": 48,
        "batch_size": 64,
        "epochs": 100,
    },
    "gru_predictor": {
        "hidden_dim": 64,
        "num_layers": 2,
        "dropout": 0.1,
        "learning_rate": 1e-3,
        "sequence_length": 24,
        "batch_size": 128,
        "epochs": 80,
    },
}


class ArchitectAgent:
    """Recommend an anomaly-detection architecture for a given data profile."""

    # ── public API ────────────────────────────────────────────────────────

    async def recommend(
        self,
        message: str,
        data_analysis: dict[str, Any] | None,
        equipment_type: str | None,
        conversation_context: Any,
        request_context: RequestContext,
    ) -> dict[str, Any]:
        """Produce an architecture recommendation.

        Parameters
        ----------
        message:
            Natural-language request.
        data_analysis:
            Output of ``DataAnalystAgent.analyze`` (may be ``None``).
        equipment_type:
            E.g. ``"chiller"``, ``"ahu"``, ``"boiler"``.
        conversation_context:
            Shared orchestrator state.
        request_context:
            ADK request context.

        Returns
        -------
        dict
            ``{"recommended_architecture", "rationale", "hyperparameters",
            "alternatives", "equipment_type", "sequence_length",
            "knowledge_base_match"}``
        """
        feature_count = (data_analysis or {}).get("feature_count", 6)
        row_count = (data_analysis or {}).get("row_count", 1000)
        quality_score = (data_analysis or {}).get("quality_score", 80.0)
        anomaly_ratio = self._estimate_anomaly_ratio(data_analysis)

        # Derive a reasonable sequence length from row count
        sequence_length = self._suggest_sequence_length(row_count, feature_count)

        # Score architectures
        scored = self._score_architectures(
            feature_count=feature_count,
            sequence_length=sequence_length,
            anomaly_ratio=anomaly_ratio,
        )

        best = scored[0]
        alternatives = scored[1:]

        # Cross-reference knowledge base for equipment-specific guidance
        kb_match: dict[str, Any] | None = None
        if equipment_type:
            kb_match = await lookup_architecture(equipment_type)

        hyperparams = _DEFAULT_HYPERPARAMS.get(best["name"], {}).copy()
        hyperparams["sequence_length"] = sequence_length

        return {
            "recommended_architecture": best["name"],
            "label": best["label"],
            "rationale": self._build_rationale(
                best, feature_count, sequence_length, anomaly_ratio, quality_score
            ),
            "hyperparameters": hyperparams,
            "alternatives": [
                {"name": a["name"], "label": a["label"], "score": a["score"]}
                for a in alternatives
            ],
            "equipment_type": equipment_type or "unknown",
            "sequence_length": sequence_length,
            "knowledge_base_match": kb_match,
        }

    # ── scoring ───────────────────────────────────────────────────────────

    def _score_architectures(
        self,
        feature_count: int,
        sequence_length: int,
        anomaly_ratio: float,
    ) -> list[dict[str, Any]]:
        """Return architectures sorted by descending suitability score."""
        results: list[dict[str, Any]] = []

        for rule in _ARCHITECTURE_RULES:
            score = rule["priority"] * 10  # base score from priority
            conds = rule["conditions"]

            # Feature count checks
            if "min_features" in conds and feature_count >= conds["min_features"]:
                score += 20
            if "max_features" in conds and feature_count <= conds["max_features"]:
                score += 10

            # Sequence length checks
            if "min_sequence_length" in conds and sequence_length >= conds["min_sequence_length"]:
                score += 20
            if "max_sequence_length" in conds and sequence_length <= conds["max_sequence_length"]:
                score += 10

            # Anomaly ratio
            if "max_anomaly_ratio" in conds and anomaly_ratio <= conds["max_anomaly_ratio"]:
                score += 15

            results.append({
                "name": rule["name"],
                "label": rule["label"],
                "score": score,
                "description": rule["description"],
            })

        results.sort(key=lambda r: r["score"], reverse=True)
        return results

    # ── helpers ───────────────────────────────────────────────────────────

    @staticmethod
    def _estimate_anomaly_ratio(analysis: dict[str, Any] | None) -> float:
        """Heuristic: derive anomaly ratio from outlier information."""
        if analysis is None:
            return 0.02  # optimistic default
        outliers = analysis.get("outliers", {})
        total = outliers.get("total_outliers", 0)
        rows = analysis.get("row_count", 1)
        return min(total / max(rows, 1), 1.0)

    @staticmethod
    def _suggest_sequence_length(row_count: int, feature_count: int) -> int:
        """Heuristic sequence-length suggestion.

        Longer look-back for large datasets / many features.
        """
        if row_count > 50_000 and feature_count >= 12:
            return 168  # ~1 week at hourly resolution
        if row_count > 10_000:
            return 48   # ~2 days
        return 24       # ~1 day

    @staticmethod
    def _build_rationale(
        arch: dict[str, Any],
        feature_count: int,
        sequence_length: int,
        anomaly_ratio: float,
        quality_score: float,
    ) -> str:
        """Human-readable explanation of the recommendation."""
        lines = [
            f"Recommended **{arch['label']}** (score {arch['score']}).",
            "",
            arch["description"],
            "",
            "Data profile considered:",
            f"  - Feature count: {feature_count}",
            f"  - Sequence length: {sequence_length}",
            f"  - Estimated anomaly ratio: {anomaly_ratio:.2%}",
            f"  - Data quality score: {quality_score}/100",
        ]
        if quality_score < 60:
            lines.append(
                "\n**Warning:** Data quality is below 60.  Consider cleaning "
                "the dataset before training."
            )
        return "\n".join(lines)
