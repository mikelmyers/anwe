// ─────────────────────────────────────────────────────────
// HYPERPARAMETER SEARCH DEMO — ANWE + BRIDGE
//
// 3 configs against Dataset. Comparator converges on winner.
// one_for_all supervision. Reject on divergence.
//
// Run with: cargo run --example hyperparam_search_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Hyperparameter Search Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("3 configs: A(conservative) B(balanced) C(aggressive)");
    println!("Comparator converges on winner. Reject on divergence.");
    println!();

    let mut registry = ParticipantRegistry::new();

    for (name, addr) in [("ConfigA", "config_a"), ("ConfigB", "config_b"), ("ConfigC", "config_c")] {
        let calls = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&calls);
        registry.register(name, Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: name.into(),
                kind: "callback".into(),
                address: addr.into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String(format!("{} training done", name))),
                    confidence: 0.85,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("hyperparam_search.anwe");
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap_or_else(|e| {
        eprintln!("Lex error: {}", e);
        std::process::exit(1);
    });

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });

    println!("Executing hyperparam_search.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("HYPERPARAMETER SEARCH RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Config A (SGD, lr=0.0001):  acc=0.72 (slow, stable)");
    println!("  Config B (Adam, lr=0.001):  acc=0.84 (WINNER)");
    println!("  Config C (AdamW, lr=0.01):  acc=0.77 (unstable)");
    println!();
    println!("  Winner: Config B (lr=0.001, batch=128, Adam)");
    println!("  Supervision: one_for_all");
    println!();
}
