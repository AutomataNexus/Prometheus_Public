// ============================================================================
// File: theme.rs
// Description: NexusEdge branded terminal theme with ANSI colors and styled output helpers
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! NexusEdge branded terminal theme for Prometheus CLI.

/// ANSI RGB escape: \x1b[38;2;R;G;Bm
const TEAL: &str = "\x1b[38;2;20;184;166m";
const RUSSET: &str = "\x1b[38;2;194;113;79m";
const TERRACOTTA: &str = "\x1b[38;2;196;164;132m";
const CREAM: &str = "\x1b[38;2;255;253;247m";
const SUCCESS: &str = "\x1b[38;2;34;197;94m";
const WARNING: &str = "\x1b[38;2;249;115;22m";
const ERROR: &str = "\x1b[38;2;220;38;38m";
const INFO: &str = "\x1b[38;2;59;130;246m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

pub fn print_banner() {
    println!();
    println!("{TEAL}{BOLD}  ██████╗ ██████╗  ██████╗ ███╗   ███╗███████╗████████╗██╗  ██╗███████╗██╗   ██╗███████╗{RESET}");
    println!("{TEAL}{BOLD}  ██╔══██╗██╔══██╗██╔═══██╗████╗ ████║██╔════╝╚══██╔══╝██║  ██║██╔════╝██║   ██║██╔════╝{RESET}");
    println!("{TEAL}{BOLD}  ██████╔╝██████╔╝██║   ██║██╔████╔██║█████╗     ██║   ███████║█████╗  ██║   ██║███████╗{RESET}");
    println!("{TEAL}{BOLD}  ██╔═══╝ ██╔══██╗██║   ██║██║╚██╔╝██║██╔══╝     ██║   ██╔══██║██╔══╝  ██║   ██║╚════██║{RESET}");
    println!("{TEAL}{BOLD}  ██║     ██║  ██║╚██████╔╝██║ ╚═╝ ██║███████╗   ██║   ██║  ██║███████╗╚██████╔╝███████║{RESET}");
    println!("{TEAL}{BOLD}  ╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝ ╚═════╝ ╚══════╝{RESET}");
    println!();
    println!("  {TERRACOTTA}AI-Forged Edge Intelligence{RESET}  {DIM}v{}{RESET}", env!("CARGO_PKG_VERSION"));
    println!("  {DIM}Cloud-orchestrated ML for the physical world{RESET}");
    println!();
}

pub fn print_prompt() {
    print!("{TEAL}{BOLD}prometheus{RESET}{RUSSET}>{RESET} ");
}

pub fn print_success(msg: &str) {
    println!("{SUCCESS}  \u{2713} {msg}{RESET}");
}

pub fn print_error(msg: &str) {
    println!("{ERROR}  \u{2717} {msg}{RESET}");
}

pub fn print_warning(msg: &str) {
    println!("{WARNING}  \u{26a0} {msg}{RESET}");
}

pub fn print_info(msg: &str) {
    println!("{INFO}  \u{2139} {msg}{RESET}");
}

pub fn styled_header(text: &str) -> String {
    format!("{TEAL}{BOLD}\u{2500}\u{2500} {text} \u{2500}\u{2500}{RESET}")
}

pub fn styled_label(label: &str, value: &str) -> String {
    format!("  {TERRACOTTA}{label:<18}{RESET} {CREAM}{value}{RESET}")
}

pub fn styled_id(id: &str) -> String {
    format!("{TEAL}{id}{RESET}")
}

pub fn styled_status(status: &str) -> String {
    match status {
        "running" | "active" | "ready" => format!("{SUCCESS}{status}{RESET}"),
        "completed" | "deployed" => format!("{SUCCESS}{status}{RESET}"),
        "failed" | "error" => format!("{ERROR}{status}{RESET}"),
        "stopping" | "cancelled" | "pending" | "queued" => format!("{WARNING}{status}{RESET}"),
        _ => format!("{DIM}{status}{RESET}"),
    }
}

