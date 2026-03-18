# =============================================================================
# File: athena.py
# Description: Top-level orchestrator agent that classifies intent and delegates to specialists
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 8, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Athena Orchestrator Agent
=========================

Top-level agent that classifies user intent and delegates to the appropriate
specialist sub-agent.  Maintains conversation context across turns so that
downstream agents can reference earlier analysis results.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

from gradient_adk import RequestContext

from agents.data_analyst import DataAnalystAgent
from agents.architect import ArchitectAgent
from agents.evaluator import EvaluatorAgent

logger = logging.getLogger(__name__)


class Intent(str, Enum):
    """Recognised user intents."""

    ANALYZE_DATA = "analyze_data"
    SELECT_ARCHITECTURE = "select_architecture"
    GENERATE_TRAINING_PLAN = "generate_training_plan"
    EVALUATE_MODEL = "evaluate_model"
    GENERAL_CHAT = "general_chat"


# ── Intent keyword map (lightweight classifier) ──────────────────────────

_INTENT_KEYWORDS: dict[Intent, list[str]] = {
    Intent.ANALYZE_DATA: [
        "analyze",
        "analyse",
        "data",
        "sensor",
        "statistics",
        "csv",
        "quality",
        "outlier",
        "missing",
        "seasonality",
    ],
    Intent.SELECT_ARCHITECTURE: [
        "architecture",
        "model selection",
        "recommend",
        "lstm",
        "gru",
        "sentinel",
        "autoencoder",
        "select model",
        "choose model",
    ],
    Intent.GENERATE_TRAINING_PLAN: [
        "train",
        "training",
        "plan",
        "schedule",
        "hyperparameter",
        "epoch",
        "batch",
        "trigger",
    ],
    Intent.EVALUATE_MODEL: [
        "evaluate",
        "evaluation",
        "metric",
        "precision",
        "recall",
        "f1",
        "auc",
        "deploy",
        "benchmark",
        "performance",
    ],
}


@dataclass
class ConversationContext:
    """Persistent state shared between orchestrator turns."""

    turns: list[dict[str, Any]] = field(default_factory=list)
    last_analysis: dict[str, Any] | None = None
    last_architecture: dict[str, Any] | None = None
    last_training_plan: dict[str, Any] | None = None
    last_evaluation: dict[str, Any] | None = None

    def add_turn(self, role: str, content: Any) -> None:
        """Append a conversation turn."""
        self.turns.append({"role": role, "content": content})

    def summary(self) -> dict[str, Any]:
        """Return a compact summary for downstream agents."""
        return {
            "num_turns": len(self.turns),
            "has_analysis": self.last_analysis is not None,
            "has_architecture": self.last_architecture is not None,
            "has_training_plan": self.last_training_plan is not None,
            "has_evaluation": self.last_evaluation is not None,
        }


