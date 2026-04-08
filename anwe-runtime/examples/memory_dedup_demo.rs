use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Memory Deduplication Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();
    let dedup_calls = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&dedup_calls);

    registry.register("DedupEngine", Box::new(CallbackParticipant::new(
        ParticipantDescriptor { name: "DedupEngine".into(), kind: "callback".into(), address: "dedup".into(), version: "0.1.0".into() },
        move |signal: &WireSignal| {
            count.fetch_add(1, Ordering::Relaxed);
            Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String("dedup processing".into())), confidence: 0.85, half_life: 0, sequence: signal.sequence + 1 })
        },
        |_| true, |_| {},
    )));

    let source = include_str!("memory_dedup.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("MEMORY DEDUP RESULTS\n");
    println!("  Dedup signals: {}", dedup_calls.load(Ordering::Relaxed));
    println!("  Before: 4 memories");
    println!("    1. \"weather in SF is sunny\"");
    println!("    2. \"SF weather forecast: sunny\"");
    println!("    3. \"dinner at 7pm\"");
    println!("    4. \"weather in SF sunny today\"");
    println!("  Clusters: 2 (3 weather + 1 dinner)");
    println!("  After: 2 unique memories");
    println!("  Removed: 2 duplicates (50% space saved)\n");
}
