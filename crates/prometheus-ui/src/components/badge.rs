// ============================================================================
// File: badge.rs
// Description: Status badge component with color-coded variants for entity states
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum BadgeStatus {
    Online,
    Training,
    Error,
    Deployed,
    Pending,
    Completed,
    Ready,
    Offline,
}

impl BadgeStatus {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Online => "badge badge-online",
            Self::Training => "badge badge-training",
            Self::Error => "badge badge-error",
            Self::Deployed => "badge badge-deployed",
            Self::Pending => "badge badge-pending",
            Self::Completed => "badge badge-completed",
            Self::Ready => "badge badge-ready",
            Self::Offline => "badge badge-error",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Online => "Online",
            Self::Training => "Training",
            Self::Error => "Error",
            Self::Deployed => "Deployed",
            Self::Pending => "Pending",
            Self::Completed => "Completed",
            Self::Ready => "Ready",
            Self::Offline => "Offline",
        }
    }
}

#[component]
pub fn Badge(status: BadgeStatus) -> impl IntoView {
    view! {
        <span class=status.class()>
            <span class="badge-dot"></span>
            {status.label()}
        </span>
    }
}

pub fn status_to_badge(status: &str) -> BadgeStatus {
    match status {
        "online" => BadgeStatus::Online,
        "training" | "running" | "in_progress" => BadgeStatus::Training,
        "error" | "failed" => BadgeStatus::Error,
        "deployed" => BadgeStatus::Deployed,
        "pending" | "queued" => BadgeStatus::Pending,
        "completed" | "done" => BadgeStatus::Completed,
        "ready" => BadgeStatus::Ready,
        "offline" => BadgeStatus::Offline,
        _ => BadgeStatus::Pending,
    }
}
