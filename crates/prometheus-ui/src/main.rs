// ============================================================================
// File: main.rs
// Description: WASM entry point that mounts the Leptos UI application to the DOM
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use prometheus_ui::App;

fn main() {
    leptos::mount::mount_to_body(App);
}
