// ============================================================================
// File: login.rs
// Description: Login and account creation page with authentication flow
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::control_flow::Show;
use crate::components::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    // Login panel state
    let show_login = RwSignal::new(false);

    // Login state
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let show_password = RwSignal::new(false);

    // Create account modal state
    let show_signup = RwSignal::new(false);
    let signup_username = RwSignal::new(String::new());
    let signup_email = RwSignal::new(String::new());
    let signup_password = RwSignal::new(String::new());
    let show_signup_password = RwSignal::new(false);
    let signup_error = RwSignal::new(None::<String>);
    let signup_loading = RwSignal::new(false);

    // Verification state
    let show_verify = RwSignal::new(false);
    let verify_code = RwSignal::new(String::new());
    let verify_error = RwSignal::new(None::<String>);
    let verify_loading = RwSignal::new(false);
    let verify_email_addr = RwSignal::new(String::new());

    // Toast state
    let toast_msg = RwSignal::new(None::<(String, &'static str)>); // (message, level)

    let error_signal: Signal<Option<String>> = error.into();
    let signup_error_signal: Signal<Option<String>> = signup_error.into();
    let verify_error_signal: Signal<Option<String>> = verify_error.into();

    // Login handler
    let on_submit = move |_| {
        error.set(None);
        let user = username.get();
        let pass = password.get();

        if user.is_empty() || pass.is_empty() {
            error.set(Some("Username and password are required".to_string()));
            return;
        }

        loading.set(true);

        leptos::task::spawn_local(async move {
            let result = gloo_net::http::Request::post("/api/v1/auth/login")
                .json(&serde_json::json!({
                    "username": user,
                    "password": pass,
                }))
                .unwrap()
                .send()
                .await;

            loading.set(false);

            match result {
                Ok(resp) if resp.ok() => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        if let Some(token) = body.get("token").and_then(|t| t.as_str()) {
                            if let Some(storage) = web_sys::window()
                                .and_then(|w| w.local_storage().ok())
                                .flatten()
                            {
                                let _ = storage.set_item("prometheus_token", token);
                            }
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href("/dashboard");
                            }
                        }
                    }
                }
                Ok(resp) if resp.status() == 403 => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        let msg = body.get("error").and_then(|e| e.as_str()).unwrap_or("");
                        if msg.contains("admin approval") {
                            toast_msg.set(Some((
                                "Account pending admin approval. You will be notified when approved.".into(),
                                "warning",
                            )));
                        } else if msg.contains("Email not verified") {
                            error.set(Some("Please verify your email before signing in.".to_string()));
                        } else {
                            error.set(Some(msg.to_string()));
                        }
                    }
                }
                Ok(resp) if resp.status() == 429 => {
                    error.set(Some("Too many login attempts. Please wait.".to_string()));
                }
                _ => {
                    error.set(Some("Invalid username or password".to_string()));
                }
            }
        });
    };

    // Signup handler
    let on_signup = move |_| {
        signup_error.set(None);
        let user = signup_username.get();
        let email = signup_email.get();
        let pass = signup_password.get();

        if user.is_empty() || email.is_empty() || pass.is_empty() {
            signup_error.set(Some("All fields are required".to_string()));
            return;
        }
        if !email.contains('@') || !email.contains('.') {
            signup_error.set(Some("Invalid email address".to_string()));
            return;
        }
        if pass.len() < 8 {
            signup_error.set(Some("Password must be at least 8 characters".to_string()));
            return;
        }

        signup_loading.set(true);

        leptos::task::spawn_local(async move {
            let result = gloo_net::http::Request::post("/api/v1/auth/signup")
                .json(&serde_json::json!({
                    "username": user,
                    "email": email.clone(),
                    "password": pass,
                }))
                .unwrap()
                .send()
                .await;

            signup_loading.set(false);

            match result {
                Ok(resp) if resp.ok() => {
                    show_signup.set(false);
                    verify_email_addr.set(email);
                    show_verify.set(true);
                }
                Ok(resp) => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        let msg = body.get("error").and_then(|e| e.as_str()).unwrap_or("Registration failed");
                        signup_error.set(Some(msg.to_string()));
                    } else {
                        signup_error.set(Some("Registration failed".to_string()));
                    }
                }
                Err(_) => {
                    signup_error.set(Some("Network error. Please try again.".to_string()));
                }
            }
        });
    };

    // Verify handler
    let on_verify = move |_| {
        verify_error.set(None);
        let code = verify_code.get();
        if code.len() != 6 {
            verify_error.set(Some("Enter the 6-digit code from your email".to_string()));
            return;
        }

        verify_loading.set(true);

        leptos::task::spawn_local(async move {
            let result = gloo_net::http::Request::post("/api/v1/auth/verify-email")
                .json(&serde_json::json!({ "code": code }))
                .unwrap()
                .send()
                .await;

            verify_loading.set(false);

            match result {
                Ok(resp) if resp.ok() => {
                    show_verify.set(false);
                    toast_msg.set(Some((
                        "Email verified! Your account is pending admin approval.".into(),
                        "success",
                    )));
                }
                Ok(resp) => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        let msg = body.get("error").and_then(|e| e.as_str()).unwrap_or("Invalid code");
                        verify_error.set(Some(msg.to_string()));
                    }
                }
                Err(_) => {
                    verify_error.set(Some("Network error".to_string()));
                }
            }
        });
    };

    // Resend verification code
    let on_resend = move |_| {
        let email = verify_email_addr.get();
        if email.is_empty() { return; }

        leptos::task::spawn_local(async move {
            let _ = gloo_net::http::Request::post("/api/v1/auth/resend-verification")
                .json(&serde_json::json!({ "email": email }))
                .unwrap()
                .send()
                .await;
            toast_msg.set(Some(("Verification code resent to your email.".into(), "info")));
        });
    };

    // Password toggle button style
    let toggle_btn_style = "position: absolute; right: 8px; top: 50%; transform: translateY(-50%); background: none; border: none; cursor: pointer; color: #6b7280; display: flex; align-items: center; padding: 4px;";

    view! {
        <div>
        <div style="min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; background: #FFFDF7; position: relative; overflow: hidden;">
            // Logo section — slides up when login shows
            <div
                style=move || if show_login.get() {
                    "text-align: center; cursor: pointer; transition: all 0.5s ease; transform: translateY(-60px); margin-bottom: 0px;"
                } else {
                    "text-align: center; cursor: pointer; transition: all 0.5s ease; transform: translateY(0); margin-bottom: 0px;"
                }
                on:click=move |_| show_login.set(true)
            >
                <img src="/Prometheus_logo.png" alt="Prometheus" style=move || if show_login.get() {
                    "width: 200px; display: block; margin: 0 auto; border-radius: 16px; box-shadow: 0 12px 40px rgba(0,0,0,0.06); transition: all 0.5s ease;"
                } else {
                    "width: 340px; display: block; margin: 0 auto; border-radius: 20px; box-shadow: 0 20px 60px rgba(0,0,0,0.08); transition: all 0.5s ease;"
                } />
                <h1 style=move || if show_login.get() {
                    "font-size: 1.4rem; font-weight: 700; color: #111827; margin-top: 12px; transition: all 0.5s ease;"
                } else {
                    "font-size: 2rem; font-weight: 700; color: #111827; margin-top: 20px; transition: all 0.5s ease;"
                }>"Prometheus"</h1>
                <p style=move || if show_login.get() {
                    "font-size: 0.8rem; color: #6b7280; margin-top: 2px; transition: all 0.5s ease;"
                } else {
                    "font-size: 1rem; color: #6b7280; margin-top: 4px; transition: all 0.5s ease;"
                }>"AI-Forged Edge Intelligence"</p>
                <p style=move || if show_login.get() { "display:none;" } else { "font-size: 0.85rem; color: #14b8a6; margin-top: 16px; animation: pulse 2s ease-in-out infinite;" }>
                    "Click to sign in"
                </p>
            </div>

            // Login card — slides up from below
            <div style=move || if show_login.get() {
                "width: 100%; max-width: 400px; padding: 24px 32px; opacity: 1; transform: translateY(0); transition: all 0.5s ease 0.1s;"
            } else {
                "width: 100%; max-width: 400px; padding: 24px 32px; opacity: 0; transform: translateY(40px); transition: all 0.5s ease; pointer-events: none;"
            }>
                <div style="width: 100%; max-width: 340px; margin: 0 auto;">
                <div style="margin-bottom: 24px;">
                    <h2 style="font-size: 1.5rem; font-weight: 700; color: #111827;">"Sign In"</h2>
                    <p class="text-sm text-muted">"Enter your credentials to continue"</p>
                </div>

                // Toast notification
                <Show when=move || toast_msg.get().is_some() fallback=|| ()>
                    {move || {
                        let (msg, level) = toast_msg.get().unwrap_or_default();
                        let bg = match level {
                            "success" => "#dcfce7",
                            "warning" => "#fef3c7",
                            "info" => "#dbeafe",
                            _ => "#fee2e2",
                        };
                        let border = match level {
                            "success" => "#16a34a",
                            "warning" => "#d97706",
                            "info" => "#2563eb",
                            _ => "#dc2626",
                        };
                        let style = format!(
                            "padding: 12px 16px; border-radius: 8px; margin-bottom: 16px; font-size: 0.875rem; background: {}; border: 1px solid {}; display: flex; justify-content: space-between; align-items: center;",
                            bg, border
                        );
                        view! {
                            <div style=style>
                                <span>{msg}</span>
                                <button
                                    style="background: none; border: none; cursor: pointer; font-size: 1.1rem; color: #6b7280; padding: 0 0 0 8px;"
                                    on:click=move |_| toast_msg.set(None)
                                >"x"</button>
                            </div>
                        }
                    }}
                </Show>

                <Card>
                    <div style="display: flex; flex-direction: column; gap: 16px;">
                        <TextInput
                            label="Username"
                            placeholder="Enter username"
                            value=username
                        />
                        // Password field with eye toggle
                        <div class="input-group">
                            <label class="input-label">"Password"</label>
                            <div style="position: relative;">
                                <input
                                    type=move || if show_password.get() { "text" } else { "password" }
                                    class="input-field"
                                    class:input-error=move || error_signal.get().is_some()
                                    placeholder="Enter password"
                                    style="padding-right: 40px;"
                                    prop:value=move || password.get()
                                    on:input=move |ev| {
                                        password.set(event_target_value(&ev));
                                    }
                                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                                        if ev.key() == "Enter" {
                                            // Trigger login via the same path
                                            error.set(None);
                                            let user = username.get();
                                            let pass = password.get();
                                            if user.is_empty() || pass.is_empty() {
                                                error.set(Some("Username and password are required".to_string()));
                                                return;
                                            }
                                            loading.set(true);
                                            leptos::task::spawn_local(async move {
                                                let result = gloo_net::http::Request::post("/api/v1/auth/login")
                                                    .json(&serde_json::json!({
                                                        "username": user,
                                                        "password": pass,
                                                    }))
                                                    .unwrap()
                                                    .send()
                                                    .await;
                                                loading.set(false);
                                                match result {
                                                    Ok(resp) if resp.ok() => {
                                                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                                                            if let Some(token) = body.get("token").and_then(|t| t.as_str()) {
                                                                if let Some(storage) = web_sys::window()
                                                                    .and_then(|w| w.local_storage().ok())
                                                                    .flatten()
                                                                {
                                                                    let _ = storage.set_item("prometheus_token", token);
                                                                }
                                                                if let Some(window) = web_sys::window() {
                                                                    let _ = window.location().set_href("/dashboard");
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Ok(resp) if resp.status() == 403 => {
                                                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                                                            let msg = body.get("error").and_then(|e| e.as_str()).unwrap_or("");
                                                            if msg.contains("admin approval") {
                                                                toast_msg.set(Some((
                                                                    "Account pending admin approval. You will be notified when approved.".into(),
                                                                    "warning",
                                                                )));
                                                            } else {
                                                                error.set(Some(msg.to_string()));
                                                            }
                                                        }
                                                    }
                                                    Ok(resp) if resp.status() == 429 => {
                                                        error.set(Some("Too many login attempts. Please wait.".to_string()));
                                                    }
                                                    _ => {
                                                        error.set(Some("Invalid username or password".to_string()));
                                                    }
                                                }
                                            });
                                        }
                                    }
                                />
                                <button
                                    type="button"
                                    style=toggle_btn_style
                                    on:click=move |_| show_password.update(|v| *v = !*v)
                                    tabindex=-1
                                >
                                    <Show
                                        when=move || show_password.get()
                                        fallback=|| view! { {crate::icons::icon_eye()} }
                                    >
                                        {crate::icons::icon_eye_off()}
                                    </Show>
                                </button>
                            </div>
                            {move || error_signal.get().map(|msg| view! { <span class="input-error-msg">{msg}</span> })}
                        </div>
                        <button
                            class="btn btn-primary"
                            style="width: 100%; justify-content: center; padding: 12px;"
                            disabled=move || loading.get()
                            on:click=on_submit
                        >
                            <Show
                                when=move || !loading.get()
                                fallback=|| view! { <Spinner size=16 /> }
                            >
                                "Sign In"
                            </Show>
                        </button>

                        <div style="text-align: center; margin-top: 4px;">
                            <button
                                class="btn btn-ghost"
                                style="font-size: 0.875rem; gap: 6px;"
                                on:click=move |_| {
                                    signup_error.set(None);
                                    signup_username.set(String::new());
                                    signup_email.set(String::new());
                                    signup_password.set(String::new());
                                    show_signup.set(true);
                                }
                            >
                                {crate::icons::icon_user_plus()}
                                " Create Account"
                            </button>
                        </div>
                    </div>
                </Card>
                </div>
            </div>
        </div>

        // Pulse animation for "Click to sign in"
        <style>"@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }"</style>

        // Create Account Modal
        <div class="modal-backdrop" style=move || if show_signup.get() { "" } else { "display: none;" }
            on:click=move |_: web_sys::MouseEvent| show_signup.set(false)
        >
            <div class="modal" style="max-width: 420px;"
                on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
            >
                <div class="flex-between mb-4">
                    <h2 class="modal-title">"Create Account"</h2>
                    <button class="btn btn-ghost btn-sm" on:click=move |_: web_sys::MouseEvent| show_signup.set(false)>
                        {crate::icons::icon_x()}
                    </button>
                </div>

                <div style="display: flex; flex-direction: column; gap: 14px;">
                    <TextInput label="Username" placeholder="your_username" value=signup_username />
                    <TextInput label="Email" placeholder="you@example.com" value=signup_email />

                    // Signup password with toggle
                    <div class="input-group">
                        <label class="input-label">"Password"</label>
                        <div style="position: relative;">
                            <input
                                type=move || if show_signup_password.get() { "text" } else { "password" }
                                class="input-field"
                                class:input-error=move || signup_error_signal.get().is_some()
                                placeholder="Minimum 8 characters"
                                style="padding-right: 40px;"
                                prop:value=move || signup_password.get()
                                on:input=move |ev| {
                                    signup_password.set(event_target_value(&ev));
                                }
                            />
                            <button
                                type="button"
                                style=toggle_btn_style
                                on:click=move |_| show_signup_password.update(|v| *v = !*v)
                                tabindex=-1
                            >
                                <Show
                                    when=move || show_signup_password.get()
                                    fallback=|| view! { {crate::icons::icon_eye()} }
                                >
                                    {crate::icons::icon_eye_off()}
                                </Show>
                            </button>
                        </div>
                        {move || signup_error_signal.get().map(|msg| view! { <span class="input-error-msg">{msg}</span> })}
                    </div>

                    <p style="font-size: 0.75rem; color: #6b7280; margin: 0;">
                        "After creating your account, you'll receive a verification email. An admin must approve your account before you can sign in."
                    </p>

                    <button
                        class="btn btn-primary"
                        style="width: 100%; justify-content: center; padding: 12px;"
                        disabled=move || signup_loading.get()
                        on:click=on_signup
                    >
                        <Show
                            when=move || !signup_loading.get()
                            fallback=|| view! { <Spinner size=16 /> }
                        >
                            {crate::icons::icon_mail()}
                            " Create Account"
                        </Show>
                    </button>
                </div>
            </div>
        </div>

        // Email Verification Modal
        <div class="modal-backdrop" style=move || if show_verify.get() { "" } else { "display: none;" }>
            <div class="modal" style="max-width: 400px;"
                on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
            >
                <div class="flex-between mb-4">
                    <h2 class="modal-title">"Verify Your Email"</h2>
                </div>

                <div style="display: flex; flex-direction: column; gap: 14px;">
                    <p style="font-size: 0.875rem; color: #374151; margin: 0;">
                        "We sent a 6-digit verification code to "
                        <strong>{move || verify_email_addr.get()}</strong>
                    </p>

                    <div class="input-group">
                        <label class="input-label">"Verification Code"</label>
                        <input
                            type="text"
                            class="input-field"
                            class:input-error=move || verify_error_signal.get().is_some()
                            placeholder="000000"
                            maxlength="6"
                            style="text-align: center; font-size: 1.5rem; letter-spacing: 0.5em; font-family: var(--font-mono);"
                            prop:value=move || verify_code.get()
                            on:input=move |ev| {
                                let val: String = event_target_value(&ev)
                                    .chars()
                                    .filter(|c| c.is_ascii_digit())
                                    .take(6)
                                    .collect();
                                verify_code.set(val);
                            }
                        />
                        {move || verify_error_signal.get().map(|msg| view! { <span class="input-error-msg">{msg}</span> })}
                    </div>

                    <button
                        class="btn btn-primary"
                        style="width: 100%; justify-content: center; padding: 12px;"
                        disabled=move || verify_loading.get()
                        on:click=on_verify
                    >
                        <Show
                            when=move || !verify_loading.get()
                            fallback=|| view! { <Spinner size=16 /> }
                        >
                            {crate::icons::icon_check()}
                            " Verify Email"
                        </Show>
                    </button>

                    <button
                        class="btn btn-ghost"
                        style="width: 100%; justify-content: center; font-size: 0.8rem;"
                        on:click=on_resend
                    >
                        "Didn't receive code? Resend"
                    </button>
                </div>
            </div>
        </div>
        </div>
    }
}
