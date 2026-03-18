// ============================================================================
// File: theme.rs
// Description: NexusEdge design system color constants, theme tokens, and global CSS styles
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

/// NexusEdge Design System — Color constants and theme configuration

// Primary accent — teal (interactive elements, buttons, links)
pub const PRIMARY: &str = "#14b8a6";
pub const PRIMARY_DARK: &str = "#0d9488";
pub const PRIMARY_HOVER: &str = "rgba(20, 184, 166, 0.9)";
pub const PRIMARY_LIGHT: &str = "rgba(20, 184, 166, 0.2)";

// Warm neutral palette (backgrounds, borders, cards)
pub const BG_CREAM: &str = "#FFFDF7";
pub const BG_OFF_WHITE: &str = "#FAF8F5";
pub const BG_WARM_BEIGE: &str = "#F5EDE8";
pub const BORDER_TAN: &str = "#E8D4C4";
pub const BORDER_TAN_HOVER: &str = "#D4B8A8";

// Accent colors
pub const TERRACOTTA: &str = "#C4A484";
pub const RUSSET: &str = "#C2714F";
pub const TEXT_PRIMARY: &str = "#111827";
pub const TEXT_SECONDARY: &str = "#6b7280";

// Domain colors
pub const COLOR_MEDICAL: &str = "#3b82f6";
pub const COLOR_FINANCE: &str = "#22c55e";
pub const COLOR_NLP: &str = "#8b5cf6";
pub const COLOR_VISION: &str = "#f97316";
pub const COLOR_INDUSTRIAL: &str = "#06b6d4";
pub const COLOR_AUDIO: &str = "#ec4899";

// Status colors
pub const SUCCESS: &str = "#22c55e";
pub const WARNING: &str = "#f97316";
pub const ERROR: &str = "#dc2626";
pub const INFO: &str = "#3b82f6";

// Toast gradient
pub const TOAST_GRADIENT: &str = "linear-gradient(to right, #14b8a6, #0d9488)";

// Sidebar — clean modern light
pub const SIDEBAR_WIDTH: &str = "220px";
pub const SIDEBAR_BG: &str = "#F4F5F7";
pub const SIDEBAR_TEXT: &str = "#5f6368";
pub const SIDEBAR_ACTIVE: &str = "rgba(20, 184, 166, 0.10)";
pub const SIDEBAR_HOVER: &str = "rgba(0, 0, 0, 0.04)";

/// Get domain color
pub fn domain_color(domain: &str) -> &'static str {
    match domain {
        "medical" => COLOR_MEDICAL,
        "finance" => COLOR_FINANCE,
        "nlp" => COLOR_NLP,
        "vision" => COLOR_VISION,
        "industrial" => COLOR_INDUSTRIAL,
        "audio" => COLOR_AUDIO,
        _ => PRIMARY,
    }
}

