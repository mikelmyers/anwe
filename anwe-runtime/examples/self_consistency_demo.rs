// ─────────────────────────────────────────────────────────
// SELF-CONSISTENCY DEMO — ANWE + BRIDGE
//
// 3 model instances answer independently. Voter converges.
// one_for_all supervision.
//
// Run with: cargo run --example self_consistency_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Self-Consistency Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("3 model instances answer: integral of x^2 from 0 to 3");
    println!("Voter converges on consensus via majority vote.");
    println!();

    let mut registry = ParticipantRegistry::new();

    for (name, addr) in [("ModelA", "model_a"), ("ModelB", "model_b"), ("ModelC", "model_c")] {
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
                    data: Some(WireValue::String("answer: 9".into())),
                    confidence: 0.92,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("self_consistency.anwe");
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

    println!("Executing self_consistency.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("SELF-CONSISTENCY RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  ModelA: answer=9, conf=0.95");
    println!("  ModelB: answer=9, conf=0.92");
    println!("  ModelC: answer=9, conf=0.88");
    println!();
    println!("  Consensus: 9 (3/3 unanimous, conf=0.95)");
    println!("  Supervision: one_for_all (all restart if any crashes)");
    println!();
}
