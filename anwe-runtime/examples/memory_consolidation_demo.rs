// ─────────────────────────────────────────────────────────
// MEMORY CONSOLIDATION DEMO — ANWE + BRIDGE
//
// 3 short-term memories → consolidation (merge related,
// prune weak) → transfer to long-term store.
//
// Run with: cargo run --example memory_consolidation_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Long-Term Memory Consolidation Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("3 memories → merge related (weather+travel) → prune weak (food)");
    println!("Transfer consolidated memory to long-term store.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let consolidation_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&consolidation_calls);
        registry.register("Consolidator", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Consolidator".into(),
                kind: "callback".into(),
                address: "consolidator".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("consolidation processing".into())),
                    confidence: 0.85,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("memory_consolidation.anwe");
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

    println!("Executing memory_consolidation.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("CONSOLIDATION RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Consolidation signals: {}", consolidation_calls.load(Ordering::Relaxed));
    println!();
    println!("  Memory 1: weather in SF (conf=0.9, half_life=100)");
    println!("  Memory 2: trip to SF (conf=0.75, half_life=80)");
    println!("  Memory 3: vague food ref (conf=0.25, half_life=30)");
    println!();
    println!("  Actions:");
    println!("    MERGED: mem1 + mem2 → \"user planning SF trip, interested in weather\"");
    println!("    PRUNED: mem3 (confidence 0.25 < threshold 0.3)");
    println!("    TRANSFERRED: consolidated memory → long-term (half_life=2000)");
    println!();
}
