# =============================================================================
# File: architecture_db.py
# Description: Gradient AI knowledge-base lookup for architecture recommendations
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 15, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Architecture Database Tool
==========================

Queries the DigitalOcean Gradient AI agent for architecture recommendations
based on equipment type, dataset characteristics, and historical training
results stored in the Prometheus knowledge base.

The Gradient AI agent has access to the uploaded knowledge documents
(equipment_specs.md, hvac_fault_catalog.md, axonml_reference.md,
training_history.md) and uses retrieval-augmented generation to provide
context-aware recommendations.

A local reference cache provides instant responses for known equipment
types and serves as a fallback if the Gradient API is unreachable.
"""

from __future__ import annotations

import json
import logging
import os
from typing import Any

import httpx

logger = logging.getLogger(__name__)


# ── Gradient AI configuration ────────────────────────────────────────────

_GENAI_ENDPOINT: str = os.getenv("DO_GENAI_ENDPOINT", "")
_GENAI_ACCESS_KEY: str = os.getenv("DO_GENAI_ACCESS_KEY", "")
_REQUEST_TIMEOUT: float = 30.0


# ── Reference cache (validated benchmark data, also serves as fallback) ──

_ARCHITECTURE_CACHE: dict[str, dict[str, Any]] = {
    "ahu": {
        "equipment_type": "Air Handling Unit (AHU)",
        "primary_architecture": "lstm_autoencoder",
        "secondary_architecture": "sentinel",
        "rationale": (
            "AHUs produce multi-variate time-series across supply/return "
            "temperatures, fan speed, damper position, and filter differential "
            "pressure.  LSTM Autoencoder captures normal operating envelopes "
            "well; Sentinel is preferred when coil valve and economiser "
            "signals are also available (>12 channels)."
        ),
        "typical_sensors": [
            "supply_air_temp", "return_air_temp", "mixed_air_temp",
            "fan_speed", "fan_power", "damper_position", "filter_dp",
            "cooling_valve", "heating_valve", "supply_air_humidity",
        ],
        "recommended_hyperparameters": {
            "sequence_length": 48, "encoder_layers": [128, 64],
            "latent_dim": 32, "learning_rate": 1e-3,
            "batch_size": 64, "epochs": 100,
        },
        "benchmark_f1": 0.87,
        "benchmark_auc": 0.93,
    },
    "boiler": {
        "equipment_type": "Boiler",
        "primary_architecture": "lstm_autoencoder",
        "secondary_architecture": "gru_predictor",
        "rationale": (
            "Boilers have relatively few but safety-critical channels "
            "(supply/return water temp, flue temp, pressure, flame signal). "
            "LSTM Autoencoder works well with moderate feature counts.  "
            "GRU Predictor is viable for single-target prediction "
            "(e.g. flue temperature forecasting)."
        ),
        "typical_sensors": [
            "supply_water_temp", "return_water_temp", "flue_gas_temp",
            "steam_pressure", "flame_signal", "combustion_air_flow",
            "fuel_flow_rate", "stack_o2",
        ],
        "recommended_hyperparameters": {
            "sequence_length": 48, "encoder_layers": [64, 32],
            "latent_dim": 16, "learning_rate": 1e-3,
            "batch_size": 64, "epochs": 80,
        },
        "benchmark_f1": 0.89,
        "benchmark_auc": 0.94,
    },
    "chiller": {
        "equipment_type": "Chiller",
        "primary_architecture": "sentinel",
        "secondary_architecture": "lstm_autoencoder",
        "rationale": (
            "Chillers are complex machines with many interrelated channels "
            "(compressor amps, condenser/evaporator pressures & temps, "
            "refrigerant levels, oil pressure).  The Sentinel architecture's "
            "CNN-attention hybrid excels at capturing cross-channel "
            "dependencies in these high-dimensional datasets."
        ),
        "typical_sensors": [
            "evaporator_leaving_temp", "evaporator_entering_temp",
            "condenser_leaving_temp", "condenser_entering_temp",
            "compressor_amps", "compressor_discharge_temp",
            "evaporator_pressure", "condenser_pressure", "oil_pressure",
            "refrigerant_level", "chilled_water_flow",
            "condenser_water_flow", "power_consumption", "cop",
        ],
        "recommended_hyperparameters": {
            "sequence_length": 168, "conv_filters": [64, 128],
            "attention_heads": 8, "hidden_dim": 256,
            "learning_rate": 5e-4, "batch_size": 32, "epochs": 150,
        },
        "benchmark_f1": 0.86,
        "benchmark_auc": 0.92,
    },
    "pump": {
        "equipment_type": "Pump",
        "primary_architecture": "gru_predictor",
        "secondary_architecture": "lstm_autoencoder",
        "rationale": (
            "Pumps typically expose fewer channels (vibration, flow, "
            "differential pressure, motor current).  GRU Predictor "
            "provides fast training and good accuracy for this simpler "
            "signal space.  Upgrade to LSTM AE if vibration spectra "
            "are included."
        ),
        "typical_sensors": [
            "vibration_x", "vibration_y", "vibration_z", "flow_rate",
            "differential_pressure", "motor_current", "bearing_temp",
            "suction_pressure",
        ],
        "recommended_hyperparameters": {
            "sequence_length": 24, "hidden_dim": 64, "num_layers": 2,
            "learning_rate": 1e-3, "batch_size": 128, "epochs": 80,
        },
        "benchmark_f1": 0.85,
        "benchmark_auc": 0.91,
    },
    "fan_coil": {
        "equipment_type": "Fan Coil Unit",
        "primary_architecture": "gru_predictor",
        "secondary_architecture": "lstm_autoencoder",
        "rationale": (
            "Fan coil units are relatively simple HVAC terminals with few "
            "sensors (discharge temp, fan speed, valve position, room temp). "
            "GRU Predictor is the most cost-effective choice."
        ),
        "typical_sensors": [
            "discharge_air_temp", "room_temp", "room_humidity",
            "fan_speed", "valve_position", "return_air_temp",
        ],
        "recommended_hyperparameters": {
            "sequence_length": 24, "hidden_dim": 32, "num_layers": 2,
            "learning_rate": 1e-3, "batch_size": 128, "epochs": 60,
        },
        "benchmark_f1": 0.83,
        "benchmark_auc": 0.89,
    },
    "steam": {
        "equipment_type": "Steam System",
        "primary_architecture": "lstm_autoencoder",
        "secondary_architecture": "sentinel",
        "rationale": (
            "Steam distribution systems have moderate complexity with "
            "pressure, temperature and flow across multiple points.  "
            "LSTM Autoencoder captures the thermodynamic relationships.  "
            "Use Sentinel if condensate-return and make-up water sensors "
            "push the feature count above 12."
        ),
        "typical_sensors": [
            "header_pressure", "header_temp", "steam_flow",
            "condensate_temp", "condensate_flow", "feedwater_temp",
            "deaerator_pressure", "blowdown_rate", "makeup_water_flow",
            "stack_temp",
        ],
        "recommended_hyperparameters": {
            "sequence_length": 48, "encoder_layers": [128, 64],
            "latent_dim": 32, "learning_rate": 1e-3,
            "batch_size": 64, "epochs": 100,
        },
        "benchmark_f1": 0.86,
        "benchmark_auc": 0.92,
    },
}


# ── Gradient AI knowledge-base query ─────────────────────────────────────


async def _query_gradient_kb(
    equipment_type: str,
    *,
    extra_context: str = "",
) -> dict[str, Any] | None:
    """Query the Gradient AI agent for architecture recommendations.

    Sends a structured prompt to the DO GenAI chat completions endpoint,
    which has access to the Prometheus knowledge base documents for
    retrieval-augmented generation.

    Returns parsed JSON recommendation or None on failure.
    """
    if not _GENAI_ENDPOINT or not _GENAI_ACCESS_KEY:
        logger.debug("Gradient AI not configured, using local cache")
        return None

    prompt = (
        f"You are the Prometheus architecture advisor. Based on the knowledge base, "
        f"recommend the best ML architecture for a '{equipment_type}' system.\n\n"
        f"Return a JSON object with these fields:\n"
        f"- equipment_type: full name\n"
        f"- primary_architecture: one of lstm_autoencoder, gru_predictor, sentinel, "
        f"resnet, vgg, vit, bert, gpt2, rnn, conv1d, conv2d, nexus, phantom\n"
        f"- secondary_architecture: fallback architecture\n"
        f"- rationale: 2-3 sentence explanation\n"
        f"- typical_sensors: list of expected sensor/feature names\n"
        f"- recommended_hyperparameters: dict with sequence_length, learning_rate, "
        f"batch_size, epochs, and architecture-specific params\n"
        f"- benchmark_f1: expected F1 score (0.0-1.0)\n"
        f"- benchmark_auc: expected AUC-ROC (0.0-1.0)\n\n"
        f"Return ONLY valid JSON, no markdown fences."
    )
    if extra_context:
        prompt += f"\n\nAdditional context:\n{extra_context}"

    body = {
        "model": "agent",
        "messages": [
            {
                "role": "system",
                "content": (
                    "You are PrometheusForge, an AI architecture advisor for the "
                    "Prometheus ML training platform. You have access to the knowledge "
                    "base containing equipment specifications, fault catalogs, AxonML "
                    "architecture references, and historical training results. "
                    "Always respond with valid JSON."
                ),
            },
            {"role": "user", "content": prompt},
        ],
        "max_tokens": 2000,
    }

    try:
        async with httpx.AsyncClient(timeout=_REQUEST_TIMEOUT) as client:
            resp = await client.post(
                _GENAI_ENDPOINT,
                headers={
                    "Authorization": f"Bearer {_GENAI_ACCESS_KEY}",
                    "Content-Type": "application/json",
                },
                json=body,
            )
            resp.raise_for_status()
            data = resp.json()

        # Parse OpenAI-compatible response
        content = (
            data.get("choices", [{}])[0]
            .get("message", {})
            .get("content", "")
        )
        if not content:
            content = data.get("response", "")

        # Strip markdown fences if present
        content = content.strip()
        if content.startswith("```"):
            content = content.split("\n", 1)[-1]
        if content.endswith("```"):
            content = content.rsplit("```", 1)[0]
        content = content.strip()

        record = json.loads(content)
        logger.info(
            "Gradient AI recommendation for %s -> %s",
            equipment_type,
            record.get("primary_architecture", "unknown"),
        )
        return record

    except httpx.HTTPStatusError as e:
        logger.warning("Gradient AI HTTP error: %s", e.response.status_code)
        return None
    except (json.JSONDecodeError, KeyError, IndexError) as e:
        logger.warning("Gradient AI response parse error: %s", e)
        return None
    except httpx.RequestError as e:
        logger.warning("Gradient AI connection error: %s", e)
        return None


# ── Public API ────────────────────────────────────────────────────────────


async def lookup_architecture(
    equipment_type: str,
    *,
    extra_context: str = "",
) -> dict[str, Any] | None:
    """Look up the best architecture for *equipment_type*.

    Queries the Gradient AI knowledge base first.  If the API is unavailable
    or the equipment type is not recognised, falls back to the local
    reference cache built from validated benchmark data.

    Parameters
    ----------
    equipment_type:
        Equipment or domain key, e.g. ``"chiller"``, ``"ahu"``, ``"medical"``.
    extra_context:
        Optional additional context (e.g. dataset column names, row count)
        to include in the Gradient AI prompt for more specific recommendations.

    Returns
    -------
    dict | None
        Full architecture record or ``None`` if the type is unknown.
    """
    key = equipment_type.lower().strip().replace(" ", "_")

    # Try Gradient AI knowledge base first
    gradient_result = await _query_gradient_kb(key, extra_context=extra_context)
    if gradient_result is not None:
        # Cache the result for future fast lookups
        _ARCHITECTURE_CACHE[key] = gradient_result
        return gradient_result

    # Fall back to local reference cache
    record = _ARCHITECTURE_CACHE.get(key)
    if record is None:
        logger.warning("No architecture record for equipment type: %s", key)
        return None
    logger.info(
        "Architecture cache hit for %s -> %s (Gradient AI unavailable)",
        key,
        record["primary_architecture"],
    )
    return record


async def list_equipment_types() -> list[str]:
    """Return all supported equipment type keys."""
    return sorted(_ARCHITECTURE_CACHE.keys())


async def get_benchmark(
    equipment_type: str,
) -> dict[str, float] | None:
    """Return benchmark metrics for the given equipment type."""
    record = await lookup_architecture(equipment_type)
    if record is None:
        return None
    return {
        "f1": record.get("benchmark_f1", 0.0),
        "auc_roc": record.get("benchmark_auc", 0.0),
    }
