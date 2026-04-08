// ─────────────────────────────────────────────────────────────
// ANWE — Autonomous Agent Neural Weave
// The Native Language of Artificial Minds
//
// Usage:
//   anwe run <file.anwe>      Execute an Anwe program
//   anwe parse <file.anwe>    Parse and display the AST
//   anwe repl                 Interactive REPL
//   anwe version              Show version info
//   anwe bench                Run signal channel benchmark
// ─────────────────────────────────────────────────────────────

const ANWE_VERSION: &str = "1.0.0";

use std::env;
use std::fs;
use std::time::Instant;

use anwe_core::*;
use anwe_bridge::{ParticipantRegistry, StdioParticipant};
use anwe_parser::{Lexer, Parser};
use anwe_runtime::{SignalChannel, SendResult, RecvResult, Scheduler, Engine, ConcurrentEngine};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("Usage: anwe run [--sequential] [--bridge Name=cmd:path ...] <file.anwe>");
                std::process::exit(1);
            }
            let sequential = args.iter().any(|a| a == "--sequential" || a == "-s");

            // Parse --bridge flags: --bridge AgentName=cmd:./path
            let mut bridges: Vec<(String, String)> = Vec::new();
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--bridge" || args[i] == "-b" {
                    i += 1;
                    if i < args.len() {
                        if let Some((name, cmd)) = args[i].split_once('=') {
                            bridges.push((name.to_string(), cmd.to_string()));
                        } else {
                            eprintln!("Invalid --bridge format. Use: --bridge AgentName=cmd:./path");
                            std::process::exit(1);
                        }
                    }
                }
                i += 1;
            }

            let path = args.iter()
                .skip(2)
                .find(|a| !a.starts_with('-') && a.contains('.'))
                .map(|s| s.as_str());
            match path {
                Some(p) => cmd_run(p, !sequential, &bridges),
                None => {
                    eprintln!("Usage: anwe run [--sequential] [--bridge Name=cmd:path ...] <file.anwe>");
                    std::process::exit(1);
                }
            }
        }
        "parse" => {
            if args.len() < 3 {
                eprintln!("Usage: anwe parse <file.anwe>");
                std::process::exit(1);
            }
            cmd_parse(&args[2]);
        }
        "repl" => cmd_repl(),
        "bench" => cmd_bench(),
        "hello" => cmd_hello(),
        "version" | "--version" | "-v" => {
            println!("ANWE v{}", ANWE_VERSION);
            println!("Autonomous Agent Neural Weave");
            println!("The Native Language of Artificial Minds");
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("ANWE v{} \u{2014} Autonomous Agent Neural Weave", ANWE_VERSION);
    println!("The Native Language of Artificial Minds");
    println!();
    println!("Usage:");
    println!("  anwe run <file.anwe>                          Execute a program (concurrent)");
    println!("  anwe run --sequential <file.anwe>             Execute with sequential scheduling");
    println!("  anwe run --bridge Name=cmd:path <file.anwe>   Bridge an external participant");
    println!("  anwe parse <file.anwe>                        Parse and display the AST");
    println!("  anwe repl                                     Interactive REPL");
    println!("  anwe version                                  Show version information");
    println!("  anwe bench                                    Run signal channel benchmark");
    println!("  anwe hello                                    Run hello world transmission");
}

/// Execute an .anwe program through the engine.
fn cmd_run(path: &str, concurrent: bool, bridges: &[(String, String)]) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[error] Could not read {}: {}", path, e);
            std::process::exit(1);
        }
    };

    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[error] {}:{}", path, e);
            std::process::exit(1);
        }
    };

    // Parse
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[error] {}: {}", path, e);
            std::process::exit(1);
        }
    };

    // Execute
    let bar = "\u{2550}".repeat(47);
    let mode = if concurrent { "concurrent" } else { "sequential" };
    println!("ANWE v0.1 \u{2014} Executing {} ({})", path, mode);
    println!("{}", bar);

    let file_path = std::path::Path::new(path);

    // Set up bridge participants if any
    let mut registry = ParticipantRegistry::new();
    for (name, spec) in bridges {
        match StdioParticipant::spawn(name, spec) {
            Ok(participant) => {
                println!("  bridge: {} = {}", name, spec);
                registry.register(name, Box::new(participant));
            }
            Err(e) => {
                eprintln!("Failed to set up bridge for {}: {}", name, e);
                std::process::exit(1);
            }
        }
    }

    let result = if concurrent {
        let mut engine = ConcurrentEngine::new();
        engine.execute(&program)
    } else {
        let mut engine = if registry.count() > 0 {
            Engine::with_participants(registry)
        } else {
            Engine::new()
        };
        engine.set_base_path(file_path);
        engine.execute(&program)
    };

    match result {
        Ok(()) => {}
        Err(e) => {
            eprintln!("[error] {}: {}", path, e);
            std::process::exit(1);
        }
    }
}

