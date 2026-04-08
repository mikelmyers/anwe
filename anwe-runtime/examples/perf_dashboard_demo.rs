use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Performance Dashboard Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();

    for (name, addr) in [("ModelA", "model_a_metrics"), ("ModelB", "model_b_metrics")] {
        let calls = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&calls);
        registry.register(name, Box::new(CallbackParticipant::new(
            ParticipantDescriptor { name: name.into(), kind: "callback".into(), address: addr.into(), version: "0.1.0".into() },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String(format!("{} metrics", name))), confidence: 0.9, half_life: 0, sequence: signal.sequence + 1 })
            },
            |_| true, |_| {},
        )));
    }

    let source = include_str!("perf_dashboard.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("PERFORMANCE DASHBOARD\n");
    println!("  ┌──────────┬───────────┬───────────┬──────────┐");
    println!("  │ Model    │ Latency   │ Throughput│ Errors   │");
    println!("  ├──────────┼───────────┼───────────┼──────────┤");
    println!("  │ Model A  │ 45ms p50  │ 150 rps   │ 2.0%     │");
    println!("  │ Model B  │ 80ms p50  │ 85 rps    │ 5.0%     │");
    println!("  ├──────────┼───────────┼───────────┼──────────┤");
    println!("  │ TOTAL    │ 62ms avg  │ 235 rps   │ 5.0% max │");
    println!("  └──────────┴───────────┴───────────┴──────────┘");
    println!("  Alert: WARNING (Model B degraded)\n");
}
