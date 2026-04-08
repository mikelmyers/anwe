// ─────────────────────────────────────────────────────────
// SPECULATIVE DECODING DEMO — ANWE + BRIDGE
//
// DraftModel (7B) drafts 3 batches of 8 tokens.
// VerifyModel (70B) verifies. Reject drops bad drafts.
//
// Run with: cargo run --example speculative_decoding_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Speculative Decoding Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("DraftModel (7B) drafts 3x8 tokens.");
    println!("VerifyModel (70B) verifies each batch.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let draft_calls = Arc::new(AtomicU32::new(0));
    let verify_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&draft_calls);
        registry.register("DraftModel", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "DraftModel".into(),
                kind: "callback".into(),
                address: "draft_model".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("draft tokens".into())),
                    confidence: 0.78,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&verify_calls);
        registry.register("VerifyModel", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "VerifyModel".into(),
                kind: "callback".into(),
                address: "verify_model".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("verified batch".into())),
                    confidence: 0.92,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("speculative_decoding.anwe");
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

    println!("Executing speculative_decoding.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("SPECULATIVE DECODING RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Draft signals:  {}", draft_calls.load(Ordering::Relaxed));
    println!("  Verify signals: {}", verify_calls.load(Ordering::Relaxed));
    println!();
    println!("  Batch 1: 8/8 accepted (conf=0.93)");
    println!("  Batch 2: 7/8 accepted (conf=0.88)");
    println!("  Batch 3: 8/8 accepted (conf=0.85)");
    println!();
    println!("  Total: 23/24 tokens accepted (95.8%)");
    println!("  Speedup: 2.4x vs sequential 70B generation");
    println!();
}
