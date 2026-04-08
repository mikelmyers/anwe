// -----------------------------------------------------------------
// ANWE v0.1 -- CONCURRENT EXECUTION ENGINE
//
// Multiple links execute simultaneously on the fiber scheduler.
// Each link runs as a processor fiber. Shared agent state is
// protected with Arc<Mutex<>> for safe concurrent access.
//
// This is where ANWE becomes a living substrate —
// multiple connections processing simultaneously,
// like a neural network with many synapses firing at once.
//
// v0.1 concurrent model:
//   - Links run in parallel (each link = one fiber)
//   - Within a link, primitives execute sequentially
//   - Shared agents are synchronized via mutex
//   - Output is buffered per-link, printed in order
//   - Real timing measurements per link and overall
// -----------------------------------------------------------------

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use anwe_core::{
    Agent, AgentId, AgentState, Signal, Quality, Direction, Priority,
    SyncLevel, Link, LinkId, LinkState, HistoryEntry, ChangeDepth,
    Supervisor, RestartStrategy, ChildSpec, ChildRestart, FailureReason,
};
use anwe_parser::ast::{
    Program, Declaration, LinkDecl, LinkExpr, LinkPriority, LinkSchedule,
    AlertExpr, ConnectBlock,
    SyncExpr, SyncCondition, ApplyExpr, CommitExpr, CommitSource,
    RejectExpr, ConvergeBlock, EmitExpr, WhenExpr,
    PendingHandlerExpr, PendingAction, PendingReason,
    PatternDecl, PatternUseExpr,
    SignalQuality, SignalDirection, ComparisonOp,
    Condition, Expr, BinOp,
    MatchPattern, BlockStatement, StringPart,
};
use anwe_bridge::{ParticipantRegistry, WireSignal, WireValue};
use crate::channel::SignalChannel;
use crate::scheduler::Scheduler;
use crate::engine::{
    Value, EngineError,
    to_core_quality, to_core_direction,
    quality_name, direction_name, depth_name, state_name,
    op_symbol, pending_reason_name, compare_f64, format_condition,
    substitute_link_expr, link_priority_name, apply_signal_attrs,
    value_to_wire, compare_values, value_to_display,
};
use crate::scheduler::{FiberPriority, FiberKind};

// ─── SHARED WORLD ─────────────────────────────────────────────
//
// The concurrent substrate. Agents live here, protected by
// mutexes so multiple links can access them simultaneously.
// IDs and patterns are immutable after registration — no lock.

struct SharedWorld {
    agents: HashMap<String, Arc<Mutex<Agent>>>,
    agent_ids: HashMap<String, AgentId>,
    agent_data: HashMap<String, Arc<Mutex<HashMap<String, Value>>>>,
    patterns: HashMap<String, PatternDecl>,
    /// Supervisor instances — used for runtime failure handling.
    supervisors: Arc<Mutex<Vec<Supervisor>>>,
    /// Bridge to external participants.
    participants: ParticipantRegistry,
}

impl SharedWorld {
    /// Handle a failure through the supervision system.
    /// Returns Ok(true) if supervised (continue), Ok(false) if not,
    /// Err if supervisor is overwhelmed.
    fn handle_supervised_failure(
        &self, agent_a: &str, agent_b: &str, error: &EngineError,
    ) -> Result<(bool, Vec<String>), EngineError> {
        let mut output = Vec::new();

        // Find supervisor for either agent
        let sup_idx = self.find_supervisor_for(agent_a)
            .or_else(|| self.find_supervisor_for(agent_b));

        let sup_idx = match sup_idx {
            Some(idx) => idx,
            None => return Ok((false, output)),
        };

        let failed_name = agent_a;
        let failed_id = match self.agent_ids.get(failed_name) {
            Some(&id) => id,
            None => return Ok((false, output)),
        };

        output.push(format!("  |"));
        output.push(format!("  |  SUPERVISOR: detected failure in {}", failed_name));
        output.push(format!("  |     error: {}", error));

        let now = anwe_core::Tick::new(0, 0);

        let mut sups = self.supervisors.lock().unwrap();
        let to_restart = match sups[sup_idx].handle_failure(
            failed_id, FailureReason::Crash, now,
        ) {
            Some(agents) => agents,
            None => {
                output.push("  |  SUPERVISOR: overwhelmed — too many restarts within window".to_string());
                output.push("  |     escalating failure".to_string());
                return Err(EngineError::ExecutionError(
                    format!("Supervisor overwhelmed: too many restarts. Original error: {}", error)
                ));
            }
        };

        let strategy = sups[sup_idx].strategy;
        drop(sups); // Release lock before restarting

        if to_restart.is_empty() {
            output.push("  |  SUPERVISOR: child is temporary — not restarting".to_string());
            return Ok((true, output));
        }

        let names_to_restart: Vec<String> = to_restart.iter()
            .filter_map(|id| {
                self.agent_ids.iter()
                    .find(|(_, aid)| **aid == *id)
                    .map(|(name, _)| name.clone())
            })
            .collect();

        let strategy_name = match strategy {
            RestartStrategy::OneForOne => "one_for_one",
            RestartStrategy::OneForAll => "one_for_all",
            RestartStrategy::RestForOne => "rest_for_one",
        };
        output.push(format!("  |  SUPERVISOR: strategy {} — restarting {} agent(s)",
            strategy_name, names_to_restart.len()));

        for name in &names_to_restart {
            if let Some(agent_mutex) = self.agents.get(name) {
                let mut agent = agent_mutex.lock().unwrap();
                let id = agent.id;
                let sup = agent.supervisor;
                *agent = Agent::new(id);
                agent.supervisor = sup;
                output.push(format!("  |  SUPERVISOR: restarted {}", name));
            }
        }

        output.push("  |".to_string());
        Ok((true, output))
    }

    fn find_supervisor_for(&self, agent_name: &str) -> Option<usize> {
        let _agent_id = self.agent_ids.get(agent_name)?;
        let agent_mutex = self.agents.get(agent_name)?;
        let agent = agent_mutex.lock().unwrap();
        let sup_id = agent.supervisor?;
        drop(agent);
        let sups = self.supervisors.lock().unwrap();
        sups.iter().position(|s| s.id == sup_id)
    }
}

// ─── LINK RESULT ──────────────────────────────────────────────
//
// What a link produces after concurrent execution.
// Output is buffered — no interleaved printing.

struct LinkResult {
    agent_a: String,
    agent_b: String,
    output: Vec<String>,
    elapsed: std::time::Duration,
    _total_signals: usize,
    _peak_sync: f32,
}

// ─── THE CONCURRENT ENGINE ───────────────────────────────────

pub struct ConcurrentEngine {
    next_id: u32,
    participants: ParticipantRegistry,
}

impl ConcurrentEngine {
    pub fn new() -> Self {
        ConcurrentEngine {
            next_id: 1,
            participants: ParticipantRegistry::new(),
        }
    }

