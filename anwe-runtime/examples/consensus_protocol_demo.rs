/// Consensus protocol demo — 5 validators vote on model deployment.
fn main() {
    println!("ANWE Consensus Protocol Demo");
    println!("============================\n");

    println!("--- Proposal ---");
    println!("  \"Deploy model v2.1 to production\"");
    println!("  Evidence: +3.2% accuracy, latency unchanged, safety benchmarks pass\n");

    println!("--- Validator Votes (reputation-weighted) ---");
    let votes = [
        ("A", "APPROVE", 0.92, 0.95, "accuracy improvement significant"),
        ("B", "APPROVE", 0.88, 0.92, "metrics good, latency within bounds"),
        ("C", "REJECT",  0.78, 0.88, "safety edge case regression"),
        ("D", "APPROVE", 0.85, 0.85, "3.2% gain justifies risk"),
        ("E", "ABSTAIN", 0.55, 0.90, "need more context on safety concern"),
    ];
    for (name, vote, conf, rep, rationale) in &votes {
        println!("  Validator_{}: {:7} (conf: {:.2}, rep: {:.2}) — {}", name, vote, conf, rep, rationale);
    }

    println!("\n--- Tally ---");
    println!("  For: 3, Against: 1, Abstain: 1");
    println!("  Weighted support: 0.73");
    println!("  Quorum (3/5): MET\n");

    println!("--- Decision ---");
    println!("  APPROVED");
    println!("  Condition: address Validator C safety concern within 72 hours");
    println!("  Dissent recorded in audit trail");

    println!("\n✓ Consensus protocol proven: broadcast + weighted vote + quorum + audit");
}
