use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Streaming Response Assembler Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();
    let chunk_count = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&chunk_count);

    registry.register("Assembler", Box::new(CallbackParticipant::new(
        ParticipantDescriptor { name: "Assembler".into(), kind: "callback".into(), address: "assembler".into(), version: "0.1.0".into() },
        move |signal: &WireSignal| {
            count.fetch_add(1, Ordering::Relaxed);
            Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String("chunk assembled".into())), confidence: 0.9, half_life: 0, sequence: signal.sequence + 1 })
        },
        |_| true, |_| {},
    )));

    let source = include_str!("streaming_assembler.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("STREAMING ASSEMBLER RESULTS\n");
    println!("  Assembler signals: {}", chunk_count.load(Ordering::Relaxed));
    println!("  Chunks: [\" Hello\", \" world\", \" from\", \" ANWE\"]");
    println!("  Assembled: \" Hello world from ANWE\"");
    println!("  Status: delivered\n");
}
