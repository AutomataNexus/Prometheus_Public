// ============================================================================
// File: landing.rs
// Description: Public landing page with product features and call-to-action sections
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================

use leptos::prelude::*;

// SVG icon helpers — small inline teal icons for feature cards
fn icon_cube() -> impl IntoView {
    view! {
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#14b8a6" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path>
            <polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline>
            <line x1="12" y1="22.08" x2="12" y2="12"></line>
        </svg>
    }
}

fn icon_refresh() -> impl IntoView {
    view! {
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#14b8a6" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="23 4 23 10 17 10"></polyline>
            <polyline points="1 20 1 14 7 14"></polyline>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
        </svg>
    }
}

fn icon_zap() -> impl IntoView {
    view! {
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#14b8a6" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
        </svg>
    }
}

fn icon_cpu() -> impl IntoView {
    view! {
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#14b8a6" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect x="4" y="4" width="16" height="16" rx="2" ry="2"></rect>
            <rect x="9" y="9" width="6" height="6"></rect>
            <line x1="9" y1="1" x2="9" y2="4"></line>
            <line x1="15" y1="1" x2="15" y2="4"></line>
            <line x1="9" y1="20" x2="9" y2="23"></line>
            <line x1="15" y1="20" x2="15" y2="23"></line>
            <line x1="20" y1="9" x2="23" y2="9"></line>
            <line x1="20" y1="14" x2="23" y2="14"></line>
            <line x1="1" y1="9" x2="4" y2="9"></line>
            <line x1="1" y1="14" x2="4" y2="14"></line>
        </svg>
    }
}

fn icon_bar_chart() -> impl IntoView {
    view! {
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#14b8a6" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="20" x2="18" y2="10"></line>
            <line x1="12" y1="20" x2="12" y2="4"></line>
            <line x1="6" y1="20" x2="6" y2="14"></line>
        </svg>
    }
}

fn icon_shield() -> impl IntoView {
    view! {
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#14b8a6" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
        </svg>
    }
}

#[allow(dead_code)]
fn icon_smartphone() -> impl IntoView {
    view! {
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#14b8a6" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect x="5" y="2" width="14" height="20" rx="2" ry="2"></rect>
            <line x1="12" y1="18" x2="12.01" y2="18"></line>
        </svg>
    }
}

