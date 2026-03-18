# =============================================================================
# File: data_analyst.py
# Description: Specialist agent for sensor data analysis with statistics and quality scoring
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 8, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Data Analyst Agent
==================

Specialist agent that ingests sensor data (CSV payload or reference) and
produces a structured analysis report covering:

* Descriptive statistics per feature
* Missing-value and duplicate-row audits
* Outlier detection (IQR method)
* Seasonality / periodicity hints
* Overall data-quality score
"""

from __future__ import annotations

import io
import logging
from typing import Any

import numpy as np
import pandas as pd
from gradient_adk import RequestContext

logger = logging.getLogger(__name__)


class DataAnalystAgent:
    """Analyse tabular sensor data and return a structured report."""

    # Configurable thresholds
    MISSING_WARN_THRESHOLD: float = 0.05   # >5 % missing  -> warning
    MISSING_FAIL_THRESHOLD: float = 0.30   # >30 % missing -> critical
    OUTLIER_IQR_FACTOR: float = 1.5

    # ── public API ────────────────────────────────────────────────────────

    async def analyze(
        self,
        message: str,
        data_payload: str | dict | None,
        conversation_context: Any,
        request_context: RequestContext,
    ) -> dict[str, Any]:
        """Run a full analysis pipeline on the supplied data.

        Parameters
        ----------
        message:
            Natural-language request from the user.
        data_payload:
            Raw CSV string **or** a dict with ``{"csv": "<csv_text>"}``
            or ``{"url": "<url_to_csv>"}``.
        conversation_context:
            Shared orchestrator context (may contain prior results).
        request_context:
            ADK request context.

        Returns
        -------
        dict
            Structured analysis report.
        """
        df = await self._load_data(data_payload)
        if df is None or df.empty:
            return {
                "status": "error",
                "error": "No data provided or data could not be parsed.",
            }

        logger.info("Analysing dataframe with shape %s", df.shape)

        stats = self._descriptive_statistics(df)
        missing = self._missing_value_audit(df)
        duplicates = self._duplicate_audit(df)
        outliers = self._outlier_detection(df)
        seasonality = self._seasonality_hints(df)
        quality = self._compute_quality_score(missing, outliers, duplicates, df)

        return {
            "status": "success",
            "row_count": int(df.shape[0]),
            "feature_count": int(df.shape[1]),
            "statistics": stats,
            "missing_values": missing,
            "duplicates": duplicates,
            "outliers": outliers,
            "seasonality": seasonality,
            "quality_score": quality["score"],
            "quality_issues": quality["issues"],
        }

    # ── data loading ──────────────────────────────────────────────────────

    async def _load_data(self, payload: str | dict | None) -> pd.DataFrame | None:
        """Parse incoming payload into a pandas DataFrame."""
        if payload is None:
            return None

        if isinstance(payload, str):
            return self._csv_to_df(payload)

        if isinstance(payload, dict):
            if "csv" in payload:
                return self._csv_to_df(payload["csv"])
            if "url" in payload:
                return await self._fetch_csv(payload["url"])

        return None

    @staticmethod
    def _csv_to_df(csv_text: str) -> pd.DataFrame:
        """Convert raw CSV text into a DataFrame."""
        try:
            return pd.read_csv(io.StringIO(csv_text), parse_dates=True, infer_datetime_format=True)
        except Exception:
            logger.exception("Failed to parse CSV text")
            return pd.DataFrame()

    @staticmethod
    async def _fetch_csv(url: str) -> pd.DataFrame:
        """Download a CSV from *url* and parse it."""
        import httpx

        try:
            async with httpx.AsyncClient(timeout=60) as client:
                resp = await client.get(url)
                resp.raise_for_status()
                return pd.read_csv(io.StringIO(resp.text), parse_dates=True, infer_datetime_format=True)
        except Exception:
            logger.exception("Failed to fetch CSV from %s", url)
            return pd.DataFrame()

    # ── analysis primitives ───────────────────────────────────────────────

    @staticmethod
    def _descriptive_statistics(df: pd.DataFrame) -> dict[str, dict[str, float]]:
        """Per-column descriptive stats for numeric features."""
        numeric = df.select_dtypes(include="number")
        result: dict[str, dict[str, float]] = {}
        for col in numeric.columns:
            series = numeric[col].dropna()
            result[col] = {
                "mean": float(series.mean()) if len(series) else 0.0,
                "std": float(series.std()) if len(series) else 0.0,
                "min": float(series.min()) if len(series) else 0.0,
                "max": float(series.max()) if len(series) else 0.0,
                "median": float(series.median()) if len(series) else 0.0,
                "skew": float(series.skew()) if len(series) > 2 else 0.0,
                "kurtosis": float(series.kurtosis()) if len(series) > 3 else 0.0,
            }
        return result

    def _missing_value_audit(self, df: pd.DataFrame) -> dict[str, Any]:
        """Identify columns with missing values and severity."""
        total = len(df)
        if total == 0:
            return {"total_missing": 0, "columns": {}}

        cols: dict[str, dict[str, Any]] = {}
        for col in df.columns:
            n_missing = int(df[col].isna().sum())
            if n_missing > 0:
                ratio = n_missing / total
                severity = "critical" if ratio > self.MISSING_FAIL_THRESHOLD else (
                    "warning" if ratio > self.MISSING_WARN_THRESHOLD else "info"
                )
                cols[col] = {
                    "count": n_missing,
                    "percentage": round(ratio * 100, 2),
                    "severity": severity,
                }

        return {
            "total_missing": int(df.isna().sum().sum()),
            "columns": cols,
        }

    @staticmethod
    def _duplicate_audit(df: pd.DataFrame) -> dict[str, Any]:
        """Check for fully-duplicated rows."""
        n_dup = int(df.duplicated().sum())
        return {
            "duplicate_rows": n_dup,
            "percentage": round(n_dup / max(len(df), 1) * 100, 2),
        }

    def _outlier_detection(self, df: pd.DataFrame) -> dict[str, Any]:
        """IQR-based outlier detection on every numeric column."""
        numeric = df.select_dtypes(include="number")
        result: dict[str, dict[str, Any]] = {}
        total_outliers = 0

        for col in numeric.columns:
            series = numeric[col].dropna()
            if len(series) < 4:
                continue
            q1 = float(np.percentile(series, 25))
            q3 = float(np.percentile(series, 75))
            iqr = q3 - q1
            lower = q1 - self.OUTLIER_IQR_FACTOR * iqr
            upper = q3 + self.OUTLIER_IQR_FACTOR * iqr
            mask = (series < lower) | (series > upper)
            n_out = int(mask.sum())
            if n_out > 0:
                result[col] = {
                    "count": n_out,
                    "percentage": round(n_out / len(series) * 100, 2),
                    "lower_bound": round(lower, 4),
                    "upper_bound": round(upper, 4),
                }
                total_outliers += n_out

        return {"total_outliers": total_outliers, "columns": result}

    @staticmethod
    def _seasonality_hints(df: pd.DataFrame) -> dict[str, Any]:
        """Lightweight seasonality detection via autocorrelation peaks.

        Looks for datetime index / column and computes autocorrelation on
        numeric columns to find dominant periodicity.
        """
        # Try to find a datetime column to set as index
        dt_col: str | None = None
        for col in df.columns:
            if pd.api.types.is_datetime64_any_dtype(df[col]):
                dt_col = col
                break

        if dt_col is None:
            # Attempt to parse common column names
            for candidate in ("timestamp", "time", "datetime", "date", "ts"):
                if candidate in df.columns:
                    try:
                        df[candidate] = pd.to_datetime(df[candidate])
                        dt_col = candidate
                        break
                    except Exception:
                        continue

        if dt_col is None:
            return {"detected": False, "reason": "No datetime column found."}

        numeric = df.select_dtypes(include="number")
        if numeric.empty:
            return {"detected": False, "reason": "No numeric columns for autocorrelation."}

        hints: dict[str, Any] = {}
        for col in numeric.columns:
            series = numeric[col].dropna()
            if len(series) < 50:
                continue
            # Compute autocorrelation for lags 1..min(len/2, 500)
            max_lag = min(len(series) // 2, 500)
            autocorr = [float(series.autocorr(lag=lag)) for lag in range(1, max_lag + 1)]
            if not autocorr:
                continue
            peak_lag = int(np.argmax(autocorr)) + 1
            peak_value = autocorr[peak_lag - 1]
            if peak_value > 0.3:
                hints[col] = {
                    "dominant_period_lag": peak_lag,
                    "autocorrelation": round(peak_value, 4),
                }

        if hints:
            return {"detected": True, "columns": hints}
        return {"detected": False, "reason": "No significant periodicity detected."}

    @staticmethod
    def _compute_quality_score(
        missing: dict[str, Any],
        outliers: dict[str, Any],
        duplicates: dict[str, Any],
        df: pd.DataFrame,
    ) -> dict[str, Any]:
        """Derive a 0-100 quality score with explanatory issues list."""
        score = 100.0
        issues: list[str] = []
        total = max(len(df), 1)

        # Penalise missing values
        missing_ratio = missing["total_missing"] / max(total * len(df.columns), 1)
        if missing_ratio > 0.30:
            score -= 40
            issues.append(f"Critical: {missing_ratio:.0%} of cells are missing.")
        elif missing_ratio > 0.05:
            penalty = missing_ratio * 100
            score -= penalty
            issues.append(f"Warning: {missing_ratio:.1%} of cells are missing.")

        # Penalise outliers
        outlier_ratio = outliers["total_outliers"] / max(total, 1)
        if outlier_ratio > 0.10:
            score -= 20
            issues.append(f"High outlier ratio: {outlier_ratio:.1%}.")
        elif outlier_ratio > 0.02:
            score -= 10
            issues.append(f"Moderate outlier ratio: {outlier_ratio:.1%}.")

        # Penalise duplicates
        dup_ratio = duplicates["duplicate_rows"] / total
        if dup_ratio > 0.05:
            score -= 15
            issues.append(f"Duplicate rows: {dup_ratio:.1%}.")

        # Penalise very small datasets
        if total < 500:
            score -= 10
            issues.append(f"Small dataset ({total} rows). Consider collecting more data.")

        if not issues:
            issues.append("No significant quality issues detected.")

        return {"score": round(max(score, 0.0), 1), "issues": issues}
