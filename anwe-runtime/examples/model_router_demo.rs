// ─────────────────────────────────────────────────────────
// MODEL ROUTER DEMO — ANWE + BRIDGE
//
// Classify → route to FastModel or PowerModel → collect.
// Link priorities model routing weights.
//
// Run with: cargo run --example model_router_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Model Router Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("Query → Classifier → FastModel | PowerModel → Response");
    println!("Routes complex queries to powerful models, simple to fast.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let classifier_calls = Arc::new(AtomicU32::new(0));
    let fast_calls = Arc::new(AtomicU32::new(0));
    let power_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&classifier_calls);
        registry.register("Classifier", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Classifier".into(),
                kind: "callback".into(),
                address: "classifier".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("classified: complex".into())),
                    confidence: 0.93,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&fast_calls);
        registry.register("FastModel", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "FastModel".into(),
                kind: "callback".into(),
                address: "fast_model".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("fast response".into())),
                    confidence: 0.65,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&power_calls);
        registry.register("PowerModel", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "PowerModel".into(),
                kind: "callback".into(),
                address: "power_model".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("detailed response".into())),
                    confidence: 0.94,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("model_router.anwe");
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

    println!("Executing model_router.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("MODEL ROUTER RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Classifier signals: {}", classifier_calls.load(Ordering::Relaxed));
    println!("  FastModel signals:  {}", fast_calls.load(Ordering::Relaxed));
    println!("  PowerModel signals: {}", power_calls.load(Ordering::Relaxed));
    println!();
    println!("  Query: 'Explain attention mechanism with math derivation'");
    println!("  Classification: complex (conf=0.93)");
    println!("  Routed to: PowerModel (llama-70b)");
    println!("  Tokens: 1024, Latency: 480ms");
    println!();
}
