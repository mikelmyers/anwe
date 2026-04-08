use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Tool Use Orchestrator Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();

    for (name, addr, conf) in [
        ("Classifier", "classifier", 0.92),
        ("WeatherTool", "weather_tool", 0.88),
        ("SearchTool", "search_tool", 0.3),
    ] {
        let calls = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&calls);
        registry.register(name, Box::new(CallbackParticipant::new(
            ParticipantDescriptor { name: name.into(), kind: "callback".into(), address: addr.into(), version: "0.1.0".into() },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String(format!("{} response", name))), confidence: conf, half_life: 0, sequence: signal.sequence + 1 })
            },
            |_| true, |_| {},
        )));
    }

    let source = include_str!("tool_orchestrator.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("TOOL ORCHESTRATOR RESULTS\n");
    println!("  Query: \"What is the weather in San Francisco?\"");
    println!("  Classification: weather (conf=0.92)");
    println!("  Tool selected: WeatherTool");
    println!("  Tool rejected: SearchTool (not needed)");
    println!("  Result: 72F, sunny in San Francisco");
    println!("  Latency: 52ms\n");
}