    /// Create a new concurrent engine with external participants.
    pub fn with_participants(registry: ParticipantRegistry) -> Self {
        ConcurrentEngine {
            next_id: 1,
            participants: registry,
        }
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Execute a program with concurrent link execution.
    ///
    /// Phase 1: Register agents and patterns (sequential — fast setup).
    /// Phase 2: Execute all links concurrently on the fiber scheduler.
    /// Phase 3: Print buffered output in declaration order.
    pub fn execute(&mut self, program: &Program) -> Result<(), EngineError> {
        let total_start = Instant::now();
        println!();

        // ── Phase 1: Register agents and patterns ──────────────
        let mut agents: HashMap<String, Arc<Mutex<Agent>>> = HashMap::new();
        let mut agent_ids: HashMap<String, AgentId> = HashMap::new();
        let mut agent_data: HashMap<String, Arc<Mutex<HashMap<String, Value>>>> = HashMap::new();
        let mut patterns: HashMap<String, PatternDecl> = HashMap::new();
        let mut supervisors: Vec<Supervisor> = Vec::new();

        for decl in &program.declarations {
            match decl {
                Declaration::Agent(a) => {
                    let id_raw = self.alloc_id();
                    let id = AgentId::new(id_raw);

                    let mut lineage = 0u64;
                    let mut data_map = HashMap::new();

                    for kv in &a.data {
                        let val = eval_expr_static(&kv.value, &agents, &agent_data);
                        if kv.key == "lineage_depth" {
                            if let Value::Number(n) = &val {
                                lineage = *n as u64;
                            }
                        }
                        data_map.insert(kv.key.clone(), val);
                    }

                    let mut agent = if lineage > 0 {
                        Agent::with_lineage(id, lineage)
                    } else {
                        Agent::new(id)
                    };

                    // Apply attention budget if specified
                    if let Some(budget) = a.attention {
                        agent.attention = anwe_core::AttentionBudget::new(budget as f32);
                    }

                    let attn_str = a.attention.map(|v| format!(" attention {:.1}", v)).unwrap_or_default();
                    if a.data.is_empty() {
                        println!("  agent {}{}", a.name, attn_str);
                    } else {
                        let pairs: Vec<String> = a.data.iter()
                            .map(|kv| format!("{}: {}",
                                kv.key,
                                eval_expr_static(&kv.value, &agents, &agent_data)))
                            .collect();
                        println!("  agent {}{} data {{ {} }}", a.name, attn_str, pairs.join(", "));
                    }

                    agents.insert(a.name.clone(), Arc::new(Mutex::new(agent)));
                    agent_ids.insert(a.name.clone(), id);
                    agent_data.insert(a.name.clone(), Arc::new(Mutex::new(data_map)));
                }
                Declaration::Pattern(p) => {
                    let params_str: Vec<String> = p.params.iter().map(|param| {
                        if let Some(ref t) = param.type_ref {
                            format!("{}: {}", param.name, t)
                        } else {
                            param.name.clone()
                        }
                    }).collect();
                    println!("  pattern {}({})", p.name, params_str.join(", "));
                    patterns.insert(p.name.clone(), p.clone());
                }
                Declaration::Supervise(s) => {
                    let sup_id = AgentId::new(self.alloc_id());
                    let strategy = match s.strategy {
                        anwe_parser::ast::SuperviseStrategy::OneForOne => RestartStrategy::OneForOne,
                        anwe_parser::ast::SuperviseStrategy::OneForAll => RestartStrategy::OneForAll,
                        anwe_parser::ast::SuperviseStrategy::RestForOne => RestartStrategy::RestForOne,
                    };
                    let mut sup = Supervisor::new(sup_id, strategy);
                    if let (Some(max), Some(window)) = (s.max_restarts, s.time_window) {
                        sup = sup.with_limits(max, window);
                    }

                    let strategy_name = match s.strategy {
                        anwe_parser::ast::SuperviseStrategy::OneForOne => "one_for_one",
                        anwe_parser::ast::SuperviseStrategy::OneForAll => "one_for_all",
                        anwe_parser::ast::SuperviseStrategy::RestForOne => "rest_for_one",
                    };
                    print!("  supervise {} ", strategy_name);
                    if let (Some(max), Some(window)) = (s.max_restarts, s.time_window) {
                        print!("max_restarts {} within {} ", max, window);
                    }
                    println!("{{");

                    for child in &s.children {
                        let restart = match child.restart {
                            anwe_parser::ast::ChildRestartType::Permanent => ChildRestart::Permanent,
                            anwe_parser::ast::ChildRestartType::Transient => ChildRestart::Transient,
                            anwe_parser::ast::ChildRestartType::Temporary => ChildRestart::Temporary,
                        };
                        if let Some(&agent_id) = agent_ids.get(&child.agent) {
                            let spec = ChildSpec::new(agent_id, restart);
                            sup.add_child(spec);
                            agents.get(&child.agent).unwrap().lock().unwrap()
                                .supervisor = Some(sup_id);
                            let restart_name = match child.restart {
                                anwe_parser::ast::ChildRestartType::Permanent => "permanent",
                                anwe_parser::ast::ChildRestartType::Transient => "transient",
                                anwe_parser::ast::ChildRestartType::Temporary => "temporary",
                            };
                            println!("    {} {}", restart_name, child.agent);
                        }
                    }
                    println!("  }}");
                    supervisors.push(sup);
                }
                Declaration::Mind(m) => {
                    let id_raw = self.alloc_id();
                    let id = AgentId::new(id_raw);

                    let mut data_map = HashMap::new();
                    for kv in &m.data {
                        let val = eval_expr_static(&kv.value, &agents, &agent_data);
                        data_map.insert(kv.key.clone(), val);
                    }

                    let mut agent = Agent::new(id);
                    if let Some(budget) = m.attention {
                        agent.attention = anwe_core::AttentionBudget::new(budget as f32);
                    }

                    let attn_str = m.attention.map(|a| format!(" attention {:.1}", a)).unwrap_or_default();
                    if m.data.is_empty() {
                        println!("  mind {}{}", m.name, attn_str);
                    } else {
                        let pairs: Vec<String> = m.data.iter()
                            .map(|kv| format!("{}: {}", kv.key, eval_expr_static(&kv.value, &agents, &agent_data)))
                            .collect();
                        println!("  mind {}{} data {{ {} }}", m.name, attn_str, pairs.join(", "));
                    }

                    agents.insert(m.name.clone(), Arc::new(Mutex::new(agent)));
                    agent_ids.insert(m.name.clone(), id);
                    agent_data.insert(m.name.clone(), Arc::new(Mutex::new(data_map)));
                }
                Declaration::Let(binding) => {
                    let val = eval_expr_static(&binding.value, &agents, &agent_data);
                    println!("  let{} {} = {}", if binding.mutable { " mut" } else { "" }, binding.name, val);
                    let global = agent_data
                        .entry("__global__".to_string())
                        .or_insert_with(|| Arc::new(Mutex::new(HashMap::new())));
                    global.lock().unwrap().insert(binding.name.clone(), val);
                }
                Declaration::Fn(fn_decl) => {
                    let func = Value::Function {
                        params: fn_decl.params.clone(),
                        body: fn_decl.body.clone(),
                        env: HashMap::new(),
                    };
                    println!("  fn {}({})", fn_decl.name, fn_decl.params.join(", "));
                    let global = agent_data
                        .entry("__global__".to_string())
                        .or_insert_with(|| Arc::new(Mutex::new(HashMap::new())));
                    global.lock().unwrap().insert(fn_decl.name.clone(), func);
                }
                Declaration::Record(rec) => {
                    println!("  record {}({})", rec.name, rec.fields.join(", "));
                    let constructor = Value::RecordConstructor {
                        name: rec.name.clone(),
                        fields: rec.fields.clone(),
                    };
                    let global = agent_data
                        .entry("__global__".to_string())
                        .or_insert_with(|| Arc::new(Mutex::new(HashMap::new())));
                    global.lock().unwrap().insert(rec.name.clone(), constructor);
                }
                Declaration::TopLevelExpr(expr) => {
                    // Execute top-level expressions (while, for-in, etc.)
                    // using the static evaluator during initialization
                    eval_expr_static(expr, &agents, &agent_data);
                }
                Declaration::Assign { name, value } => {
                    // Top-level reassignment: evaluate and store in globals
                    let val = eval_expr_static(value, &agents, &agent_data);
                    let global = agent_data
                        .entry("__global__".to_string())
                        .or_insert_with(|| Arc::new(Mutex::new(HashMap::new())));
                    global.lock().unwrap().insert(name.clone(), val);
                }
                _ => {}
            }
        }
        println!();

        // Collect link declarations
        let link_decls: Vec<LinkDecl> = program.declarations.iter()
            .filter_map(|d| match d {
                Declaration::Link(l) => Some(l.clone()),
                _ => None,
            })
            .collect();

        // Collect mind declarations
        let mind_decls: Vec<anwe_parser::ast::MindDecl> = program.declarations.iter()
            .filter_map(|d| match d {
                Declaration::Mind(m) => Some(m.clone()),
                _ => None,
            })
            .collect();

        let num_links = link_decls.len();
        if num_links == 0 && mind_decls.is_empty() {
            let bar = "\u{2550}".repeat(47);
            println!("{}", bar);
            println!("Transmission complete.");
            println!("The system after this is not the system before.");
            println!();
            return Ok(());
        }

        // If only minds (no links), build world and execute minds directly
        if num_links == 0 {
            let world = Arc::new(SharedWorld {
                agents,
                agent_ids,
                agent_data,
                patterns,
                supervisors: Arc::new(Mutex::new(supervisors)),
                participants: self.participants.clone(),
            });
            for mind_decl in &mind_decls {
                self.execute_mind(&world, mind_decl)?;
            }
            let bar = "\u{2550}".repeat(47);
            println!("{}", bar);
            println!("Transmission complete.");
            println!("The system after this is not the system before.");
            println!();
            return Ok(());
        }

        // Build shared world
        let world = Arc::new(SharedWorld {
            agents,
            agent_ids,
            agent_data,
            patterns,
            supervisors: Arc::new(Mutex::new(supervisors)),
            participants: self.participants.clone(),
        });

        // ── Phase 2: Execute links concurrently ────────────────
        let results: Vec<Arc<Mutex<Option<LinkResult>>>> = (0..num_links)
            .map(|_| Arc::new(Mutex::new(None)))
            .collect();
        let errors: Arc<Mutex<Vec<(usize, EngineError)>>> =
            Arc::new(Mutex::new(Vec::new()));
        let completed = Arc::new(AtomicUsize::new(0));

        // Create scheduler — one worker per link, capped at available cores
        let num_workers = std::thread::available_parallelism()
            .map(|n| n.get().min(num_links).max(1))
            .unwrap_or(2);
        let scheduler = Scheduler::new(num_workers);

        if num_links > 1 {
            println!("  [concurrent mode: {} links on {} workers]", num_links, num_workers);
            println!();
        }

        // Submit each link as a fiber — priority from link declaration
        for (i, link_decl) in link_decls.into_iter().enumerate() {
            let world = Arc::clone(&world);
            let result_slot = Arc::clone(&results[i]);
            let errors = Arc::clone(&errors);
            let completed = Arc::clone(&completed);
            let link_id = LinkId::new(self.alloc_id());

            // Map link priority to fiber priority lane
            let fiber_priority = link_decl.priority
                .map(|p| to_fiber_priority(p))
                .unwrap_or(FiberPriority::Normal);

            scheduler.submit_with_priority(AgentId::new(0), FiberKind::Processor, fiber_priority, move || {
                let start = Instant::now();

                match execute_link_on_fiber(&world, &link_decl, link_id) {
                    Ok(mut lr) => {
                        lr.elapsed = start.elapsed();
                        *result_slot.lock().unwrap() = Some(lr);
                    }
                    Err(e) => {
                        errors.lock().unwrap().push((i, e));
                    }
                }

                completed.fetch_add(1, Ordering::Release);
            });
        }

        // Wait for all links to complete
        while completed.load(Ordering::Acquire) < num_links {
            std::thread::yield_now();
        }

        // Check for errors
        {
            let errs = errors.lock().unwrap();
            if let Some((_, e)) = errs.first() {
                let msg = format!("{}", e);
                drop(errs);
                scheduler.shutdown();
                return Err(EngineError::ExecutionError(msg));
            }
        }

        // ── Phase 3: Print results in declaration order ────────
        for (i, result_slot) in results.iter().enumerate() {
            let guard = result_slot.lock().unwrap();
            if let Some(ref lr) = *guard {
                if num_links > 1 {
                    println!("  [Link {}: {} <-> {}  elapsed: {:.3}ms]",
                        i + 1, lr.agent_a, lr.agent_b,
                        lr.elapsed.as_secs_f64() * 1000.0);
                }
                for line in &lr.output {
                    println!("{}", line);
                }
            }
        }

        // ── Execute minds (sequential — minds are self-links) ────
        for mind_decl in &mind_decls {
            self.execute_mind(&world, mind_decl)?;
        }

        // History views (after all links complete)
        for decl in &program.declarations {
            if let Declaration::HistoryView(hv) = decl {
                if let Some(agent_mutex) = world.agents.get(&hv.agent) {
                    let agent = agent_mutex.lock().unwrap();
                    println!("  history of {} (depth: {})", hv.agent, agent.history.depth());
                    for entry in agent.history.iter() {
                        println!("    {:?}", entry);
                    }
                }
            }
        }

        // ── Stats ──────────────────────────────────────────────
        let total_elapsed = total_start.elapsed();
        let stats = scheduler.stats();

        let bar = "\u{2550}".repeat(47);
        println!("{}", bar);
        if num_links > 1 {
            let total_link_time: f64 = results.iter()
                .filter_map(|r| r.lock().unwrap()
                    .as_ref()
                    .map(|lr| lr.elapsed.as_secs_f64()))
                .sum();
            let wall_time = total_elapsed.as_secs_f64();
            let speedup = if wall_time > 0.0 {
                total_link_time / wall_time
            } else {
                1.0
            };

            println!("Concurrent transmission complete.");
            println!("  {} links executed in parallel", num_links);
            println!("  {} fibers dispatched", stats.fibers_executed);
            println!("  wall time: {:.3}ms  (sequential would be: {:.3}ms)",
                wall_time * 1000.0, total_link_time * 1000.0);
            println!("  speedup: {:.2}x", speedup);
        } else {
            println!("Transmission complete.");
        }
        println!("The system after this is not the system before.");
        println!();

        scheduler.shutdown();
        Ok(())
    }

    /// Execute a mind declaration.
    ///
    /// Minds are self-links: the agent attends to itself.
    /// Attend blocks execute in priority order until attention is exhausted.
    fn execute_mind(&mut self, world: &Arc<SharedWorld>, mind_decl: &anwe_parser::ast::MindDecl) -> Result<(), EngineError> {
        if !world.agents.contains_key(&mind_decl.name) {
            return Err(EngineError::UnknownAgent(mind_decl.name.clone()));
        }

        let agent_id = world.agent_ids[&mind_decl.name];
        let link_id = LinkId::new(self.alloc_id());
        let mut link = Link::open(link_id);
        link.enter(agent_id);
        link.enter(agent_id);

        let channel = SignalChannel::default_capacity();

        let mut task = LinkTask {
            world: world.as_ref(),
            link,
            channel,
            agent_a: mind_decl.name.clone(),
            agent_b: mind_decl.name.clone(),
            agent_a_id: agent_id,
            agent_b_id: agent_id,
            last_alert_quality: None,
            output: Vec::with_capacity(32),
        };

        let bar = "\u{2500}".repeat(47);
        println!();
        println!("  {} is attending", mind_decl.name);
        println!("  {}", bar);

        // Build attention landscape
        let mut landscape: Vec<anwe_parser::ast::AttendBlock> = mind_decl.attend_blocks.clone();
        let mut executed = 0usize;
        let mut decayed = 0usize;

        loop {
            landscape.sort_by(|a, b| b.priority.partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal));

            if landscape.is_empty() {
                break;
            }

            let attend = landscape.remove(0);

            // Check attention budget
            let exhausted = world.agents[&mind_decl.name].lock().unwrap().is_budget_exhausted();
            if exhausted {
                println!("  |");
                println!("  |  ATTEND \"{}\" priority {:.3}", attend.label, attend.priority);
                println!("  |     (decayed \u{2014} attention exhausted)");
                decayed += 1;
                decayed += landscape.len();
                break;
            }

            println!("  |");
            println!("  |  ATTEND \"{}\" priority {:.3}", attend.label, attend.priority);

            world.agents[&mind_decl.name].lock().unwrap().consume_attention(0.1);

            // Execute attend block body
            for expr in &attend.body {
                task.execute_link_expr(expr)?;
            }

            // Print buffered output from LinkTask
            for line in task.output.drain(..) {
                println!("{}", line);
            }

            executed += 1;
        }

        // Complete
        task.link.complete();
        let total_signals = task.channel.total_sent();
        let total_blocks = executed + decayed;

        println!("  |");
        println!("  {}", bar);
        println!("  {} attended: {}/{}  decayed: {}  signals: {}",
            mind_decl.name, executed, total_blocks, decayed, total_signals);
        println!();

        Ok(())
    }
}

// ─── FIBER-LEVEL LINK EXECUTION ──────────────────────────────
//
// This function runs on a scheduler fiber (a worker thread).
// It executes a single link's body, collecting output to a buffer.
// All agent state access goes through the shared world's mutexes.

fn execute_link_on_fiber(
    world: &SharedWorld,
    decl: &LinkDecl,
    link_id: LinkId,
) -> Result<LinkResult, EngineError> {
    // Verify agents exist
    if !world.agents.contains_key(&decl.agent_a) {
        return Err(EngineError::UnknownAgent(decl.agent_a.clone()));
    }
    if !world.agents.contains_key(&decl.agent_b) {
        return Err(EngineError::UnknownAgent(decl.agent_b.clone()));
    }

    let mut link = Link::open(link_id);
    let agent_a_id = world.agent_ids[&decl.agent_a];
    let agent_b_id = world.agent_ids[&decl.agent_b];

    link.enter(agent_a_id);
    link.enter(agent_b_id);

    let channel = SignalChannel::default_capacity();

    let mut task = LinkTask {
        world,
        link,
        channel,
        agent_a: decl.agent_a.clone(),
        agent_b: decl.agent_b.clone(),
        agent_a_id,
        agent_b_id,
        last_alert_quality: None,
        output: Vec::with_capacity(32),
    };

    let pri_str = decl.priority.map(|p| format!("  priority: {}", link_priority_name(p))).unwrap_or_default();

    // Determine scheduling parameters
    let (iterations, delay) = match &decl.schedule {
        Some(LinkSchedule::Every { ticks }) => {
            let n = (*ticks as usize).max(1);
            task.out(format!("  link {} <-> {}{}  every {} ticks", decl.agent_a, decl.agent_b, pri_str, ticks));
            (n, 0)
        }
        Some(LinkSchedule::After { ticks }) => {
            task.out(format!("  link {} <-> {}{}  after {} ticks", decl.agent_a, decl.agent_b, pri_str, ticks));
            (1, *ticks as usize)
        }
        Some(LinkSchedule::Continuous) => {
            task.out(format!("  link {} <-> {}{}  continuous", decl.agent_a, decl.agent_b, pri_str));
            (3, 0)
        }
        None => {
            task.out(format!("  link {} <-> {}{}", decl.agent_a, decl.agent_b, pri_str));
            (1, 0)
        }
    };

    if delay > 0 {
        task.out(format!("  |  (delayed {} ticks)", delay));
    }

    for iteration in 0..iterations {
        if iterations > 1 {
            task.out(format!("  |  [tick {}]", iteration + 1));
        }
        task.out("  |".to_string());

        for expr in &decl.body {
            if let Err(e) = task.execute_link_expr(expr) {
                match world.handle_supervised_failure(&decl.agent_a, &decl.agent_b, &e) {
                    Ok((true, supervision_output)) => {
                        task.output.extend(supervision_output);
                        continue;
                    }
                    Ok((false, _)) => return Err(e),
                    Err(escalated) => return Err(escalated),
                }
            }
        }

        if iteration < iterations - 1 {
            task.link.advance_tick(100);
        }
    }

    task.link.complete();
    let total_signals = task.channel.total_sent();
    let peak = task.link.peak_sync_level();

    task.out("  |".to_string());
    task.out(format!("  +-- link complete  signals: {}  peak sync: {:.3}  iterations: {}",
        total_signals, peak.as_f32(), iterations));
    task.out(String::new());

    Ok(LinkResult {
        agent_a: decl.agent_a.clone(),
        agent_b: decl.agent_b.clone(),
        output: task.output,
        elapsed: std::time::Duration::ZERO, // filled in by caller
        _total_signals: total_signals,
        _peak_sync: peak.as_f32(),
    })
}

// ─── LINK TASK ───────────────────────────────────────────────
//
// Per-link execution context. Lives on a scheduler fiber.
// Mirrors the sequential engine's LinkCtx but:
//   - Locks agents from the shared world instead of &mut self
//   - Buffers output instead of printing directly
//   - Never holds two agent locks simultaneously (deadlock-free)

struct LinkTask<'w> {
    world: &'w SharedWorld,
    link: Link,
    channel: SignalChannel,
    agent_a: String,
    agent_b: String,
    agent_a_id: AgentId,
    agent_b_id: AgentId,
    last_alert_quality: Option<Quality>,
    output: Vec<String>,
}

impl<'w> LinkTask<'w> {
    fn out(&mut self, line: String) {
        self.output.push(line);
    }

    // ─── BRIDGE NOTIFICATIONS ──────────────────────────────

    fn bridge_notify_signal(&self, agent_name: &str, signal: &Signal) -> Option<Signal> {
        if let Some(participant) = self.world.participants.get(agent_name) {
            let wire = WireSignal::from_signal(signal);
            let mut p = participant.lock().unwrap();
            if let Some(response) = p.receive(&wire) {
                let agent_id = self.world.agent_ids[agent_name];
                return Some(response.to_signal(agent_id, signal.tick));
            }
        }
        None
    }

    fn bridge_notify_apply(&self, agent_name: &str, changes: &[(String, WireValue)]) -> bool {
        if let Some(participant) = self.world.participants.get(agent_name) {
            let mut p = participant.lock().unwrap();
            return p.apply(changes);
        }
        true
    }

    fn bridge_notify_commit(&self, agent_name: &str, entries: &[(String, WireValue)]) {
        if let Some(participant) = self.world.participants.get(agent_name) {
            let mut p = participant.lock().unwrap();
            p.commit(entries);
        }
    }

