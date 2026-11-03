use clap::Parser;
use openreview::cli::{Cli, Commands, E2eSubcommands};
use openreview::e2e::E2eSubsystem;
use openreview::e2e::actions::{ActionEngine, E2eAction, Locator};
use openreview::e2e::finder::E2eErrorFinder;
use openreview::e2e::normalizer::SemanticNormalizer;
use openreview::e2e::pruner::DomPruner;
use openreview::e2e::replay::{RecordedStep, ReplayAnchor, ReplayEngine, ReplayManifest};
use openreview::e2e::security::SecuritySanitizer;
use openreview::e2e::settlement::{SettlementConfig, SettlementOracle};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_cli_e2e_subcommands_parsing() {
    let args = Cli::try_parse_from([
        "openreview",
        "e2e",
        "find-errors",
        "--url",
        "https://staging.app.example.com",
        "--depth",
        "4",
    ])
    .expect("parse find-errors");

    assert!(matches!(
        args.command,
        Commands::E2e {
            subcommand: E2eSubcommands::FindErrors { depth: 4, .. }
        }
    ));

    let run_args = Cli::try_parse_from([
        "openreview",
        "e2e",
        "run",
        "--test-name",
        "cart_checkout",
        "--url",
        "http://localhost:3000",
    ])
    .expect("parse run");

    assert!(matches!(
        run_args.command,
        Commands::E2e {
            subcommand: E2eSubcommands::Run { .. }
        }
    ));

    let cache_args =
        Cli::try_parse_from(["openreview", "e2e", "cache", "clean"]).expect("parse cache");
    assert!(matches!(
        cache_args.command,
        Commands::E2e {
            subcommand: E2eSubcommands::Cache { .. }
        }
    ));
}

#[test]
fn test_semantic_normalizer_resolves_all_tester_army_cache_bugs() {
    let normalizer = SemanticNormalizer::new();

    // Issue #1496: Ordinal dates
    let date1 = "Created October 26th";
    let date2 = "Created October 27th";
    assert_eq!(
        normalizer.normalize_text(date1),
        normalizer.normalize_text(date2)
    );

    // Issue #1459: Relative times
    let time1 = "Saved 5 minutes ago by user";
    let time2 = "Saved 2 hours ago by user";
    let time3 = "Saved just now by user";
    assert_eq!(
        normalizer.normalize_text(time1),
        normalizer.normalize_text(time2)
    );
    assert_eq!(
        normalizer.normalize_text(time1),
        normalizer.normalize_text(time3)
    );

    // Issue #769: Minted record IDs
    let id1 = "Viewing invoice PROJ-016";
    let id2 = "Viewing invoice PROJ-999";
    assert_eq!(
        normalizer.normalize_text(id1),
        normalizer.normalize_text(id2)
    );

    // Issue #1057: Next.js route announcer
    let next_page =
        "Dashboard <next-route-announcer>Route changed to /dashboard</next-route-announcer>";
    assert_eq!(normalizer.normalize_text(next_page), "Dashboard");

    // Deterministic state hashes
    assert_eq!(
        normalizer.compute_semantic_hash(date1),
        normalizer.compute_semantic_hash(date2)
    );
}

#[test]
fn test_security_sanitizer_zero_trust_redaction() {
    let sanitizer = SecuritySanitizer::new();

    let fake_jwt = [
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.",
        "eyJzdWIiOiIxMjM0NTY3ODkwIn0.",
        "signature",
    ]
    .concat();
    let fake_key = ["sk-", "1234567890abcdef1234567890"].concat();
    let fake_gh = ["ghp_", "TEST11111111112222222222333333333344"].concat();
    let fake_card = ["4111", "2222", "3333", "4444"].join("-");

    let fake_pass = ["sample", "_", "trace", "_", "password"].concat();

    let raw_trace = format!(
        r#"
        Request headers:
        Authorization: Bearer {fake_jwt}
        Cookie: session={fake_key}
        GitHub-Token: {fake_gh}
        Billing Card: {fake_card}
        DOM: <input type="password" name="auth_pass" value="{fake_pass}" />
    "#
    );

    let sanitized = sanitizer.sanitize(&raw_trace);

    assert!(!sanitized.contains(&fake_pass));
    assert!(!sanitized.contains("1234567890abcdef"));
    assert!(!sanitized.contains("TEST1111111111"));
    assert!(!sanitized.contains(&fake_card));
    assert!(sanitized.contains("••••••••"));
    assert!(sanitized.contains("[REDACTED_API_KEY]"));
    assert!(sanitized.contains("[REDACTED_GITHUB_TOKEN]"));
    assert!(sanitized.contains("[REDACTED_CREDIT_CARD]"));
    assert!(!sanitizer.contains_unredacted_secrets(&sanitized));
}

#[test]
fn test_dom_pruner_compresses_and_bounds_tokens() {
    let pruner = DomPruner::new(50, 100);

    let html = r#"
        <html>
            <head>
                <style>body { background: white; }</style>
                <script>window.__PRELOADED_STATE__ = { huge: "data" };</script>
            </head>
            <body>
                <!-- Header navigation -->
                <nav>
                    <a href="/dashboard">Dashboard</a>
                    <button data-testid="settings-btn">Settings</button>
                </nav>
                <main>
                    <h1>Overview</h1>
                    <div style="display:none">Hidden secret debug info</div>
                    <form>
                        <input type="text" name="query" placeholder="Search resources" />
                        <button type="submit">Search</button>
                    </form>
                </main>
            </body>
        </html>
    "#;

    let tree = pruner.extract_semantic_tree(html);
    assert!(tree.len() >= 4);

    let rendered = pruner.render_pruned_tree(html);
    assert!(!rendered.contains("window.__PRELOADED_STATE__"));
    assert!(!rendered.contains("Hidden secret debug info"));
    assert!(rendered.contains("Settings"));
    assert!(rendered.contains("Search resources"));
}

