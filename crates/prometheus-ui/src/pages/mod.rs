// ============================================================================
// File: mod.rs
// Description: Page module declarations and re-exports for all application views
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

pub mod home;
pub mod login;
pub mod datasets;
pub mod dataset_detail;
pub mod training;
pub mod training_detail;
pub mod models;
pub mod model_detail;
pub mod convert;
pub mod deployment;
pub mod evaluation;
pub mod agent;
pub mod settings;
pub mod landing;
pub mod cli_verify;
pub mod monitor;
pub mod billing;
pub mod quantize;
pub mod admin;

pub use home::*;
pub use login::*;
pub use datasets::*;
pub use dataset_detail::*;
pub use training::*;
pub use training_detail::*;
pub use models::*;
pub use model_detail::*;
pub use convert::*;
pub use deployment::*;
pub use evaluation::*;
pub use agent::*;
pub use settings::*;
pub use landing::*;
pub use cli_verify::*;
pub use monitor::*;
pub use billing::*;
pub use quantize::*;
pub use admin::*;
