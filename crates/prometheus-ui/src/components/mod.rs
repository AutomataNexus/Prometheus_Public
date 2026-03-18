// ============================================================================
// File: mod.rs
// Description: Component module declarations and re-exports for the UI component library
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

pub mod layout;
pub mod sidebar;
pub mod header;
pub mod card;
pub mod button;
pub mod input;
pub mod modal;
pub mod table;
pub mod chart;
pub mod metric_card;
pub mod toast;
pub mod loader;
pub mod badge;
pub mod file_upload;
pub mod code_block;
pub mod icon;
pub mod ingestion_keys;

pub use layout::*;
pub use sidebar::*;
pub use header::*;
pub use card::*;
pub use button::*;
pub use input::*;
pub use modal::*;
pub use table::*;
pub use chart::*;
pub use metric_card::*;
pub use toast::*;
pub use loader::*;
pub use badge::*;
pub use file_upload::*;
pub use code_block::*;
pub use icon::*;
pub use ingestion_keys::*;
