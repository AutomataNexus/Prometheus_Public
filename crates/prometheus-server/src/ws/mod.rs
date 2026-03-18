// ============================================================================
// File: mod.rs
// Description: WebSocket handler for real-time training progress streaming to connected clients
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use crate::state::AppState;

pub async fn training_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_training_ws(socket, state, run_id))
}

async fn handle_training_ws(socket: WebSocket, state: AppState, run_id: String) {
    let (mut sender, mut receiver) = socket.split();

    // Poll training progress and send updates
    let send_task = tokio::spawn(async move {
        let mut last_epoch = 0u64;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Get current training state
            let result = state
                .aegis_request(
                    reqwest::Method::GET,
                    &format!("/api/v1/documents/collections/training_plans/documents/{run_id}"),
                    None,
                )
                .await;

            match result {
                Ok(run) => {
                    let current_epoch = run
                        .get("current_epoch")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let status = run
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    if current_epoch > last_epoch {
                        let msg = json!({
                            "type": "epoch_update",
                            "run_id": run_id,
                            "current_epoch": current_epoch,
                            "status": status,
                            "best_val_loss": run.get("best_val_loss"),
                            "epoch_metrics": run.get("epoch_metrics"),
                        });

                        if sender
                            .send(Message::Text(msg.to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        last_epoch = current_epoch;
                    }

                    if status == "completed" || status == "failed" || status == "cancelled" {
                        let msg = json!({
                            "type": "training_complete",
                            "run_id": run_id,
                            "status": status,
                            "final_metrics": run.get("epoch_metrics"),
                        });
                        let _ = sender.send(Message::Text(msg.to_string().into())).await;
                        break;
                    }
                }
                Err(_) => {
                    // Training run not found, close connection
                    break;
                }
            }
        }
    });

    // Handle incoming messages (ping/pong, close)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
