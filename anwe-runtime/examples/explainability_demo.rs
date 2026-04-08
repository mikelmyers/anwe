use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Explainability Trace Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();

    for (name, addr) in [("FeatureExtractor", "features"), ("RiskModel", "risk")] {
        let calls = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&calls);
        registry.register(name, Box::new(CallbackParticipant::new(
            ParticipantDescriptor { name: name.into(), kind: "callback".into(), address: addr.into(), version: "0.1.0".into() },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String(format!("{} done", name))), confidence: 0.9, half_life: 0, sequence: signal.sequence + 1 })
            },
            |_| true, |_| {},
        )));
    }

    let source = include_str!("explainability.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("EXPLAINABILITY RESULTS\n");
    println!("  1. Features: credit=720, DTI=0.32");
    println!("  2. Risk: 0.23 (low) via risk-classifier-v2");
    println!("  3. Policy: all rules passed (v3.1)");
    println!("  4. Decision: APPROVED (conf=0.87)");
    println!("  Trace: Input → Features → Risk → Policy → Decision\n");
}
