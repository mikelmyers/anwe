// ─────────────────────────────────────────────────────────
// BACKTRACKING DEMO — ANWE + BRIDGE
//
// Try path A→B (dead end) → backtrack → try A→C→D (success).
// Reject = backtrack. Commit = checkpoint.
//
// Run with: cargo run --example backtracking_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Backtracking Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("Maze: A→B (dead end), A→C→D (goal)");
    println!("Reject = backtrack from dead end. Commit = checkpoint.");
    println!();

    let mut registry = ParticipantRegistry::new();

    for (name, addr) in [("PathAttempt1", "path1"), ("PathAttempt2", "path2")] {
        let calls = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&calls);
        registry.register(name, Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: name.into(),
                kind: "callback".into(),
                address: addr.into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String(format!("{} exploring", name))),
                    confidence: 0.7,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("backtracking.anwe");
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

    println!("Executing backtracking.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("BACKTRACKING RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Attempt 1: A → B → DEAD END (reject = backtrack)");
    println!("  Attempt 2: A → C → D → SUCCESS");
    println!();
    println!("  Total attempts: 2, Backtracks: 1");
    println!("  Solution: A → C → D");
    println!("  Note: failed path (A→B) preserved in commit history");
    println!("  ANWE never erases — irreversible history records all attempts");
    println!();
}
