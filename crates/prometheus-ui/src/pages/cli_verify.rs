// ============================================================================
// File: cli_verify.rs
// Description: CLI device verification page for authenticating CLI sessions
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;

#[component]
pub fn CliVerifyPage() -> impl IntoView {
    let code = RwSignal::new(String::new());
    let status = RwSignal::new("loading".to_string()); // loading | pending | verified | error
    let error_msg = RwSignal::new(None::<String>);
    let verifying = RwSignal::new(false);

    // Extract code from URL query params
    {
        leptos::task::spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let search = window.location().search().unwrap_or_default();
                let params = web_sys::UrlSearchParams::new_with_str(&search).ok();
                if let Some(c) = params.and_then(|p| p.get("code")) {
                    code.set(c);
                    status.set("pending".to_string());
                } else {
                    status.set("error".to_string());
                    error_msg.set(Some("No verification code provided.".to_string()));
                }
            }
        });
    }

    let on_verify = move |_| {
        let session_code = code.get();
        if session_code.is_empty() {
            return;
        }

        // Get the user's token from localStorage
        let token = web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
            .and_then(|s| s.get_item("prometheus_token").ok())
            .flatten()
            .unwrap_or_default();

        if token.is_empty() {
            error_msg.set(Some("You must be signed in to verify a CLI session. Please sign in first.".to_string()));
            return;
        }

        verifying.set(true);
        error_msg.set(None);

        leptos::task::spawn_local(async move {
            // Get current user info
            let user_resp = crate::api::auth_get("/api/v1/auth/session")
                .send()
                .await;

            let (username, role) = match user_resp {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let u = data.pointer("/user/username")
                            .and_then(|v| v.as_str())
                            .unwrap_or("user")
                            .to_string();
                        let r = data.pointer("/user/role")
                            .and_then(|v| v.as_str())
                            .unwrap_or("operator")
                            .to_string();
                        (u, r)
                    } else {
                        ("user".to_string(), "operator".to_string())
                    }
                }
                Err(_) => {
                    verifying.set(false);
                    error_msg.set(Some("Session expired. Please sign in again.".to_string()));
                    return;
                }
            };

            // Verify the CLI session
            let verify_resp = crate::api::auth_post("/api/v1/auth/cli/verify")
                .json(&serde_json::json!({
                    "code": session_code,
                    "token": token,
                    "username": username,
                    "role": role,
                }))
                .unwrap()
                .send()
                .await;

            verifying.set(false);

            match verify_resp {
                Ok(resp) if resp.ok() => {
                    status.set("verified".to_string());
                }
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    let msg = serde_json::from_str::<serde_json::Value>(&text)
                        .ok()
                        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                        .unwrap_or_else(|| "Verification failed".to_string());
                    error_msg.set(Some(msg));
                }
                Err(e) => {
                    error_msg.set(Some(format!("Network error: {e}")));
                }
            }
        });
    };

    view! {
        <div style="min-height: 100vh; display: flex; align-items: center; justify-content: center; background: #FFFDF7;">
            <div style="width: 100%; max-width: 440px; padding: 24px;">
                <div style="text-align: center; margin-bottom: 32px;">
                    <img src="/assets/logo.png?v=3" alt="Prometheus" style="width: 72px; height: 72px; margin: 0 auto 16px; border-radius: 12px;" />
                    <h1 style="font-size: 1.5rem; font-weight: 700; color: #111827;">"CLI Session Verification"</h1>
                    <p style="font-size: 0.875rem; color: #6b7280; margin-top: 4px;">"Confirm this terminal session in your browser"</p>
                </div>

                // Loading state
                <Show when=move || status.get() == "loading" fallback=|| ()>
                    <div style="text-align: center; padding: 40px 0; color: #6b7280;">
                        "Loading..."
                    </div>
                </Show>

                // Pending — show verify button
                <Show when=move || status.get() == "pending" fallback=|| ()>
                    <div style="background: #FFFDF7; border: 1px solid #E8D4C4; border-radius: 12px; padding: 24px;">
                        <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 20px; padding: 14px; background: #FAF8F5; border-radius: 8px; border: 1px solid #f0ebe6;">
                            <div style="width: 40px; height: 40px; border-radius: 8px; background: rgba(20,184,166,0.1); display: flex; align-items: center; justify-content: center; flex-shrink: 0;">
                                {crate::icons::icon_shield()}
                            </div>
                            <div>
                                <div style="font-size: 0.8125rem; font-weight: 600; color: #374151;">"Session Code"</div>
                                <div style="font-size: 0.9rem; font-family: 'JetBrains Mono', monospace; color: #14b8a6; letter-spacing: 0.05em;">
                                    {move || code.get()}
                                </div>
                            </div>
                        </div>

                        <p style="font-size: 0.875rem; color: #374151; margin-bottom: 20px; line-height: 1.5;">
                            "A terminal session is requesting access to your Prometheus account. Click below to authorize it."
                        </p>

                        // Error
                        {move || error_msg.get().map(|msg| view! {
                            <div style="background: rgba(239,68,68,0.08); border: 1px solid rgba(239,68,68,0.3); border-radius: 8px; padding: 10px 14px; margin-bottom: 16px; color: #dc2626; font-size: 0.8125rem;">
                                {msg}
                            </div>
                        })}

                        <button
                            style="width: 100%; padding: 12px; border-radius: 8px; background: #14b8a6; color: white; border: none; font-size: 0.9375rem; font-weight: 600; cursor: pointer; font-family: inherit; display: flex; align-items: center; justify-content: center; gap: 8px;"
                            disabled=move || verifying.get()
                            on:click=on_verify
                        >
                            {move || if verifying.get() {
                                "Verifying...".to_string()
                            } else {
                                "Authorize CLI Session".to_string()
                            }}
                        </button>

                        <p style="font-size: 0.75rem; color: #9ca3af; margin-top: 16px; text-align: center; line-height: 1.4;">
                            "Only approve this if you initiated a login from the Prometheus CLI. If you did not, ignore this page."
                        </p>
                    </div>
                </Show>

                // Verified — success state
                <Show when=move || status.get() == "verified" fallback=|| ()>
                    <div style="background: #FFFDF7; border: 1px solid #E8D4C4; border-radius: 12px; padding: 32px; text-align: center;">
                        <div style="width: 56px; height: 56px; border-radius: 50%; background: rgba(34,197,94,0.12); display: flex; align-items: center; justify-content: center; margin: 0 auto 16px; color: #22c55e;">
                            {crate::icons::icon_check()}
                        </div>
                        <h2 style="font-size: 1.25rem; font-weight: 700; color: #111827; margin-bottom: 8px;">"Session Verified"</h2>
                        <p style="font-size: 0.875rem; color: #6b7280; line-height: 1.5;">
                            "Your CLI session has been authorized. You can close this tab and return to your terminal."
                        </p>
                    </div>
                </Show>

                // Error state (no code)
                <Show when=move || { status.get() == "error" && error_msg.get().is_some() && code.get().is_empty() } fallback=|| ()>
                    <div style="background: #FFFDF7; border: 1px solid #E8D4C4; border-radius: 12px; padding: 32px; text-align: center;">
                        <div style="width: 56px; height: 56px; border-radius: 50%; background: rgba(239,68,68,0.12); display: flex; align-items: center; justify-content: center; margin: 0 auto 16px; color: #dc2626;">
                            {crate::icons::icon_x()}
                        </div>
                        <h2 style="font-size: 1.25rem; font-weight: 700; color: #111827; margin-bottom: 8px;">"Invalid Link"</h2>
                        <p style="font-size: 0.875rem; color: #6b7280;">
                            {move || error_msg.get().unwrap_or_default()}
                        </p>
                    </div>
                </Show>
            </div>
        </div>
    }
}
