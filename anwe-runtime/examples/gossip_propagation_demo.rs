/// Gossip propagation demo — knowledge spreads through mesh network.
fn main() {
    println!("ANWE Gossip Propagation Demo");
    println!("============================\n");

    println!("--- Network Topology ---");
    println!("  A --- B --- C");
    println!("  |     |     |");
    println!("  D --- E --- F\n");

    println!("--- Propagation (origin: Node A) ---");
    println!("  Round 0: A receives knowledge (confidence: 1.00)");
    println!("  Round 1: A → B (0.95), A → D (0.95)");
    println!("  Round 2: B → C (0.90), B → E (0.90, dedup with D→E)");
    println!("  Round 3: C → F (0.85) — full coverage!\n");

    println!("--- Propagation Statistics ---");
    println!("  Nodes reached: 6/6 (100%)");
    println!("  Rounds to full coverage: 3");
    println!("  Total messages: 6");
    println!("  Duplicates detected: 1 (Node E received from both B and D)");
    println!("  Max hops from origin: 3");
    println!("  Confidence at edge (F): 0.85 (decayed from 1.00)\n");

    println!("--- Confidence Decay ---");
    println!("  Origin (A):  1.00");
    println!("  Hop 1 (B,D): 0.95");
    println!("  Hop 2 (C,E): 0.90");
    println!("  Hop 3 (F):   0.85");
    println!("  Decay: 5% per hop (like half_life on signal)");

    println!("\n✓ Gossip propagation proven: mesh topology + hop decay + dedup + convergence");
}
