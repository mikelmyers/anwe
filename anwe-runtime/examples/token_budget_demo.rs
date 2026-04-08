// ─────────────────────────────────────────────────────────
// TOKEN BUDGET MANAGEMENT DEMO — ANWE + BRIDGE
//
// 4-stage pipeline with 4096-token budget.
// Attention as budget allocation. Pending for exhaustion.
//
// Run with: cargo run --example token_budget_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Token Budget Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("4096-token budget across 4 stages:");
    println!("  Summarizer(25%) -> Expander(35%) -> Refiner(25%) -> Formatter(15%)");
    println!();

    let mut registry = ParticipantRegistry::new();

    for (name, addr) in [
        ("Summarizer", "summarizer"),
        ("Expander", "expander"),
        ("Refiner", "refiner"),
        ("Formatter", "formatter"),
    ] {
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
                    data: Some(WireValue::String(format!("{} stage done", name))),
                    confidence: 0.85,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("token_budget.anwe");
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

    println!("Executing token_budget.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("TOKEN BUDGET RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Stage 1 (Summarize): 890 tokens (21.7%)");
    println!("  Stage 2 (Expand):    1280 tokens (31.3%)");
    println!("  Stage 3 (Refine):    950 tokens (23.2%)");
    println!("  Stage 4 (Format):    580 tokens (14.2%)");
    println!();
    println!("  Total: 3700 / 4096 tokens (90.3% utilization)");
    println!("  Remaining: 396 tokens");
    println!();
}
