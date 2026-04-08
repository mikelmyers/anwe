// ─────────────────────────────────────────────────────────
// PROMPT CHAIN DEMO — ANWE + BRIDGE
//
// 3-stage LLM pipeline: Summarizer → Extractor → Reporter.
// FieldAccess chains data between stages.
//
// Run with: cargo run --example prompt_chain_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Prompt Chain Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("Document → Summarizer → Extractor → Reporter");
    println!("FieldAccess chains data between LLM stages.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let summarizer_calls = Arc::new(AtomicU32::new(0));
    let extractor_calls = Arc::new(AtomicU32::new(0));
    let reporter_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&summarizer_calls);
        registry.register("Summarizer", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Summarizer".into(),
                kind: "callback".into(),
                address: "summarizer".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("summary generated".into())),
                    confidence: 0.92,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&extractor_calls);
        registry.register("Extractor", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Extractor".into(),
                kind: "callback".into(),
                address: "extractor".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("key points extracted".into())),
                    confidence: 0.90,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&reporter_calls);
        registry.register("Reporter", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Reporter".into(),
                kind: "callback".into(),
                address: "reporter".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("report written".into())),
                    confidence: 0.88,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("prompt_chain.anwe");
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

    println!("Executing prompt_chain.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("PROMPT CHAIN RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Summarizer signals: {}", summarizer_calls.load(Ordering::Relaxed));
    println!("  Extractor signals:  {}", extractor_calls.load(Ordering::Relaxed));
    println!("  Reporter signals:   {}", reporter_calls.load(Ordering::Relaxed));
    println!();
    println!("  Stage 1: Summarize (conf=0.92, 42 tokens)");
    println!("  Stage 2: Extract 5 key points (conf=0.90)");
    println!("  Stage 3: Executive report (conf=0.88, 48 tokens)");
    println!();
    println!("  Total pipeline tokens: ~90");
    println!("  Chain: Document -> Summarizer -> Extractor -> Reporter");
    println!();
}