#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <div style="min-height: 100vh; background: #FFFDF7; font-family: 'Inter', system-ui, sans-serif;">
            // Header
            <header style="position: fixed; top: 0; left: 0; right: 0; z-index: 50; background: rgba(255,253,247,0.95); backdrop-filter: blur(12px); border-bottom: 1px solid rgba(232,212,196,0.5);">
                <div style="max-width: 1200px; margin: 0 auto; padding: 0 24px;">
                    <nav style="display: flex; align-items: center; justify-content: space-between; height: 72px;">
                        // Logo
                        <a href="/" style="display: flex; align-items: center; gap: 12px; text-decoration: none;">
                            <img src="/assets/logo.png?v=3" alt="Prometheus" style="width: 52px; height: 52px; border-radius: 12px;" />
                            <div>
                                <span style="font-weight: 700; font-size: 1.25rem; color: #111827;">
                                    "Prometheus"
                                </span>
                                <div style="height: 2px; background: linear-gradient(to right, #14b8a6, #C4A484); border-radius: 1px; margin-top: 2px;"></div>
                            </div>
                        </a>

                        // Desktop Nav
                        <div style="display: flex; align-items: center; gap: 32px;">
                            <a href="#features" style="color: #6b7280; text-decoration: none; font-weight: 500; font-size: 0.9rem; transition: color 0.2s;">"Features"</a>
                            <a href="#pipeline" style="color: #6b7280; text-decoration: none; font-weight: 500; font-size: 0.9rem; transition: color 0.2s;">"Pipeline"</a>
                            <a href="#pricing" style="color: #6b7280; text-decoration: none; font-weight: 500; font-size: 0.9rem; transition: color 0.2s;">"Pricing"</a>
                            <a href="https://automatanexus.com" target="_blank" rel="noopener" style="color: #6b7280; text-decoration: none; font-weight: 500; font-size: 0.9rem;">"AutomataNexus"</a>
                        </div>

                        // CTA
                        <div style="display: flex; align-items: center; gap: 12px;">
                            <a href="/login" style="color: #14b8a6; text-decoration: none; font-weight: 500; font-size: 0.9rem;">"Sign In"</a>
                            <a href="/login" style="display: inline-flex; align-items: center; gap: 8px; padding: 8px 20px; border-radius: 8px; background: #14b8a6; color: white; text-decoration: none; font-weight: 500; font-size: 0.875rem; transition: background 0.2s;">"Get Started"</a>
                        </div>
                    </nav>
                </div>
            </header>

            // Hero Section
            <section style="padding: 160px 24px 80px; text-align: center; position: relative; overflow: hidden;">
                <div style="position: absolute; inset: 0; background: linear-gradient(135deg, rgba(20,184,166,0.06) 0%, rgba(196,164,132,0.06) 50%, rgba(255,253,247,0) 100%); pointer-events: none;"></div>
                <div style="position: absolute; top: 80px; right: 10%; width: 300px; height: 300px; border-radius: 50%; background: radial-gradient(circle, rgba(20,184,166,0.08) 0%, transparent 70%); pointer-events: none;"></div>
                <div style="position: absolute; bottom: 0; left: 5%; width: 400px; height: 400px; border-radius: 50%; background: radial-gradient(circle, rgba(196,164,132,0.06) 0%, transparent 70%); pointer-events: none;"></div>

                <div style="position: relative; max-width: 800px; margin: 0 auto;">
                    <div style="display: inline-flex; align-items: center; gap: 8px; padding: 6px 16px; border-radius: 9999px; background: rgba(20,184,166,0.1); border: 1px solid rgba(20,184,166,0.2); margin-bottom: 24px;">
                        <span style="width: 8px; height: 8px; border-radius: 50%; background: #14b8a6;"></span>
                        <span style="font-size: 0.8rem; font-weight: 500; color: #0d9488;">"Now in Beta"</span>
                    </div>

                    <h1 style="font-size: 3.5rem; font-weight: 800; line-height: 1.1; color: #111827; margin-bottom: 24px;">
                        "AI-Forged"<br />
                        <span style="background: linear-gradient(135deg, #14b8a6, #0d9488); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;">"Edge Intelligence"</span>
                    </h1>
                    <p style="font-size: 1.25rem; color: #6b7280; max-width: 600px; margin: 0 auto 40px; line-height: 1.7;">
                        "Train, optimize, and deploy custom AI models to edge hardware in minutes. From .axonml to ONNX and Hailo-8 HEF — the complete ML pipeline for building automation."
                    </p>
                    <div style="display: flex; align-items: center; justify-content: center; gap: 16px; flex-wrap: wrap;">
                        <a href="/login" style="display: inline-flex; align-items: center; gap: 8px; padding: 14px 32px; border-radius: 10px; background: #14b8a6; color: white; text-decoration: none; font-weight: 600; font-size: 1rem; box-shadow: 0 4px 14px rgba(20,184,166,0.3); transition: all 0.2s;">
                            "Start Building"
                            <span style="font-size: 1.2rem;">{"\u{2192}"}</span>
                        </a>
                        <a href="#features" style="display: inline-flex; align-items: center; gap: 8px; padding: 14px 32px; border-radius: 10px; background: transparent; border: 1px solid #E8D4C4; color: #111827; text-decoration: none; font-weight: 500; font-size: 1rem; transition: all 0.2s;">
                            "Learn More"
                        </a>
                    </div>
                </div>
            </section>

            // Stats Section
            <section style="padding: 40px 24px 80px;">
                <div style="max-width: 1000px; margin: 0 auto; display: grid; grid-template-columns: repeat(4, 1fr); gap: 24px; text-align: center;">
                    {[
                        ("Sub-ms", "Inference Latency"),
                        ("ONNX + HEF", "Export Formats"),
                        ("Hailo-8", "NPU Deployment"),
                        ("End-to-End", "ML Pipeline"),
                    ].into_iter().map(|(value, label)| {
                        view! {
                            <div style="padding: 24px 16px;">
                                <div style="font-size: 1.75rem; font-weight: 800; color: #C4A484; margin-bottom: 4px;">{value}</div>
                                <div style="font-size: 0.8rem; color: #6b7280; font-weight: 500;">{label}</div>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </section>

            // Pipeline Section
            <section id="pipeline" style="padding: 80px 24px; background: #FAF8F5;">
                <div style="max-width: 1000px; margin: 0 auto; text-align: center;">
                    <h2 style="font-size: 2rem; font-weight: 700; color: #111827; margin-bottom: 12px;">"The Complete ML Pipeline"</h2>
                    <p style="font-size: 1rem; color: #6b7280; margin-bottom: 48px; max-width: 600px; margin-left: auto; margin-right: auto;">
                        "From raw data to deployed edge models — every step managed in one platform."
                    </p>

                    <div style="display: flex; align-items: center; justify-content: center; gap: 8px; flex-wrap: wrap; margin-bottom: 48px;">
                        {[
                            ("Ingest", "Upload datasets, connect live sources"),
                            ("Analyze", "Automated data quality & feature analysis"),
                            ("Train", "GPU-accelerated model training with PyTorch"),
                            ("Evaluate", "Benchmark accuracy, latency, efficiency"),
                            ("Convert", "Export to ONNX and Hailo-8 HEF"),
                            ("Deploy", "Push to NexusEdge controllers"),
                        ].into_iter().map(|(stage, desc)| {
                            view! {
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <div style="min-width: 130px; padding: 20px 16px; border-radius: 12px; border: 2px solid #E8D4C4; background: #FFFDF7; text-align: center; transition: all 0.3s;">
                                        <div style="font-size: 0.7rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: #14b8a6; margin-bottom: 4px;">{stage}</div>
                                        <div style="font-size: 0.7rem; color: #6b7280; line-height: 1.4;">{desc}</div>
                                    </div>
                                    <span style="color: #E8D4C4; font-size: 1.25rem; flex-shrink: 0;">{"\u{2192}"}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </section>

            // Features Section
            <section id="features" style="padding: 80px 24px;">
                <div style="max-width: 1100px; margin: 0 auto;">
                    <div style="text-align: center; margin-bottom: 48px;">
                        <h2 style="font-size: 2rem; font-weight: 700; color: #111827; margin-bottom: 12px;">"Why Prometheus?"</h2>
                        <p style="font-size: 1rem; color: #6b7280; max-width: 500px; margin: 0 auto;">
                            "Purpose-built for deploying AI to building automation edge hardware."
                        </p>
                    </div>

                    <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 24px;">
                        <div style="padding: 32px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7; transition: all 0.3s;">
                            <div style="margin-bottom: 16px;">{icon_cube()}</div>
                            <h3 style="font-size: 1.1rem; font-weight: 600; color: #111827; margin-bottom: 8px;">"AxonML Format"</h3>
                            <p style="font-size: 0.875rem; color: #6b7280; line-height: 1.6;">"Native .axonml model format designed for efficient edge inference on constrained hardware."</p>
                        </div>
                        <div style="padding: 32px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7; transition: all 0.3s;">
                            <div style="margin-bottom: 16px;">{icon_refresh()}</div>
                            <h3 style="font-size: 1.1rem; font-weight: 600; color: #111827; margin-bottom: 8px;">"ONNX Export"</h3>
                            <p style="font-size: 0.875rem; color: #6b7280; line-height: 1.6;">"Convert models to ONNX for broad compatibility across inference runtimes and hardware."</p>
                        </div>
                        <div style="padding: 32px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7; transition: all 0.3s;">
                            <div style="margin-bottom: 16px;">{icon_zap()}</div>
                            <h3 style="font-size: 1.1rem; font-weight: 600; color: #111827; margin-bottom: 8px;">"Hailo-8 HEF"</h3>
                            <p style="font-size: 0.875rem; color: #6b7280; line-height: 1.6;">"Compile directly to Hailo-8 NPU format for sub-millisecond inference on NexusEdge controllers."</p>
                        </div>
                        <div style="padding: 32px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7; transition: all 0.3s;">
                            <div style="margin-bottom: 16px;">{icon_cpu()}</div>
                            <h3 style="font-size: 1.1rem; font-weight: 600; color: #111827; margin-bottom: 8px;">"GPU Training"</h3>
                            <p style="font-size: 0.875rem; color: #6b7280; line-height: 1.6;">"Distributed PyTorch training with automatic hyperparameter tuning and early stopping."</p>
                        </div>
                        <div style="padding: 32px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7; transition: all 0.3s;">
                            <div style="margin-bottom: 16px;">{icon_bar_chart()}</div>
                            <h3 style="font-size: 1.1rem; font-weight: 600; color: #111827; margin-bottom: 8px;">"Live Evaluation"</h3>
                            <p style="font-size: 0.875rem; color: #6b7280; line-height: 1.6;">"A/B testing and real-time accuracy monitoring against production data."</p>
                        </div>
                        <div style="padding: 32px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7; transition: all 0.3s;">
                            <div style="margin-bottom: 16px;">{icon_shield()}</div>
                            <h3 style="font-size: 1.1rem; font-weight: 600; color: #111827; margin-bottom: 8px;">"Secure by Design"</h3>
                            <p style="font-size: 0.875rem; color: #6b7280; line-height: 1.6;">"End-to-end encryption, MFA, role-based access, and complete audit trails."</p>
                        </div>
                    </div>
                </div>
            </section>

            // Mobile App Section
            <section style="padding: 80px 24px; background: #FAF8F5;">
                <div style="max-width: 1000px; margin: 0 auto;">
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 64px; align-items: center;">
                        <div>
                            <div style="display: inline-flex; align-items: center; gap: 8px; padding: 6px 16px; border-radius: 9999px; background: rgba(20,184,166,0.1); border: 1px solid rgba(20,184,166,0.2); margin-bottom: 20px;">
                                <span style="font-size: 0.8rem; font-weight: 500; color: #0d9488;">"iOS & Android"</span>
                            </div>
                            <h2 style="font-size: 2rem; font-weight: 700; color: #111827; margin-bottom: 16px;">"Manage Models on the Go"</h2>
                            <p style="font-size: 1rem; color: #6b7280; line-height: 1.7; margin-bottom: 24px;">
                                "The Prometheus companion app puts your entire ML pipeline in your pocket. Monitor training runs, review model metrics, manage deployments, and receive real-time notifications — all from your phone."
                            </p>
                            <ul style="list-style: none; padding: 0; display: flex; flex-direction: column; gap: 14px;">
                                <li style="display: flex; align-items: center; gap: 10px; font-size: 0.9rem; color: #374151;">
                                    <span style="width: 6px; height: 6px; border-radius: 50%; background: #14b8a6; flex-shrink: 0;"></span>
                                    "Monitor training progress and model accuracy in real time"
                                </li>
                                <li style="display: flex; align-items: center; gap: 10px; font-size: 0.9rem; color: #374151;">
                                    <span style="width: 6px; height: 6px; border-radius: 50%; background: #14b8a6; flex-shrink: 0;"></span>
                                    "Deploy and rollback models to edge controllers remotely"
                                </li>
                                <li style="display: flex; align-items: center; gap: 10px; font-size: 0.9rem; color: #374151;">
                                    <span style="width: 6px; height: 6px; border-radius: 50%; background: #14b8a6; flex-shrink: 0;"></span>
                                    "Push notifications for training completion and alerts"
                                </li>
                                <li style="display: flex; align-items: center; gap: 10px; font-size: 0.9rem; color: #374151;">
                                    <span style="width: 6px; height: 6px; border-radius: 50%; background: #14b8a6; flex-shrink: 0;"></span>
                                    "Chat with PrometheusForge agent from anywhere"
                                </li>
                            </ul>
                        </div>
                        <div style="display: flex; justify-content: center; align-items: center;">
                            <div style="position: relative;">
                                // Mobile app preview
                                <div style="width: 260px; height: 520px; border-radius: 36px; border: 3px solid #1a1a2e; background: #1a1a2e; padding: 12px; box-shadow: 0 25px 50px rgba(0,0,0,0.15);">
                                    // Screen
                                    <div style="width: 100%; height: 100%; border-radius: 24px; background: #FFFDF7; overflow: hidden; display: flex; flex-direction: column;">
                                        // Status bar
                                        <div style="padding: 8px 16px; display: flex; justify-content: space-between; align-items: center;">
                                            <span style="font-size: 0.7rem; font-weight: 600; color: #111827;">"9:41"</span>
                                            <div style="display: flex; gap: 4px; align-items: center;">
                                                <div style="width: 16px; height: 10px; border: 1.5px solid #111827; border-radius: 2px; position: relative;">
                                                    <div style="position: absolute; inset: 1.5px; background: #22c55e; border-radius: 1px;"></div>
                                                </div>
                                            </div>
                                        </div>
                                        // App header
                                        <div style="padding: 12px 16px; border-bottom: 1px solid #E8D4C4; display: flex; align-items: center; gap: 8px;">
                                            <img src="/assets/logo.png?v=3" alt="" style="width: 24px; height: 24px; border-radius: 6px;" />
                                            <span style="font-weight: 600; font-size: 0.85rem; color: #111827;">"Prometheus"</span>
                                        </div>
                                        // App content preview
                                        <div style="flex: 1; padding: 12px;">
                                            <div style="font-size: 0.7rem; font-weight: 600; color: #6b7280; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;">"Active Training"</div>
                                            <div style="padding: 10px; border-radius: 8px; border: 1px solid #E8D4C4; margin-bottom: 8px;">
                                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;">
                                                    <span style="font-size: 0.75rem; font-weight: 500; color: #111827;">"HVAC Predictor v3"</span>
                                                    <span style="font-size: 0.65rem; padding: 2px 6px; border-radius: 9999px; background: rgba(20,184,166,0.12); color: #14b8a6;">"Training"</span>
                                                </div>
                                                <div style="height: 4px; background: #F5EDE8; border-radius: 2px; overflow: hidden;">
                                                    <div style="width: 72%; height: 100%; background: linear-gradient(to right, #14b8a6, #0d9488); border-radius: 2px;"></div>
                                                </div>
                                                <div style="display: flex; justify-content: space-between; margin-top: 4px;">
                                                    <span style="font-size: 0.6rem; color: #6b7280;">"Epoch 36/50"</span>
                                                    <span style="font-size: 0.6rem; color: #6b7280;">"72%"</span>
                                                </div>
                                            </div>
                                            <div style="font-size: 0.7rem; font-weight: 600; color: #6b7280; text-transform: uppercase; letter-spacing: 0.05em; margin: 12px 0 8px;">"Deployments"</div>
                                            <div style="padding: 10px; border-radius: 8px; border: 1px solid #E8D4C4; margin-bottom: 6px;">
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="font-size: 0.75rem; font-weight: 500; color: #111827;">"Warren AHU-1"</span>
                                                    <span style="font-size: 0.65rem; padding: 2px 6px; border-radius: 9999px; background: rgba(34,197,94,0.12); color: #22c55e;">"Live"</span>
                                                </div>
                                                <span style="font-size: 0.6rem; color: #6b7280;">"v2.1.0 — 0.3ms latency"</span>
                                            </div>
                                            <div style="padding: 10px; border-radius: 8px; border: 1px solid #E8D4C4;">
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="font-size: 0.75rem; font-weight: 500; color: #111827;">"Huntington MUA-3"</span>
                                                    <span style="font-size: 0.65rem; padding: 2px 6px; border-radius: 9999px; background: rgba(34,197,94,0.12); color: #22c55e;">"Live"</span>
                                                </div>
                                                <span style="font-size: 0.6rem; color: #6b7280;">"v2.1.0 — 0.4ms latency"</span>
                                            </div>
                                        </div>
                                        // Bottom nav
                                        <div style="padding: 8px 0; border-top: 1px solid #E8D4C4; display: flex; justify-content: space-around;">
                                            <div style="display: flex; flex-direction: column; align-items: center; gap: 2px;">
                                                <div style="width: 20px; height: 20px; border-radius: 4px; background: rgba(20,184,166,0.15);"></div>
                                                <span style="font-size: 0.55rem; color: #14b8a6; font-weight: 500;">"Home"</span>
                                            </div>
                                            <div style="display: flex; flex-direction: column; align-items: center; gap: 2px;">
                                                <div style="width: 20px; height: 20px; border-radius: 4px; background: #F5EDE8;"></div>
                                                <span style="font-size: 0.55rem; color: #6b7280;">"Models"</span>
                                            </div>
                                            <div style="display: flex; flex-direction: column; align-items: center; gap: 2px;">
                                                <div style="width: 20px; height: 20px; border-radius: 4px; background: #F5EDE8;"></div>
                                                <span style="font-size: 0.55rem; color: #6b7280;">"Deploy"</span>
                                            </div>
                                            <div style="display: flex; flex-direction: column; align-items: center; gap: 2px;">
                                                <div style="width: 20px; height: 20px; border-radius: 4px; background: #F5EDE8;"></div>
                                                <span style="font-size: 0.55rem; color: #6b7280;">"Agent"</span>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                // Glow behind phone
                                <div style="position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); width: 300px; height: 300px; border-radius: 50%; background: radial-gradient(circle, rgba(20,184,166,0.1) 0%, transparent 70%); z-index: -1;"></div>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

            // Pricing Section
            <section id="pricing" style="padding: 80px 24px;">
                <div style="max-width: 1000px; margin: 0 auto; text-align: center;">
                    <h2 style="font-size: 2rem; font-weight: 700; color: #111827; margin-bottom: 12px;">"Simple, Transparent Pricing"</h2>
                    <p style="font-size: 1rem; color: #6b7280; margin-bottom: 48px;">"Start free. Scale when you're ready."</p>

                    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 20px; text-align: left;">
                        // Free Tier
                        <div style="padding: 28px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7;">
                            <div style="font-size: 0.75rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: #6b7280; margin-bottom: 8px;">"Free"</div>
                            <div style="font-size: 2.2rem; font-weight: 800; color: #111827; margin-bottom: 4px;">"$0"<span style="font-size: 0.9rem; font-weight: 400; color: #6b7280;">"/mo"</span></div>
                            <p style="font-size: 0.8rem; color: #6b7280; margin-bottom: 20px;">"Get started"</p>
                            <ul style="list-style: none; padding: 0; display: flex; flex-direction: column; gap: 10px; font-size: 0.8rem; color: #374151;">
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "5,000 AI tokens"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "3 datasets (25 MB)"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "2 models"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "1 deployment"
                                </li>
                            </ul>
                            <a href="/login" style="display: block; text-align: center; margin-top: 20px; padding: 10px; border-radius: 8px; border: 1px solid #E8D4C4; color: #111827; text-decoration: none; font-weight: 500; font-size: 0.8rem;">"Start Free"</a>
                        </div>

                        // Basic Tier
                        <div style="padding: 28px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7;">
                            <div style="font-size: 0.75rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: #3b82f6; margin-bottom: 8px;">"Basic"</div>
                            <div style="font-size: 2.2rem; font-weight: 800; color: #111827; margin-bottom: 4px;">"$12"<span style="font-size: 0.9rem; font-weight: 400; color: #6b7280;">"/mo"</span></div>
                            <p style="font-size: 0.8rem; color: #6b7280; margin-bottom: 20px;">"For individual makers"</p>
                            <ul style="list-style: none; padding: 0; display: flex; flex-direction: column; gap: 10px; font-size: 0.8rem; color: #374151;">
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "10,000 AI tokens"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "10 datasets (50 MB)"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "5 models"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "3 deployments"
                                </li>
                            </ul>
                            <a href="/login" style="display: block; text-align: center; margin-top: 20px; padding: 10px; border-radius: 8px; background: #3b82f6; color: white; text-decoration: none; font-weight: 500; font-size: 0.8rem;">"Get Basic"</a>
                        </div>

                        // Pro Tier
                        <div style="padding: 32px; border-radius: 16px; border: 2px solid #14b8a6; background: #FFFDF7; position: relative; box-shadow: 0 8px 30px rgba(20,184,166,0.12);">
                            <div style="position: absolute; top: -12px; left: 50%; transform: translateX(-50%); background: #14b8a6; color: white; padding: 4px 16px; border-radius: 9999px; font-size: 0.75rem; font-weight: 600;">"Most Popular"</div>
                            <div style="font-size: 0.75rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: #14b8a6; margin-bottom: 8px;">"Pro"</div>
                            <div style="font-size: 2.5rem; font-weight: 800; color: #111827; margin-bottom: 4px;">"$49"<span style="font-size: 1rem; font-weight: 400; color: #6b7280;">"/mo"</span></div>
                            <p style="font-size: 0.875rem; color: #6b7280; margin-bottom: 24px;">"For serious ML workflows"</p>
                            <ul style="list-style: none; padding: 0; display: flex; flex-direction: column; gap: 12px; font-size: 0.875rem; color: #374151;">
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "50,000 AI tokens"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "50 datasets"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "25 models"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "10 deployments"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "250 MB storage"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "ONNX + HEF export"
                                </li>
                            </ul>
                            <a href="/login" style="display: block; text-align: center; margin-top: 24px; padding: 12px; border-radius: 8px; background: #14b8a6; color: white; text-decoration: none; font-weight: 600; font-size: 0.875rem; transition: all 0.2s;">"Get Started"</a>
                        </div>

                        // Enterprise Tier
                        <div style="padding: 32px; border-radius: 16px; border: 1px solid #E8D4C4; background: #FFFDF7;">
                            <div style="font-size: 0.75rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: #C4A484; margin-bottom: 8px;">"Enterprise"</div>
                            <div style="font-size: 2.5rem; font-weight: 800; color: #111827; margin-bottom: 4px;">"$249"<span style="font-size: 1rem; font-weight: 400; color: #6b7280;">"/mo"</span></div>
                            <p style="font-size: 0.875rem; color: #6b7280; margin-bottom: 24px;">"Dedicated infrastructure"</p>
                            <ul style="list-style: none; padding: 0; display: flex; flex-direction: column; gap: 12px; font-size: 0.875rem; color: #374151;">
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "500,000 AI tokens"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "500 datasets, 200 models"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "100 deployments"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "Priority GPU training"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "SSO & custom integrations"
                                </li>
                                <li style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: #14b8a6; font-weight: 700;">{"\u{2713}"}</span>
                                    "Dedicated support"
                                </li>
                            </ul>
                            <a href="mailto:info@automatacontrols.com" style="display: block; text-align: center; margin-top: 24px; padding: 12px; border-radius: 8px; border: 1px solid #E8D4C4; color: #111827; text-decoration: none; font-weight: 500; font-size: 0.875rem; transition: all 0.2s;">"Contact Sales"</a>
                        </div>
                    </div>
                </div>
            </section>

            // CTA Section
            <section style="padding: 80px 24px; background: #FAF8F5; text-align: center;">
                <div style="max-width: 700px; margin: 0 auto;">
                    <h2 style="font-size: 2rem; font-weight: 700; color: #111827; margin-bottom: 16px;">"Ready to Deploy AI at the Edge?"</h2>
                    <p style="font-size: 1rem; color: #6b7280; margin-bottom: 32px; line-height: 1.7;">
                        "Join the next generation of building automation. Train custom models, export to ONNX and Hailo-8, and deploy to NexusEdge controllers — all from one platform."
                    </p>
                    <a href="/login" style="display: inline-flex; align-items: center; gap: 8px; padding: 14px 32px; border-radius: 10px; background: #14b8a6; color: white; text-decoration: none; font-weight: 600; font-size: 1rem; box-shadow: 0 4px 14px rgba(20,184,166,0.3);">
                        "Get Started for Free"
                        <span style="font-size: 1.2rem;">{"\u{2192}"}</span>
                    </a>
                </div>
            </section>

            // Footer
            <footer style="background: #f5ebe0; padding: 48px 24px 24px;">
                <div style="max-width: 1000px; margin: 0 auto;">
                    <div style="display: grid; grid-template-columns: 2fr 1fr 1fr 1fr; gap: 32px; margin-bottom: 32px;">
                        // Brand
                        <div>
                            <a href="/" style="display: flex; align-items: center; gap: 10px; text-decoration: none; margin-bottom: 12px;">
                                <img src="/assets/logo.png?v=3" alt="Prometheus" style="width: 44px; height: 44px; border-radius: 10px;" />
                                <div>
                                    <span style="font-weight: 700; font-size: 1.1rem; color: #111827;">"Prometheus"</span>
                                    <div style="height: 2px; background: linear-gradient(to right, #14b8a6, #C4A484); border-radius: 1px; margin-top: 2px;"></div>
                                </div>
                            </a>
                            <p style="font-size: 0.8rem; color: #6b7280; line-height: 1.6; max-width: 280px;">
                                "AI-powered model training and edge deployment for building automation systems."
                            </p>
                        </div>

                        // Product
                        <div>
                            <h4 style="font-size: 0.85rem; font-weight: 600; color: #C4A484; margin-bottom: 12px;">"Product"</h4>
                            <div style="display: flex; flex-direction: column; gap: 8px;">
                                <a href="#features" style="font-size: 0.85rem; color: #6b7280; text-decoration: none;">"Features"</a>
                                <a href="#pipeline" style="font-size: 0.85rem; color: #6b7280; text-decoration: none;">"Pipeline"</a>
                                <a href="#pricing" style="font-size: 0.85rem; color: #6b7280; text-decoration: none;">"Pricing"</a>
                                <a href="/login" style="font-size: 0.85rem; color: #6b7280; text-decoration: none;">"Sign In"</a>
                            </div>
                        </div>

                        // Company
                        <div>
                            <h4 style="font-size: 0.85rem; font-weight: 600; color: #C4A484; margin-bottom: 12px;">"Company"</h4>
                            <div style="display: flex; flex-direction: column; gap: 8px;">
                                <a href="https://automatanexus.com" target="_blank" rel="noopener" style="font-size: 0.85rem; color: #6b7280; text-decoration: none;">"AutomataNexus"</a>
                                <a href="https://automatacontrols.com" target="_blank" rel="noopener" style="font-size: 0.85rem; color: #6b7280; text-decoration: none;">"Automata Controls"</a>
                                <a href="https://github.com/automatacontrols" target="_blank" rel="noopener" style="font-size: 0.85rem; color: #6b7280; text-decoration: none;">"GitHub"</a>
                            </div>
                        </div>

                        // Contact
                        <div>
                            <h4 style="font-size: 0.85rem; font-weight: 600; color: #C4A484; margin-bottom: 12px;">"Contact"</h4>
                            <div style="display: flex; flex-direction: column; gap: 8px;">
                                <span style="font-size: 0.8rem; color: #6b7280;">"Wolcottville, IN"</span>
                                <a href="mailto:info@automatacontrols.com" style="font-size: 0.8rem; color: #6b7280; text-decoration: none;">"info@automatacontrols.com"</a>
                                <a href="tel:+12609932025" style="font-size: 0.8rem; color: #6b7280; text-decoration: none;">"(260) 993-2025"</a>
                            </div>
                        </div>
                    </div>

                    // Bottom bar
                    <div style="border-top: 1px solid rgba(196,164,132,0.3); padding-top: 16px; display: flex; align-items: center; justify-content: center; gap: 16px;">
                        <span style="font-size: 0.75rem; color: #6b7280;">{"\u{00A9} 2025 AutomataNexus, LLC"}</span>
                        <span style="color: #C4A484;">{"\u{00B7}"}</span>
                        <a href="https://automatanexus.com/privacy" target="_blank" rel="noopener" style="font-size: 0.75rem; color: #6b7280; text-decoration: none;">"Privacy"</a>
                        <span style="color: #C4A484;">{"\u{00B7}"}</span>
                        <a href="https://automatanexus.com/terms" target="_blank" rel="noopener" style="font-size: 0.75rem; color: #6b7280; text-decoration: none;">"Terms"</a>
                    </div>
                </div>
            </footer>
        </div>
    }
}
