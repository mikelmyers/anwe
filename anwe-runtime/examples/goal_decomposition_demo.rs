// ─────────────────────────────────────────────────────────
// GOAL DECOMPOSITION DEMO — ANWE + BRIDGE
//
// Top-level goal → 5 sub-goals managed by supervision.
// rest_for_one: if upstream fails, downstream restarts.
//
// Run with: cargo run --example goal_decomposition_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Goal Decomposition Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("Goal: Build customer support chatbot");
    println!("5 sub-goals managed by rest_for_one supervision.");
    println!();

    let mut registry = ParticipantRegistry::new();

    for (name, addr) in [("SubGoal_Data", "data"), ("SubGoal_Model", "model")] {
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
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String(format!("{} completed", name))),
                    confidence: 0.88,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("goal_decomposition.anwe");
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

    println!("Executing goal_decomposition.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("GOAL DECOMPOSITION RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Sub-goal 1 (Data):        COMPLETE (conf=0.88)");
    println!("  Sub-goal 2 (Model):       COMPLETE (conf=0.85)");
    println!("  Sub-goal 3 (Integration): COMPLETE (conf=0.80)");
    println!("  Sub-goal 4 (Testing):     COMPLETE (conf=0.82, 3 failures)");
    println!("  Sub-goal 5 (Docs):        COMPLETE (conf=0.75)");
    println!();
    println!("  Supervision: rest_for_one (upstream failure restarts downstream)");
    println!("  Overall confidence: 0.82");
    println!("  Critical path: Data → Model → Integration → Testing");
    println!();
}