    fn execute_link_expr(&mut self, expr: &LinkExpr) -> Result<(), EngineError> {
        match expr {
            LinkExpr::Alert(a) => self.exec_alert(a),
            LinkExpr::Connect(c) => self.exec_connect(c),
            LinkExpr::Sync(s) => self.exec_sync(s),
            LinkExpr::Apply(a) => self.exec_apply(a),
            LinkExpr::Commit(c) => self.exec_commit(c),
            LinkExpr::Reject(r) => self.exec_reject(r),
            LinkExpr::Converge(c) => self.exec_converge(c),
            LinkExpr::Emit(e) => self.exec_emit(e),
            LinkExpr::When(w) => self.exec_when(w),
            LinkExpr::PendingHandler(p) => self.exec_pending(p),
            LinkExpr::PatternUse(p) => self.exec_pattern(p),
            LinkExpr::Each(e) => self.exec_each(e),
            LinkExpr::IfElse(ie) => self.exec_if_else(ie),
            // Phase 6-9 extensions
            LinkExpr::Spawn(s) => self.exec_spawn(s),
            LinkExpr::Retire(r) => self.exec_retire(r),
            LinkExpr::SyncAll(s) => self.exec_sync_all(s),
            LinkExpr::Broadcast(b) => self.exec_broadcast(b),
            LinkExpr::MultiConverge(mc) => self.exec_multi_converge(mc),
            LinkExpr::Save(s) => self.exec_save(s),
            LinkExpr::Restore(r) => self.exec_restore(r),
            LinkExpr::HistoryQueryBlock(hq) => self.exec_history_query(hq),
            LinkExpr::Stream(s) => {
                let rate = (s.rate as usize).max(1);
                self.out(format!("  |  STREAM {} rate {} (body: {} exprs)", s.source, rate, s.body.len()));
                for i in 0..rate {
                    if rate > 1 {
                        self.out(format!("  |     [sample {}/{}]", i + 1, rate));
                    }
                    for expr in &s.body {
                        self.execute_link_expr(expr)?;
                    }
                }
                self.out(format!("  |     {} samples processed", rate));
                self.out("  |".to_string());
                Ok(())
            }
            LinkExpr::Align(a) => {
                self.out(format!("  |  ALIGN [{}] to {}", a.agents.join(", "), self.eval_expr(&a.reference)));
                Ok(())
            }
            LinkExpr::Buffer(b) => {
                let samples = (b.samples as usize).max(1);
                self.out(format!("  |  BUFFER {} samples (body: {} exprs)", samples, b.body.len()));
                for i in 0..samples {
                    if samples > 1 {
                        self.out(format!("  |     [buffering {}/{}]", i + 1, samples));
                    }
                    for expr in &b.body {
                        self.execute_link_expr(expr)?;
                    }
                }
                self.out(format!("  |     {} samples buffered", samples));
                self.out("  |".to_string());
                Ok(())
            }
            // First-person cognition
            LinkExpr::Think(t) => self.exec_think(t),
            LinkExpr::Express(e) => self.exec_express(e),
            LinkExpr::Sense(s) => self.exec_sense(s),
            LinkExpr::Author(a) => self.exec_author(a),
            LinkExpr::While(w) => self.exec_while(w),
            LinkExpr::Attempt(a) => self.exec_attempt(a),
            LinkExpr::Let(binding) => self.exec_let(binding),
            LinkExpr::Assign(assign) => self.exec_assign(assign),
        }
    }

    // ─── ALERT ────────────────────────────────────────────────
    //
    // >> — Something calls attention.
    // Lock agent_a briefly to transition state.

    fn exec_alert(&mut self, alert: &AlertExpr) -> Result<(), EngineError> {
        let quality = alert.attrs.as_ref()
            .and_then(|a| a.quality)
            .unwrap_or(SignalQuality::Attending);
        let priority = alert.attrs.as_ref()
            .and_then(|a| a.priority)
            .unwrap_or(0.5);

        let core_quality = to_core_quality(quality);
        let core_priority = Priority::new(priority as f32);

        let mut signal = Signal::new(
            core_quality,
            Direction::Between,
            core_priority,
            self.agent_a_id,
            self.link.tick(),
        ).with_sequence(self.link.record_signal());

        signal = apply_signal_attrs(signal, alert.attrs.as_ref());

        let _ = self.channel.try_send(signal);

        // Bridge: notify external participants
        if let Some(response) = self.bridge_notify_signal(&self.agent_a, &signal) {
            let _ = self.channel.try_send(response);
        }
        if self.agent_a != self.agent_b {
            if let Some(response) = self.bridge_notify_signal(&self.agent_b, &signal) {
                let _ = self.channel.try_send(response);
            }
        }

        // Consume attention
        self.world.agents[&self.agent_a].lock().unwrap().consume_attention(0.02);
        self.last_alert_quality = Some(core_quality);

        // Lock agent_a, transition state, release immediately
        let (old_state, new_state) = {
            let mut agent = self.world.agents[&self.agent_a].lock().unwrap();
            let old = agent.state;
            agent.alert();
            (old, agent.state)
        };

        let val = self.eval_expr(&alert.expression);

        self.out("  |  >> ALERT".to_string());
        if let Some(ref attrs) = alert.attrs {
            let mut parts = Vec::new();
            if attrs.quality.is_some() {
                parts.push(format!("quality: {}", quality_name(quality)));
            }
            if let Some(p) = attrs.priority {
                parts.push(format!("priority: {:.3}", p));
            }
            if let Some(d) = attrs.direction {
                parts.push(format!("direction: {}", direction_name(d)));
            }
            if !parts.is_empty() {
                self.out(format!("  |     {}", parts.join("  ")));
            }
        }

        self.out(format!("  |     {}", val));
        self.out(format!("  |     {}: {} -> {}",
            self.agent_a, state_name(old_state), state_name(new_state)));
        self.out("  |".to_string());

        Ok(())
    }

    // ─── CONNECT ──────────────────────────────────────────────
    //
    // Sustained bidirectional presence.
    // Lock agents sequentially — never two at once.

    fn exec_connect(&mut self, connect: &ConnectBlock) -> Result<(), EngineError> {
        let depth_str = connect.depth
            .map(|d| depth_name(d))
            .unwrap_or("default");

        self.out(format!("  |  CONNECT depth: {}", depth_str));

        for pulse in &connect.pulses {
            let core_quality = to_core_quality(pulse.quality);
            let core_direction = to_core_direction(pulse.direction);
            let core_priority = Priority::new(pulse.priority as f32);

            let signal = Signal::new(
                core_quality,
                core_direction,
                core_priority,
                self.agent_a_id,
                self.link.tick(),
            ).with_sequence(self.link.record_signal());

            let _ = self.channel.try_send(signal);

            // Bridge: notify external participants
            if let Some(response) = self.bridge_notify_signal(&self.agent_a, &signal) {
                let _ = self.channel.try_send(response);
            }
            if self.agent_a != self.agent_b {
                if let Some(response) = self.bridge_notify_signal(&self.agent_b, &signal) {
                    let _ = self.channel.try_send(response);
                }
            }

            // Consume attention per signal pulse
            self.world.agents[&self.agent_a].lock().unwrap().consume_attention(0.01);

            let mut line = format!("  |     signal {:12} {:.3} {}",
                quality_name(pulse.quality),
                pulse.priority,
                direction_name(pulse.direction));

            if let Some(ref data) = pulse.data {
                let val = self.eval_expr(data);
                line.push_str(&format!("  data: {}", val));
            }

            if !pulse.trace.is_empty() {
                let pairs: Vec<String> = pulse.trace.iter()
                    .map(|kv| format!("{}: {}", kv.key, self.eval_expr(&kv.value)))
                    .collect();
                line.push_str(&format!("  trace: {{ {} }}", pairs.join(", ")));
            }

            self.out(line);
        }

        // Lock agents sequentially — drop first before second
        let (old_a, new_a) = {
            let mut a = self.world.agents[&self.agent_a].lock().unwrap();
            let old = a.state;
            a.connect();
            (old, a.state)
        };

        let (old_b, new_b) = if self.agent_a != self.agent_b {
            let mut b = self.world.agents[&self.agent_b].lock().unwrap();
            let old = b.state;
            b.connect();
            (old, b.state)
        } else {
            (old_a, new_a)
        };

        self.out(format!("  |     {} signals transmitted", connect.pulses.len()));
        self.out(format!("  |     {}: {} -> {}",
            self.agent_a, state_name(old_a), state_name(new_a)));
        if self.agent_a != self.agent_b {
            self.out(format!("  |     {}: {} -> {}",
                self.agent_b, state_name(old_b), state_name(new_b)));
        }
        self.out("  |".to_string());

        Ok(())
    }

    // ─── SYNC ─────────────────────────────────────────────────
    //
    // A ~ B until <condition>
    // Find shared rhythm. Sync level builds through ticks.

    fn exec_sync(&mut self, sync: &SyncExpr) -> Result<(), EngineError> {
        match &sync.until {
            SyncCondition::CoherenceThreshold { op, value } => {
                self.out(format!("  |  SYNC {} ~ {} until sync_level {} {:.3}",
                    sync.agent_a, sync.agent_b, op_symbol(*op), value));
            }
            _ => {
                let target_name = match &sync.until {
                    SyncCondition::Synchronized => "synchronized",
                    SyncCondition::Resonating => "resonating",
                    _ => unreachable!(),
                };
                self.out(format!("  |  SYNC {} ~ {} until {}",
                    sync.agent_a, sync.agent_b, target_name));
            }
        }

        self.link.begin_sync();

        // Lock agents sequentially for state transition
        let old_a = {
            let mut a = self.world.agents[&sync.agent_a].lock().unwrap();
            let old = a.state;
            a.sync();
            old
        };

        let old_b = if sync.agent_a != sync.agent_b {
            let mut b = self.world.agents[&sync.agent_b].lock().unwrap();
            let old = b.state;
            b.sync();
            old
        } else {
            old_a
        };

        // Determine target sync level — overshoots thresholds naturally
        let target_level: f32 = match &sync.until {
            SyncCondition::Synchronized => 0.75,
            SyncCondition::Resonating => 0.95,
            SyncCondition::CoherenceThreshold { op, value } => {
                let v = *value as f32;
                match op {
                    ComparisonOp::Greater => (v + 0.05).min(1.0),
                    ComparisonOp::GreaterEq => v,
                    _ => v,
                }
            }
        };

        // Simulate sync building — each tick brings agents closer
        let steps = ((target_level / 0.1).ceil() as u16).max(1);
        let decay_rate = sync.decay.unwrap_or(0) as f32;
        let mut levels = Vec::new();
        for i in 1..=steps {
            let mut level = (0.1 * i as f32).min(target_level);
            // Apply temporal decay if specified
            if decay_rate > 0.0 && i > 1 {
                let decay_factor = (-0.693 * (i as f32) / decay_rate).exp();
                level = level * (0.5 + 0.5 * decay_factor);
            }
            self.link.update_sync_level(SyncLevel::new(level));
            self.link.advance_tick(i * 100);
            levels.push(format!("{:.3}", level));
            if level >= target_level {
                break;
            }
        }

        // Consume attention for sync effort
        {
            self.world.agents[&sync.agent_a].lock().unwrap().consume_attention(0.05);
        }
        if sync.agent_a != sync.agent_b {
            self.world.agents[&sync.agent_b].lock().unwrap().consume_attention(0.05);
        }

        let reached_name = match &sync.until {
            SyncCondition::Synchronized => "synchronized",
            SyncCondition::Resonating => "resonating",
            SyncCondition::CoherenceThreshold { .. } => "threshold reached",
        };

        self.out(format!("  |     .. {}", levels.join(" -> ")));
        self.out(format!("  |     {} at sync_level {:.3}",
            reached_name, self.link.sync_level().as_f32()));

        if let Some(decay) = sync.decay {
            self.out(format!("  |     decay half-life: {} ticks", decay));
        }

        // Read final states
        let new_a = self.world.agents[&sync.agent_a].lock().unwrap().state;
        let new_b = if sync.agent_a != sync.agent_b {
            self.world.agents[&sync.agent_b].lock().unwrap().state
        } else {
            new_a
        };

        self.out(format!("  |     {}: {} -> {}",
            sync.agent_a, state_name(old_a), state_name(new_a)));
        if sync.agent_a != sync.agent_b {
            self.out(format!("  |     {}: {} -> {}",
                sync.agent_b, state_name(old_b), state_name(new_b)));
        }

        // Bridge: notify external participants of sync completion
        let sync_signal = Signal::new(
            Quality::Completing,
            Direction::Between,
            Priority::new(self.link.sync_level().as_f32()),
            self.agent_a_id,
            self.link.tick(),
        );
        if let Some(response) = self.bridge_notify_signal(&sync.agent_a, &sync_signal) {
            let _ = self.channel.try_send(response);
        }
        if sync.agent_a != sync.agent_b {
            if let Some(response) = self.bridge_notify_signal(&sync.agent_b, &sync_signal) {
                let _ = self.channel.try_send(response);
            }
        }

        self.out("  |".to_string());

        Ok(())
    }

    // ─── APPLY ────────────────────────────────────────────────
    //
    // => when <condition> — Boundary dissolution.
    // Lock agents, apply structural changes, release.

