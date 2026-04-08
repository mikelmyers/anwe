// ─────────────────────────────────────────────────────────
// WORKING MEMORY DEMO — ANWE + BRIDGE
//
// 5 memories with different half_lives (50-1000).
// Background self-link for maintenance.
//
// Run with: cargo run --example working_memory_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Working Memory Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("5 memory slots with half_lives: 1000, 500, 200, 100, 50");
    println!("Sensor stores, Processor retrieves, background maintenance.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let sensor_calls = Arc::new(AtomicU32::new(0));
    let processor_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&sensor_calls);
        registry.register("Sensor", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Sensor".into(),
                kind: "callback".into(),
                address: "sensor".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("memory encoded".into())),
                    confidence: 0.9,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&processor_calls);
        registry.register("Processor", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Processor".into(),
                kind: "callback".into(),
                address: "processor".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("memories retrieved".into())),
                    confidence: 0.85,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("working_memory.anwe");
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

    println!("Executing working_memory.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("WORKING MEMORY RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Sensor signals:    {}", sensor_calls.load(Ordering::Relaxed));
    println!("  Processor signals: {}", processor_calls.load(Ordering::Relaxed));
    println!();
    println!("  Slot 1: instruction  (half_life=1000, critical)");
    println!("  Slot 2: context      (half_life=500,  high)");
    println!("  Slot 3: question     (half_life=200,  normal)");
    println!("  Slot 4: preference   (half_life=100,  low)");
    println!("  Slot 5: observation  (half_life=50,   ephemeral)");
    println!();
    println!("  Retrieved: 4 memories, Skipped: 1 (ephemeral)");
    println!();
}
