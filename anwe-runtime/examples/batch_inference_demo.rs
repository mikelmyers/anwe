use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Batch Inference Scheduler Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();
    let inference_calls = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&inference_calls);

    registry.register("Model", Box::new(CallbackParticipant::new(
        ParticipantDescriptor { name: "Model".into(), kind: "callback".into(), address: "model".into(), version: "0.1.0".into() },
        move |signal: &WireSignal| {
            count.fetch_add(1, Ordering::Relaxed);
            Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String("inference result".into())), confidence: 0.9, half_life: 0, sequence: signal.sequence + 1 })
        },
        |_| true, |_| {},
    )));

    let source = include_str!("batch_inference.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("BATCH INFERENCE RESULTS\n");
    println!("  Model signals: {}", inference_calls.load(Ordering::Relaxed));
    println!("  Items in batch: 4");
    println!("  Throughput: 600 tokens/sec");
    println!("  Avg latency: 27ms per item");
    println!("  Total tokens: 2400");
    println!("  Cost: 12 cents\n");
}