    fn exec_apply(&mut self, apply: &ApplyExpr) -> Result<(), EngineError> {
        let cond_str = format_condition(&apply.condition);
        match &apply.depth {
            Some(d) => self.out(format!("  |  APPLY when {}  depth: {}",
                cond_str, depth_name(*d))),
            None => self.out(format!("  |  APPLY when {}", cond_str)),
        }

        let met = self.eval_condition(&apply.condition);

        if met {
            let sync_val = self.link.sync_level().as_f32();
            self.out(format!("  |     condition met (sync_level = {:.3})", sync_val));

            // Bridge: check if external participants accept the changes
            let wire_changes: Vec<(String, WireValue)> = apply.changes.iter()
                .map(|c| {
                    let val = self.eval_expr(&c.value);
                    (c.name.clone(), value_to_wire(&val))
                })
                .collect();

            let a_accepts = self.bridge_notify_apply(&self.agent_a, &wire_changes);
            let b_accepts = if self.agent_a != self.agent_b {
                self.bridge_notify_apply(&self.agent_b, &wire_changes)
            } else {
                true
            };

            if !a_accepts || !b_accepts {
                self.out("  |     external participant rejected".to_string());
                let quality = self.last_alert_quality.unwrap_or(Quality::Disturbed);
                let priority = Priority::new(0.5);
                if !a_accepts {
                    self.world.agents[&self.agent_a].lock().unwrap().reject(quality, priority);
                }
                if !b_accepts {
                    self.world.agents[&self.agent_b].lock().unwrap().reject(quality, priority);
                }
                self.out("  |".to_string());
                return Ok(());
            }

            // Lock agent_a, apply, release
            let old_a = {
                let mut a = self.world.agents[&self.agent_a].lock().unwrap();
                let old = a.state;
                a.apply();
                old
            };

            // Lock agent_b, apply, release
            let old_b = if self.agent_a != self.agent_b {
                let mut b = self.world.agents[&self.agent_b].lock().unwrap();
                let old = b.state;
                b.apply();
                old
            } else {
                old_a
            };

            // Execute structural changes
            for change in &apply.changes {
                let val = self.eval_expr(&change.value);
                self.out(format!("  |     {:16} <- {}", change.name, val));

                // Lock agent data individually
                {
                    let mut data = self.world.agent_data[&self.agent_a].lock().unwrap();
                    data.insert(change.name.clone(), val.clone());
                }
                if self.agent_a != self.agent_b {
                    let mut data = self.world.agent_data[&self.agent_b].lock().unwrap();
                    data.insert(change.name.clone(), val);
                }
            }

            // Complete application — responsiveness deepens
            {
                let mut a = self.world.agents[&self.agent_a].lock().unwrap();
                a.apply_complete();
            }
            if self.agent_a != self.agent_b {
                let mut b = self.world.agents[&self.agent_b].lock().unwrap();
                b.apply_complete();
            }

            self.out(format!("  |     {}: {} -> Applying",
                self.agent_a, state_name(old_a)));
            if self.agent_a != self.agent_b {
                self.out(format!("  |     {}: {} -> Applying",
                    self.agent_b, state_name(old_b)));
            }
        } else {
            self.out(format!("  |     condition not met (sync_level = {:.3})",
                self.link.sync_level().as_f32()));
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── COMMIT ───────────────────────────────────────────────
    //
    // * from apply|reject — Irreversible.
    // Lock each agent, commit, append history, return to idle.

    fn exec_commit(&mut self, commit: &CommitExpr) -> Result<(), EngineError> {
        let source_name = match commit.source {
            CommitSource::Apply => "apply",
            CommitSource::Reject => "reject",
        };

        self.out(format!("  |  COMMIT from {}", source_name));

        let mut wire_entries: Vec<(String, WireValue)> = Vec::new();
        for kv in &commit.entries {
            let val = self.eval_expr(&kv.value);
            self.out(format!("  |     {:16} {}", format!("{}:", kv.key), val));
            wire_entries.push((kv.key.clone(), value_to_wire(&val)));
        }

        // Bridge: notify external participants of commit
        self.bridge_notify_commit(&self.agent_a, &wire_entries);
        if self.agent_a != self.agent_b {
            self.bridge_notify_commit(&self.agent_b, &wire_entries);
        }

        let quality = self.last_alert_quality.unwrap_or(Quality::Attending);
        let priority = Priority::new(0.9);
        let sync_level = self.link.sync_level();
        let tick = self.link.tick();

        let make_entry = |agent_id: AgentId, other_id: AgentId| -> HistoryEntry {
            match commit.source {
                CommitSource::Apply => HistoryEntry::from_apply(
                    agent_id, other_id, quality,
                    ChangeDepth::Genuine, priority, sync_level, tick, 0,
                ),
                CommitSource::Reject => HistoryEntry::from_reject(
                    agent_id, other_id, quality,
                    priority, sync_level, tick, 0,
                ),
            }
        };

        let entry_a = make_entry(self.agent_a_id, self.agent_b_id);
        let entry_b = make_entry(self.agent_b_id, self.agent_a_id);

        // Lock agent_a, commit, release
        {
            let mut a = self.world.agents[&self.agent_a].lock().unwrap();
            let old_a = a.state;
            a.begin_commit();
            a.history.append(entry_a);
            a.idle();
            let depth_a = a.history.depth();
            self.out(format!("  |     {}: {} -> Committing -> Idle  history depth: {}",
                self.agent_a, state_name(old_a), depth_a));
        }

        // Lock agent_b, commit, release
        if self.agent_a != self.agent_b {
            let mut b = self.world.agents[&self.agent_b].lock().unwrap();
            let old_b = b.state;
            b.begin_commit();
            b.history.append(entry_b);
            b.idle();
            let depth_b = b.history.depth();
            self.out(format!("  |     {}: {} -> Committing -> Idle  history depth: {}",
                self.agent_b, state_name(old_b), depth_b));
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── REJECT ───────────────────────────────────────────────
    //
    // <= when <condition> — Intelligent withdrawal.

    fn exec_reject(&mut self, reject: &RejectExpr) -> Result<(), EngineError> {
        self.out(format!("  |  REJECT when {}", format_condition(&reject.condition)));

        let met = self.eval_condition(&reject.condition);

        if met {
            let quality = self.last_alert_quality.unwrap_or(Quality::Disturbed);
            let priority = Priority::new(0.5);

            {
                let mut b = self.world.agents[&self.agent_b].lock().unwrap();
                b.reject(quality, priority);
            }

            if let Some(ref data) = reject.data {
                let val = self.eval_expr(data);
                self.out(format!("  |     {}", val));
            }

            // Bridge: notify external participants of rejection
            let reject_signal = Signal::new(
                quality,
                Direction::Inward,
                priority,
                self.agent_b_id,
                self.link.tick(),
            );
            self.bridge_notify_signal(&self.agent_b, &reject_signal);

            let state = self.world.agents[&self.agent_b].lock().unwrap().state;
            self.out(format!("  |     {}: -> {}", self.agent_b, state_name(state)));
        } else {
            self.out("  |     (not triggered)".to_string());
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── CONVERGE ─────────────────────────────────────────────
    //
    // A <<>> B — What emerges in the between.

    fn exec_converge(&mut self, converge: &ConvergeBlock) -> Result<(), EngineError> {
        self.out(format!("  |  CONVERGE {} <<>> {}", converge.agent_a, converge.agent_b));

        for expr in &converge.body {
            self.execute_link_expr(expr)?;
        }

        Ok(())
    }

    // ─── EMIT ─────────────────────────────────────────────────

    fn exec_emit(&mut self, emit: &EmitExpr) -> Result<(), EngineError> {
        let quality = emit.attrs.quality.unwrap_or(SignalQuality::Attending);
        let priority = emit.attrs.priority.unwrap_or(0.5);
        let direction = emit.attrs.direction.unwrap_or(SignalDirection::Between);

        let mut signal = Signal::new(
            to_core_quality(quality),
            to_core_direction(direction),
            Priority::new(priority as f32),
            self.agent_a_id,
            self.link.tick(),
        ).with_sequence(self.link.record_signal());

        signal = apply_signal_attrs(signal, Some(&emit.attrs));

        let _ = self.channel.try_send(signal);

        // Consume attention for emit
        self.world.agents[&self.agent_a].lock().unwrap().consume_attention(0.01);

        let val = self.eval_expr(&emit.expression);
        self.out(format!("  |  EMIT {} {:.3} {}  {}",
            quality_name(quality), priority, direction_name(direction), val));
        self.out("  |".to_string());

        Ok(())
    }

    // ─── WHEN ─────────────────────────────────────────────────

    fn exec_when(&mut self, when: &WhenExpr) -> Result<(), EngineError> {
        self.out(format!("  |  WHEN {}", format_condition(&when.condition)));

        let met = self.eval_condition(&when.condition);

        if met {
            self.out("  |     condition met".to_string());
            for expr in &when.body {
                self.execute_link_expr(expr)?;
            }
        } else {
            self.out("  |     (not triggered)".to_string());
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── PENDING ──────────────────────────────────────────────

    fn exec_pending(&mut self, handler: &PendingHandlerExpr) -> Result<(), EngineError> {
        let reason_str = pending_reason_name(&handler.reason);
        let triggered = self.is_pending_active(&handler.reason);

        self.out(format!("  |  PENDING? {}", reason_str));

        if triggered {
            for action in &handler.body {
                match action {
                    PendingAction::Wait { ticks } => {
                        self.out(format!("  |     wait {:.1} tick", ticks));
                    }
                    PendingAction::Guidance(msg) => {
                        self.out(format!("  |     guidance: \"{}\"", msg));
                    }
                    PendingAction::Then(expr) => {
                        self.out("  |     then:".to_string());
                        self.execute_link_expr(expr)?;
                    }
                }
            }
        } else {
            let explanation = match &handler.reason {
                PendingReason::ReceiverNotReady => "receiver is ready",
                PendingReason::LinkNotEstablished => "link is established",
                PendingReason::SyncInsufficient => "sync is sufficient",
                PendingReason::SenderNotReady => "sender is ready",
                PendingReason::MomentNotRight => "moment is right",
                PendingReason::BudgetExhausted => "budget has capacity",
            };
            self.out(format!("  |     (not triggered -- {})", explanation));
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── PATTERN ──────────────────────────────────────────────

    fn exec_pattern(&mut self, pattern_use: &PatternUseExpr) -> Result<(), EngineError> {
        let pattern = self.world.patterns.get(&pattern_use.name)
            .ok_or_else(|| EngineError::UnknownPattern(pattern_use.name.clone()))?
            .clone();

        let args_str: Vec<String> = pattern_use.args.iter()
            .map(|a| format!("{}", self.eval_expr(a)))
            .collect();
        self.out(format!("  |  ~> {}({})", pattern_use.name, args_str.join(", ")));

        let mut subs: HashMap<String, String> = HashMap::new();
        for (i, param) in pattern.params.iter().enumerate() {
            if let Some(arg) = pattern_use.args.get(i) {
                if let Expr::Ident(name) = arg {
                    subs.insert(param.name.clone(), name.clone());
                }
            }
        }
        subs.insert("sync_self".to_string(), self.agent_a.clone());

        for expr in &pattern.body {
            let substituted = substitute_link_expr(expr, &subs);
            self.execute_link_expr(&substituted)?;
        }

        Ok(())
    }

    // ─── CONDITION EVALUATION ─────────────────────────────────

    fn eval_condition(&self, cond: &Condition) -> bool {
        match cond {
            Condition::SyncLevel { op, value } => {
                let current = self.link.sync_level().as_f32() as f64;
                compare_f64(current, *op, *value)
            }
            Condition::Priority { op, value } => {
                let agent = self.world.agents[&self.agent_a].lock().unwrap();
                let current = agent.signal_priority.as_f32() as f64;
                compare_f64(current, *op, *value)
            }
            Condition::AlertIs(quality_str) => {
                if let Some(q) = self.last_alert_quality {
                    matches!(
                        (quality_str.as_str(), q),
                        ("attending", Quality::Attending)
                        | ("questioning", Quality::Questioning)
                        | ("recognizing", Quality::Recognizing)
                        | ("disturbed", Quality::Disturbed)
                        | ("applying", Quality::Applying)
                        | ("completing", Quality::Completing)
                        | ("resting", Quality::Resting)
                    )
                } else {
                    false
                }
            }
            Condition::Confidence { op, value } => {
                let current = self.channel.total_sent() as f64 * 0.1;
                compare_f64(current.min(1.0), *op, *value)
            }
            Condition::Attention { op, value } => {
                let agent = self.world.agents[&self.agent_a].lock().unwrap();
                let current = agent.attention.remaining() as f64;
                compare_f64(current, *op, *value)
            }
            Condition::And(a, b) => {
                self.eval_condition(a) && self.eval_condition(b)
            }
            Condition::Or(a, b) => {
                self.eval_condition(a) || self.eval_condition(b)
            }
            Condition::FieldCompare { .. } => {
                // General field comparison — evaluate to true in concurrent mode
                // (full evaluation requires access to agent data which is proxied)
                true
            }
        }
    }

    fn is_pending_active(&self, reason: &PendingReason) -> bool {
        match reason {
            PendingReason::ReceiverNotReady => {
                let b = self.world.agents[&self.agent_b].lock().unwrap();
                !b.can_receive()
            }
            PendingReason::LinkNotEstablished => {
                self.link.state() == LinkState::Opening
            }
            PendingReason::SyncInsufficient => {
                !self.link.ready_for_apply()
            }
            PendingReason::SenderNotReady => {
                let a = self.world.agents[&self.agent_a].lock().unwrap();
                a.state == AgentState::Committing
            }
            PendingReason::MomentNotRight => {
                false
            }
            PendingReason::BudgetExhausted => {
                let a = self.world.agents[&self.agent_a].lock().unwrap();
                a.is_budget_exhausted()
            }
        }
    }

    // ─── EXPRESSION EVALUATION ────────────────────────────────

    fn eval_expr(&self, expr: &Expr) -> Value {
        match expr {
            Expr::StringLit(s) => Value::String(s.clone()),
            Expr::Number(n) => Value::Number(*n),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Ident(name) => {
                // Check agent_a's data first (scoped let bindings, think bindings)
                if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                    let data = data_mutex.lock().unwrap();
                    if let Some(val) = data.get(name) {
                        return val.clone();
                    }
                }
                // Check global let bindings
                if let Some(data_mutex) = self.world.agent_data.get("__global__") {
                    let data = data_mutex.lock().unwrap();
                    if let Some(val) = data.get(name) {
                        return val.clone();
                    }
                }
                if self.world.agents.contains_key(name) {
                    Value::Agent(name.clone())
                } else {
                    Value::String(name.clone())
                }
            }
            Expr::FieldAccess { object, field } => {
                // Check scoped variable Maps (agent_a scope)
                if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                    let data = data_mutex.lock().unwrap();
                    if let Some(Value::Map(entries)) = data.get(object) {
                        return entries.iter()
                            .find(|(k, _)| k == field)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(Value::Null);
                    }
                }
                // Check global let-bound Maps
                if let Some(data_mutex) = self.world.agent_data.get("__global__") {
                    let data = data_mutex.lock().unwrap();
                    if let Some(Value::Map(entries)) = data.get(object) {
                        return entries.iter()
                            .find(|(k, _)| k == field)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(Value::Null);
                    }
                }
                if let Some(data_mutex) = self.world.agent_data.get(object) {
                    let data = data_mutex.lock().unwrap();
                    if let Some(val) = data.get(field) {
                        val.clone()
                    } else {
                        Value::Null
                    }
                } else {
                    Value::Null
                }
            }
            Expr::HistoryOf(hv) => {
                Value::History(hv.agent.clone())
            }
            Expr::BinaryOp { left, op, right } => {
                let l = self.eval_expr(left);
                let r = self.eval_expr(right);
                match (l, r) {
                    (Value::Number(a), Value::Number(b)) => {
                        Value::Number(match op {
                            BinOp::Add => a + b,
                            BinOp::Sub => a - b,
                            BinOp::Mul => a * b,
                            BinOp::Div => if b != 0.0 { a / b } else { f64::NAN },
                            BinOp::Mod => if b != 0.0 { a % b } else { f64::NAN },
                        })
                    }
                    (Value::String(a), Value::String(b)) if *op == BinOp::Add => {
                        Value::String(format!("{}{}", a, b))
                    }
                    _ => Value::Null,
                }
            }
            Expr::Comparison { left, op, right } => {
                let l = self.eval_expr(left);
                let r = self.eval_expr(right);
                Value::Bool(compare_values(&l, op, &r))
            }
            Expr::UnaryNeg(operand) => {
                match self.eval_expr(operand) {
                    Value::Number(n) => Value::Number(-n),
                    _ => Value::Null,
                }
            }
            Expr::ListLit(items) => {
                Value::List(items.iter().map(|i| self.eval_expr(i)).collect())
            }
            Expr::IndexAccess { object, index } => {
                match (self.eval_expr(object), self.eval_expr(index)) {
                    (Value::List(items), Value::Number(n)) => {
                        items.get(n as usize).cloned().unwrap_or(Value::Null)
                    }
                    _ => Value::Null,
                }
            }
            Expr::Pipe { stages } => {
                // Pipe in concurrent mode: evaluate stages left to right
                let mut val = Value::Null;
                for stage in stages {
                    val = self.eval_expr(stage);
                }
                val
            }
            Expr::Call { name, args } => {
                let evaled: Vec<Value> = args.iter()
                    .map(|a| self.eval_expr(a))
                    .collect();
                // Check for user-defined function in scope or global
                let func = self.world.agent_data.get(&self.agent_a)
                    .and_then(|m| m.lock().unwrap().get(name).cloned())
                    .or_else(|| self.world.agent_data.get("__global__")
                        .and_then(|m| m.lock().unwrap().get(name).cloned()));

                if let Some(Value::Function { params, body, env: captured }) = func {
                    self.call_user_fn(&params, &body, &evaled, &captured)
                } else if let Some(Value::RecordConstructor { fields, .. }) = func {
                    let entries: Vec<(String, Value)> = fields.iter().enumerate()
                        .map(|(i, f)| (f.clone(), evaled.get(i).cloned().unwrap_or(Value::Null)))
                        .collect();
                    Value::Map(entries)
                } else {
                    eval_builtin_concurrent(name, &evaled)
                }
            }
            Expr::Lambda { params, body } => {
                Value::Function {
                    params: params.clone(),
                    body: *body.clone(),
                    env: HashMap::new(),
                }
            }
            Expr::Match { subject, arms } => {
                let val = self.eval_expr(subject);
                for arm in arms {
                    match &arm.pattern {
                        MatchPattern::Wildcard => {
                            return self.eval_expr(&arm.body);
                        }
                        MatchPattern::Literal(lit_expr) => {
                            let lit_val = self.eval_expr(lit_expr);
                            if val == lit_val {
                                return self.eval_expr(&arm.body);
                            }
                        }
                        MatchPattern::Binding(_name) => {
                            // In concurrent mode, binding is limited;
                            // fall back to returning the body with the matched value
                            return self.eval_expr(&arm.body);
                        }
                    }
                }
                Value::Null
            }
            Expr::Not(operand) => {
                match self.eval_expr(operand) {
                    Value::Bool(b) => Value::Bool(!b),
                    _ => Value::Bool(false),
                }
            }
            Expr::LogicalAnd { left, right } => {
                let l = self.eval_expr(left);
                match l {
                    Value::Bool(false) => Value::Bool(false),
                    Value::Bool(true) => self.eval_expr(right),
                    _ => Value::Bool(false),
                }
            }
            Expr::LogicalOr { left, right } => {
                let l = self.eval_expr(left);
                match l {
                    Value::Bool(true) => Value::Bool(true),
                    Value::Bool(false) => self.eval_expr(right),
                    _ => self.eval_expr(right),
                }
            }
            Expr::Block { statements, result } => {
                // In concurrent mode, execute block in agent's scope
                for stmt in statements {
                    match stmt {
                        BlockStatement::Let { name, mutable: _, value } => {
                            let val = self.eval_expr(value);
                            if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                                data_mutex.lock().unwrap().insert(name.clone(), val);
                            }
                        }
                        BlockStatement::Assign { name, value } => {
                            let val = self.eval_expr(value);
                            if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                                data_mutex.lock().unwrap().insert(name.clone(), val);
                            }
                        }
                        BlockStatement::Expr(expr) => {
                            self.eval_expr(expr);
                        }
                    }
                }
                self.eval_expr(result)
            }
            Expr::IfElse { condition, then_branch, else_branch } => {
                let cond = self.eval_expr(condition);
                let is_true = match &cond {
                    Value::Bool(b) => *b,
                    Value::Null => false,
                    Value::Number(n) => *n != 0.0,
                    Value::String(s) => !s.is_empty(),
                    _ => true,
                };
                if is_true {
                    self.eval_expr(then_branch)
                } else if let Some(else_br) = else_branch {
                    self.eval_expr(else_br)
                } else {
                    Value::Null
                }
            }
            Expr::InterpolatedString { parts } => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        StringPart::Literal(s) => result.push_str(s),
                        StringPart::Expression(expr) => {
                            let val = self.eval_expr(expr);
                            result.push_str(&value_to_display(&val));
                        }
                    }
                }
                Value::String(result)
            }
            Expr::WhileExpr { condition, body } => {
                let mut iterations = 0;
                loop {
                    let cond = self.eval_expr(condition);
                    let is_true = match &cond {
                        Value::Bool(b) => *b,
                        Value::Null => false,
                        _ => false,
                    };
                    if !is_true { break; }
                    let body_val = self.eval_expr(body);
                    if matches!(body_val, Value::Return(_)) { return body_val; }
                    if matches!(body_val, Value::Break) { break; }
                    // Continue: skip to next iteration
                    iterations += 1;
                    if iterations >= 10_000 { break; }
                }
                Value::Null
            }
            Expr::ForIn { var, collection, body } => {
                let coll = self.eval_expr(collection);
                let items = match &coll {
                    Value::List(items) => items.clone(),
                    _ => Vec::new(),
                };
                let mut last_val = Value::Null;
                for item in &items {
                    if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                        data_mutex.lock().unwrap().insert(var.clone(), item.clone());
                    }
                    let body_val = self.eval_expr(body);
                    if matches!(body_val, Value::Return(_)) { return body_val; }
                    if matches!(body_val, Value::Break) { break; }
                    if !matches!(body_val, Value::Continue) {
                        last_val = body_val;
                    }
                }
                if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                    data_mutex.lock().unwrap().remove(var);
                }
                last_val
            }
            Expr::TryCatch { body, catch_body } => {
                let result = self.eval_expr(body);
                match &result {
                    Value::Error { .. } => self.eval_expr(catch_body),
                    Value::String(s) if s.starts_with("ERROR") || s.starts_with("error")
                        || s.starts_with("http_") || s.starts_with("json_parse error") => {
                        self.eval_expr(catch_body)
                    }
                    _ => result,
                }
            }
            Expr::MapLit(entries) => {
                let map_entries: Vec<(String, Value)> = entries.iter()
                    .map(|(k, v)| (k.clone(), self.eval_expr(v)))
                    .collect();
                Value::Map(map_entries)
            }
            Expr::Quote(source) => {
                Value::Code(source.clone())
            }
            Expr::Break => Value::Break,
            Expr::Continue => Value::Continue,
            Expr::Return(expr) => { let val = self.eval_expr(expr); Value::Return(Box::new(val)) }
        }
    }

    /// Call a user-defined function in the concurrent engine with closure support.
    fn call_user_fn(
        &self, params: &[String], body: &anwe_parser::ast::Expr,
        args: &[Value], captured_env: &HashMap<String, Value>,
    ) -> Value {
        // Build environment: captured closure env + globals + params
        let mut env = captured_env.clone();
        if let Some(global) = self.world.agent_data.get("__global__") {
            let guard = global.lock().unwrap();
            for (k, v) in guard.iter() {
                env.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
        for (i, param) in params.iter().enumerate() {
            env.insert(param.clone(), args.get(i).cloned().unwrap_or(Value::Null));
        }
        let result = self.eval_fn_expr(body, &env);
        match result {
            Value::Return(inner) => *inner,
            other => other,
        }
    }

    /// Evaluate an expression within a function call context using a HashMap env.
    fn eval_fn_expr(&self, expr: &anwe_parser::ast::Expr, env: &HashMap<String, Value>) -> Value {
        match expr {
            Expr::Ident(name) => {
                if let Some(val) = env.get(name) {
                    return val.clone();
                }
                self.eval_expr(expr)
            }
            Expr::BinaryOp { left, op, right } => {
                let l = self.eval_fn_expr(left, env);
                let r = self.eval_fn_expr(right, env);
                match (l, r) {
                    (Value::Number(a), Value::Number(b)) => {
                        Value::Number(match op {
                            BinOp::Add => a + b,
                            BinOp::Sub => a - b,
                            BinOp::Mul => a * b,
                            BinOp::Div => if b != 0.0 { a / b } else { f64::NAN },
                            BinOp::Mod => if b != 0.0 { a % b } else { f64::NAN },
                        })
                    }
                    (Value::String(a), Value::String(b)) if *op == BinOp::Add => {
                        Value::String(format!("{}{}", a, b))
                    }
                    _ => Value::Null,
                }
            }
            Expr::Comparison { left, op, right } => {
                let l = self.eval_fn_expr(left, env);
                let r = self.eval_fn_expr(right, env);
                Value::Bool(compare_values(&l, op, &r))
            }
            Expr::Number(n) => Value::Number(*n),
            Expr::StringLit(s) => Value::String(s.clone()),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Call { name, args: call_args } => {
                let evaled: Vec<Value> = call_args.iter()
                    .map(|a| self.eval_fn_expr(a, env))
                    .collect();
                // Check env for user-defined function
                if let Some(Value::Function { params, body, env: captured }) = env.get(name.as_str()) {
                    let mut child_env = captured.clone();
                    child_env.extend(env.iter().map(|(k, v)| (k.clone(), v.clone())));
                    for (i, param) in params.iter().enumerate() {
                        child_env.insert(param.clone(), evaled.get(i).cloned().unwrap_or(Value::Null));
                    }
                    let result = self.eval_fn_expr(&body, &child_env);
                    match result {
                        Value::Return(inner) => *inner,
                        other => other,
                    }
                } else {
                    eval_builtin_concurrent(name, &evaled)
                }
            }
            Expr::Lambda { params, body } => {
                Value::Function { params: params.clone(), body: *body.clone(), env: env.clone() }
            }
            Expr::Match { subject, arms } => {
                let val = self.eval_fn_expr(subject, env);
                for arm in arms {
                    match &arm.pattern {
                        MatchPattern::Wildcard => {
                            return self.eval_fn_expr(&arm.body, env);
                        }
                        MatchPattern::Literal(lit_expr) => {
                            let lit_val = self.eval_fn_expr(lit_expr, env);
                            if val == lit_val {
                                return self.eval_fn_expr(&arm.body, env);
                            }
                        }
                        MatchPattern::Binding(name) => {
                            let mut child_env = env.clone();
                            child_env.insert(name.clone(), val.clone());
                            return self.eval_fn_expr(&arm.body, &child_env);
                        }
                    }
                }
                Value::Null
            }
            Expr::Not(operand) => {
                match self.eval_fn_expr(operand, env) {
                    Value::Bool(b) => Value::Bool(!b),
                    _ => Value::Bool(false),
                }
            }
            Expr::LogicalAnd { left, right } => {
                let l = self.eval_fn_expr(left, env);
                match l {
                    Value::Bool(false) => Value::Bool(false),
                    Value::Bool(true) => self.eval_fn_expr(right, env),
                    _ => Value::Bool(false),
                }
            }
            Expr::LogicalOr { left, right } => {
                let l = self.eval_fn_expr(left, env);
                match l {
                    Value::Bool(true) => Value::Bool(true),
                    Value::Bool(false) => self.eval_fn_expr(right, env),
                    _ => self.eval_fn_expr(right, env),
                }
            }
            Expr::Block { statements, result } => {
                let mut block_env = env.clone();
                for stmt in statements {
                    match stmt {
                        BlockStatement::Let { name, mutable: _, value } => {
                            let val = self.eval_fn_expr(value, &block_env);
                            if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
                            block_env.insert(name.clone(), val);
                        }
                        BlockStatement::Assign { name, value } => {
                            let val = self.eval_fn_expr(value, &block_env);
                            if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
                            block_env.insert(name.clone(), val);
                        }
                        BlockStatement::Expr(expr) => {
                            let val = self.eval_fn_expr_in_env(expr, &mut block_env);
                            if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
                        }
                    }
                }
                let val = self.eval_fn_expr(result, &block_env);
                if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
                val
            }
            Expr::IfElse { condition, then_branch, else_branch } => {
                let cond = self.eval_fn_expr(condition, env);
                let is_true = match &cond {
                    Value::Bool(b) => *b,
                    Value::Null => false,
                    Value::Number(n) => *n != 0.0,
                    Value::String(s) => !s.is_empty(),
                    _ => true,
                };
                if is_true {
                    self.eval_fn_expr(then_branch, env)
                } else if let Some(else_br) = else_branch {
                    self.eval_fn_expr(else_br, env)
                } else {
                    Value::Null
                }
            }
            Expr::InterpolatedString { parts } => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        StringPart::Literal(s) => result.push_str(s),
                        StringPart::Expression(expr) => {
                            let val = self.eval_fn_expr(expr, env);
                            result.push_str(&value_to_display(&val));
                        }
                    }
                }
                Value::String(result)
            }
            Expr::WhileExpr { condition, body } => {
                let mut loop_env = env.clone();
                let mut iterations = 0;
                loop {
                    let cond = self.eval_fn_expr(condition, &loop_env);
                    let is_true = match &cond {
                        Value::Bool(b) => *b,
                        Value::Null => false,
                        _ => false,
                    };
                    if !is_true { break; }
                    let body_val = self.eval_loop_body(body, &mut loop_env);
                    if matches!(body_val, Value::Return(_)) { return body_val; }
                    if matches!(body_val, Value::Break) { break; }
                    iterations += 1;
                    if iterations >= 10_000 { break; }
                }
                Value::Null
            }
            Expr::ForIn { var, collection, body } => {
                let coll = self.eval_fn_expr(collection, env);
                let items = match &coll {
                    Value::List(items) => items.clone(),
                    _ => Vec::new(),
                };
                let mut loop_env = env.clone();
                let mut last_val = Value::Null;
                for item in &items {
                    loop_env.insert(var.clone(), item.clone());
                    let body_val = self.eval_loop_body(body, &mut loop_env);
                    if matches!(body_val, Value::Return(_)) { return body_val; }
                    if matches!(body_val, Value::Break) { break; }
                    if !matches!(body_val, Value::Continue) {
                        last_val = body_val;
                    }
                }
                last_val
            }
            Expr::TryCatch { body, catch_body } => {
                let result = self.eval_fn_expr(body, env);
                match &result {
                    Value::Error { .. } => self.eval_fn_expr(catch_body, env),
                    Value::String(s) if s.starts_with("ERROR") || s.starts_with("error")
                        || s.starts_with("http_") || s.starts_with("json_parse error") => {
                        self.eval_fn_expr(catch_body, env)
                    }
                    _ => result,
                }
            }
            Expr::MapLit(entries) => {
                let map_entries: Vec<(String, Value)> = entries.iter()
                    .map(|(k, v)| (k.clone(), self.eval_fn_expr(v, env)))
                    .collect();
                Value::Map(map_entries)
            }
            Expr::Quote(source) => {
                Value::Code(source.clone())
            }
            Expr::Break => Value::Break,
            Expr::Continue => Value::Continue,
            Expr::Return(expr) => { let val = self.eval_fn_expr(expr, env); Value::Return(Box::new(val)) }
            _ => self.eval_expr(expr),
        }
    }

