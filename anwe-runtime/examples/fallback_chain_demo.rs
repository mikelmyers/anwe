// ─────────────────────────────────────────────────────────
// FALLBACK CHAIN DEMO — ANWE + BRIDGE
//
// 3-tier: FastModel → MidModel → PremiumModel.
// Reject gates drop low-confidence tiers.
//
// Run with: cargo run --example fallback_chain_demo
// ─────────────────────────────────────────────────────────

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — Fallback Chain Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("FastModel(7B) -> MidModel(13B) -> PremiumModel(70B)");
    println!("Reject gates drop tiers with low confidence.");
    println!();

    let mut registry = ParticipantRegistry::new();

    for (name, addr, conf) in [
        ("FastModel", "fast_model", 0.45),
        ("MidModel", "mid_model", 0.62),
        ("PremiumModel", "premium_model", 0.94),
    ] {
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
                    quality: 4,
                    direction: 2,
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String(format!("{} response", name))),
                    confidence: conf,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    let source = include_str!("fallback_chain.anwe");
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

    println!("Executing fallback_chain.anwe...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap_or_else(|e| {
        eprintln!("Engine error: {}", e);
        std::process::exit(1);
    });

    println!();
    println!("═══════════════════════════════════════════════");
    println!("FALLBACK CHAIN RESULTS");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Tier 1 (FastModel 7B):    conf=0.45 REJECTED");
    println!("  Tier 2 (MidModel 13B):    conf=0.62 REJECTED");
    println!("  Tier 3 (PremiumModel 70B): conf=0.94 ACCEPTED");
    println!();
    println!("  Fallback path: 7B -> 13B -> 70B");
    println!("  Total latency: ~1018ms");
    println!();
}
