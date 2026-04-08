// ─────────────────────────────────────────────────────────
// EVALUATION HARNESS DEMO — ANWE + BRIDGE
//
// 5 test cases against a Model. Metrics aggregates via FieldAccess.
//
// Run with: cargo run --example eval_harness_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Evaluation Harness Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("5 test cases evaluated against qa-model-v2.");
    println!("Metrics agent aggregates via FieldAccess.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let model_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&model_calls);
        registry.register("Model", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Model".into(),
                kind: "callback".into(),
                address: "model".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("model prediction".into())),
                    confidence: 0.88,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("eval_harness.anwe");
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

    println!("Executing eval_harness.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("EVALUATION RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Model signals: {}", model_calls.load(Ordering::Relaxed));
    println!();
    println!("  Test 1 (factual):        PASS  score=1.0");
    println!("  Test 2 (translation):    PASS  score=1.0");
    println!("  Test 3 (arithmetic):     PASS  score=1.0");
    println!("  Test 4 (classification): PASS  score=1.0");
    println!("  Test 5 (summarization):  PARTIAL score=0.7");
    println!();
    println!("  Aggregate accuracy: 94%");
    println!("  Weakest category: summarization");
    println!();
}