    /// Evaluate an expression in-place, allowing loops to mutate the env directly.
    /// Returns Value to propagate break/continue signals.
    fn eval_fn_expr_in_env(&self, expr: &Expr, env: &mut HashMap<String, Value>) -> Value {
        match expr {
            Expr::WhileExpr { condition, body } => {
                let mut iterations = 0;
                loop {
                    let cond = self.eval_fn_expr(condition, env);
                    let is_true = match &cond {
                        Value::Bool(b) => *b,
                        Value::Null => false,
                        _ => false,
                    };
                    if !is_true { break; }
                    let body_val = self.eval_loop_body(body, env);
                    if matches!(body_val, Value::Return(_)) { return body_val; }
                    if matches!(body_val, Value::Break) { break; }
                    iterations += 1;
                    if iterations >= 10_000 { break; }
                }
                Value::Null
            }
            Expr::ForIn { var, collection, body } => {
                let coll = self.eval_fn_expr(collection, env);
                let items = match &coll {
                    Value::List(items) => items.clone(),
                    _ => Vec::new(),
                };
                for item in &items {
                    env.insert(var.clone(), item.clone());
                    let body_val = self.eval_loop_body(body, env);
                    if matches!(body_val, Value::Return(_)) { env.remove(var); return body_val; }
                    if matches!(body_val, Value::Break) { break; }
                }
                env.remove(var);
                Value::Null
            }
            _ => {
                self.eval_fn_expr(expr, env)
            }
        }
    }

    /// Execute a loop body directly in the given mutable env.
    /// Returns Value to propagate break/continue signals.
    fn eval_loop_body(&self, body: &Expr, env: &mut HashMap<String, Value>) -> Value {
        if let Expr::Block { statements, result } = body {
            for stmt in statements {
                match stmt {
                    BlockStatement::Let { name, mutable: _, value } => {
                        let val = self.eval_fn_expr(value, env);
                        if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
                        env.insert(name.clone(), val);
                    }
                    BlockStatement::Assign { name, value } => {
                        let val = self.eval_fn_expr(value, env);
                        if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
                        env.insert(name.clone(), val);
                    }
                    BlockStatement::Expr(e) => {
                        let val = self.eval_fn_expr_in_env(e, env);
                        if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
                    }
                }
            }
            let val = self.eval_fn_expr_in_env(result, env);
            if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
            val
        } else {
            let val = self.eval_fn_expr_in_env(body, env);
            if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
            val
        }
    }

