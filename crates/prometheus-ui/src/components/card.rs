// ============================================================================
// File: card.rs
// Description: Card container component with optional title and header actions
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos::children::Children;

#[component]
pub fn Card(
    #[prop(optional)] title: Option<&'static str>,
    #[prop(optional)] class: Option<&'static str>,
    #[prop(optional)] header_right: Option<Children>,
    children: Children,
) -> impl IntoView {
    let class_str = format!("card {}", class.unwrap_or(""));
    view! {
        <div class=class_str>
            {title.map(|t| view! {
                <div class="card-header">
                    <h3 class="card-title">{t}</h3>
                    {header_right.map(|hr| hr())}
                </div>
            })}
            {children()}
        </div>
    }
}
