// ============================================================================
// File: mod.rs
// Description: Email template module re-exports for all transactional email types
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
pub mod layout;
pub mod welcome;
pub mod verification;
pub mod password_reset;
pub mod support;
pub mod security_alert;
pub mod daily_report;

pub use security_alert::{SecurityAlert, AlertSeverity};
pub use daily_report::{DailyReport, ThreatEntry};