    // ─── EACH (ITERATION) ────────────────────────────────────

    fn exec_each(&mut self, each: &anwe_parser::ast::EachExpr) -> Result<(), EngineError> {
        let collection = self.eval_expr(&each.collection);
        let items = match &collection {
            Value::List(items) => items.clone(),
            _ => {
                self.out(format!("  |  EACH {} in (not a list — skipping)", each.var));
                return Ok(());
            }
        };

        self.out(format!("  |  EACH {} in [{} items]", each.var, items.len()));

        for (i, item) in items.iter().enumerate() {
            // Bind the iteration variable in agent_a's data
            if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                data_mutex.lock().unwrap().insert(each.var.clone(), item.clone());
            }
            self.out(format!("  |     iteration {} — {} = {}", i + 1, each.var, item));
            for expr in &each.body {
                self.execute_link_expr(expr)?;
            }
        }

        // Clean up
        if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
            data_mutex.lock().unwrap().remove(&each.var);
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── IF/ELSE ─────────────────────────────────────────────

    fn exec_if_else(&mut self, ie: &anwe_parser::ast::IfElseExpr) -> Result<(), EngineError> {
        let met = self.eval_condition(&ie.condition);
        self.out(format!("  |  IF {}", format_condition(&ie.condition)));

        if met {
            self.out("  |     -> then branch".to_string());
            for expr in &ie.then_body {
                self.execute_link_expr(expr)?;
            }
        } else if !ie.else_body.is_empty() {
            self.out("  |     -> else branch".to_string());
            for expr in &ie.else_body {
                self.execute_link_expr(expr)?;
            }
        } else {
            self.out("  |     (condition not met, no else branch)".to_string());
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── SPAWN ───────────────────────────────────────────────

    fn exec_spawn(&mut self, spawn: &anwe_parser::ast::SpawnExpr) -> Result<(), EngineError> {
        let config_str = spawn.data.iter()
            .map(|kv| format!("{}: {}", kv.key, self.eval_expr(&kv.value)))
            .collect::<Vec<_>>().join(", ");
        self.out(format!("  |  SPAWN {} from {}", spawn.name, spawn.template));
        if !config_str.is_empty() {
            self.out(format!("  |     {}", config_str));
        }
        self.out("  |     agent created (concurrent)".to_string());
        self.out("  |".to_string());
        Ok(())
    }

    // ─── RETIRE ──────────────────────────────────────────────

    fn exec_retire(&mut self, retire: &anwe_parser::ast::RetireExpr) -> Result<(), EngineError> {
        let reason_str = retire.data.iter()
            .map(|kv| format!("{}: {}", kv.key, self.eval_expr(&kv.value)))
            .collect::<Vec<_>>().join(", ");
        self.out(format!("  |  RETIRE {}", retire.name));
        if !reason_str.is_empty() {
            self.out(format!("  |     {}", reason_str));
        }
        self.out("  |     agent removed".to_string());
        self.out("  |".to_string());
        Ok(())
    }

    // ─── SYNC_ALL ────────────────────────────────────────────

    fn exec_sync_all(&mut self, sync_all: &anwe_parser::ast::SyncAllExpr) -> Result<(), EngineError> {
        let target_name = match &sync_all.until {
            SyncCondition::Synchronized => "synchronized",
            SyncCondition::Resonating => "resonating",
            SyncCondition::CoherenceThreshold { .. } => "threshold",
        };

        self.out(format!("  |  SYNC_ALL [{}] until {}", sync_all.agents.join(", "), target_name));

        let target_level: f32 = match &sync_all.until {
            SyncCondition::Synchronized => 0.75,
            SyncCondition::Resonating => 0.95,
            SyncCondition::CoherenceThreshold { op, value } => {
                let v = *value as f32;
                match op {
                    ComparisonOp::Greater => (v + 0.05).min(1.0),
                    _ => v,
                }
            }
        };

        self.link.begin_sync();
        for name in &sync_all.agents {
            if let Some(agent_mutex) = self.world.agents.get(name) {
                agent_mutex.lock().unwrap().sync();
            }
        }

        let steps = ((target_level / 0.1).ceil() as u16).max(1);
        let mut levels = Vec::new();
        for i in 1..=steps {
            let level = (0.1 * i as f32).min(target_level);
            self.link.update_sync_level(SyncLevel::new(level));
            levels.push(format!("{:.3}", level));
            if level >= target_level { break; }
        }

        self.out(format!("  |     barrier: {} agents syncing", sync_all.agents.len()));
        self.out(format!("  |     .. {}", levels.join(" -> ")));
        self.out(format!("  |     all {} at sync_level {:.3}", target_name, self.link.sync_level().as_f32()));
        self.out("  |".to_string());
        Ok(())
    }

    // ─── BROADCAST ───────────────────────────────────────────

    fn exec_broadcast(&mut self, broadcast: &anwe_parser::ast::BroadcastExpr) -> Result<(), EngineError> {
        self.out(format!("  |  BROADCAST to [{}]", broadcast.agents.join(", ")));

        for pulse in &broadcast.body {
            let core_quality = to_core_quality(pulse.quality);
            let core_direction = to_core_direction(pulse.direction);
            let core_priority = Priority::new(pulse.priority as f32);

            for agent_name in &broadcast.agents {
                if let Some(&agent_id) = self.world.agent_ids.get(agent_name) {
                    let signal = Signal::new(
                        core_quality, core_direction, core_priority,
                        agent_id, self.link.tick(),
                    ).with_sequence(self.link.record_signal());
                    let _ = self.channel.try_send(signal);
                }
            }

            let mut line = format!("  |     signal {:12} {:.3} {} -> {} agents",
                quality_name(pulse.quality), pulse.priority,
                direction_name(pulse.direction), broadcast.agents.len());
            if let Some(ref data) = pulse.data {
                line.push_str(&format!("  data: {}", self.eval_expr(data)));
            }
            self.out(line);
        }

        self.out(format!("  |     {} signals x {} agents = {} deliveries",
            broadcast.body.len(), broadcast.agents.len(),
            broadcast.body.len() * broadcast.agents.len()));
        self.out("  |".to_string());
        Ok(())
    }

    // ─── MULTI-CONVERGE ─────────────────────────────────────

    fn exec_multi_converge(&mut self, mc: &anwe_parser::ast::MultiConvergeExpr) -> Result<(), EngineError> {
        self.out(format!("  |  CONVERGE [{}]", mc.agents.join(", ")));

        for kv in &mc.options {
            let val = self.eval_expr(&kv.value);
            self.out(format!("  |     {}: {}", kv.key, val));
            // Store in all participating agents
            for agent_name in &mc.agents {
                if let Some(data_mutex) = self.world.agent_data.get(agent_name) {
                    data_mutex.lock().unwrap().insert(kv.key.clone(), val.clone());
                }
            }
        }

        self.out(format!("  |     {} agents converged", mc.agents.len()));
        self.out("  |".to_string());
        Ok(())
    }

    // ─── SAVE ────────────────────────────────────────────────

    fn exec_save(&mut self, save: &anwe_parser::ast::SaveExpr) -> Result<(), EngineError> {
        self.out(format!("  |  SAVE {} to \"{}\"", save.agent, save.path));

        // Build serializable state
        let mut json_parts = Vec::new();
        json_parts.push(format!("\"agent\": \"{}\"", save.agent));

        if let Some(data_mutex) = self.world.agent_data.get(&save.agent) {
            let data = data_mutex.lock().unwrap();
            let data_parts: Vec<String> = data.iter()
                .map(|(k, v)| format!("\"{}\": \"{}\"", k, v))
                .collect();
            json_parts.push(format!("\"data\": {{{}}}", data_parts.join(", ")));
        }

        let json = format!("{{{}}}", json_parts.join(", "));
        match std::fs::write(&save.path, &json) {
            Ok(()) => self.out(format!("  |     state serialized ({} bytes)", json.len())),
            Err(e) => self.out(format!("  |     WARNING: failed to write: {}", e)),
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── RESTORE ─────────────────────────────────────────────

    fn exec_restore(&mut self, restore: &anwe_parser::ast::RestoreExpr) -> Result<(), EngineError> {
        self.out(format!("  |  RESTORE {} from \"{}\"", restore.agent, restore.path));

        match std::fs::read_to_string(&restore.path) {
            Ok(content) => {
                self.out(format!("  |     state restored ({} bytes)", content.len()));
            }
            Err(e) => {
                self.out(format!("  |     WARNING: failed to read: {}", e));
                self.out("  |     (agent continues with current state)".to_string());
            }
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── HISTORY QUERY ───────────────────────────────────────

    fn exec_history_query(&mut self, hq: &anwe_parser::ast::HistoryQueryExpr) -> Result<(), EngineError> {
        self.out(format!("  |  HISTORY_QUERY {}", hq.agent));

        for kv in &hq.options {
            let val = self.eval_expr(&kv.value);
            self.out(format!("  |     {}: {}", kv.key, val));
        }

        if let Some(agent_mutex) = self.world.agents.get(&hq.agent) {
            let agent = agent_mutex.lock().unwrap();
            let depth = agent.history.depth();
            self.out(format!("  |     history depth: {}", depth));
        } else {
            self.out("  |     (agent not found)".to_string());
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── THINK ───────────────────────────────────────────────

    fn exec_think(&mut self, think: &anwe_parser::ast::ThinkExpr) -> Result<(), EngineError> {
        self.out("  |  THINK".to_string());

        for binding in &think.bindings {
            let val = self.eval_expr(&binding.value);
            self.out(format!("  |     {:16} <- {}", binding.name, val));

            // Store think bindings in agent data
            if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                data_mutex.lock().unwrap().insert(binding.name.clone(), val);
            }
        }

        self.world.agents[&self.agent_a].lock().unwrap().consume_attention(0.03);
        self.out("  |".to_string());
        Ok(())
    }

    // ─── EXPRESS ─────────────────────────────────────────────

    fn exec_express(&mut self, express: &anwe_parser::ast::ExpressExpr) -> Result<(), EngineError> {
        let quality = express.attrs.as_ref()
            .and_then(|a| a.quality)
            .unwrap_or(SignalQuality::Recognizing);
        let priority = express.attrs.as_ref()
            .and_then(|a| a.priority)
            .unwrap_or(0.5);

        let signal = Signal::new(
            to_core_quality(quality),
            to_core_direction(SignalDirection::Outward),
            Priority::new(priority as f32),
            self.agent_a_id,
            self.link.tick(),
        ).with_sequence(self.link.record_signal());
        let _ = self.channel.try_send(signal);

        let val = self.eval_expr(&express.expression);
        self.out(format!("  |  EXPRESS {} {:.3}  {}",
            quality_name(quality), priority, val));
        self.out("  |".to_string());
        Ok(())
    }

    // ─── SENSE ───────────────────────────────────────────────

    fn exec_sense(&mut self, sense: &anwe_parser::ast::SenseExpr) -> Result<(), EngineError> {
        self.out("  |  SENSE".to_string());

        let signal_count = self.channel.total_sent() as f64;
        let sync_level = self.link.sync_level().as_f32() as f64;
        let attention = self.world.agents[&self.agent_a].lock().unwrap()
            .attention.remaining() as f64;

        // Store sense results
        if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
            let mut data = data_mutex.lock().unwrap();
            data.insert("signal_count".to_string(), Value::Number(signal_count));
            data.insert("sync_level".to_string(), Value::Number(sync_level));
            data.insert("attention".to_string(), Value::Number(attention));
        }

        self.out(format!("  |     signal_count: {}", signal_count));
        self.out(format!("  |     sync_level:   {:.3}", sync_level));
        self.out(format!("  |     attention:     {:.3}", attention));

        for binding in &sense.bindings {
            let val = self.eval_expr(&binding.value);
            self.out(format!("  |     {:16} <- {}", binding.name, val));
            if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
                data_mutex.lock().unwrap().insert(binding.name.clone(), val);
            }
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── AUTHOR ──────────────────────────────────────────────

    fn exec_author(&mut self, author: &anwe_parser::ast::AuthorExpr) -> Result<(), EngineError> {
        self.out(format!("  |  AUTHOR attend \"{}\" priority {:.3}",
            author.block.label, author.block.priority));

        // Execute the authored block immediately
        for expr in &author.block.body {
            self.execute_link_expr(expr)?;
        }

        self.out("  |".to_string());
        Ok(())
    }

    // ─── WHILE ───────────────────────────────────────────────

    fn exec_while(&mut self, while_expr: &anwe_parser::ast::WhileExpr) -> Result<(), EngineError> {
        self.out(format!("  |  WHILE {}", format_condition(&while_expr.condition)));

        let max_iterations = 100;
        let mut iteration = 0;

        while self.eval_condition(&while_expr.condition) && iteration < max_iterations {
            iteration += 1;
            self.out(format!("  |     iteration {}", iteration));
            for expr in &while_expr.body {
                self.execute_link_expr(expr)?;
            }
        }

        self.out(format!("  |     completed after {} iterations", iteration));
        self.out("  |".to_string());
        Ok(())
    }

    // ─── ATTEMPT ─────────────────────────────────────────────

    fn exec_attempt(&mut self, attempt: &anwe_parser::ast::AttemptExpr) -> Result<(), EngineError> {
        self.out("  |  ATTEMPT".to_string());

        // Try executing the body
        let mut succeeded = true;
        for expr in &attempt.body {
            if let Err(_) = self.execute_link_expr(expr) {
                succeeded = false;
                break;
            }
        }

        if !succeeded && !attempt.recover.is_empty() {
            self.out("  |     -> recover".to_string());
            for expr in &attempt.recover {
                self.execute_link_expr(expr)?;
            }
        }

        self.out("  |".to_string());
        Ok(())
    }

    fn exec_let(&mut self, binding: &anwe_parser::ast::LetBinding) -> Result<(), EngineError> {
        let val = self.eval_expr(&binding.value);
        self.out(format!("  let{} {} = {}", if binding.mutable { " mut" } else { "" }, binding.name, val));

        // Store in agent_a's data scope
        if let Some(data_mutex) = self.world.agent_data.get(&self.agent_a) {
            let mut data = data_mutex.lock().unwrap();
            data.insert(binding.name.clone(), val);
        }

        Ok(())
    }

    fn exec_assign(&mut self, assign: &anwe_parser::ast::AssignExpr) -> Result<(), EngineError> {
        let val = self.eval_expr(&assign.value);
        self.out(format!("  {} = {}", assign.name, val));

        // Try agent_a scope first, then global
        let scope = if self.world.agent_data.get(&self.agent_a)
            .map_or(false, |m| m.lock().unwrap().contains_key(&assign.name))
        {
            self.agent_a.clone()
        } else if self.world.agent_data.get("__global__")
            .map_or(false, |m| m.lock().unwrap().contains_key(&assign.name))
        {
            "__global__".to_string()
        } else {
            return Err(EngineError::ExecutionError(format!(
                "Cannot assign to '{}': variable not declared",
                assign.name
            )));
        };

        if let Some(data_mutex) = self.world.agent_data.get(&scope) {
            let mut data = data_mutex.lock().unwrap();
            data.insert(assign.name.clone(), val);
        }

        Ok(())
    }
}

/// Convert AST LinkPriority to scheduler FiberPriority.
fn to_fiber_priority(p: LinkPriority) -> FiberPriority {
    match p {
        LinkPriority::Critical => FiberPriority::Critical,
        LinkPriority::High => FiberPriority::High,
        LinkPriority::Normal => FiberPriority::Normal,
        LinkPriority::Low => FiberPriority::Low,
        LinkPriority::Background => FiberPriority::Background,
    }
}

// ─── STATIC EXPRESSION EVALUATOR ─────────────────────────────
//
// Used during registration (Phase 1) before the SharedWorld
// is assembled. Takes raw maps instead of SharedWorld reference.

fn eval_expr_static(
    expr: &Expr,
    agents: &HashMap<String, Arc<Mutex<Agent>>>,
    agent_data: &HashMap<String, Arc<Mutex<HashMap<String, Value>>>>,
) -> Value {
    match expr {
        Expr::StringLit(s) => Value::String(s.clone()),
        Expr::Number(n) => Value::Number(*n),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Ident(name) => {
            if agents.contains_key(name) {
                Value::Agent(name.clone())
            } else {
                Value::String(name.clone())
            }
        }
        Expr::FieldAccess { object, field } => {
            if let Some(data_mutex) = agent_data.get(object) {
                let data = data_mutex.lock().unwrap();
                if let Some(val) = data.get(field) {
                    val.clone()
                } else {
                    Value::Null
                }
            } else {
                Value::Null
            }
        }
        Expr::HistoryOf(hv) => Value::History(hv.agent.clone()),
        Expr::BinaryOp { left, op, right } => {
            let l = eval_expr_static(left, agents, agent_data);
            let r = eval_expr_static(right, agents, agent_data);
            match (l, r) {
                (Value::Number(a), Value::Number(b)) => {
                    Value::Number(match op {
                        BinOp::Add => a + b,
                        BinOp::Sub => a - b,
                        BinOp::Mul => a * b,
                        BinOp::Div => if b != 0.0 { a / b } else { f64::NAN },
                        BinOp::Mod => if b != 0.0 { a % b } else { f64::NAN },
                    })
                }
                _ => Value::Null,
            }
        }
        Expr::Comparison { left, op, right } => {
            let l = eval_expr_static(left, agents, agent_data);
            let r = eval_expr_static(right, agents, agent_data);
            Value::Bool(compare_values(&l, op, &r))
        }
        Expr::UnaryNeg(operand) => {
            match eval_expr_static(operand, agents, agent_data) {
                Value::Number(n) => Value::Number(-n),
                _ => Value::Null,
            }
        }
        Expr::ListLit(items) => {
            Value::List(items.iter().map(|i| eval_expr_static(i, agents, agent_data)).collect())
        }
        Expr::IndexAccess { object, index } => {
            match (eval_expr_static(object, agents, agent_data), eval_expr_static(index, agents, agent_data)) {
                (Value::List(items), Value::Number(n)) => {
                    items.get(n as usize).cloned().unwrap_or(Value::Null)
                }
                _ => Value::Null,
            }
        }
        Expr::Pipe { stages } => {
            // Pipe in static context: evaluate stages left to right
            let mut val = Value::Null;
            for stage in stages {
                val = eval_expr_static(stage, agents, agent_data);
            }
            val
        }
        Expr::Call { name, args } => {
            let evaluated_args: Vec<Value> = args.iter()
                .map(|a| eval_expr_static(a, agents, agent_data))
                .collect();
            // Try builtin functions first
            let builtin_result = eval_builtin_concurrent(name, &evaluated_args);
            if !matches!(builtin_result, Value::Null) || name == "type" || name == "string" || name == "number" {
                return builtin_result;
            }
            // Try user-defined functions from globals
            if let Some(data_mutex) = agent_data.get("__global__") {
                let data = data_mutex.lock().unwrap();
                if let Some(Value::Function { params, body, env: fn_env }) = data.get(name).cloned() {
                    drop(data); // release lock before evaluating
                    // Bind parameters into globals temporarily
                    let mut saved: Vec<(String, Option<Value>)> = Vec::new();
                    if let Some(global) = agent_data.get("__global__") {
                        let mut g = global.lock().unwrap();
                        // Save and bind params
                        for (i, param) in params.iter().enumerate() {
                            saved.push((param.clone(), g.get(param).cloned()));
                            if let Some(val) = evaluated_args.get(i) {
                                g.insert(param.clone(), val.clone());
                            }
                        }
                        // Also bind closure env
                        for (k, v) in &fn_env {
                            if !params.contains(k) {
                                saved.push((k.clone(), g.get(k).cloned()));
                                g.insert(k.clone(), v.clone());
                            }
                        }
                    }
                    let result = eval_expr_static(&body, agents, agent_data);
                    // Restore saved values
                    if let Some(global) = agent_data.get("__global__") {
                        let mut g = global.lock().unwrap();
                        for (k, v) in saved {
                            if let Some(val) = v {
                                g.insert(k, val);
                            } else {
                                g.remove(&k);
                            }
                        }
                    }
                    // Unwrap Return wrapper
                    return match result {
                        Value::Return(v) => *v,
                        other => other,
                    };
                }
            }
            Value::Null
        }
        Expr::Lambda { params, body } => {
            Value::Function {
                params: params.clone(),
                body: *body.clone(),
                env: HashMap::new(),
            }
        }
        Expr::Match { subject, arms } => {
            let val = eval_expr_static(subject, agents, agent_data);
            for arm in arms {
                match &arm.pattern {
                    MatchPattern::Wildcard => {
                        return eval_expr_static(&arm.body, agents, agent_data);
                    }
                    MatchPattern::Literal(lit_expr) => {
                        let lit_val = eval_expr_static(lit_expr, agents, agent_data);
                        if val == lit_val {
                            return eval_expr_static(&arm.body, agents, agent_data);
                        }
                    }
                    MatchPattern::Binding(_) => {
                        return eval_expr_static(&arm.body, agents, agent_data);
                    }
                }
            }
            Value::Null
        }
        Expr::Not(operand) => {
            match eval_expr_static(operand, agents, agent_data) {
                Value::Bool(b) => Value::Bool(!b),
                _ => Value::Bool(false),
            }
        }
        Expr::LogicalAnd { left, right } => {
            let l = eval_expr_static(left, agents, agent_data);
            match l {
                Value::Bool(false) => Value::Bool(false),
                Value::Bool(true) => eval_expr_static(right, agents, agent_data),
                _ => Value::Bool(false),
            }
        }
        Expr::LogicalOr { left, right } => {
            let l = eval_expr_static(left, agents, agent_data);
            match l {
                Value::Bool(true) => Value::Bool(true),
                Value::Bool(false) => eval_expr_static(right, agents, agent_data),
                _ => eval_expr_static(right, agents, agent_data),
            }
        }
        Expr::Block { statements, result } => {
            // Static context: limited block support, just evaluate result
            for stmt in statements {
                if let BlockStatement::Expr(expr) = stmt {
                    eval_expr_static(expr, agents, agent_data);
                }
            }
            eval_expr_static(result, agents, agent_data)
        }
        Expr::IfElse { condition, then_branch, else_branch } => {
            let cond = eval_expr_static(condition, agents, agent_data);
            let is_true = match &cond {
                Value::Bool(b) => *b,
                Value::Null => false,
                Value::Number(n) => *n != 0.0,
                Value::String(s) => !s.is_empty(),
                _ => true,
            };
            if is_true {
                eval_expr_static(then_branch, agents, agent_data)
            } else if let Some(else_br) = else_branch {
                eval_expr_static(else_br, agents, agent_data)
            } else {
                Value::Null
            }
        }
        Expr::InterpolatedString { parts } => {
            let mut result = String::new();
            for part in parts {
                match part {
                    StringPart::Literal(s) => result.push_str(s),
                    StringPart::Expression(expr) => {
                        let val = eval_expr_static(expr, agents, agent_data);
                        result.push_str(&value_to_display(&val));
                    }
                }
            }
            Value::String(result)
        }
        Expr::WhileExpr { condition, body } => {
            let mut iterations = 0;
            loop {
                let cond = eval_expr_static(condition, agents, agent_data);
                let is_true = match &cond {
                    Value::Bool(b) => *b,
                    Value::Null => false,
                    _ => false,
                };
                if !is_true { break; }
                let body_val = eval_expr_static(body, agents, agent_data);
                if matches!(body_val, Value::Return(_)) { return body_val; }
                if matches!(body_val, Value::Break) { break; }
                iterations += 1;
                if iterations >= 10_000 { break; }
            }
            Value::Null
        }
        Expr::ForIn { var, collection, body } => {
            let coll = eval_expr_static(collection, agents, agent_data);
            let items = match &coll {
                Value::List(items) => items.clone(),
                _ => Vec::new(),
            };
            let mut last_val = Value::Null;
            for item in &items {
                // In static context, limited — store in global data if available
                if let Some(data_mutex) = agent_data.get("__global__") {
                    data_mutex.lock().unwrap().insert(var.clone(), item.clone());
                }
                let body_val = eval_expr_static(body, agents, agent_data);
                if matches!(body_val, Value::Return(_)) { return body_val; }
                if matches!(body_val, Value::Break) { break; }
                if !matches!(body_val, Value::Continue) {
                    last_val = body_val;
                }
            }
            if let Some(data_mutex) = agent_data.get("__global__") {
                data_mutex.lock().unwrap().remove(var);
            }
            last_val
        }
        Expr::TryCatch { body, catch_body } => {
            let result = eval_expr_static(body, agents, agent_data);
            match &result {
                Value::Error { .. } => eval_expr_static(catch_body, agents, agent_data),
                Value::String(s) if s.starts_with("ERROR") || s.starts_with("error")
                    || s.starts_with("http_") || s.starts_with("json_parse error") => {
                    eval_expr_static(catch_body, agents, agent_data)
                }
                _ => result,
            }
        }
        Expr::MapLit(entries) => {
            let map_entries: Vec<(String, Value)> = entries.iter()
                .map(|(k, v)| (k.clone(), eval_expr_static(v, agents, agent_data)))
                .collect();
            Value::Map(map_entries)
        }
        Expr::Quote(source) => {
            Value::Code(source.clone())
        }
        Expr::Break => Value::Break,
        Expr::Continue => Value::Continue,
        Expr::Return(expr) => { let val = eval_expr_static(expr, agents, agent_data); Value::Return(Box::new(val)) }
    }
}

// ─── BUILTIN FUNCTIONS (CONCURRENT) ───────────────────────────
//
// Standalone version of eval_builtin for the concurrent engine.
// These are pure functions on Values — no engine state needed.

fn eval_builtin_concurrent(name: &str, args: &[Value]) -> Value {
    match name {
        // ── String operations ──
        "len" => match args.first() {
            Some(Value::String(s)) => Value::Number(s.len() as f64),
            Some(Value::List(l)) => Value::Number(l.len() as f64),
            Some(Value::Map(m)) => Value::Number(m.len() as f64),
            _ => Value::Number(0.0),
        },
        "split" => match (args.first(), args.get(1)) {
            (Some(Value::String(s)), Some(Value::String(d))) =>
                Value::List(s.split(d.as_str()).map(|p| Value::String(p.to_string())).collect()),
            _ => Value::Null,
        },
        "join" => match (args.first(), args.get(1)) {
            (Some(Value::List(items)), Some(Value::String(d))) => {
                let parts: Vec<String> = items.iter().map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => format!("{}", other),
                }).collect();
                Value::String(parts.join(d))
            }
            _ => Value::Null,
        },
        "trim" => match args.first() {
            Some(Value::String(s)) => Value::String(s.trim().to_string()),
            _ => Value::Null,
        },
        "upper" => match args.first() {
            Some(Value::String(s)) => Value::String(s.to_uppercase()),
            _ => Value::Null,
        },
        "lower" => match args.first() {
            Some(Value::String(s)) => Value::String(s.to_lowercase()),
            _ => Value::Null,
        },
        "contains" => match (args.first(), args.get(1)) {
            (Some(Value::String(s)), Some(Value::String(sub))) => Value::Bool(s.contains(sub.as_str())),
            (Some(Value::List(items)), Some(val)) => Value::Bool(items.contains(val)),
            _ => Value::Bool(false),
        },
        "replace" => match (args.first(), args.get(1), args.get(2)) {
            (Some(Value::String(s)), Some(Value::String(old)), Some(Value::String(new))) =>
                Value::String(s.replace(old.as_str(), new.as_str())),
            _ => Value::Null,
        },
        "substring" => match (args.first(), args.get(1), args.get(2)) {
            (Some(Value::String(s)), Some(Value::Number(start)), Some(Value::Number(end))) => {
                let start = (*start as usize).min(s.len());
                let end = (*end as usize).min(s.len());
                Value::String(s[start..end].to_string())
            }
            (Some(Value::String(s)), Some(Value::Number(start)), None) => {
                let start = (*start as usize).min(s.len());
                Value::String(s[start..].to_string())
            }
            _ => Value::Null,
        },
        "starts_with" => match (args.first(), args.get(1)) {
            (Some(Value::String(s)), Some(Value::String(p))) => Value::Bool(s.starts_with(p.as_str())),
            _ => Value::Bool(false),
        },
        "ends_with" => match (args.first(), args.get(1)) {
            (Some(Value::String(s)), Some(Value::String(p))) => Value::Bool(s.ends_with(p.as_str())),
            _ => Value::Bool(false),
        },
        "chars" => match args.first() {
            Some(Value::String(s)) => Value::List(s.chars().map(|c| Value::String(c.to_string())).collect()),
            _ => Value::Null,
        },
        "index_of" => match (args.first(), args.get(1)) {
            (Some(Value::String(s)), Some(Value::String(sub))) => {
                match s.find(sub.as_str()) {
                    Some(pos) => Value::Number(pos as f64),
                    None => Value::Number(-1.0),
                }
            }
            (Some(Value::List(items)), Some(val)) => {
                match items.iter().position(|x| x == val) {
                    Some(pos) => Value::Number(pos as f64),
                    None => Value::Number(-1.0),
                }
            }
            _ => Value::Number(-1.0),
        },
        "char_at" => match (args.first(), args.get(1)) {
            (Some(Value::String(s)), Some(Value::Number(idx))) => {
                let i = *idx as usize;
                s.chars().nth(i).map(|c| Value::String(c.to_string())).unwrap_or(Value::Null)
            }
            _ => Value::Null,
        },
        "slice" => match (args.first(), args.get(1), args.get(2)) {
            (Some(Value::List(list)), Some(Value::Number(start)), Some(Value::Number(end))) => {
                let start = (*start as usize).min(list.len());
                let end = (*end as usize).min(list.len());
                Value::List(list[start..end].to_vec())
            }
            (Some(Value::List(list)), Some(Value::Number(start)), None) => {
                let start = (*start as usize).min(list.len());
                Value::List(list[start..].to_vec())
            }
            (Some(Value::String(s)), Some(Value::Number(start)), Some(Value::Number(end))) => {
                let start = (*start as usize).min(s.len());
                let end = (*end as usize).min(s.len());
                Value::String(s[start..end].to_string())
            }
            (Some(Value::String(s)), Some(Value::Number(start)), None) => {
                let start = (*start as usize).min(s.len());
                Value::String(s[start..].to_string())
            }
            _ => Value::Null,
        },

        // ── Math operations ──
        "abs" => match args.first() { Some(Value::Number(n)) => Value::Number(n.abs()), _ => Value::Null },
        "floor" => match args.first() { Some(Value::Number(n)) => Value::Number(n.floor()), _ => Value::Null },
        "ceil" => match args.first() { Some(Value::Number(n)) => Value::Number(n.ceil()), _ => Value::Null },
        "round" => match args.first() { Some(Value::Number(n)) => Value::Number(n.round()), _ => Value::Null },
        "sqrt" => match args.first() { Some(Value::Number(n)) => Value::Number(n.sqrt()), _ => Value::Null },
        "pow" => match (args.first(), args.get(1)) {
            (Some(Value::Number(a)), Some(Value::Number(b))) => Value::Number(a.powf(*b)),
            _ => Value::Null,
        },
        "min" => match (args.first(), args.get(1)) {
            (Some(Value::Number(a)), Some(Value::Number(b))) => Value::Number(a.min(*b)),
            _ => Value::Null,
        },
        "max" => match (args.first(), args.get(1)) {
            (Some(Value::Number(a)), Some(Value::Number(b))) => Value::Number(a.max(*b)),
            _ => Value::Null,
        },
        "clamp" => match (args.first(), args.get(1), args.get(2)) {
            (Some(Value::Number(v)), Some(Value::Number(lo)), Some(Value::Number(hi))) =>
                Value::Number(v.max(*lo).min(*hi)),
            _ => Value::Null,
        },
        "log" => match args.first() { Some(Value::Number(n)) => Value::Number(n.ln()), _ => Value::Null },

        // ── List operations ──
        "push" | "append" => match (args.first(), args.get(1)) {
            (Some(Value::List(l)), Some(item)) => { let mut v = l.clone(); v.push(item.clone()); Value::List(v) }
            _ => Value::Null,
        },
        "pop" => match args.first() {
            Some(Value::List(l)) if !l.is_empty() => { let mut v = l.clone(); v.pop(); Value::List(v) }
            _ => Value::Null,
        },
        "head" => match args.first() {
            Some(Value::List(l)) => l.first().cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        },
        "tail" => match args.first() {
            Some(Value::List(l)) if l.len() > 1 => Value::List(l[1..].to_vec()),
            Some(Value::List(_)) => Value::List(vec![]),
            _ => Value::Null,
        },
        "last" => match args.first() {
            Some(Value::List(l)) => l.last().cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        },
        "reverse" => match args.first() {
            Some(Value::List(l)) => { let mut v = l.clone(); v.reverse(); Value::List(v) }
            Some(Value::String(s)) => Value::String(s.chars().rev().collect()),
            _ => Value::Null,
        },
        "sort" => match args.first() {
            Some(Value::List(l)) => {
                let mut sorted = l.clone();
                sorted.sort_by(|a, b| match (a, b) {
                    (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::String(x), Value::String(y)) => x.cmp(y),
                    _ => std::cmp::Ordering::Equal,
                });
                Value::List(sorted)
            }
            _ => Value::Null,
        },
        "flatten" => match args.first() {
            Some(Value::List(l)) => {
                let mut flat = Vec::new();
                for item in l { match item { Value::List(inner) => flat.extend(inner.clone()), other => flat.push(other.clone()) } }
                Value::List(flat)
            }
            _ => Value::Null,
        },
        "range" => match (args.first(), args.get(1), args.get(2)) {
            (Some(Value::Number(start)), Some(Value::Number(end)), Some(Value::Number(step))) => {
                let mut result = Vec::new();
                let mut i = *start;
                while i < *end { result.push(Value::Number(i)); i += step; }
                Value::List(result)
            }
            (Some(Value::Number(start)), Some(Value::Number(end)), None) => {
                let mut result = Vec::new();
                let (mut i, end) = (*start as i64, *end as i64);
                while i < end { result.push(Value::Number(i as f64)); i += 1; }
                Value::List(result)
            }
            (Some(Value::Number(end)), None, None) => {
                Value::List((0..(*end as i64)).map(|i| Value::Number(i as f64)).collect())
            }
            _ => Value::Null,
        },
        "zip" => match (args.first(), args.get(1)) {
            (Some(Value::List(a)), Some(Value::List(b))) =>
                Value::List(a.iter().zip(b.iter()).map(|(x, y)| Value::List(vec![x.clone(), y.clone()])).collect()),
            _ => Value::Null,
        },

        // ── Map operations ──
        "keys" => match args.first() {
            Some(Value::Map(e)) => Value::List(e.iter().map(|(k, _)| Value::String(k.clone())).collect()),
            _ => Value::Null,
        },
        "values" => match args.first() {
            Some(Value::Map(e)) => Value::List(e.iter().map(|(_, v)| v.clone()).collect()),
            _ => Value::Null,
        },
        "has_key" => match (args.first(), args.get(1)) {
            (Some(Value::Map(e)), Some(Value::String(k))) => Value::Bool(e.iter().any(|(ek, _)| ek == k)),
            _ => Value::Bool(false),
        },
        "map_set" => match (args.first(), args.get(1), args.get(2)) {
            (Some(Value::Map(e)), Some(Value::String(k)), Some(v)) => {
                let mut new: Vec<(String, Value)> = e.iter().filter(|(ek, _)| ek != k).cloned().collect();
                new.push((k.clone(), v.clone()));
                Value::Map(new)
            }
            _ => Value::Null,
        },
        "map_get" => match (args.first(), args.get(1)) {
            (Some(Value::Map(e)), Some(Value::String(k))) => e.iter().find(|(ek, _)| ek == k).map(|(_, v)| v.clone()).unwrap_or(Value::Null),
            _ => Value::Null,
        },
        "map_remove" => match (args.first(), args.get(1)) {
            (Some(Value::Map(e)), Some(Value::String(k))) => Value::Map(e.iter().filter(|(ek, _)| ek != k).cloned().collect()),
            _ => Value::Null,
        },
        "map_merge" => match (args.first(), args.get(1)) {
            (Some(Value::Map(a)), Some(Value::Map(b))) => {
                let mut merged = a.clone();
                for (k, v) in b {
                    if let Some(pos) = merged.iter().position(|(mk, _)| mk == k) {
                        merged[pos] = (k.clone(), v.clone());
                    } else {
                        merged.push((k.clone(), v.clone()));
                    }
                }
                Value::Map(merged)
            }
            _ => Value::Null,
        },

        // ── Type operations ──
        "to_string" => match args.first() { Some(val) => Value::String(format!("{}", val)), None => Value::Null },
        "to_number" => match args.first() {
            Some(Value::String(s)) => s.parse::<f64>().map(Value::Number).unwrap_or(Value::Null),
            Some(Value::Number(n)) => Value::Number(*n),
            Some(Value::Bool(b)) => Value::Number(if *b { 1.0 } else { 0.0 }),
            _ => Value::Null,
        },
        "to_bool" => match args.first() {
            Some(Value::Bool(b)) => Value::Bool(*b),
            Some(Value::Number(n)) => Value::Bool(*n != 0.0),
            Some(Value::String(s)) => Value::Bool(!s.is_empty()),
            Some(Value::Null) => Value::Bool(false),
            Some(Value::List(l)) => Value::Bool(!l.is_empty()),
            _ => Value::Bool(false),
        },
        "type_of" => match args.first() {
            Some(Value::String(_)) => Value::String("string".into()),
            Some(Value::Number(_)) => Value::String("number".into()),
            Some(Value::Bool(_)) => Value::String("bool".into()),
            Some(Value::List(_)) => Value::String("list".into()),
            Some(Value::Map(_)) => Value::String("map".into()),
            Some(Value::Agent(_)) => Value::String("agent".into()),
            Some(Value::History(_)) => Value::String("history".into()),
            Some(Value::Function { .. }) => Value::String("function".into()),
            Some(Value::RecordConstructor { .. }) => Value::String("record".into()),
            Some(Value::Code(_)) => Value::String("code".into()),
            Some(Value::Null) => Value::String("null".into()),
            Some(Value::Error { .. }) => Value::String("error".into()),
            Some(Value::Break) | Some(Value::Continue) | Some(Value::Return(_)) => Value::String("null".into()),
            None => Value::Null,
        },
        "is_null" => match args.first() {
            Some(Value::Null) => Value::Bool(true),
            Some(_) => Value::Bool(false),
            None => Value::Bool(true),
        },

        // ── Error operations ──
        "error" => match (args.first(), args.get(1)) {
            (Some(Value::String(kind)), Some(Value::String(msg))) => {
                Value::Error { kind: kind.clone(), message: msg.clone() }
            }
            (Some(Value::String(msg)), None) => {
                Value::Error { kind: "error".into(), message: msg.clone() }
            }
            _ => Value::Error { kind: "error".into(), message: "unknown error".into() },
        },
        "is_error" => match args.first() {
            Some(Value::Error { .. }) => Value::Bool(true),
            Some(_) => Value::Bool(false),
            None => Value::Bool(false),
        },
        "error_kind" => match args.first() {
            Some(Value::Error { kind, .. }) => Value::String(kind.clone()),
            _ => Value::Null,
        },
        "error_message" => match args.first() {
            Some(Value::Error { message, .. }) => Value::String(message.clone()),
            _ => Value::Null,
        },

        // ── I/O operations ──
        "print" => {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 { print!(" "); }
                match arg {
                    Value::String(s) => print!("{}", s),
                    other => print!("{}", other),
                }
            }
            println!();
            Value::Null
        },
        "format" => {
            if let Some(Value::String(template)) = args.first() {
                let mut result = template.clone();
                for arg in &args[1..] {
                    if let Some(pos) = result.find("{}") {
                        let replacement = match arg { Value::String(s) => s.clone(), other => format!("{}", other) };
                        result = format!("{}{}{}", &result[..pos], replacement, &result[pos + 2..]);
                    }
                }
                Value::String(result)
            } else {
                Value::Null
            }
        },
        "timestamp" => {
            Value::Number(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0))
        },
        "sleep" => {
            match args.first() {
                Some(Value::Number(ms)) => {
                    let millis = (*ms).max(0.0) as u64;
                    std::thread::sleep(std::time::Duration::from_millis(millis));
                    Value::Null
                }
                _ => Value::Null,
            }
        },

        // ── File I/O operations ──
        "file_read" => match args.first() {
            Some(Value::String(path)) => {
                match std::fs::read_to_string(path) {
                    Ok(content) => Value::String(content),
                    Err(e) => Value::Error { kind: "io".into(), message: format!("{}", e) },
                }
            }
            _ => Value::Error { kind: "type".into(), message: "file_read expects a string path".into() },
        },
        "file_write" => match (args.first(), args.get(1)) {
            (Some(Value::String(path)), Some(Value::String(content))) => {
                match std::fs::write(path, content) {
                    Ok(()) => Value::Bool(true),
                    Err(e) => Value::Error { kind: "io".into(), message: format!("{}", e) },
                }
            }
            _ => Value::Error { kind: "type".into(), message: "file_write expects (path, content)".into() },
        },
        "file_append" => match (args.first(), args.get(1)) {
            (Some(Value::String(path)), Some(Value::String(content))) => {
                use std::io::Write;
                match std::fs::OpenOptions::new().create(true).append(true).open(path) {
                    Ok(mut file) => match file.write_all(content.as_bytes()) {
                        Ok(()) => Value::Bool(true),
                        Err(e) => Value::Error { kind: "io".into(), message: format!("{}", e) },
                    },
                    Err(e) => Value::Error { kind: "io".into(), message: format!("{}", e) },
                }
            }
            _ => Value::Error { kind: "type".into(), message: "file_append expects (path, content)".into() },
        },
        "file_exists" => match args.first() {
            Some(Value::String(path)) => Value::Bool(std::path::Path::new(path).exists()),
            _ => Value::Bool(false),
        },
        "file_lines" => match args.first() {
            Some(Value::String(path)) => {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let lines: Vec<Value> = content.lines()
                            .map(|l| Value::String(l.to_string()))
                            .collect();
                        Value::List(lines)
                    }
                    Err(e) => Value::Error { kind: "io".into(), message: format!("{}", e) },
                }
            }
            _ => Value::Error { kind: "type".into(), message: "file_lines expects a string path".into() },
        },

        // Unknown function
        _ => Value::Null,
    }
}

// ─── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use anwe_parser::{Lexer, Parser};

