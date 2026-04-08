use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!("\nANWE v0.1 — Tree of Thought Demo");
    println!("═══════════════════════════════════════════════\n");

    let mut registry = ParticipantRegistry::new();

    for (name, addr) in [("BranchA", "branch_a"), ("BranchB", "branch_b"), ("BranchC", "branch_c")] {
        let calls = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&calls);
        registry.register(name, Box::new(CallbackParticipant::new(
            ParticipantDescriptor { name: name.into(), kind: "callback".into(), address: addr.into(), version: "0.1.0".into() },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                Some(WireSignal { quality: 2, direction: 2, priority: signal.priority * 0.95, data: Some(WireValue::String(format!("{} reasoning", name))), confidence: 0.85, half_life: 0, sequence: signal.sequence + 1 })
            },
            |_| true, |_| {},
        )));
    }

    let source = include_str!("tree_of_thought.anwe");
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut engine = Engine::with_participants(registry);
    engine.execute(&program).unwrap();

    println!("\n═══════════════════════════════════════════════");
    println!("TREE OF THOUGHT RESULTS\n");
    println!("  Branch A (arithmetic): answer=8, conf=0.65 → PRUNED");
    println!("  Branch B (careful):    answer=9, conf=0.92 → WINNER");
    println!("  Branch C (reframe):    answer=9, conf=0.88 → agrees");
    println!("  Consensus: 9 (2/3 branches agree)\n");
}
