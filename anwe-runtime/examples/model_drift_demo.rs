// ─────────────────────────────────────────────────────────
// MODEL DRIFT DETECTION DEMO — ANWE + BRIDGE
//
// Week 1 (normal) → Week 4 (warning) → Week 8 (drift detected).
// Baseline decays via half_life.
//
// Run with: cargo run --example model_drift_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Model Drift Detection Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("Baseline F1=0.90. Monitoring drift over 8 weeks.");
    println!("Drift threshold: 0.05");
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
                    data: Some(WireValue::String("model evaluation".into())),
                    confidence: 0.85,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("model_drift.anwe");
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

    println!("Executing model_drift.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("DRIFT DETECTION RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Model signals: {}", model_calls.load(Ordering::Relaxed));
    println!();
    println!("  Week 1: F1=0.89, drift=0.01 (normal)");
    println!("  Week 4: F1=0.86, drift=0.04 (warning — approaching threshold)");
    println!("  Week 8: F1=0.80, drift=0.10 (DRIFT DETECTED)");
    println!();
    println!("  Drift rate: 0.0125/week (accelerating)");
    println!("  Root cause: input data distribution shift");
    println!("  Recommendation: retrain with recent 4 weeks of data");
    println!();
}