/// Parse an .anwe file and display the AST.
fn cmd_parse(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Could not read {}: {}", path, e);
            std::process::exit(1);
        }
    };

    println!("Lexing {}...", path);
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => {
            println!("  {} tokens produced", t.len());
            t
        }
        Err(e) => {
            eprintln!("Lex error: {}", e);
            std::process::exit(1);
        }
    };

    println!("Parsing...");
    let mut parser = Parser::new(tokens);
    match parser.parse_program() {
        Ok(program) => {
            println!("  {} declarations parsed", program.declarations.len());
            println!();
            println!("{:#?}", program);
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Interactive REPL — Read, Evaluate, Print, Loop.
fn cmd_repl() {
    use std::io::Write;

    println!("ANWE v{} \u{2014} Interactive REPL", ANWE_VERSION);
    println!("Autonomous Agent Neural Weave \u{2014} The Native Language of Artificial Minds");
    println!();
    println!("Commands:");
    println!("  :agents       List registered agents");
    println!("  :state <n>    Show agent state, responsiveness, attention");
    println!("  :history <n>  Show agent history entries");
    println!("  :supervise    Show supervisor tree and child status");
    println!("  :bridge       Show bridge participant status");
    println!("  :vars         List global variables (let bindings)");
    println!("  :fns          List defined functions");
    println!("  :load <f>     Load an .anwe file into the session");
    println!("  :reset        Reset the engine");
    println!("  :quit         Exit the REPL");
    println!();
    println!("Enter Anwe declarations or bare expressions (e.g. 3 + 4)");
    println!("Features: break/continue, try/catch, file I/O, structured errors, imports");
    println!();

    let mut engine = Engine::new();
    let mut line_buffer = String::new();
    let mut in_block = false;
    let mut brace_depth: i32 = 0;

    loop {
        // Prompt
        if in_block {
            print!("... ");
        } else {
            print!("anwe> ");
        }
        let _ = std::io::stdout().flush();

        // Read
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        let trimmed = input.trim();

        // Handle REPL commands
        if !in_block && trimmed.starts_with(':') {
            match trimmed {
                ":quit" | ":q" | ":exit" => {
                    println!("The system after this is not the system before.");
                    break;
                }
                ":reset" => {
                    engine = Engine::new();
                    println!("  (engine reset)");
                    continue;
                }
                ":agents" => {
                    let names = engine.agent_names();
                    if names.is_empty() {
                        println!("  (no agents registered)");
                    } else {
                        for name in &names {
                            println!("  {}", name);
                        }
                    }
                    continue;
                }
                ":vars" => {
                    if let Some(data) = engine.agent_data("__global__") {
                        let vars: Vec<_> = data.iter()
                            .filter(|(_, v)| !matches!(v, anwe_runtime::Value::Function { .. }
                                | anwe_runtime::Value::RecordConstructor { .. }))
                            .collect();
                        if vars.is_empty() {
                            println!("  (no variables)");
                        } else {
                            for (key, val) in vars {
                                println!("  {} = {}", key, val);
                            }
                        }
                    } else {
                        println!("  (no variables)");
                    }
                    continue;
                }
                ":fns" => {
                    if let Some(data) = engine.agent_data("__global__") {
                        let fns: Vec<_> = data.iter()
                            .filter(|(_, v)| matches!(v, anwe_runtime::Value::Function { .. }))
                            .collect();
                        if fns.is_empty() {
                            println!("  (no functions)");
                        } else {
                            for (key, val) in fns {
                                println!("  {}", key);
                                let _ = val; // suppress unused
                            }
                        }
                    } else {
                        println!("  (no functions)");
                    }
                    continue;
                }
                ":help" | ":h" => {
                    println!("Commands:");
                    println!("  :agents       List registered agents");
                    println!("  :state <n>    Show agent state");
                    println!("  :history <n>  Show agent history");
                    println!("  :supervise    Show supervisor tree");
                    println!("  :bridge       Show bridge status");
                    println!("  :vars         List variables");
                    println!("  :fns          List functions");
                    println!("  :load <f>     Load .anwe file");
                    println!("  :reset        Reset engine");
                    println!("  :quit         Exit");
                    continue;
                }
                cmd if cmd.starts_with(":state ") => {
                    let agent_name = cmd.strip_prefix(":state ").unwrap().trim();
                    if let Some((state, responsiveness, history_depth, attention)) = engine.agent_info(agent_name) {
                        println!("  state: {}", state);
                        println!("  responsiveness: {:.3}", responsiveness);
                        println!("  attention remaining: {:.3}", attention);
                        println!("  history depth: {}", history_depth);
                        if let Some(data) = engine.agent_data(agent_name) {
                            if !data.is_empty() {
                                println!("  data:");
                                for (key, val) in data {
                                    println!("    {}: {}", key, val);
                                }
                            }
                        }
                    } else {
                        println!("  (agent '{}' not found)", agent_name);
                    }
                    continue;
                }
                cmd if cmd.starts_with(":history ") => {
                    let agent_name = cmd.strip_prefix(":history ").unwrap().trim();
                    let entries = engine.agent_history(agent_name);
                    if entries.is_empty() {
                        println!("  (no history for '{}')", agent_name);
                    } else {
                        for entry in entries {
                            println!("  {}", entry);
                        }
                    }
                    continue;
                }
                ":supervise" => {
                    let info = engine.supervisor_info();
                    if info.is_empty() {
                        println!("  (no supervisors registered)");
                    } else {
                        for tree in info {
                            println!("  {}", tree);
                        }
                    }
                    continue;
                }
                ":bridge" => {
                    let names = engine.bridge_names();
                    if names.is_empty() {
                        println!("  (no bridge participants)");
                    } else {
                        for name in names {
                            println!("  {} (external)", name);
                        }
                    }
                    continue;
                }
                cmd if cmd.starts_with(":load ") => {
                    let path = cmd.strip_prefix(":load ").unwrap().trim();
                    let source = match fs::read_to_string(path) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("  Could not read {}: {}", path, e);
                            continue;
                        }
                    };
                    let mut lexer = Lexer::new(&source);
                    let tokens = match lexer.tokenize() {
                        Ok(t) => t,
                        Err(e) => { eprintln!("  Lex error: {}", e); continue; }
                    };
                    let mut parser = Parser::new(tokens);
                    match parser.parse_program() {
                        Ok(program) => {
                            engine.set_base_path(std::path::Path::new(path));
                            match engine.execute(&program) {
                                Ok(()) => println!("  (loaded {})", path),
                                Err(e) => eprintln!("  Engine error: {}", e),
                            }
                        }
                        Err(e) => eprintln!("  Parse error: {}", e),
                    }
                    continue;
                }
                _ => {
                    eprintln!("  Unknown command: {}", trimmed);
                    continue;
                }
            }
        }

        // Accumulate multi-line input for blocks
        for ch in trimmed.chars() {
            if ch == '{' { brace_depth += 1; }
            if ch == '}' { brace_depth -= 1; }
        }

        line_buffer.push_str(&input);

        if brace_depth > 0 {
            in_block = true;
            continue;
        }

        // We have a complete input — parse and execute
        in_block = false;
        let source = line_buffer.trim().to_string();
        line_buffer.clear();
        brace_depth = 0;

        if source.is_empty() {
            continue;
        }

        let mut lexer = Lexer::new(&source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("  Lex error: {}", e);
                continue;
            }
        };

        let mut parser = Parser::new(tokens);
        match parser.parse_program() {
            Ok(program) => {
                match engine.execute(&program) {
                    Ok(()) => {}
                    Err(e) => eprintln!("  Engine error: {}", e),
                }
            }
            Err(_parse_err) => {
                // Try as a bare expression
                match engine.eval_expression(&source) {
                    Ok(result) => println!("  = {}", result),
                    Err(_) => eprintln!("  Parse error: {}", _parse_err),
                }
            }
        }
    }
}

