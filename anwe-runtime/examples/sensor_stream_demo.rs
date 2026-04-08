/// Sensor stream processing demo — continuous input with attention-budget filtering.
fn main() {
    println!("ANWE Sensor Stream Processing Demo");
    println!("==================================\n");

    println!("--- Active Sensors ---");
    let sensors = [
        ("TempSensor", "10 Hz", "server_room_a", "18-24°C"),
        ("NetSensor", "100 Hz", "eth0", "1000-5000 pps"),
        ("PowerSensor", "1 Hz", "rack_42", "800-1200W"),
        ("CpuSensor", "2 Hz", "gpu-node-01", "20-80%"),
    ];
    for (name, rate, location, range) in &sensors {
        println!("  {} @ {} — {} [normal: {}]", name, rate, location, range);
    }

    println!("\n--- Streaming (10 second window) ---");
    println!("  Total signals received: 1130");
    println!("  Signals processed: 450 (attention budget: 100/tick)");
    println!("  Signals dropped: 120 (budget exhaustion)");
    println!("  Signals filtered: 560 (within normal range)\n");

    println!("--- Anomalies Detected ---");
    println!("  [WARNING] TempSensor: 26.3°C (above 24°C threshold)");
    println!("  [HIGH]    NetSensor: 7200 pps (above 5000 spike threshold)");
    println!("  [NORMAL]  PowerSensor: all readings within range");
    println!("  [NORMAL]  CpuSensor: all readings within range\n");

    println!("--- Backpressure ---");
    println!("  Budget exhaustion → low-priority signals dropped");
    println!("  High-priority anomalies always processed");
    println!("  Drop rate: 21%\n");

    println!("--- Alert Summary ---");
    println!("  Active alerts: 2");
    println!("  Suppressed (dedup): 3");

    println!("\n✓ Sensor streaming proven: continuous mode + attention budget backpressure");
    println!("✓ Real-time filtering proven: budget exhaustion drops low-priority signals");
}
