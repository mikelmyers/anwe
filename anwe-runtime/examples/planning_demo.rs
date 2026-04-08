// ─────────────────────────────────────────────────────────
// PLANNING WITH UNCERTAINTY DEMO — ANWE + BRIDGE
//
// 4-step plan: train → evaluate → review → deploy.
// Confidence compounds across steps.
//
// Run with: cargo run --example planning_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Planning with Uncertainty Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("Goal: Deploy new ML model to production.");
    println!("4 steps, each with confidence. Uncertainty compounds.");
    println!();

    let mut registry = ParticipantRegistry::new();
    let planner_calls = Arc::new(AtomicU32::new(0));

    {
        let count = Arc::clone(&planner_calls);
        registry.register("Step1_Train", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Step1_Train".into(),
                kind: "callback".into(),
                address: "planner".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal {
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String("step planned".into())),
                    confidence: 0.9,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("planning.anwe");
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

    println!("Executing planning.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("PLANNING RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Planner signals: {}", planner_calls.load(Ordering::Relaxed));
    println!();
    println!("  Step 1 (Train):    conf=0.90, cumulative=0.90");
    println!("  Step 2 (Evaluate): conf=0.85, cumulative=0.765");
    println!("  Step 3 (Review):   conf=0.75, cumulative=0.574");
    println!("  Step 4 (Deploy):   conf=0.70, cumulative=0.402");
    println!();
    println!("  Overall plan confidence: 40.2%");
    println!("  Bottleneck: Step 3 (reviewer availability)");
    println!("  Recommendation: schedule reviewer early, have backup");
    println!();
}
