# =============================================================================
# File: main.py
# Description: Gradient AI ADK entrypoint for the Athena orchestrator agent
# Author: Andrew Jewell Sr. - AutomataNexus
# Updated: March 8, 2026
#
# DISCLAIMER: This software is provided "as is", without warranty of any kind,
# express or implied. Use at your own risk. AutomataNexus and the author assume
# no liability for any damages arising from the use of this software.
# =============================================================================
"""
Prometheus Agent - Athena ADK Entrypoint
=========================================

DigitalOcean Gradient AI ADK agent for the Prometheus predictive-maintenance
platform.  Athena orchestrates specialist sub-agents that analyse sensor data,
recommend model architectures, trigger training runs, and evaluate results.
"""

from __future__ import annotations

from gradient_adk import entrypoint, RequestContext

from agents.athena import AthenaOrchestrator


@entrypoint
async def main(input: dict, context: RequestContext) -> dict:
    """Primary ADK entrypoint.

    Parameters
    ----------
    input:
        Incoming request payload from the Gradient runtime.
    context:
        ADK request context carrying auth, tracing and session metadata.

    Returns
    -------
    dict
        Structured response produced by the Athena orchestrator.
    """
    athena = AthenaOrchestrator()
    return await athena.process(input, context)
