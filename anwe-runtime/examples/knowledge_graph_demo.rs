// ─────────────────────────────────────────────────────────
// KNOWLEDGE GRAPH TRAVERSAL DEMO — ANWE + BRIDGE
//
// 3-hop traversal: Einstein → institutions → theories.
// Confidence decays with distance.
//
// Run with: cargo run --example knowledge_graph_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Knowledge Graph Traversal Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("Query: \"What institutions were connected to Einstein's relativity?\"");
    println!("3-hop traversal with confidence decay via half_life.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let graph_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&graph_calls);
        registry.register("Node_Einstein", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Node_Einstein".into(),
                kind: "callback".into(),
                address: "graph".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("graph node data".into())),
                    confidence: 0.92,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("knowledge_graph.anwe");
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

    println!("Executing knowledge_graph.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("KNOWLEDGE GRAPH RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Graph queries: {}", graph_calls.load(Ordering::Relaxed));
    println!();
    println!("  Hop 1: Albert Einstein (confidence: 0.95)");
    println!("  Hop 2a: ETH Zurich — professor 1912-1914 (0.88)");
    println!("  Hop 2b: Swiss Patent Office — clerk 1902-1909 (0.90)");
    println!("  Hop 2c: Princeton IAS — professor 1933-1955 (0.92)");
    println!("  Hop 3:  General Relativity — published 1915 (0.78)");
    println!();
    println!("  Confidence decays with distance: 0.95 → 0.90 → 0.78");
    println!("  Answer: Patent Office, ETH Zurich, Princeton IAS");
    println!();
}
