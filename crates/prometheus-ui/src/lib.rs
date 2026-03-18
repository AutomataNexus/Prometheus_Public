// ============================================================================
// File: lib.rs
// Description: Crate root for prometheus-ui exposing all modules and the App component
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

pub mod api;
pub mod app;
pub mod theme;
pub mod components;
pub mod pages;
pub mod icons;

pub use app::App;
