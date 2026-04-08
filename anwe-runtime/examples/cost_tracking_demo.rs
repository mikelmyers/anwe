use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Cost Tracking Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();
    let cost_calls = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&cost_calls);

    registry.register("CostEngine", Box::new(CallbackParticipant::new(
        ParticipantDescriptor { name: "CostEngine".into(), kind: "callback".into(), address: "cost_engine".into(), version: "0.1.0".into() },
        move |signal: &WireSignal| {
            count.fetch_add(1, Ordering::Relaxed);
            Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String("cost computed".into())), confidence: 0.9, half_life: 0, sequence: signal.sequence + 1 })
        },
        |_| true, |_| {},
    )));

    let source = include_str!("cost_tracking.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("COST TRACKING RESULTS\n");
    println!("  Cost engine signals: {}", cost_calls.load(Ordering::Relaxed));
    println!("  Request: 150 input + 300 output tokens");
    println!("  Input cost:  $0.0045 (150 * $0.03/1k)");
    println!("  Output cost: $0.0180 (300 * $0.06/1k)");
    println!("  Total: $0.0225");
    println!("  Budget: $26.53 remaining of $100.00 daily");
    println!("  Utilization: 73.47%\n");
}
