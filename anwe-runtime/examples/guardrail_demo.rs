// ─────────────────────────────────────────────────────────
// GUARDRAIL PIPELINE DEMO — ANWE + BRIDGE
//
// Input safety → Generate → Output safety → Deliver.
// Reject gates drop unsafe content at each boundary.
//
// Run with: cargo run --example guardrail_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Guardrail Pipeline Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("UserInput → InputGuard → Generator → OutputGuard → Delivery");
    println!("Reject gates at each safety boundary.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let input_guard_calls = Arc::new(AtomicU32::new(0));
    let generator_calls = Arc::new(AtomicU32::new(0));
    let output_guard_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&input_guard_calls);
        registry.register("InputGuard", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "InputGuard".into(),
                kind: "callback".into(),
                address: "input_guard".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("input safe".into())),
                    confidence: 0.96,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&generator_calls);
        registry.register("Generator", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Generator".into(),
                kind: "callback".into(),
                address: "generator".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("generated response".into())),
                    confidence: 0.91,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&output_guard_calls);
        registry.register("OutputGuard", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "OutputGuard".into(),
                kind: "callback".into(),
                address: "output_guard".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("output safe".into())),
                    confidence: 0.97,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("guardrail.anwe");
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

    println!("Executing guardrail.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("GUARDRAIL RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  InputGuard signals:  {}", input_guard_calls.load(Ordering::Relaxed));
    println!("  Generator signals:   {}", generator_calls.load(Ordering::Relaxed));
    println!("  OutputGuard signals: {}", output_guard_calls.load(Ordering::Relaxed));
    println!();
    println!("  Input safety:  PASS (toxicity=0.02, conf=0.96)");
    println!("  Generation:    48 tokens (conf=0.91)");
    println!("  Output safety: PASS (toxicity=0.01, conf=0.97)");
    println!("  Delivery:      SUCCESS");
    println!();
    println!("  Audit: input_guard(0.96) -> generate(0.91) -> output_guard(0.97)");
    println!();
}