#[tokio::test]
async fn test_settlement_oracle_stability() {
    let oracle = SettlementOracle::new(SettlementConfig {
        debounce_duration: Duration::from_millis(40),
        max_wait_deadline: Duration::from_millis(400),
        poll_interval: Duration::from_millis(15),
    });

    let counter = Arc::new(AtomicUsize::new(0));
    let c_clone = Arc::clone(&counter);

    let hash = oracle
        .wait_for_settlement(move || {
            let c = Arc::clone(&c_clone);
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    format!("mutating_{n}")
                } else {
                    "stable_state_hash".to_string()
                }
            }
        })
        .await;

    assert_eq!(hash, "stable_state_hash");
}

#[test]
fn test_action_engine_multi_layer_element_piercing() {
    let engine = ActionEngine::new();

    // Element Plus radio with hidden input behind styled label & span
    let element_plus_html = r#"<label class="el-radio is-checked"><span class="el-radio__inner"></span><input type="radio" value="monthly" style="display:none"></label>"#;
    let (target, pierced) = engine.resolve_pierced_target(element_plus_html);
    assert!(pierced);
    assert!(target.contains("input[type=\"radio\"]"));

    // URL assertion test
    assert!(
        engine
            .verify_url_assertion(
                "https://app.example.com/checkout/success",
                "checkout/success"
            )
            .is_ok()
    );
    assert!(
        engine
            .verify_url_assertion(
                "https://app.example.com/checkout/failed",
                "checkout/success"
            )
            .is_err()
    );
}

#[tokio::test]
async fn test_replay_engine_atomic_write_and_poll() {
    let dir = tempdir().unwrap();
    let engine = ReplayEngine::new(dir.path().to_path_buf());

    let manifest = ReplayManifest {
        test_name: "order_confirmation".to_string(),
        initial_route: "/order/123".to_string(),
        recorded_at: "2026-11-03T11:50:00Z".to_string(),
        steps: vec![RecordedStep {
            step_index: 0,
            action: E2eAction::AssertUrl {
                expected_pattern: "/order/123".to_string(),
            },
            pre_state_hash: "hash_start".to_string(),
            post_state_hash: "hash_confirmed".to_string(),
            appeared_anchors: vec![ReplayAnchor {
                role: "status".to_string(),
                name: Some("Confirmation".to_string()),
                normalized_text: "Order confirmed on [DATE]".to_string(),
                test_id: None,
            }],
            gone_anchors: vec![],
            duration_ms: 150,
        }],
    };

    engine
        .save_manifest_atomic(&manifest)
        .expect("atomic save succeeds");

    let loaded = engine
        .load_manifest("order_confirmation")
        .unwrap()
        .expect("manifest loaded");
    assert_eq!(loaded.test_name, "order_confirmation");
    assert_eq!(loaded.steps.len(), 1);

    // Monotonic deadline polling under transient delay
    let mut poll_count = 0;
    let result = engine
        .poll_until_anchor_matches(
            Some(Duration::from_millis(300)),
            &loaded.steps[0].appeared_anchors[0],
            || {
                poll_count += 1;
                async move {
                    if poll_count < 3 {
                        "Loading order status...".to_string()
                    } else {
                        "Order confirmed on October 26th".to_string()
                    }
                }
            },
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_causal_delta_debugging_ddmin_minimization() {
    let finder = E2eErrorFinder::new();

    let full_journey = vec![
        E2eAction::Navigate {
            url: "/store".to_string(),
        },
        E2eAction::Click {
            locator: Locator::Selector("#item-1".to_string()),
        },
        E2eAction::Click {
            locator: Locator::Selector("#item-2".to_string()),
        },
        E2eAction::Click {
            locator: Locator::Selector("#buggy-trigger".to_string()),
        },
        E2eAction::Click {
            locator: Locator::Selector("#item-3".to_string()),
        },
        E2eAction::Click {
            locator: Locator::Selector("#footer".to_string()),
        },
    ];

    let minimal = finder
        .minimize_failing_sequence(&full_journey, |steps| async move {
            steps.iter().any(|s| matches!(s, E2eAction::Click { locator: Locator::Selector(sel) } if sel == "#buggy-trigger"))
        })
        .await;

    assert_eq!(minimal.len(), 2); // [Navigate, Click(#buggy-trigger)]
    assert!(matches!(minimal[0], E2eAction::Navigate { .. }));
    assert!(
        matches!(&minimal[1], E2eAction::Click { locator: Locator::Selector(sel) } if sel == "#buggy-trigger")
    );

    let finding = finder.create_finding(
        "Cart Buggy Trigger Crash",
        "/store",
        "Uncaught TypeError: cannot read properties of undefined",
        minimal,
        1,
        Some("src/components/BuggyTrigger.tsx"),
        Some("if (!props.item) return null;"),
        r#"<div id="buggy-trigger">Crash</div>"#,
    );

    let md = finding.to_markdown();
    assert!(md.contains("🔴 BLOCKER Cart Buggy Trigger Crash"));
    assert!(md.contains("1-Minimal Reproduction Sequence (ddmin)"));
    assert!(md.contains("```suggestion\nif (!props.item) return null;\n```"));
}

#[test]
fn test_e2e_subsystem_facade_integrity() {
    let subsystem = E2eSubsystem::new();
    let dir = tempdir().unwrap();

    let _replay = subsystem.replay_engine(dir.path().to_path_buf());
    let _finder = subsystem.error_finder();

    let normalized = subsystem
        .normalizer
        .normalize_text("Ticket TASK-123 generated");
    assert_eq!(normalized, "Ticket [MINTED_ID] generated");
}
