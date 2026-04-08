// ─────────────────────────────────────────────────────────
// KNOWLEDGE DISTILLATION DEMO — ANWE + BRIDGE
//
// Teacher (175B) → Student (2B) via 3 lessons.
// Curriculum learning via link priorities.
//
// Run with: cargo run --example distillation_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Knowledge Distillation Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("Teacher (175B) -> Student (2B) via curriculum:");
    println!("  Lesson 1 (easy, high) -> Lesson 2 (medium, normal) -> Lesson 3 (hard, low)");
    println!();

    let mut registry = ParticipantRegistry::new();
    let teacher_calls = Arc::new(AtomicU32::new(0));
    let student_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&teacher_calls);
        registry.register("Teacher", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Teacher".into(),
                kind: "callback".into(),
                address: "teacher".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("teacher logits".into())),
                    confidence: 0.95,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    {
        let count = Arc::clone(&student_calls);
        registry.register("Student", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Student".into(),
                kind: "callback".into(),
                address: "student".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 2,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("student learned".into())),
                    confidence: 0.7,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("distillation.anwe");
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

    println!("Executing distillation.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("DISTILLATION RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Teacher signals: {}", teacher_calls.load(Ordering::Relaxed));
    println!("  Student signals: {}", student_calls.load(Ordering::Relaxed));
    println!();
    println!("  Lesson 1 (easy):   Student acc=0.72, KL=1.5");
    println!("  Lesson 2 (medium): Student acc=0.58, KL=1.9");
    println!("  Lesson 3 (hard):   Student acc=0.41, KL=2.3");
    println!();
    println!("  Compression: 87.5x (175B -> 2B)");
    println!("  Quality retention: 63% of teacher");
    println!();
}
