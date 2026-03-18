// ============================================================================
// File: api.rs
// Description: Authenticated HTTP request helpers using bearer tokens from localStorage
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use gloo_net::http::RequestBuilder;

/// Read the bearer token from localStorage.
pub fn get_token() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item("prometheus_token").ok())
        .flatten()
}

/// Build an authenticated GET request.
pub fn auth_get(url: &str) -> RequestBuilder {
    let req = gloo_net::http::Request::get(url);
    if let Some(token) = get_token() {
        req.header("Authorization", &format!("Bearer {token}"))
    } else {
        req
    }
}

/// Build an authenticated POST request.
pub fn auth_post(url: &str) -> RequestBuilder {
    let req = gloo_net::http::Request::post(url);
    if let Some(token) = get_token() {
        req.header("Authorization", &format!("Bearer {token}"))
    } else {
        req
    }
}

/// Build an authenticated PUT request.
pub fn auth_put(url: &str) -> RequestBuilder {
    let req = gloo_net::http::Request::put(url);
    if let Some(token) = get_token() {
        req.header("Authorization", &format!("Bearer {token}"))
    } else {
        req
    }
}

/// Build an authenticated DELETE request.
pub fn auth_delete(url: &str) -> RequestBuilder {
    let req = gloo_net::http::Request::delete(url);
    if let Some(token) = get_token() {
        req.header("Authorization", &format!("Bearer {token}"))
    } else {
        req
    }
}

/// Clear token and redirect to login page.
pub fn redirect_to_login() {
    if let Some(window) = web_sys::window() {
        if let Some(storage) = window.local_storage().ok().flatten() {
            let _ = storage.remove_item("prometheus_token");
        }
        let _ = window.location().set_href("/login");
    }
}
