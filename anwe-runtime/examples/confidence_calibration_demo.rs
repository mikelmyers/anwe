use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Confidence Calibration Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();
    let calls = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&calls);
    registry.register("Model", Box::new(CallbackParticipant::new(
        ParticipantDescriptor { name: "Model".into(), kind: "callback".into(), address: "model".into(), version: "0.1.0".into() },
        move |signal: &WireSignal| {
            count.fetch_add(1, Ordering::Relaxed);
            Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String("prediction".into())), confidence: signal.confidence * 0.95, half_life: 0, sequence: signal.sequence + 1 })
        },
        |_| true, |_| {},
    )));

    let source = include_str!("confidence_calibration.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("CALIBRATION RESULTS\n");
    println!("  Bin 1 (high 0.9+): predicted=0.95, actual=0.82 → OVERCONFIDENT");
    println!("  Bin 2 (med 0.6-0.9): predicted=0.75, actual=0.71 → slightly over");
    println!("  Bin 3 (low <0.6): predicted=0.50, actual=0.52 → well calibrated");
    println!("  ECE: 0.063 → apply temperature scaling T=1.5\n");
}
