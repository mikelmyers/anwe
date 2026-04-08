// ─────────────────────────────────────────────────────────
// ALERT ESCALATION DEMO — ANWE + BRIDGE
//
// background → low → normal → high → critical
// Each level increases priority and attention.
//
// Run with: cargo run --example alert_escalation_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Alert Escalation Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("background(healthy) → low(elevated) → normal(degraded)");
    println!("  → high(incident) → critical(outage)");
    println!();

    let mut registry = ParticipantRegistry::new();
    let system_calls = Arc::new(AtomicU32::new(0));
    let responder_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&system_calls);
        registry.register("System", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "System".into(),
                kind: "callback".into(),
                address: "system".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("system metrics".into())),
                    confidence: 0.8,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&responder_calls);
        registry.register("Responder", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Responder".into(),
                kind: "callback".into(),
                address: "responder".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.9,
                    data: Some(WireValue::String("responder acknowledged".into())),
                    confidence: 0.9,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("alert_escalation.anwe");
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

    println!("Executing alert_escalation.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("ESCALATION RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  System signals:    {}", system_calls.load(Ordering::Relaxed));
    println!("  Responder signals: {}", responder_calls.load(Ordering::Relaxed));
    println!();
    println!("  Level 0 (background): healthy — latency 48ms, errors 0.9%");
    println!("  Level 1 (low):        elevated — latency 95ms");
    println!("  Level 2 (normal):     degraded — latency 180ms, errors 8%");
    println!("  Level 3 (high):       incident — latency 350ms, errors 15%");
    println!("  Level 4 (critical):   OUTAGE — errors 95%");
    println!();
    println!("  Root cause: database connection pool exhausted");
    println!("  Time to detection: 2min, Time to escalation: 12min");
    println!();
}
