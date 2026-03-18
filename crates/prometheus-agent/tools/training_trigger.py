# =============================================================================
# File: training_trigger.py
# Description: API client tool that triggers model training jobs on the Prometheus platform
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 8, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Training Trigger Tool
=====================

Calls the Prometheus platform API to start a model-training job.

In production, this sends a POST request to the training service with the
selected architecture, hyperparameters and data reference.  The service
returns a ``run_id`` that can be used to poll status and retrieve metrics.
"""

from __future__ import annotations

import logging
import os
from typing import Any

import httpx

logger = logging.getLogger(__name__)

_TRAINING_URL: str = os.getenv(
    "TRAINING_SERVICE_URL", "http://localhost:3030"
)
_TRAINING_KEY: str = os.getenv("TRAINING_API_KEY", "")


# ── Public API ────────────────────────────────────────────────────────────


async def trigger_training(
    architecture_name: str,
    hyperparameters: dict[str, Any],
    equipment_type: str,
    *,
    dataset_id: str | None = None,
    tags: dict[str, str] | None = None,
    timeout: float = 30.0,
) -> dict[str, Any]:
    """Trigger a Prometheus training run.

    Parameters
    ----------
    architecture_name:
        One of ``lstm_autoencoder``, ``gru_predictor``, ``sentinel``.
    hyperparameters:
        Full hyperparameter dict (epochs, lr, batch_size, etc.).
    equipment_type:
        Equipment type key, e.g. ``"chiller"``.
    dataset_id:
        Optional reference to a pre-uploaded dataset.
    tags:
        Optional key-value metadata tags for the run.
    timeout:
        HTTP request timeout in seconds.

    Returns
    -------
    dict
        ``{"status": "triggered"|"error", "run_id": ..., ...}``
    """
    payload = {
        "architecture": architecture_name,
        "hyperparameters": hyperparameters,
        "equipment_type": equipment_type,
        "dataset_id": dataset_id,
        "tags": tags or {},
    }

    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {_TRAINING_KEY}",
    }

    logger.info(
        "Triggering training: arch=%s equip=%s",
        architecture_name,
        equipment_type,
    )

    try:
        async with httpx.AsyncClient(timeout=timeout) as client:
            response = await client.post(
                f"{_TRAINING_URL}/api/v1/training/start",
                json=payload,
                headers=headers,
            )
            response.raise_for_status()
            data = response.json()
            logger.info("Training triggered successfully: %s", data.get("run_id"))
            return {
                "status": "triggered",
                "run_id": data.get("run_id"),
                "message": data.get("message", "Training run started."),
                "estimated_duration_minutes": data.get("estimated_duration_minutes"),
            }
    except httpx.HTTPStatusError as exc:
        logger.error("Training API HTTP error: %s %s", exc.response.status_code, exc.response.text)
        return {
            "status": "error",
            "error": f"HTTP {exc.response.status_code}: {exc.response.text}",
        }
    except httpx.RequestError as exc:
        logger.error("Training API request error: %s", exc)
        return {
            "status": "error",
            "error": f"Request failed: {exc}",
        }


async def get_training_status(run_id: str, *, timeout: float = 15.0) -> dict[str, Any]:
    """Poll the status of an existing training run.

    Parameters
    ----------
    run_id:
        Identifier returned by ``trigger_training``.
    timeout:
        HTTP request timeout in seconds.

    Returns
    -------
    dict
        ``{"status": "running"|"completed"|"failed"|"error", ...}``
    """
    headers = {"Authorization": f"Bearer {_TRAINING_KEY}"}

    try:
        async with httpx.AsyncClient(timeout=timeout) as client:
            response = await client.get(
                f"{_TRAINING_URL}/api/v1/training/{run_id}",
                headers=headers,
            )
            response.raise_for_status()
            data = response.json()
            return {
                "status": data.get("status", "unknown"),
                "run_id": run_id,
                "progress": data.get("progress"),
                "current_epoch": data.get("current_epoch"),
                "total_epochs": data.get("total_epochs"),
                "metrics": data.get("metrics"),
                "started_at": data.get("started_at"),
                "estimated_completion": data.get("estimated_completion"),
            }
    except httpx.HTTPStatusError as exc:
        return {
            "status": "error",
            "error": f"HTTP {exc.response.status_code}: {exc.response.text}",
        }
    except httpx.RequestError as exc:
        return {
            "status": "error",
            "error": f"Request failed: {exc}",
        }


async def cancel_training(run_id: str, *, timeout: float = 15.0) -> dict[str, Any]:
    """Cancel a running training job.

    Parameters
    ----------
    run_id:
        Identifier returned by ``trigger_training``.
    timeout:
        HTTP request timeout in seconds.

    Returns
    -------
    dict
        ``{"status": "cancelled"|"error", ...}``
    """
    headers = {
        "Authorization": f"Bearer {_TRAINING_KEY}",
        "Content-Type": "application/json",
    }

    try:
        async with httpx.AsyncClient(timeout=timeout) as client:
            response = await client.post(
                f"{_TRAINING_URL}/api/v1/training/{run_id}/stop",
                headers=headers,
            )
            response.raise_for_status()
            return {"status": "cancelled", "run_id": run_id}
    except httpx.HTTPStatusError as exc:
        return {
            "status": "error",
            "error": f"HTTP {exc.response.status_code}: {exc.response.text}",
        }
    except httpx.RequestError as exc:
        return {
            "status": "error",
            "error": f"Request failed: {exc}",
        }
