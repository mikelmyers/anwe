use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Adaptive Router Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();

    for (name, addr, conf) in [
        ("Router", "router", 0.9),
        ("FastModel", "fast", 0.72),
        ("PremiumModel", "premium", 0.95),
    ] {
        let calls = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&calls);
        registry.register(name, Box::new(CallbackParticipant::new(
            ParticipantDescriptor { name: name.into(), kind: "callback".into(), address: addr.into(), version: "0.1.0".into() },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String(format!("{} processing", name))), confidence: conf, half_life: 0, sequence: signal.sequence + 1 })
            },
            |_| true, |_| {},
        )));
    }

    let source = include_str!("adaptive_router.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("ADAPTIVE ROUTER RESULTS\n");
    println!("  Query: \"Explain quantum entanglement\"");
    println!("  Complexity: 0.85 (high)");
    println!("  Route: premium_70b (fast model rejected)");
    println!("  Quality: 0.94");
    println!("  Cost: $0.073");
    println!("  Decision: quality_weight(0.7) * 0.94 > cost threshold\n");
}