/// CSS for the entire application — embedded inline to avoid external CSS build step
pub fn global_styles() -> &'static str {
    r#"
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');

    *, *::before, *::after {
        box-sizing: border-box;
        margin: 0;
        padding: 0;
    }

    /* Hide scrollbars globally while keeping scroll functional */
    html, body {
        scrollbar-width: none; /* Firefox */
        -ms-overflow-style: none; /* IE/Edge */
    }
    html::-webkit-scrollbar, body::-webkit-scrollbar {
        display: none; /* Chrome/Safari/Opera */
    }
    /* Also hide on all scrollable containers */
    .main-content, .sidebar, .sidebar-nav {
        scrollbar-width: none;
        -ms-overflow-style: none;
    }
    .main-content::-webkit-scrollbar, .sidebar::-webkit-scrollbar, .sidebar-nav::-webkit-scrollbar {
        display: none;
    }

    body {
        font-family: 'Inter', system-ui, -apple-system, sans-serif;
        background: #FFFDF7;
        color: #111827;
        line-height: 1.6;
        -webkit-font-smoothing: antialiased;
    }

    code, pre, .mono {
        font-family: 'JetBrains Mono', 'Fira Code', monospace;
    }

    .app-shell {
        display: flex;
        min-height: 100vh;
    }

    .sidebar {
        background: #F4F5F7;
        color: #374151;
        display: flex;
        flex-direction: column;
        position: fixed;
        top: 0;
        left: 0;
        bottom: 0;
        z-index: 50;
        transition: width 0.25s ease;
        overflow: hidden;
        border-right: 1px solid #e0e2e6;
    }

    .sidebar-logo {
        padding: 16px 16px;
        display: flex;
        align-items: center;
        gap: 10px;
        border-bottom: 1px solid #e0e2e6;
        background: #fafbfc;
    }

    .sidebar-logo img {
        width: 36px;
        height: 36px;
        border-radius: 8px;
    }

    .sidebar-logo h1, .sidebar-logo-title {
        font-size: 1.1rem;
        font-weight: 700;
        color: #111827;
        line-height: 1.2;
    }

    .sidebar-slogan {
        font-size: 0.625rem;
        font-weight: 400;
        color: #9ca3af;
        letter-spacing: 0.03em;
    }

    .sidebar-nav {
        flex: 1;
        padding: 12px 10px;
        display: flex;
        flex-direction: column;
        gap: 3px;
    }

    .nav-item {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 7px 12px;
        border-radius: 7px;
        color: #5f6368;
        text-decoration: none;
        font-size: 0.8125rem;
        font-weight: 500;
        transition: all 0.15s ease;
        cursor: pointer;
        background: linear-gradient(to bottom, #ffffff, #f7f8fa);
        border: 1px solid #dde0e4;
        box-shadow: 0 1px 3px rgba(0,0,0,0.06), inset 0 1px 0 rgba(255,255,255,0.8);
    }

    .nav-item:hover {
        background: linear-gradient(to bottom, #ffffff, #f0f1f3);
        color: #1f2937;
        border-color: #c8ccd2;
        box-shadow: 0 2px 6px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.9);
        transform: translateY(-1px);
    }

    .nav-item:active {
        background: linear-gradient(to bottom, #eef0f2, #e6e8ec);
        box-shadow: inset 0 1px 3px rgba(0,0,0,0.08);
        transform: translateY(0);
    }

    .nav-item.active {
        background: linear-gradient(to bottom, #edf9f7, #e2f5f3);
        color: #0d9488;
        font-weight: 600;
        border-color: rgba(20,184,166,0.3);
        box-shadow: 0 1px 4px rgba(20,184,166,0.10), inset 0 1px 0 rgba(255,255,255,0.6);
    }

    .nav-item.active:hover {
        background: linear-gradient(to bottom, #e2f5f3, #d5efec);
        border-color: rgba(20,184,166,0.4);
        box-shadow: 0 2px 6px rgba(20,184,166,0.14), inset 0 1px 0 rgba(255,255,255,0.7);
    }

    .nav-item svg {
        width: 16px;
        height: 16px;
        flex-shrink: 0;
    }

    .sidebar-secured {
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 8px 16px;
        padding: 8px 12px;
        background: #edf9f7;
        border: 1px solid rgba(20, 184, 166, 0.2);
        border-radius: 9px;
        font-size: 0.75rem;
        font-weight: 500;
        color: #0d9488;
        white-space: nowrap;
    }

    .sidebar-secured svg {
        flex-shrink: 0;
    }

    .sidebar-toggle {
        margin: 8px 12px;
        padding: 8px;
        background: linear-gradient(to bottom, #ffffff, #f3f4f6);
        border: 1px solid #dde0e4;
        border-radius: 9px;
        color: #9ca3af;
        cursor: pointer;
        font-size: 1.25rem;
        transition: all 0.15s ease;
        box-shadow: 0 1px 3px rgba(0,0,0,0.06), inset 0 1px 0 rgba(255,255,255,0.8);
    }

    .sidebar-toggle:hover {
        background: linear-gradient(to bottom, #ffffff, #eef0f2);
        color: #374151;
        box-shadow: 0 2px 5px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.9);
    }

    .sidebar-toggle:active {
        background: linear-gradient(to bottom, #eef0f2, #e6e8ec);
        box-shadow: inset 0 1px 3px rgba(0,0,0,0.08);
    }

    .sidebar-version {
        padding: 16px 24px;
        border-top: 1px solid #e0e2e6;
        font-size: 0.75rem;
        color: #9ca3af;
        white-space: nowrap;
        background: #eef0f2;
    }

    .main-content {
        flex: 1;
        display: flex;
        flex-direction: column;
        transition: margin-left 0.25s ease;
    }

    .header {
        height: 64px;
        background: #FFFDF7;
        border-bottom: 1px solid #E8D4C4;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 32px;
        position: sticky;
        top: 0;
        z-index: 40;
    }

    .header-breadcrumbs {
        font-size: 0.875rem;
        color: #6b7280;
    }

    .header-breadcrumbs span {
        color: #111827;
        font-weight: 500;
    }

    .header-actions {
        display: flex;
        align-items: center;
        gap: 16px;
    }

    .header-user {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 6px 12px;
        border-radius: 8px;
        cursor: pointer;
        transition: background 0.2s;
    }

    .header-user:hover {
        background: #F5EDE8;
    }

    .page-content {
        flex: 1;
        padding: 32px;
    }

    .page-title {
        font-size: 1.75rem;
        font-weight: 700;
        color: #111827;
        margin-bottom: 8px;
    }

    .page-subtitle {
        font-size: 0.875rem;
        color: #6b7280;
        margin-bottom: 32px;
    }

    /* Cards */
    .card {
        background: #FFFDF7;
        border: 1px solid #E8D4C4;
        border-radius: 12px;
        padding: 24px;
        transition: border-color 0.2s;
    }

    .card:hover {
        border-color: #D4B8A8;
    }

    .card-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 16px;
    }

    .card-title {
        font-size: 1rem;
        font-weight: 600;
        color: #111827;
    }

    /* Metric Cards */
    .metric-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 16px;
        margin-bottom: 24px;
    }

    .metric-card {
        background: #FAF8F5;
        border: 1px solid #E8D4C4;
        border-radius: 12px;
        padding: 20px;
    }

    .metric-card .label {
        font-size: 0.75rem;
        font-weight: 500;
        color: #6b7280;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .metric-card .value {
        font-size: 2rem;
        font-weight: 700;
        color: #C4A484;
        margin-top: 4px;
    }

    .metric-card .trend {
        font-size: 0.75rem;
        margin-top: 4px;
        display: flex;
        align-items: center;
        gap: 4px;
    }

    .trend-up { color: #22c55e; }
    .trend-down { color: #dc2626; }

    /* Buttons */
    .btn {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 8px 16px;
        border-radius: 8px;
        font-size: 0.875rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
        font-family: inherit;
    }

    .btn-primary {
        background: #14b8a6;
        color: white;
    }

    .btn-primary:hover {
        background: rgba(20, 184, 166, 0.9);
    }

    .btn-ghost {
        background: transparent;
        border: 1px solid #14b8a6;
        color: #14b8a6;
    }

    .btn-ghost:hover {
        background: rgba(20, 184, 166, 0.1);
    }

    .btn-danger {
        background: #dc2626;
        color: white;
    }

    .btn-danger:hover {
        background: #b91c1c;
    }

    .btn-sm {
        padding: 4px 12px;
        font-size: 0.8125rem;
    }

    /* Inputs */
    .input-group {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .input-label {
        font-size: 0.875rem;
        font-weight: 500;
        color: #111827;
    }

    .input-field {
        border: 1px solid #E8D4C4;
        border-radius: 8px;
        padding: 8px 12px;
        background: #FAF8F5;
        font-size: 0.875rem;
        font-family: inherit;
        color: #111827;
        outline: none;
        transition: border-color 0.2s;
    }

    .input-field:focus {
        border-color: #14b8a6;
        box-shadow: 0 0 0 3px rgba(20, 184, 166, 0.1);
    }

    .input-error {
        border-color: #dc2626;
    }

    .input-help {
        font-size: 0.75rem;
        color: #6b7280;
    }

    .input-error-msg {
        font-size: 0.75rem;
        color: #dc2626;
    }

    /* Table */
    .data-table {
        width: 100%;
        border-collapse: collapse;
    }

    .data-table th {
        text-align: left;
        padding: 12px 16px;
        font-size: 0.75rem;
        font-weight: 600;
        color: #6b7280;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        border-bottom: 2px solid #E8D4C4;
    }

    .data-table td {
        padding: 12px 16px;
        font-size: 0.875rem;
        border-bottom: 1px solid #F5EDE8;
    }

    .data-table tr:hover td {
        background: #FAF8F5;
    }

    /* Badges */
    .badge {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 2px 10px;
        border-radius: 9999px;
        font-size: 0.75rem;
        font-weight: 500;
    }

    .badge-online { background: rgba(34,197,94,0.12); color: #22c55e; }
    .badge-training { background: rgba(20,184,166,0.12); color: #14b8a6; }
    .badge-error { background: rgba(220,38,38,0.12); color: #dc2626; }
    .badge-deployed { background: rgba(59,130,246,0.12); color: #3b82f6; }
    .badge-pending { background: rgba(249,115,22,0.12); color: #f97316; }
    .badge-completed { background: rgba(34,197,94,0.12); color: #22c55e; }
    .badge-ready { background: rgba(20,184,166,0.12); color: #14b8a6; }

    .badge-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: currentColor;
    }

    /* Modal */
    .modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0,0,0,0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
    }

    .modal {
        background: #FFFDF7;
        border-radius: 16px;
        padding: 32px;
        max-width: 500px;
        width: 90%;
        box-shadow: 0 25px 50px -12px rgba(0,0,0,0.25);
    }

    .modal-title {
        font-size: 1.25rem;
        font-weight: 600;
        margin-bottom: 16px;
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 12px;
        margin-top: 24px;
    }

    /* Toast — slide in from right */
    .toast-container {
        position: fixed;
        top: 24px;
        right: 24px;
        z-index: 200;
        display: flex;
        flex-direction: column;
        gap: 10px;
        max-width: 400px;
    }

    .toast {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 14px 18px;
        border-radius: 10px;
        font-size: 0.875rem;
        font-weight: 500;
        color: white;
        background: linear-gradient(135deg, #14b8a6, #0d9488);
        box-shadow: 0 8px 24px rgba(0,0,0,0.12), 0 2px 8px rgba(0,0,0,0.06);
        animation: toast-slide-in 0.35s cubic-bezier(0.21, 1.02, 0.73, 1) forwards;
        cursor: pointer;
        min-width: 280px;
    }

    .toast.toast-out {
        animation: toast-slide-out 0.3s ease-in forwards;
    }

    .toast-error { background: linear-gradient(135deg, #ef4444, #dc2626); }
    .toast-warning { background: linear-gradient(135deg, #D4A574, #C4A484); }
    .toast-info { background: linear-gradient(135deg, #3b82f6, #2563eb); }

    .toast-icon {
        flex-shrink: 0;
        width: 20px;
        height: 20px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 16px;
    }

    .toast-body {
        flex: 1;
        line-height: 1.4;
    }

    .toast-close {
        flex-shrink: 0;
        background: none;
        border: none;
        color: rgba(255,255,255,0.7);
        cursor: pointer;
        font-size: 18px;
        padding: 0 2px;
        line-height: 1;
        transition: color 0.15s;
    }

    .toast-close:hover {
        color: white;
    }

    .toast-progress {
        position: absolute;
        bottom: 0;
        left: 0;
        height: 3px;
        background: rgba(255,255,255,0.35);
        border-radius: 0 0 10px 10px;
        animation: toast-progress-shrink 4s linear forwards;
    }

    .toast { position: relative; overflow: hidden; }

    @keyframes toast-slide-in {
        from { transform: translateX(120%); opacity: 0; }
        to { transform: translateX(0); opacity: 1; }
    }

    @keyframes toast-slide-out {
        from { transform: translateX(0); opacity: 1; }
        to { transform: translateX(120%); opacity: 0; }
    }

    @keyframes toast-progress-shrink {
        from { width: 100%; }
        to { width: 0%; }
    }

    /* File Upload */
    .file-upload {
        border: 2px dashed #E8D4C4;
        border-radius: 12px;
        padding: 48px;
        text-align: center;
        cursor: pointer;
        transition: all 0.2s;
        background: #FAF8F5;
    }

    .file-upload:hover, .file-upload.drag-over {
        border-color: #14b8a6;
        background: rgba(20, 184, 166, 0.05);
    }

    .file-upload-icon {
        width: 48px;
        height: 48px;
        margin: 0 auto 16px;
        color: #C4A484;
    }

    .file-upload-text {
        font-size: 0.875rem;
        color: #6b7280;
    }

    .file-upload-text strong {
        color: #14b8a6;
    }

    /* Progress Bar */
    .progress-bar {
        height: 8px;
        background: #F5EDE8;
        border-radius: 4px;
        overflow: hidden;
    }

    .progress-bar-fill {
        height: 100%;
        background: linear-gradient(to right, #14b8a6, #0d9488);
        border-radius: 4px;
        transition: width 0.3s ease;
    }

    /* Skeleton Loader */
    .skeleton {
        background: linear-gradient(90deg, #F5EDE8 25%, #FAF8F5 50%, #F5EDE8 75%);
        background-size: 200% 100%;
        animation: shimmer 1.5s infinite;
        border-radius: 8px;
    }

    @keyframes shimmer {
        0% { background-position: 200% 0; }
        100% { background-position: -200% 0; }
    }

    /* Chart Container */
    .chart-container {
        width: 100%;
        padding: 16px;
    }

    .chart-container svg {
        width: 100%;
        height: auto;
    }

    /* Code Block */
    .code-block {
        background: #1a1a2e;
        color: #e2e8f0;
        border-radius: 8px;
        padding: 16px;
        font-family: 'JetBrains Mono', monospace;
        font-size: 0.8125rem;
        overflow-x: auto;
        line-height: 1.6;
    }

    /* Pipeline Visualization */
    .pipeline {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 24px;
        overflow-x: auto;
    }

    .pipeline-stage {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        padding: 16px 24px;
        border-radius: 12px;
        border: 2px solid #E8D4C4;
        background: #FAF8F5;
        min-width: 120px;
        text-align: center;
        transition: all 0.3s;
    }

    .pipeline-stage.active {
        border-color: #14b8a6;
        background: rgba(20, 184, 166, 0.08);
    }

    .pipeline-stage.completed {
        border-color: #22c55e;
        background: rgba(34, 197, 94, 0.08);
    }

    .pipeline-arrow {
        color: #E8D4C4;
        font-size: 1.5rem;
        flex-shrink: 0;
    }

    .pipeline-stage-label {
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: #6b7280;
    }

    .pipeline-stage-name {
        font-size: 0.875rem;
        font-weight: 500;
        color: #111827;
    }

    /* Chat */
    .chat-container {
        display: flex;
        flex-direction: column;
        height: calc(100vh - 160px);
    }

    .chat-messages {
        flex: 1;
        overflow-y: auto;
        padding: 24px;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .chat-message {
        max-width: 80%;
        padding: 12px 16px;
        border-radius: 12px;
        font-size: 0.875rem;
        line-height: 1.6;
    }

    .chat-message.user {
        align-self: flex-end;
        background: #14b8a6;
        color: white;
        border-bottom-right-radius: 4px;
    }

    .chat-message.assistant {
        align-self: flex-start;
        background: #FAF8F5;
        border: 1px solid #E8D4C4;
        color: #111827;
        border-bottom-left-radius: 4px;
    }

    .chat-input-area {
        padding: 16px 24px;
        border-top: 1px solid #E8D4C4;
        display: flex;
        gap: 12px;
    }

    .chat-input {
        flex: 1;
        border: 1px solid #E8D4C4;
        border-radius: 12px;
        padding: 12px 16px;
        font-size: 0.875rem;
        font-family: inherit;
        resize: none;
        background: #FAF8F5;
        outline: none;
    }

    .chat-input:focus {
        border-color: #14b8a6;
    }

    /* Grid layouts */
    .grid-2 { display: grid; grid-template-columns: repeat(2, 1fr); gap: 24px; }
    .grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 24px; }
    .grid-4 { display: grid; grid-template-columns: repeat(4, 1fr); gap: 24px; }

    .flex-between {
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    .text-sm { font-size: 0.875rem; }
    .text-xs { font-size: 0.75rem; }
    .text-muted { color: #6b7280; }
    .text-primary { color: #14b8a6; }
    .text-bold { font-weight: 600; }
    .mt-4 { margin-top: 16px; }
    .mt-8 { margin-top: 32px; }
    .mb-4 { margin-bottom: 16px; }
    .mb-8 { margin-bottom: 32px; }
    .gap-4 { gap: 16px; }

    /* Responsive */
    @media (max-width: 768px) {
        .sidebar { transform: translateX(-100%); }
        .sidebar.open { transform: translateX(0); }
        .main-content { margin-left: 0; }
        .metric-grid { grid-template-columns: repeat(2, 1fr); }
        .grid-2, .grid-3, .grid-4 { grid-template-columns: 1fr; }
        .page-content { padding: 16px; }
    }
    "#
}