class AthenaOrchestrator:
    """Main orchestrator that routes requests to specialist agents.

    Lifecycle
    ---------
    1. Receive ``input`` dict from the ADK runtime.
    2. Classify intent via keyword scoring (fast, deterministic).
    3. Delegate to the matching sub-agent, forwarding conversation context.
    4. Store the sub-agent result in context and return.
    """

    def __init__(self) -> None:
        self._context = ConversationContext()
        self._data_analyst = DataAnalystAgent()
        self._architect = ArchitectAgent()
        self._evaluator = EvaluatorAgent()

    # ── public API ────────────────────────────────────────────────────────

    async def process(self, input: dict, context: RequestContext) -> dict:
        """Process a single inbound request.

        Parameters
        ----------
        input:
            Must contain at least a ``"message"`` key with the user text.
            May also carry ``"data"`` (e.g. CSV payload), ``"model_id"``, etc.
        context:
            ADK request context.

        Returns
        -------
        dict
            ``{"intent", "response", "context_summary"}``
        """
        message: str = input.get("message", "")
        intent = self._classify_intent(message)
        logger.info("Classified intent: %s for message: %s", intent.value, message[:80])

        self._context.add_turn("user", message)

        response: dict[str, Any]

        if intent == Intent.ANALYZE_DATA:
            response = await self._handle_analyze_data(input, context)
        elif intent == Intent.SELECT_ARCHITECTURE:
            response = await self._handle_select_architecture(input, context)
        elif intent == Intent.GENERATE_TRAINING_PLAN:
            response = await self._handle_generate_training_plan(input, context)
        elif intent == Intent.EVALUATE_MODEL:
            response = await self._handle_evaluate_model(input, context)
        else:
            response = await self._handle_general_chat(input, context)

        self._context.add_turn("assistant", response)

        return {
            "intent": intent.value,
            "response": response,
            "context_summary": self._context.summary(),
        }

    # ── intent classification ─────────────────────────────────────────────

    def _classify_intent(self, message: str) -> Intent:
        """Score each intent by keyword overlap and return the best match."""
        lower = message.lower()
        scores: dict[Intent, int] = {}
        for intent, keywords in _INTENT_KEYWORDS.items():
            scores[intent] = sum(1 for kw in keywords if kw in lower)

        best = max(scores, key=scores.get)  # type: ignore[arg-type]
        if scores[best] == 0:
            return Intent.GENERAL_CHAT
        return best

    # ── handlers ──────────────────────────────────────────────────────────

    async def _handle_analyze_data(
        self, input: dict, context: RequestContext
    ) -> dict[str, Any]:
        """Delegate to the DataAnalystAgent."""
        result = await self._data_analyst.analyze(
            message=input.get("message", ""),
            data_payload=input.get("data"),
            conversation_context=self._context,
            request_context=context,
        )
        self._context.last_analysis = result
        return result

    async def _handle_select_architecture(
        self, input: dict, context: RequestContext
    ) -> dict[str, Any]:
        """Delegate to the ArchitectAgent."""
        result = await self._architect.recommend(
            message=input.get("message", ""),
            data_analysis=self._context.last_analysis,
            equipment_type=input.get("equipment_type"),
            conversation_context=self._context,
            request_context=context,
        )
        self._context.last_architecture = result
        return result

    async def _handle_generate_training_plan(
        self, input: dict, context: RequestContext
    ) -> dict[str, Any]:
        """Build a training plan from current context and trigger if requested."""
        from tools.training_trigger import trigger_training

        architecture = self._context.last_architecture or {}
        analysis = self._context.last_analysis or {}

        plan: dict[str, Any] = {
            "architecture": architecture.get("recommended_architecture", "unknown"),
            "hyperparameters": {
                "epochs": input.get("epochs", 100),
                "batch_size": input.get("batch_size", 64),
                "learning_rate": input.get("learning_rate", 1e-3),
                "early_stopping_patience": input.get("patience", 10),
                "sequence_length": architecture.get("sequence_length", 48),
            },
            "data_summary": {
                "features": analysis.get("feature_count", 0),
                "rows": analysis.get("row_count", 0),
                "quality_score": analysis.get("quality_score", 0.0),
            },
            "equipment_type": input.get("equipment_type", architecture.get("equipment_type", "unknown")),
        }

        if input.get("auto_trigger", False):
            trigger_result = await trigger_training(
                architecture_name=plan["architecture"],
                hyperparameters=plan["hyperparameters"],
                equipment_type=plan["equipment_type"],
            )
            plan["trigger_result"] = trigger_result

        self._context.last_training_plan = plan
        return plan

    async def _handle_evaluate_model(
        self, input: dict, context: RequestContext
    ) -> dict[str, Any]:
        """Delegate to the EvaluatorAgent."""
        result = await self._evaluator.evaluate(
            message=input.get("message", ""),
            model_id=input.get("model_id"),
            metrics_payload=input.get("metrics"),
            conversation_context=self._context,
            request_context=context,
        )
        self._context.last_evaluation = result
        return result

    async def _handle_general_chat(
        self, input: dict, _context: RequestContext
    ) -> dict[str, Any]:
        """Handle free-form questions using platform knowledge."""
        return {
            "type": "general_chat",
            "message": (
                "I'm Athena, the Prometheus AI assistant.  I can help you "
                "analyse sensor data, choose a model architecture, set up "
                "training plans, and evaluate trained models.  What would "
                "you like to do?"
            ),
            "suggestions": [
                "Analyze my sensor data for anomalies",
                "Recommend an architecture for my chiller",
                "Generate a training plan",
                "Evaluate my latest model run",
            ],
        }