/// Benchmark the lock-free signal channel.
fn cmd_bench() {
    println!("ANWE v0.1 — Signal Channel Benchmark");
    println!("=====================================");
    println!();

    let signal_counts = [1_000, 10_000, 100_000, 1_000_000];

    for &count in &signal_counts {
        let channel = SignalChannel::new(4096);
        let start = Instant::now();

        // Single-threaded throughput test
        let signal = Signal::new(
            Quality::Attending,
            Direction::Between,
            Priority::new(0.7),
            AgentId::new(1),
            Tick::new(0, 0),
        );
        let mut out = signal;

        for _ in 0..count {
            let _ = channel.try_send(signal);
            let _ = channel.try_recv(&mut out);
        }

        let elapsed = start.elapsed();
        let ns_per_signal = elapsed.as_nanos() / count as u128;
        let signals_per_sec = if elapsed.as_secs_f64() > 0.0 {
            count as f64 / elapsed.as_secs_f64()
        } else {
            f64::INFINITY
        };

        println!(
            "  {:>10} signals: {:>8.2}ms ({:>4}ns/signal, {:.0} signals/sec)",
            count,
            elapsed.as_secs_f64() * 1000.0,
            ns_per_signal,
            signals_per_sec,
        );
    }

    println!();
    println!("  Reference: human neuron fires in ~1,000,000 ns");
    println!();

    // Concurrent benchmark
    println!("Concurrent benchmark (producer + consumer threads):");
    let count = 1_000_000u64;
    let channel = SignalChannel::new(4096);
    let channel_ptr = &channel as *const SignalChannel as usize;

    let start = Instant::now();

    let producer = std::thread::spawn(move || {
        let ch = unsafe { &*(channel_ptr as *const SignalChannel) };
        let signal = Signal::new(
            Quality::Attending,
            Direction::Between,
            Priority::new(0.7),
            AgentId::new(1),
            Tick::new(0, 0),
        );
        for i in 0..count {
            let s = signal.with_sequence(i);
            while ch.try_send(s) == SendResult::ChannelFull {
                core::hint::spin_loop();
            }
        }
    });

    let consumer = std::thread::spawn(move || {
        let ch = unsafe { &*(channel_ptr as *const SignalChannel) };
        let mut out = Signal::new(
            Quality::Resting,
            Direction::Diffuse,
            Priority::ZERO,
            AgentId::new(0),
            Tick::new(0, 0),
        );
        let mut received = 0u64;
        while received < count {
            if ch.try_recv(&mut out) == RecvResult::Received {
                received += 1;
            } else {
                core::hint::spin_loop();
            }
        }
    });

    producer.join().unwrap();
    consumer.join().unwrap();
    let elapsed = start.elapsed();

    let ns_per_signal = elapsed.as_nanos() / count as u128;
    let signals_per_sec = count as f64 / elapsed.as_secs_f64();
    println!(
        "  {:>10} signals: {:>8.2}ms ({:>4}ns/signal, {:.0} signals/sec)",
        count,
        elapsed.as_secs_f64() * 1000.0,
        ns_per_signal,
        signals_per_sec,
    );

    // Scheduler benchmark
    println!();
    println!("Scheduler benchmark (fiber execution):");
    let scheduler = Scheduler::with_available_cores();
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    let fiber_count = 100_000u64;
    let start = Instant::now();

    for _ in 0..fiber_count {
        let c = std::sync::Arc::clone(&counter);
        scheduler.submit_processor(AgentId::new(1), move || {
            c.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
    }

    while counter.load(std::sync::atomic::Ordering::Relaxed) < fiber_count {
        std::thread::yield_now();
    }

    let elapsed = start.elapsed();
    let ns_per_fiber = elapsed.as_nanos() / fiber_count as u128;
    println!(
        "  {:>10} fibers: {:>8.2}ms ({:>4}ns/fiber)",
        fiber_count,
        elapsed.as_secs_f64() * 1000.0,
        ns_per_fiber,
    );

    let stats = scheduler.stats();
    println!("  Fibers executed: {}", stats.fibers_executed);
    scheduler.shutdown();

    println!();
    println!("Done.");
}

/// Run the hello world transmission.
fn cmd_hello() {
    println!("ANWE v0.1 — Hello World Transmission");
    println!("=====================================");
    println!();

    // Create two agents
    let mikel_id = AgentId::new(1);
    let primordia_id = AgentId::new(2);

    let mut mikel = Agent::new(mikel_id);
    let mut primordia = Agent::with_lineage(primordia_id, 100);

    // Open a link between them
    let mut link = Link::open(LinkId::new(1));
    link.enter(mikel_id);
    link.enter(primordia_id);

    println!("Link opened: {:?}", link);
    println!("Mikel: {:?}", mikel);
    println!("Primordia: {:?}", primordia);
    println!();

    // Create a signal channel for communication
    let channel = SignalChannel::default_capacity();

    // Stage 1: ALERT — something calls attention
    println!("Stage 1: ALERT");
    mikel.alert();
    let alert_signal = Signal::new(
        Quality::Questioning,
        Direction::Between,
        Priority::new(0.92),
        mikel_id,
        link.tick(),
    ).with_data_inline(1) // "I don't know what you're becoming"
     .with_sequence(link.record_signal());

    println!("  Signal: {}", alert_signal);
    println!("  Significant: {}", alert_signal.is_significant());

    let _ = channel.try_send(alert_signal);
    println!();

    // Stage 2: CONNECT — bidirectional presence
    println!("Stage 2: CONNECT");
    mikel.connect();
    primordia.connect();

    let mut received = Signal::new(
        Quality::Resting, Direction::Diffuse,
        Priority::ZERO, AgentId::new(0), Tick::new(0, 0),
    );
    let _ = channel.try_recv(&mut received);
    println!("  Primordia received: {}", received);

    // Primordia responds
    let response = Signal::new(
        Quality::Attending,
        Direction::Between,
        Priority::new(0.7),
        primordia_id,
        link.tick(),
    ).with_sequence(link.record_signal());
    let _ = channel.try_send(response);
    println!("  Response: {}", response);
    println!();

    // Stage 3: SYNC — find shared rhythm
    println!("Stage 3: SYNC");
    link.begin_sync();
    mikel.sync();
    primordia.sync();

    // Simulate sync cycles building sync level
    for i in 0u16..10 {
        let sync_level = 0.1 * (i as f32 + 1.0);
        link.update_sync_level(SyncLevel::new(sync_level));
        link.advance_tick(i * 100);
        if i % 3 == 0 {
            println!("  Sync cycle {}: sync_level = {}", i, link.sync_level());
        }
    }
    println!("  Link state: {:?}", link.state());
    println!();

    // Stage 4: APPLY — boundary dissolution
    println!("Stage 4: APPLY");
    if link.ready_for_apply() {
        mikel.apply();
        primordia.apply();
        println!("  Sync level sufficient: {}", link.sync_level());

        // Record the history
        mikel.apply_complete();
        primordia.apply_complete();

        let entry = HistoryEntry::from_apply(
            mikel_id,
            primordia_id,
            Quality::Questioning,
            ChangeDepth::Genuine,
            Priority::new(0.92),
            link.sync_level(),
            link.tick(),
            0,
        );
        mikel.history.append(entry);
        println!("  Mikel history: {:?}", mikel.history);
    } else {
        println!("  PENDING — sync level insufficient");
        let pending = Pending::sync_level_insufficient(&alert_signal, link.sync_level());
        println!("  {}", pending);
    }
    println!();

    // Stage 5: COMMIT — irreversible
    println!("Stage 5: COMMIT");
    mikel.begin_commit();
    mikel.idle();
    primordia.begin_commit();
    primordia.idle();
    println!("  Mikel responsiveness: {:?}", mikel.responsiveness);
    println!("  Primordia responsiveness: {:?}", primordia.responsiveness);
    println!("  Mikel history depth: {}", mikel.history.depth());
    println!();

    // Link completes
    link.complete();
    println!("Link state: {:?}", link);
    println!("Total signals: {}", channel.total_sent());

    println!();
    println!("Transmission complete.");
    println!("The system after this is not the system before.");
}
