# =============================================================================
# File: sensor_analysis.py
# Description: Statistical analysis tool for raw sensor CSV data via Gradient ADK
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 8, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Sensor Analysis Tool
====================

Gradient ADK tool that performs statistical analysis on raw sensor CSV data.
Designed to be invoked by the Athena orchestrator or directly via the tool
registry.

The tool accepts a CSV string, runs a comprehensive statistical pipeline, and
returns a structured JSON report.
"""

from __future__ import annotations

import io
import logging
from typing import Any

import numpy as np
import pandas as pd

logger = logging.getLogger(__name__)


# ── Public tool function ──────────────────────────────────────────────────


async def analyze_sensor_csv(
    csv_data: str,
    *,
    timestamp_column: str | None = None,
    iqr_factor: float = 1.5,
    seasonality_min_rows: int = 50,
    seasonality_max_lag: int = 500,
) -> dict[str, Any]:
    """Analyse a sensor CSV and return a structured report.

    Parameters
    ----------
    csv_data:
        Raw CSV text with a header row.
    timestamp_column:
        Name of the datetime column.  Auto-detected if ``None``.
    iqr_factor:
        Multiplier for IQR-based outlier detection.
    seasonality_min_rows:
        Minimum rows required to attempt seasonality analysis.
    seasonality_max_lag:
        Maximum lag for autocorrelation computation.

    Returns
    -------
    dict
        Keys: ``status``, ``shape``, ``columns``, ``statistics``,
        ``missing_values``, ``duplicates``, ``outliers``, ``correlations``,
        ``seasonality``, ``quality_score``.
    """
    try:
        df = pd.read_csv(
            io.StringIO(csv_data), parse_dates=True, infer_datetime_format=True
        )
    except Exception as exc:
        logger.error("CSV parse error: %s", exc)
        return {"status": "error", "error": f"Failed to parse CSV: {exc}"}

    if df.empty:
        return {"status": "error", "error": "CSV parsed but produced an empty DataFrame."}

    # Resolve timestamp column
    ts_col = _resolve_timestamp_column(df, timestamp_column)

    numeric_df = df.select_dtypes(include="number")

    report: dict[str, Any] = {
        "status": "success",
        "shape": {"rows": int(df.shape[0]), "columns": int(df.shape[1])},
        "column_names": list(df.columns),
        "numeric_columns": list(numeric_df.columns),
        "timestamp_column": ts_col,
        "statistics": _descriptive_stats(numeric_df),
        "missing_values": _missing_values(df),
        "duplicates": _duplicate_rows(df),
        "outliers": _detect_outliers(numeric_df, iqr_factor),
        "correlations": _top_correlations(numeric_df, top_n=10),
        "seasonality": _seasonality(
            df, numeric_df, ts_col, seasonality_min_rows, seasonality_max_lag
        ),
    }
    report["quality_score"] = _quality_score(report)
    return report


# ── Internal helpers ──────────────────────────────────────────────────────


def _resolve_timestamp_column(
    df: pd.DataFrame, hint: str | None
) -> str | None:
    """Find the datetime column, trying *hint* first then common names."""
    if hint and hint in df.columns:
        try:
            df[hint] = pd.to_datetime(df[hint])
            return hint
        except Exception:
            pass

    for col in df.columns:
        if pd.api.types.is_datetime64_any_dtype(df[col]):
            return col

    for candidate in ("timestamp", "time", "datetime", "date", "ts", "Timestamp"):
        if candidate in df.columns:
            try:
                df[candidate] = pd.to_datetime(df[candidate])
                return candidate
            except Exception:
                continue
    return None


def _descriptive_stats(numeric_df: pd.DataFrame) -> dict[str, dict[str, float]]:
    """Per-column descriptive statistics."""
    result: dict[str, dict[str, float]] = {}
    for col in numeric_df.columns:
        s = numeric_df[col].dropna()
        if s.empty:
            result[col] = {
                "mean": 0.0, "std": 0.0, "min": 0.0, "max": 0.0,
                "median": 0.0, "skew": 0.0, "kurtosis": 0.0, "count": 0,
            }
            continue
        result[col] = {
            "count": int(s.count()),
            "mean": round(float(s.mean()), 6),
            "std": round(float(s.std()), 6),
            "min": round(float(s.min()), 6),
            "max": round(float(s.max()), 6),
            "median": round(float(s.median()), 6),
            "skew": round(float(s.skew()), 6) if len(s) > 2 else 0.0,
            "kurtosis": round(float(s.kurtosis()), 6) if len(s) > 3 else 0.0,
        }
    return result


def _missing_values(df: pd.DataFrame) -> dict[str, Any]:
    """Audit missing values per column."""
    total_cells = df.shape[0] * df.shape[1]
    total_missing = int(df.isna().sum().sum())
    cols: dict[str, dict[str, Any]] = {}
    for col in df.columns:
        n = int(df[col].isna().sum())
        if n > 0:
            cols[col] = {
                "count": n,
                "percentage": round(n / df.shape[0] * 100, 2),
            }
    return {
        "total_missing": total_missing,
        "total_cells": total_cells,
        "overall_percentage": round(total_missing / max(total_cells, 1) * 100, 2),
        "columns": cols,
    }


def _duplicate_rows(df: pd.DataFrame) -> dict[str, Any]:
    """Count fully-duplicated rows."""
    n = int(df.duplicated().sum())
    return {
        "count": n,
        "percentage": round(n / max(df.shape[0], 1) * 100, 2),
    }


def _detect_outliers(
    numeric_df: pd.DataFrame, iqr_factor: float
) -> dict[str, Any]:
    """IQR-based outlier detection."""
    total = 0
    cols: dict[str, dict[str, Any]] = {}
    for col in numeric_df.columns:
        s = numeric_df[col].dropna()
        if len(s) < 4:
            continue
        q1 = float(np.percentile(s, 25))
        q3 = float(np.percentile(s, 75))
        iqr = q3 - q1
        lo = q1 - iqr_factor * iqr
        hi = q3 + iqr_factor * iqr
        n = int(((s < lo) | (s > hi)).sum())
        if n:
            cols[col] = {
                "count": n,
                "percentage": round(n / len(s) * 100, 2),
                "lower_fence": round(lo, 4),
                "upper_fence": round(hi, 4),
            }
            total += n
    return {"total": total, "columns": cols}


def _top_correlations(
    numeric_df: pd.DataFrame, top_n: int = 10
) -> list[dict[str, Any]]:
    """Return the *top_n* strongest pairwise Pearson correlations."""
    if numeric_df.shape[1] < 2:
        return []
    corr = numeric_df.corr()
    pairs: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()
    for i, col_a in enumerate(corr.columns):
        for j, col_b in enumerate(corr.columns):
            if i >= j:
                continue
            key = (col_a, col_b)
            if key in seen:
                continue
            seen.add(key)
            val = corr.iloc[i, j]
            if pd.notna(val):
                pairs.append({
                    "feature_a": col_a,
                    "feature_b": col_b,
                    "correlation": round(float(val), 4),
                })
    pairs.sort(key=lambda p: abs(p["correlation"]), reverse=True)
    return pairs[:top_n]


def _seasonality(
    df: pd.DataFrame,
    numeric_df: pd.DataFrame,
    ts_col: str | None,
    min_rows: int,
    max_lag: int,
) -> dict[str, Any]:
    """Autocorrelation-based seasonality detection."""
    if ts_col is None:
        return {"detected": False, "reason": "No timestamp column found."}
    if numeric_df.empty or len(df) < min_rows:
        return {"detected": False, "reason": "Insufficient data for seasonality analysis."}

    hints: dict[str, dict[str, Any]] = {}
    for col in numeric_df.columns:
        s = numeric_df[col].dropna()
        if len(s) < min_rows:
            continue
        lag_limit = min(len(s) // 2, max_lag)
        autocorrs = [float(s.autocorr(lag=lag)) for lag in range(1, lag_limit + 1)]
        if not autocorrs:
            continue
        peak_idx = int(np.argmax(autocorrs))
        peak_val = autocorrs[peak_idx]
        if peak_val > 0.3:
            hints[col] = {
                "dominant_lag": peak_idx + 1,
                "autocorrelation": round(peak_val, 4),
            }

    if hints:
        return {"detected": True, "columns": hints}
    return {"detected": False, "reason": "No significant periodicity found."}


def _quality_score(report: dict[str, Any]) -> float:
    """Compute 0-100 quality score from the report sections."""
    score = 100.0

    # Missing values penalty
    missing_pct = report["missing_values"]["overall_percentage"]
    if missing_pct > 30:
        score -= 40
    elif missing_pct > 5:
        score -= missing_pct

    # Outlier penalty
    rows = report["shape"]["rows"]
    outlier_ratio = report["outliers"]["total"] / max(rows, 1)
    if outlier_ratio > 0.10:
        score -= 20
    elif outlier_ratio > 0.02:
        score -= 10

    # Duplicate penalty
    dup_pct = report["duplicates"]["percentage"]
    if dup_pct > 5:
        score -= 15

    # Small dataset penalty
    if rows < 500:
        score -= 10

    return round(max(score, 0.0), 1)
