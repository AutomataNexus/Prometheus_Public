# =============================================================================
# File: model_evaluator.py
# Description: Fetches and normalizes model evaluation metrics from the metrics service
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 8, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Model Evaluator Tool
====================

Fetches evaluation metrics from the Prometheus metrics service for a given
model run, normalises them and returns a structured report that the
EvaluatorAgent can interpret.
"""

from __future__ import annotations

import logging
import os
from typing import Any

import httpx

logger = logging.getLogger(__name__)

_METRICS_URL: str = os.getenv(
    "METRICS_SERVICE_URL", "http://localhost:3030"
)
_METRICS_KEY: str = os.getenv("METRICS_API_KEY", "")


# ── Public API ────────────────────────────────────────────────────────────


async def fetch_evaluation_metrics(
    model_id: str,
    *,
    timeout: float = 15.0,
) -> dict[str, Any]:
    """Fetch evaluation metrics for *model_id* from the metrics service.

    Parameters
    ----------
    model_id:
        Unique model or run identifier.
    timeout:
        HTTP request timeout in seconds.

    Returns
    -------
    dict
        ``{"status", "model_id", "metrics", "metadata"}``
    """
    headers = {"Authorization": f"Bearer {_METRICS_KEY}"}

    try:
        async with httpx.AsyncClient(timeout=timeout) as client:
            response = await client.get(
                f"{_METRICS_URL}/api/v1/models/{model_id}",
                headers=headers,
            )
            response.raise_for_status()
            data = response.json()

            metrics = _normalise_metrics(data.get("metrics", {}))

            return {
                "status": "success",
                "model_id": model_id,
                "metrics": metrics,
                "metadata": {
                    "architecture": data.get("architecture"),
                    "equipment_type": data.get("equipment_type"),
                    "trained_at": data.get("trained_at"),
                    "training_duration_minutes": data.get("training_duration_minutes"),
                    "dataset_size": data.get("dataset_size"),
                },
            }
    except httpx.HTTPStatusError as exc:
        logger.error("Metrics API HTTP error: %s", exc.response.status_code)
        return {
            "status": "error",
            "model_id": model_id,
            "error": f"HTTP {exc.response.status_code}: {exc.response.text}",
        }
    except httpx.RequestError as exc:
        logger.error("Metrics API request error: %s", exc)
        return {
            "status": "error",
            "model_id": model_id,
            "error": f"Request failed: {exc}",
        }


async def fetch_confusion_matrix(
    model_id: str,
    *,
    timeout: float = 15.0,
) -> dict[str, Any]:
    """Fetch the confusion matrix for *model_id*.

    Returns
    -------
    dict
        ``{"status", "model_id", "matrix"}``
        where *matrix* has keys ``tp``, ``fp``, ``tn``, ``fn``.
    """
    headers = {"Authorization": f"Bearer {_METRICS_KEY}"}

    try:
        async with httpx.AsyncClient(timeout=timeout) as client:
            response = await client.get(
                f"{_METRICS_URL}/api/v1/evaluations/{model_id}/gradient",
                headers=headers,
            )
            response.raise_for_status()
            data = response.json()
            return {
                "status": "success",
                "model_id": model_id,
                "matrix": {
                    "tp": data.get("true_positives", 0),
                    "fp": data.get("false_positives", 0),
                    "tn": data.get("true_negatives", 0),
                    "fn": data.get("false_negatives", 0),
                },
            }
    except Exception as exc:
        return {
            "status": "error",
            "model_id": model_id,
            "error": str(exc),
        }


async def compare_models(
    model_ids: list[str],
    *,
    timeout: float = 20.0,
) -> dict[str, Any]:
    """Fetch metrics for multiple models and return a comparison table.

    Parameters
    ----------
    model_ids:
        List of model/run identifiers to compare.
    timeout:
        Per-request HTTP timeout.

    Returns
    -------
    dict
        ``{"status", "models"}`` where *models* is a list of metric dicts.
    """
    results: list[dict[str, Any]] = []
    for mid in model_ids:
        result = await fetch_evaluation_metrics(mid, timeout=timeout)
        results.append(result)

    # Determine overall best by F1
    successful = [r for r in results if r["status"] == "success"]
    best_id: str | None = None
    if successful:
        best = max(
            successful,
            key=lambda r: r["metrics"].get("f1", 0.0),
        )
        best_id = best["model_id"]

    return {
        "status": "success" if successful else "error",
        "models": results,
        "best_model_id": best_id,
    }


# ── Helpers ───────────────────────────────────────────────────────────────


def _normalise_metrics(raw: dict[str, Any]) -> dict[str, float]:
    """Normalise metric keys to a canonical set.

    The API may return ``f1_score`` or ``f1``, ``auc_roc`` or ``roc_auc``,
    etc.  We canonicalise to: ``precision``, ``recall``, ``f1``, ``auc_roc``,
    ``accuracy``, ``mse``, ``mae``.
    """
    mapping: dict[str, str] = {
        "f1_score": "f1",
        "roc_auc": "auc_roc",
        "auc": "auc_roc",
        "mean_squared_error": "mse",
        "mean_absolute_error": "mae",
    }

    normalised: dict[str, float] = {}
    for key, value in raw.items():
        canon = mapping.get(key, key)
        if isinstance(value, (int, float)):
            normalised[canon] = round(float(value), 6)

    return normalised
