// ============================================================================
// File: code_block.rs
// Description: Preformatted code display component with language support
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;

#[component]
pub fn CodeBlock(
    code: String,
    #[prop(optional)] language: Option<&'static str>,
) -> impl IntoView {
    let _lang = language.unwrap_or("json");
    view! {
        <div class="code-block">
            <pre><code>{code}</code></pre>
        </div>
    }
}
