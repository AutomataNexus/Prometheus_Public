// ============================================================================
// File: app.rs
// Description: Root App component with client-side router and route definitions
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use crate::components::layout::AppShell;
use crate::pages::*;
use crate::theme;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <style>{theme::global_styles()}</style>
        <Router>
            <Routes fallback=|| "Page not found.">
                <Route path=path!("/") view=LandingPage />
                <Route path=path!("/login") view=LoginPage />
                <Route path=path!("/auth/verify") view=CliVerifyPage />
                <ParentRoute path=path!("/") view=AuthenticatedLayout>
                    <Route path=path!("/dashboard") view=HomePage />
                    <Route path=path!("/datasets") view=DatasetsPage />
                    <Route path=path!("/datasets/:id") view=DatasetDetailPage />
                    <Route path=path!("/training") view=TrainingPage />
                    <Route path=path!("/training/:id") view=TrainingDetailPage />
                    <Route path=path!("/monitor") view=MonitorPage />
                    <Route path=path!("/models") view=ModelsPage />
                    <Route path=path!("/models/:id") view=ModelDetailPage />
                    <Route path=path!("/convert") view=ConvertPage />
                    <Route path=path!("/quantize") view=QuantizePage />
                    <Route path=path!("/deployment") view=DeploymentPage />
                    <Route path=path!("/evaluation") view=EvaluationPage />
                    <Route path=path!("/agent") view=AgentPage />
                    <Route path=path!("/billing") view=BillingPage />
                    <Route path=path!("/settings") view=SettingsPage />
                    <Route path=path!("/admin") view=AdminPage />
                </ParentRoute>
            </Routes>
        </Router>
    }
}

#[component]
fn AuthenticatedLayout() -> impl IntoView {
    // Check for auth token — redirect to login if missing
    let has_token = crate::api::get_token().is_some();
    if !has_token {
        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href("/login");
        }
        return view! { <div>"Redirecting..."</div> }.into_any();
    }

    view! {
        <AppShell>
            <Outlet />
        </AppShell>
    }.into_any()
}