pub fn table_header(cols: &[(&str, usize)]) -> String {
    let header = cols.iter()
        .map(|(val, width)| format!("{BOLD}{:<width$}{RESET}", val, width = width))
        .collect::<Vec<_>>()
        .join("  ");
    let divider = cols.iter()
        .map(|(_, width)| "\u{2500}".repeat(*width))
        .collect::<Vec<_>>()
        .join("\u{2500}\u{2500}");
    format!("  {TERRACOTTA}{header}{RESET}\n  {DIM}{divider}{RESET}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styled_header_contains_text() {
        let result = styled_header("My Section");
        assert!(result.contains("My Section"));
        // Should contain ANSI reset at end
        assert!(result.contains(RESET));
        // Should contain the box-drawing characters
        assert!(result.contains("\u{2500}\u{2500}"));
    }

    #[test]
    fn test_styled_label_contains_label_and_value() {
        let result = styled_label("Name", "TestModel");
        assert!(result.contains("Name"));
        assert!(result.contains("TestModel"));
        assert!(result.contains(RESET));
    }

    #[test]
    fn test_styled_id_wraps_with_teal() {
        let result = styled_id("abc-123");
        assert!(result.contains("abc-123"));
        assert!(result.starts_with(TEAL));
        assert!(result.ends_with(RESET));
    }

    #[test]
    fn test_styled_status_running() {
        let result = styled_status("running");
        assert!(result.contains("running"));
        assert!(result.contains(SUCCESS));
    }

    #[test]
    fn test_styled_status_active() {
        let result = styled_status("active");
        assert!(result.contains("active"));
        assert!(result.contains(SUCCESS));
    }

    #[test]
    fn test_styled_status_ready() {
        let result = styled_status("ready");
        assert!(result.contains(SUCCESS));
    }

    #[test]
    fn test_styled_status_completed() {
        let result = styled_status("completed");
        assert!(result.contains("completed"));
        assert!(result.contains(SUCCESS));
    }

    #[test]
    fn test_styled_status_deployed() {
        let result = styled_status("deployed");
        assert!(result.contains(SUCCESS));
    }

    #[test]
    fn test_styled_status_failed() {
        let result = styled_status("failed");
        assert!(result.contains("failed"));
        assert!(result.contains(ERROR));
    }

    #[test]
    fn test_styled_status_error() {
        let result = styled_status("error");
        assert!(result.contains(ERROR));
    }

    #[test]
    fn test_styled_status_pending() {
        let result = styled_status("pending");
        assert!(result.contains("pending"));
        assert!(result.contains(WARNING));
    }

    #[test]
    fn test_styled_status_stopping() {
        let result = styled_status("stopping");
        assert!(result.contains(WARNING));
    }

    #[test]
    fn test_styled_status_cancelled() {
        let result = styled_status("cancelled");
        assert!(result.contains(WARNING));
    }

    #[test]
    fn test_styled_status_unknown() {
        let result = styled_status("something_else");
        assert!(result.contains("something_else"));
        // Unknown statuses use DIM
        assert!(result.contains(DIM));
        // Should NOT contain SUCCESS/ERROR/WARNING colors
        assert!(!result.contains(SUCCESS));
        assert!(!result.contains(ERROR));
        assert!(!result.contains(WARNING));
    }

    #[test]
    fn test_table_header_single_column() {
        let result = table_header(&[("Name", 20)]);
        assert!(result.contains("Name"));
        // Should contain divider line with box-drawing characters
        assert!(result.contains("\u{2500}"));
        // Should have a newline separating header from divider
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_table_header_multiple_columns() {
        let result = table_header(&[("ID", 10), ("Status", 12), ("Name", 20)]);
        assert!(result.contains("ID"));
        assert!(result.contains("Status"));
        assert!(result.contains("Name"));
    }

    #[test]
    fn test_table_header_divider_length() {
        // The divider should use the specified widths
        let cols = &[("A", 5), ("B", 10)];
        let result = table_header(cols);
        // Divider: 5 box-chars + 2 separator box-chars + 10 box-chars = 17 box-chars total
        let divider_line = result.lines().nth(1).unwrap();
        let box_count = divider_line.matches('\u{2500}').count();
        assert_eq!(box_count, 5 + 2 + 10, "Divider should have correct number of box-drawing chars");
    }

    #[test]
    fn test_styled_header_empty_string() {
        let result = styled_header("");
        // Even with empty text, should still have the framing chars
        assert!(result.contains("\u{2500}\u{2500}"));
    }

    #[test]
    fn test_styled_label_alignment() {
        // Label is left-aligned in an 18-char field
        let result = styled_label("X", "Y");
        assert!(result.contains("X"));
        assert!(result.contains("Y"));
    }
}