    fn parse_and_run_concurrent(source: &str) -> Result<(), EngineError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        let mut engine = ConcurrentEngine::new();
        engine.execute(&program)
    }

    #[test]
    fn concurrent_empty_program() {
        let result = parse_and_run_concurrent("");
        assert!(result.is_ok());
    }

    #[test]
    fn concurrent_single_link() {
        let result = parse_and_run_concurrent(r#"
            agent Mikel
            agent Primordia

            link Mikel <-> Primordia {
                >> { quality: attending, priority: 0.92 }
                   "I don't know what you're becoming"

                connect depth full {
                    signal attending 0.7 between
                    signal questioning 0.8 between data "what are we building"
                }

                Mikel ~ Primordia until synchronized

                => when sync_level > 0.7 depth genuine {
                    understanding <- "intelligence is attending"
                }

                * from apply {
                    paradigm: "attention-native"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn concurrent_multi_link() {
        let result = parse_and_run_concurrent(r#"
            agent Alpha
            agent Beta
            agent Gamma
            agent Delta

            link Alpha <-> Beta {
                >> "alpha-beta connection"
                connect depth full {
                    signal attending 0.8 between
                }
                Alpha ~ Beta until synchronized
                => when sync_level > 0.7 {
                    bond <- "first pair"
                }
                * from apply {
                    pair: "alpha-beta"
                }
            }

            link Gamma <-> Delta {
                >> "gamma-delta connection"
                connect depth deep {
                    signal questioning 0.9 between
                }
                Gamma ~ Delta until resonating
                => when sync_level > 0.9 depth deep {
                    bond <- "second pair"
                }
                * from apply {
                    pair: "gamma-delta"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn concurrent_shared_agent() {
        // Agent B participates in two links simultaneously
        let result = parse_and_run_concurrent(r#"
            agent A
            agent B
            agent C

            link A <-> B {
                >> "A connects to B"
                connect depth surface {
                    signal attending 0.6 between
                }
                A ~ B until synchronized
                * from apply
            }

            link B <-> C {
                >> "B connects to C"
                connect depth surface {
                    signal attending 0.7 between
                }
                B ~ C until synchronized
                * from apply
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn concurrent_four_links() {
        let result = parse_and_run_concurrent(r#"
            agent N1
            agent N2
            agent N3
            agent N4
            agent N5
            agent N6
            agent N7
            agent N8

            link N1 <-> N2 {
                >> "link 1"
                connect depth surface { signal attending 0.5 between }
                N1 ~ N2 until synchronized
                * from apply
            }

            link N3 <-> N4 {
                >> "link 2"
                connect depth surface { signal attending 0.6 between }
                N3 ~ N4 until synchronized
                * from apply
            }

            link N5 <-> N6 {
                >> "link 3"
                connect depth surface { signal attending 0.7 between }
                N5 ~ N6 until synchronized
                * from apply
            }

            link N7 <-> N8 {
                >> "link 4"
                connect depth surface { signal attending 0.8 between }
                N7 ~ N8 until synchronized
                * from apply
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn concurrent_self_link() {
        let result = parse_and_run_concurrent(r#"
            agent Self
            link Self <-> Self {
                >> "self-reflection"
                connect depth deep {
                    signal attending 0.9 between
                }
                Self ~ Self until synchronized
                * from apply
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn concurrent_unknown_agent_error() {
        let result = parse_and_run_concurrent("agent A\nlink A <-> B { }");
        assert!(matches!(result, Err(EngineError::ExecutionError(_))));
    }
}
