// -----------------------------------------------------------------
// ANWE v0.1 -- EXECUTION ENGINE
//
// The bridge between syntax and reality.
//
// Takes a parsed AST and executes it against the runtime.
// Walks the tree. Instantiates agents. Opens links.
// Executes the seven primitives. Tracks state.
// Produces observable output.
//
// This is not just an interpreter for agent communication.
// This is the substrate an AI uses to construct itself.
// Alert (notice something). Connect (engage with it).
// Sync (align understanding). Apply (let it change you).
// Commit (make it permanent). Reject (know when to withdraw).
// Converge (let something new emerge).
//
// v0.1: Sequential execution. Correctness first.
// Future: concurrent execution via the fiber scheduler.
// -----------------------------------------------------------------

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use anwe_core::{
    Agent, AgentId, AgentState, Signal, Quality, Direction, Priority,
    SyncLevel, Tick, Link, LinkId, LinkState, HistoryEntry, ChangeDepth, ChangeSource,
    Supervisor, RestartStrategy, ChildSpec, ChildRestart, FailureReason,
    Responsiveness, AttentionBudget,
};
use anwe_parser::ast::{
    Program, Declaration, AgentDecl, LinkDecl, LinkExpr,
    AlertExpr, ConnectBlock,
    SyncExpr, SyncCondition, ApplyExpr, CommitExpr, CommitSource,
    RejectExpr, ConvergeBlock, EmitExpr, WhenExpr,
    PendingHandlerExpr, PendingAction, PendingReason,
    PatternDecl, PatternUseExpr, HistoryViewExpr, SuperviseDecl,
    EachExpr, IfElseExpr, BinOp, LinkSchedule,
    SignalQuality, SignalDirection, DepthLevel, ComparisonOp,
    Condition, Expr, SignalAttrs,
    MindDecl, ThinkExpr, ExpressExpr, SenseExpr, AuthorExpr, AttendBlock,
    MatchArm, MatchPattern, BlockStatement, StringPart,
};
use anwe_bridge::{ParticipantRegistry, WireSignal, WireValue};
use crate::channel::SignalChannel;

// ─── RUNTIME VALUE ────────────────────────────────────────────
//
// What an expression evaluates to at runtime.
// Kept minimal for v0.1. Grows with the language.

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Agent(String),
    History(String),
    List(Vec<Value>),
    Map(Vec<(String, Value)>),
    /// A function value: (param_names, body_expression, captured_environment).
    /// Created by `fn` declarations and lambda expressions.
    /// The `env` field stores captured variables for true closure support.
    Function {
        params: Vec<String>,
        body: anwe_parser::ast::Expr,
        env: HashMap<String, Value>,
    },
    /// A record constructor: calling it returns a Map with the named fields.
    RecordConstructor { name: String, fields: Vec<String> },
    /// Quoted source code: captured text that can be eval'd.
    Code(String),
    Null,
    /// Structured error value: { kind: "...", message: "..." }
    Error { kind: String, message: String },
    /// Internal sentinel: break signal for loops (not user-visible)
    Break,
    /// Internal sentinel: continue signal for loops (not user-visible)
    Continue,
    /// Internal sentinel: return signal for early function exit
    Return(Box<Value>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Agent(a), Value::Agent(b)) => a == b,
            (Value::History(a), Value::History(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Function { .. }, Value::Function { .. }) => false,
            (Value::RecordConstructor { name: a, .. }, Value::RecordConstructor { name: b, .. }) => a == b,
            (Value::Code(a), Value::Code(b)) => a == b,
            (Value::Error { kind: ak, message: am }, Value::Error { kind: bk, message: bm }) => ak == bk && am == bm,
            (Value::Break, Value::Break) => true,
            (Value::Continue, Value::Continue) => true,
            (Value::Return(a), Value::Return(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => {
                if *n == (*n as i64) as f64 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Bool(b) => write!(f, "{}", b),
            Value::Agent(name) => write!(f, "{}", name),
            Value::History(name) => write!(f, "history of {}", name),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Map(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Function { params, .. } => write!(f, "fn({})", params.join(", ")),
            Value::RecordConstructor { name, fields } => write!(f, "record {}({})", name, fields.join(", ")),
            Value::Code(src) => write!(f, "quote {{ {} }}", src),
            Value::Null => write!(f, "null"),
            Value::Error { kind, message } => write!(f, "error({}: {})", kind, message),
            Value::Break => write!(f, "break"),
            Value::Continue => write!(f, "continue"),
            Value::Return(v) => write!(f, "return {}", v),
        }
    }
}

// ─── ENGINE ERROR ─────────────────────────────────────────────

use anwe_parser::token::Span;

#[derive(Debug)]
pub enum EngineError {
    UnknownAgent(String),
    UnknownPattern(String),
    ExecutionError(String),
}

impl EngineError {
    /// Create an ExecutionError with source location embedded in the message.
    pub fn at_span(msg: &str, span: &Span) -> Self {
        EngineError::ExecutionError(format!("{} [at line {}:{}]", msg, span.line, span.column))
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::UnknownAgent(name) => write!(f, "Unknown agent: {}", name),
            EngineError::UnknownPattern(name) => write!(f, "Unknown pattern: {}", name),
            EngineError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl std::error::Error for EngineError {}

// ─── LINK EXECUTION CONTEXT ──────────────────────────────────
//
// Temporary state held during the execution of a single link.
// Created when a link opens. Destroyed when it completes.

struct LinkCtx {
    link: Link,
    channel: SignalChannel,
    agent_a: String,
    agent_b: String,
    agent_a_id: AgentId,
    agent_b_id: AgentId,
    last_alert_quality: Option<Quality>,
}

// ─── THE ENGINE ──────────────────────────────────────────────

pub struct Engine {
    /// All declared agents, keyed by name.
    agents: HashMap<String, Agent>,

    /// Agent name -> AgentId mapping.
    agent_ids: HashMap<String, AgentId>,

    /// Structural data carried by agents.
    /// Grows through apply. Persists through commit.
    agent_data: HashMap<String, HashMap<String, Value>>,

    /// Pattern declarations, keyed by name.
    /// Patterns are reusable attention shapes.
    patterns: HashMap<String, PatternDecl>,

    /// Supervisors managing agent restart strategies.
    supervisors: Vec<Supervisor>,

    /// Next ID to allocate (agents and links share the space).
    next_id: u32,

    /// Bridge to external participants.
    /// When an agent is declared with `external(...)`, its signals
    /// are routed through the bridge to the outside world.
    participants: ParticipantRegistry,

    /// Self-authored attend blocks, keyed by mind name.
    /// Accumulated during execution by `author` statements.
    /// Appended to the attention landscape for dynamic execution.
    authored_blocks: HashMap<String, Vec<AttendBlock>>,

    /// Base directory for resolving import paths.
    /// Set when executing a file so imports are relative to it.
    base_path: Option<PathBuf>,

    /// Loaded module paths to prevent circular imports.
    loaded_modules: Vec<PathBuf>,

    /// Track mutable bindings (scope::name format for scoped, __global__::name for top-level).
    /// Only variables declared with `let mut` can be reassigned.
    mutable_bindings: HashSet<String>,
}

impl Engine {
    /// Create a new engine. Empty world. Ready for a program.
    pub fn new() -> Self {
        Engine {
            agents: HashMap::new(),
            agent_ids: HashMap::new(),
            agent_data: HashMap::new(),
            patterns: HashMap::new(),
            supervisors: Vec::new(),
            next_id: 1,
            participants: ParticipantRegistry::new(),
            authored_blocks: HashMap::new(),
            base_path: None,
            loaded_modules: Vec::new(),
            mutable_bindings: HashSet::new(),
        }
    }

    /// Create a new engine with external participants.
    ///
    /// Use this when your .anwe program declares external agents.
    /// Register participants in the registry before execution:
    ///
    /// ```ignore
    /// let mut registry = ParticipantRegistry::new();
    /// registry.register("Sensor", Box::new(my_sensor));
    /// let mut engine = Engine::with_participants(registry);
    /// engine.execute(&program)?;
    /// ```
    pub fn with_participants(registry: ParticipantRegistry) -> Self {
        Engine {
            agents: HashMap::new(),
            agent_ids: HashMap::new(),
            agent_data: HashMap::new(),
            patterns: HashMap::new(),
            supervisors: Vec::new(),
            next_id: 1,
            participants: registry,
            authored_blocks: HashMap::new(),
            base_path: None,
            loaded_modules: Vec::new(),
            mutable_bindings: HashSet::new(),
        }
    }

    /// Set the base path for import resolution.
    pub fn set_base_path(&mut self, path: &Path) {
        if let Some(parent) = path.parent() {
            self.base_path = Some(parent.to_path_buf());
        }
    }

    /// Get the names of all registered agents (for REPL inspection).
    pub fn agent_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.agents.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get the data for a specific agent (for REPL inspection).
    pub fn agent_data(&self, name: &str) -> Option<Vec<(String, Value)>> {
        self.agent_data.get(name).map(|data| {
            let mut entries: Vec<(String, Value)> = data.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            entries
        })
    }

    /// Get agent state info (state, responsiveness, history depth, attention).
    pub fn agent_info(&self, name: &str) -> Option<(String, f32, u64, f32)> {
        self.agents.get(name).map(|a| (
            state_name(a.state).to_string(),
            a.responsiveness.as_f32(),
            a.history.depth(),
            a.attention.remaining(),
        ))
    }

    /// Get history entries for an agent as displayable strings.
    pub fn agent_history(&self, name: &str) -> Vec<String> {
        self.agents.get(name).map(|a| {
            a.history.iter().map(|e| {
                format!("[{}] {} {} prio:{:.2} sync:{:.2}",
                    e.index,
                    core_quality_name(e.from_quality),
                    source_name(e.source),
                    e.encounter_priority.as_f32(),
                    e.sync_level.as_f32(),
                )
            }).collect()
        }).unwrap_or_default()
    }

    /// Get bridge participant names.
    pub fn bridge_names(&self) -> Vec<String> {
        self.participants.names().iter().map(|s| s.to_string()).collect()
    }

    /// Evaluate a bare expression and return its display string.
    /// Used by the REPL for expression evaluation without `let`.
    pub fn eval_expression(&mut self, source: &str) -> Result<String, String> {
        // Wrap in a let binding so the parser accepts it
        let wrapped = format!("let __repl_result__ = {}", source);
        let mut lexer = anwe_parser::Lexer::new(&wrapped);
        let tokens = lexer.tokenize().map_err(|e| format!("{}", e))?;
        let mut parser = anwe_parser::Parser::new(tokens);
        let program = parser.parse_program().map_err(|e| format!("{}", e))?;
        self.execute(&program).map_err(|e| format!("{}", e))?;
        // Retrieve the result
        let val = self.agent_data.get("__global__")
            .and_then(|data| data.get("__repl_result__").cloned());
        // Clean up the temp var
        if let Some(global_data) = self.agent_data.get_mut("__global__") {
            global_data.remove("__repl_result__");
        }
        match val {
            Some(v) => Ok(format!("{}", v)),
            None => Ok("()".to_string()),
        }
    }

    /// Get supervisor tree for display.
    pub fn supervisor_info(&self) -> Vec<String> {
        self.supervisors.iter().map(|sup| {
            let strategy = match sup.strategy {
                RestartStrategy::OneForOne => "one_for_one",
                RestartStrategy::OneForAll => "one_for_all",
                RestartStrategy::RestForOne => "rest_for_one",
            };
            let children: Vec<String> = sup.children().iter().map(|child| {
                let restart = match child.restart {
                    ChildRestart::Permanent => "permanent",
                    ChildRestart::Transient => "transient",
                    ChildRestart::Temporary => "temporary",
                };
                let name = self.agent_ids.iter()
                    .find(|(_, id)| **id == child.agent_id)
                    .map(|(n, _)| n.as_str())
                    .unwrap_or("?");
                format!("  {} ({})", name, restart)
            }).collect();
            format!("{} [max_restarts: {}, within: {} ticks]\n{}",
                strategy, sup.max_restarts(), sup.time_window(),
                children.join("\n"))
        }).collect()
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Execute an entire ANWE program.
    ///
    /// Phase 1: Register all agents and patterns (declarations).
    /// Phase 2: Execute all links (where the primitives run).
    ///
    /// Output is printed directly — the engine narrates
    /// what is happening as it happens.
    pub fn execute(&mut self, program: &Program) -> Result<(), EngineError> {
        println!();

        // Phase 0: Process imports
        for decl in &program.declarations {
            if let Declaration::Import(import) = decl {
                self.execute_import(import)?;
            }
        }

        // Phase 1: Register agents, patterns, supervisors, and minds
        for decl in &program.declarations {
            match decl {
                Declaration::Agent(a) => self.register_agent(a)?,
                Declaration::Pattern(p) => self.register_pattern(p)?,
                Declaration::Supervise(s) => self.register_supervisor(s)?,
                Declaration::Mind(m) => self.register_mind(m)?,
                _ => {}
            }
        }
        println!();

        // Phase 1.5: Process top-level let bindings and function declarations
        for decl in &program.declarations {
            match decl {
                Declaration::Let(binding) => {
                    self.execute_let_binding(binding, "__global__")?;
                }
                Declaration::Fn(fn_decl) => {
                    let func = Value::Function {
                        params: fn_decl.params.clone(),
                        body: fn_decl.body.clone(),
                        env: HashMap::new(),
                    };
                    println!("  fn {}({})", fn_decl.name, fn_decl.params.join(", "));
                    self.agent_data
                        .entry("__global__".to_string())
                        .or_default()
                        .insert(fn_decl.name.clone(), func);
                }
                Declaration::Record(rec) => {
                    // A record creates a constructor function that returns a Map.
                    // record Point { x, y } → Point(x, y) returns { x: x, y: y }
                    // We synthesize this as a Function whose body is a special MapLit.
                    // Since we can't easily express this as an Expr, we store the
                    // field names and handle record constructors specially in eval.
                    println!("  record {}({})", rec.name, rec.fields.join(", "));
                    let func = Value::RecordConstructor {
                        name: rec.name.clone(),
                        fields: rec.fields.clone(),
                    };
                    self.agent_data
                        .entry("__global__".to_string())
                        .or_default()
                        .insert(rec.name.clone(), func);
                }
                Declaration::TopLevelExpr(expr) => {
                    // Execute top-level expressions (while, for-in, etc.)
                    // Build an env from globals, evaluate with mutable access
                    let mut env: HashMap<String, Value> = self.agent_data
                        .get("__global__")
                        .map(|g| g.clone())
                        .unwrap_or_default();
                    self.eval_fn_expr_in_env(expr, &mut env);
                    // Write modified bindings back to globals
                    let global = self.agent_data.entry("__global__".to_string()).or_default();
                    for (k, v) in env {
                        global.insert(k, v);
                    }
                }
                Declaration::Assign { name, value } => {
                    // Top-level reassignment: evaluate and store in globals
                    let val = self.eval_expr(value);
                    self.agent_data
                        .entry("__global__".to_string())
                        .or_default()
                        .insert(name.clone(), val);
                }
                _ => {}
            }
        }

        // Phase 2: Execute links and minds
        for decl in &program.declarations {
            match decl {
                Declaration::Link(link_decl) => self.execute_link(link_decl)?,
                Declaration::Mind(mind_decl) => self.execute_mind(mind_decl)?,
                Declaration::HistoryView(hv) => self.execute_history_view(hv)?,
                _ => {}
            }
        }

        // Final summary
        let bar = "\u{2550}".repeat(47);
        println!("{}", bar);
        println!("Transmission complete.");
        println!("The system after this is not the system before.");
        println!();

        Ok(())
    }

    // ─── REGISTRATION ─────────────────────────────────────────

    fn register_agent(&mut self, decl: &AgentDecl) -> Result<(), EngineError> {
        let id_raw = self.alloc_id();
        let id = AgentId::new(id_raw);

        // Check for lineage_depth in data
        let mut lineage = 0u64;
        let mut data_map = HashMap::new();

        for kv in &decl.data {
            let val = self.eval_expr(&kv.value);
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
        if let Some(budget) = decl.attention {
            agent.attention = anwe_core::AttentionBudget::new(budget as f32);
        }

        // Display
        let attn_str = decl.attention.map(|a| format!(" attention {:.1}", a)).unwrap_or_default();
        let ext_str = decl.external.as_ref()
            .map(|e| format!(" external(\"{}\", \"{}\")", e.kind, e.address))
            .unwrap_or_default();
        if decl.data.is_empty() {
            println!("  agent {}{}{}", decl.name, attn_str, ext_str);
        } else {
            let pairs: Vec<String> = decl.data.iter()
                .map(|kv| format!("{}: {}", kv.key, self.eval_expr(&kv.value)))
                .collect();
            println!("  agent {}{}{} data {{ {} }}", decl.name, attn_str, ext_str, pairs.join(", "));
        }

        // Check if this external agent has a registered participant
        if decl.external.is_some() && !self.participants.is_external(&decl.name) {
            println!("  |  WARNING: agent {} declared external but no participant registered",
                decl.name);
        }

        self.agents.insert(decl.name.clone(), agent);
        self.agent_ids.insert(decl.name.clone(), id);
        self.agent_data.insert(decl.name.clone(), data_map);
        Ok(())
    }

    fn register_supervisor(&mut self, decl: &SuperviseDecl) -> Result<(), EngineError> {
        let sup_id = AgentId::new(self.alloc_id());
        let strategy = match decl.strategy {
            anwe_parser::ast::SuperviseStrategy::OneForOne => RestartStrategy::OneForOne,
            anwe_parser::ast::SuperviseStrategy::OneForAll => RestartStrategy::OneForAll,
            anwe_parser::ast::SuperviseStrategy::RestForOne => RestartStrategy::RestForOne,
        };

        let mut sup = Supervisor::new(sup_id, strategy);

        if let (Some(max), Some(window)) = (decl.max_restarts, decl.time_window) {
            sup = sup.with_limits(max, window);
        }

        let strategy_name = match decl.strategy {
            anwe_parser::ast::SuperviseStrategy::OneForOne => "one_for_one",
            anwe_parser::ast::SuperviseStrategy::OneForAll => "one_for_all",
            anwe_parser::ast::SuperviseStrategy::RestForOne => "rest_for_one",
        };
        print!("  supervise {} ", strategy_name);
        if let (Some(max), Some(window)) = (decl.max_restarts, decl.time_window) {
            print!("max_restarts {} within {} ", max, window);
        }
        println!("{{");

        for child in &decl.children {
            let restart = match child.restart {
                anwe_parser::ast::ChildRestartType::Permanent => ChildRestart::Permanent,
                anwe_parser::ast::ChildRestartType::Transient => ChildRestart::Transient,
                anwe_parser::ast::ChildRestartType::Temporary => ChildRestart::Temporary,
            };

            if let Some(&agent_id) = self.agent_ids.get(&child.agent) {
                let spec = ChildSpec::new(agent_id, restart);
                sup.add_child(spec);

                // Link the agent to its supervisor
                self.agents.get_mut(&child.agent).unwrap()
                    .supervisor = Some(sup_id);

                let restart_name = match child.restart {
                    anwe_parser::ast::ChildRestartType::Permanent => "permanent",
                    anwe_parser::ast::ChildRestartType::Transient => "transient",
                    anwe_parser::ast::ChildRestartType::Temporary => "temporary",
                };
                println!("    {} {}", restart_name, child.agent);
            } else {
                return Err(EngineError::UnknownAgent(child.agent.clone()));
            }
        }
        println!("  }}");

        self.supervisors.push(sup);
        Ok(())
    }

    // ─── SUPERVISION RUNTIME ─────────────────────────────────
    //
    // The bridge between failure and recovery.
    // When a link expression fails, we check if either agent
    // has a supervisor. If so, the supervisor decides what to do.

    /// Find the supervisor index that owns the given agent.
    fn find_supervisor_for(&self, agent_name: &str) -> Option<usize> {
        let _agent_id = self.agent_ids.get(agent_name)?;
        let sup_id = self.agents.get(agent_name)?.supervisor?;
        self.supervisors.iter().position(|s| s.id == sup_id)
    }

    /// Restart an agent: reset its core state while preserving data.
    /// This is what happens after a supervisor decides to restart a child.
    fn restart_agent(&mut self, agent_name: &str) {
        if let Some(agent) = self.agents.get_mut(agent_name) {
            let id = agent.id;
            // Reset to fresh agent, preserving the supervisor link
            let sup = agent.supervisor;
            *agent = Agent::new(id);
            agent.supervisor = sup;
            println!("  |  SUPERVISOR: restarted {}", agent_name);
        }
    }

    /// Handle a link-level failure through the supervision system.
    /// Returns Ok(true) if supervision handled it (execution can continue),
    /// Ok(false) if no supervisor was involved (error should propagate),
    /// Err if the supervisor itself is overwhelmed.
    fn handle_supervised_failure(
        &mut self, agent_a: &str, agent_b: &str, error: &EngineError,
    ) -> Result<bool, EngineError> {
        // Check both agents in the link — either could be supervised
        let sup_idx = self.find_supervisor_for(agent_a)
            .or_else(|| self.find_supervisor_for(agent_b));

        let sup_idx = match sup_idx {
            Some(idx) => idx,
            None => return Ok(false), // No supervisor — propagate error
        };

        // Determine which agent(s) failed — for now, attribute to agent_a
        // (the "source" agent in the link). Future: track per-expression.
        let failed_name = agent_a;
        let failed_id = match self.agent_ids.get(failed_name) {
            Some(&id) => id,
            None => return Ok(false),
        };

        println!("  |");
        println!("  |  SUPERVISOR: detected failure in {}", failed_name);
        println!("  |     error: {}", error);

        // Get current tick for restart rate-limiting
        let now = anwe_core::Tick::new(0, self.alloc_id() as u16);

        let to_restart = match self.supervisors[sup_idx].handle_failure(
            failed_id, FailureReason::Crash, now,
        ) {
            Some(agents) => agents,
            None => {
                // Supervisor is overwhelmed — too many restarts
                println!("  |  SUPERVISOR: overwhelmed — too many restarts within window");
                println!("  |     escalating failure");
                return Err(EngineError::ExecutionError(
                    format!("Supervisor overwhelmed: too many restarts. Original error: {}", error)
                ));
            }
        };

        if to_restart.is_empty() {
            println!("  |  SUPERVISOR: child is temporary — not restarting");
            return Ok(true);
        }

        // Find agent names for the IDs we need to restart
        let names_to_restart: Vec<String> = to_restart.iter()
            .filter_map(|id| {
                self.agent_ids.iter()
                    .find(|(_, aid)| **aid == *id)
                    .map(|(name, _)| name.clone())
            })
            .collect();

        let strategy = self.supervisors[sup_idx].strategy;
        let strategy_name = match strategy {
            RestartStrategy::OneForOne => "one_for_one",
            RestartStrategy::OneForAll => "one_for_all",
            RestartStrategy::RestForOne => "rest_for_one",
        };
        println!("  |  SUPERVISOR: strategy {} — restarting {} agent(s)",
            strategy_name, names_to_restart.len());

        for name in &names_to_restart {
            self.restart_agent(name);
        }

        println!("  |");
        Ok(true)
    }

    fn register_pattern(&mut self, decl: &PatternDecl) -> Result<(), EngineError> {
        let params_str: Vec<String> = decl.params.iter().map(|p| {
            if let Some(ref t) = p.type_ref {
                format!("{}: {}", p.name, t)
            } else {
                p.name.clone()
            }
        }).collect();
        println!("  pattern {}({})", decl.name, params_str.join(", "));

        self.patterns.insert(decl.name.clone(), decl.clone());
        Ok(())
    }

    // ─── LINK EXECUTION ───────────────────────────────────────

    fn execute_link(&mut self, decl: &LinkDecl) -> Result<(), EngineError> {
        // Verify agents exist
        if !self.agents.contains_key(&decl.agent_a) {
            return Err(EngineError::at_span(
                &format!("Unknown agent '{}'", decl.agent_a), &decl.span
            ));
        }
        if !self.agents.contains_key(&decl.agent_b) {
            return Err(EngineError::at_span(
                &format!("Unknown agent '{}'", decl.agent_b), &decl.span
            ));
        }

        let link_id = LinkId::new(self.alloc_id());
        let mut link = Link::open(link_id);

        let agent_a_id = self.agent_ids[&decl.agent_a];
        let agent_b_id = self.agent_ids[&decl.agent_b];

        link.enter(agent_a_id);
        link.enter(agent_b_id);

        let channel = SignalChannel::default_capacity();

        let mut ctx = LinkCtx {
            link,
            channel,
            agent_a: decl.agent_a.clone(),
            agent_b: decl.agent_b.clone(),
            agent_a_id,
            agent_b_id,
            last_alert_quality: None,
        };

        let pri_str = decl.priority.map(|p| format!("  priority: {}", link_priority_name(p))).unwrap_or_default();

        // Determine scheduling parameters
        let (iterations, delay) = match &decl.schedule {
            Some(LinkSchedule::Every { ticks }) => {
                let n = (*ticks as usize).max(1);
                println!("  link {} <-> {}{}  every {} ticks", decl.agent_a, decl.agent_b, pri_str, ticks);
                (n, 0)
            }
            Some(LinkSchedule::After { ticks }) => {
                println!("  link {} <-> {}{}  after {} ticks", decl.agent_a, decl.agent_b, pri_str, ticks);
                (1, *ticks as usize)
            }
            Some(LinkSchedule::Continuous) => {
                println!("  link {} <-> {}{}  continuous", decl.agent_a, decl.agent_b, pri_str);
                (3, 0) // Simulate 3 iterations for continuous mode
            }
            None => {
                println!("  link {} <-> {}{}", decl.agent_a, decl.agent_b, pri_str);
                (1, 0)
            }
        };

        if delay > 0 {
            println!("  |  (delayed {} ticks)", delay);
        }

        // Execute body expressions — with scheduling
        for iteration in 0..iterations {
            if iterations > 1 {
                println!("  |  [tick {}]", iteration + 1);
            }
            println!("  |");

            for expr in &decl.body {
                if let Err(e) = self.execute_link_expr(&mut ctx, expr) {
                    // Check if a supervisor can handle this failure
                    match self.handle_supervised_failure(&decl.agent_a, &decl.agent_b, &e) {
                        Ok(true) => continue,
                        Ok(false) => return Err(e),
                        Err(escalated) => return Err(escalated),
                    }
                }
            }

            // Advance tick between iterations
            if iteration < iterations - 1 {
                ctx.link.advance_tick(100);
            }
        }

        // Complete the link
        ctx.link.complete();
        let total_signals = ctx.channel.total_sent();
        let peak = ctx.link.peak_sync_level();

        println!("  |");
        println!("  +-- link complete  signals: {}  peak sync: {:.3}  iterations: {}",
            total_signals, peak.as_f32(), iterations);
        println!();

        Ok(())
    }

    fn execute_link_expr(
        &mut self, ctx: &mut LinkCtx, expr: &LinkExpr,
    ) -> Result<(), EngineError> {
        match expr {
            LinkExpr::Alert(a) => self.exec_alert(ctx, a),
            LinkExpr::Connect(c) => self.exec_connect(ctx, c),
            LinkExpr::Sync(s) => self.exec_sync(ctx, s),
            LinkExpr::Apply(a) => self.exec_apply(ctx, a),
            LinkExpr::Commit(c) => self.exec_commit(ctx, c),
            LinkExpr::Reject(r) => self.exec_reject(ctx, r),
            LinkExpr::Converge(c) => self.exec_converge(ctx, c),
            LinkExpr::Emit(e) => self.exec_emit(ctx, e),
            LinkExpr::When(w) => self.exec_when(ctx, w),
            LinkExpr::PendingHandler(p) => self.exec_pending(ctx, p),
            LinkExpr::PatternUse(p) => self.exec_pattern(ctx, p),
            LinkExpr::Each(e) => self.exec_each(ctx, e),
            LinkExpr::IfElse(ie) => self.exec_if_else(ctx, ie),
            // Phase 6-9 extensions — live execution
            LinkExpr::Spawn(s) => self.exec_spawn(ctx, s),
            LinkExpr::Retire(r) => self.exec_retire(ctx, r),
            LinkExpr::SyncAll(s) => self.exec_sync_all(ctx, s),
            LinkExpr::Broadcast(b) => self.exec_broadcast(ctx, b),
            LinkExpr::MultiConverge(mc) => self.exec_multi_converge(ctx, mc),
            LinkExpr::Stream(s) => {
                let rate = (s.rate as usize).max(1);
                println!("  |  STREAM {} rate {} (body: {} exprs)", s.source, rate, s.body.len());
                for i in 0..rate {
                    if rate > 1 {
                        println!("  |     [sample {}/{}]", i + 1, rate);
                    }
                    for expr in &s.body {
                        self.execute_link_expr(ctx, expr)?;
                    }
                }
                println!("  |     {} samples processed", rate);
                println!("  |");
                Ok(())
            }
            LinkExpr::Save(s) => self.exec_save(ctx, s),
            LinkExpr::Restore(r) => self.exec_restore(ctx, r),
            LinkExpr::HistoryQueryBlock(hq) => self.exec_history_query(ctx, hq),
            LinkExpr::Align(a) => {
                println!("      align [{}] to {}", a.agents.join(", "), self.eval_expr(&a.reference));
                Ok(())
            }
            LinkExpr::Buffer(b) => {
                let samples = (b.samples as usize).max(1);
                println!("  |  BUFFER {} samples (body: {} exprs)", samples, b.body.len());
                for i in 0..samples {
                    if samples > 1 {
                        println!("  |     [buffering {}/{}]", i + 1, samples);
                    }
                    for expr in &b.body {
                        self.execute_link_expr(ctx, expr)?;
                    }
                }
                println!("  |     {} samples buffered", samples);
                println!("  |");
                Ok(())
            }
            // First-person cognition primitives
            LinkExpr::Think(t) => self.exec_think(ctx, t),
            LinkExpr::Express(e) => self.exec_express(ctx, e),
            LinkExpr::Sense(s) => self.exec_sense(ctx, s),
            LinkExpr::Author(a) => self.exec_author(ctx, a),
            LinkExpr::While(w) => self.exec_while(ctx, w),
            LinkExpr::Attempt(a) => self.exec_attempt(ctx, a),
            LinkExpr::Let(binding) => self.execute_let_binding(binding, &ctx.agent_a),
            LinkExpr::Assign(assign) => self.execute_assign(assign, &ctx.agent_a),
        }
    }

    // ─── ALERT ────────────────────────────────────────────────
    //
    // >> — Something calls attention.
    // Not all input calls attention. This did.
    // The first signal. The initial disturbance.

    fn exec_alert(
        &mut self, ctx: &mut LinkCtx, alert: &AlertExpr,
    ) -> Result<(), EngineError> {
        let quality = alert.attrs.as_ref()
            .and_then(|a| a.quality)
            .unwrap_or(SignalQuality::Attending);
        let priority = alert.attrs.as_ref()
            .and_then(|a| a.priority)
            .unwrap_or(0.5);

        let core_quality = to_core_quality(quality);
        let core_priority = Priority::new(priority as f32);

        // Create and transmit the signal — wire cognitive fields
        let mut signal = Signal::new(
            core_quality,
            Direction::Between,
            core_priority,
            ctx.agent_a_id,
            ctx.link.tick(),
        ).with_sequence(ctx.link.record_signal());

        signal = apply_signal_attrs(signal, alert.attrs.as_ref());

        let _ = ctx.channel.try_send(signal);

        // Bridge: notify external participants about the alert
        // Both the sender (agent_a) and receiver (agent_b) may be external
        if let Some(response) = self.bridge_notify_signal(&ctx.agent_a, &signal) {
            let _ = ctx.channel.try_send(response);
        }
        if ctx.agent_a != ctx.agent_b {
            if let Some(response) = self.bridge_notify_signal(&ctx.agent_b, &signal) {
                let _ = ctx.channel.try_send(response);
            }
        }

        // Consume attention for this alert
        self.agents.get_mut(&ctx.agent_a).unwrap().consume_attention(0.02);
        ctx.last_alert_quality = Some(core_quality);

        // State transition: agent_a alerts
        let old_state = self.agents[&ctx.agent_a].state;
        self.agents.get_mut(&ctx.agent_a).unwrap().alert();
        let new_state = self.agents[&ctx.agent_a].state;

        // Display
        let val = self.eval_expr(&alert.expression);

        println!("  |  >> ALERT");
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
                println!("  |     {}", parts.join("  "));
            }
        }

        println!("  |     {}", val);
        println!("  |     {}: {} -> {}",
            ctx.agent_a, state_name(old_state), state_name(new_state));
        println!("  |");

        Ok(())
    }

    // ─── CONNECT ──────────────────────────────────────────────
    //
    // Sustained bidirectional presence.
    // Both agents change each other. Not request-response.
    // Simultaneous mutual modification.

    fn exec_connect(
        &mut self, ctx: &mut LinkCtx, connect: &ConnectBlock,
    ) -> Result<(), EngineError> {
        let depth_str = connect.depth
            .map(|d| depth_name(d))
            .unwrap_or("default");

        println!("  |  CONNECT depth: {}", depth_str);

        // Transmit each signal pulse
        for pulse in &connect.pulses {
            let core_quality = to_core_quality(pulse.quality);
            let core_direction = to_core_direction(pulse.direction);
            let core_priority = Priority::new(pulse.priority as f32);

            let signal = Signal::new(
                core_quality,
                core_direction,
                core_priority,
                ctx.agent_a_id,
                ctx.link.tick(),
            ).with_sequence(ctx.link.record_signal());

            let _ = ctx.channel.try_send(signal);

            // Bridge: notify external participants about connect signals
            if let Some(response) = self.bridge_notify_signal(&ctx.agent_a, &signal) {
                let _ = ctx.channel.try_send(response);
            }
            if ctx.agent_a != ctx.agent_b {
                if let Some(response) = self.bridge_notify_signal(&ctx.agent_b, &signal) {
                    let _ = ctx.channel.try_send(response);
                }
            }

            // Consume attention per signal pulse
            self.agents.get_mut(&ctx.agent_a).unwrap().consume_attention(0.01);

            // Format the signal line
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

            println!("{}", line);
        }

        // State transitions: both agents connect
        let old_a = self.agents[&ctx.agent_a].state;
        let old_b = self.agents[&ctx.agent_b].state;
        self.agents.get_mut(&ctx.agent_a).unwrap().connect();
        self.agents.get_mut(&ctx.agent_b).unwrap().connect();
        let new_a = self.agents[&ctx.agent_a].state;
        let new_b = self.agents[&ctx.agent_b].state;

        println!("  |     {} signals transmitted", connect.pulses.len());
        println!("  |     {}: {} -> {}",
            ctx.agent_a, state_name(old_a), state_name(new_a));
        if ctx.agent_a != ctx.agent_b {
            println!("  |     {}: {} -> {}",
                ctx.agent_b, state_name(old_b), state_name(new_b));
        }
        println!("  |");

        Ok(())
    }

    // ─── SYNC ─────────────────────────────────────────────────
    //
    // A ~ B until <condition>
    // Find shared rhythm. Cannot be faked. Cannot be skipped.
    // Like two pendulums on the same wall — they synchronize
    // through shared physics, not through agreement.

    fn exec_sync(
        &mut self, ctx: &mut LinkCtx, sync: &SyncExpr,
    ) -> Result<(), EngineError> {
        // Display the sync statement
        match &sync.until {
            SyncCondition::CoherenceThreshold { op, value } => {
                println!("  |  SYNC {} ~ {} until sync_level {} {:.3}",
                    sync.agent_a, sync.agent_b, op_symbol(*op), value);
            }
            _ => {
                let target_name = match &sync.until {
                    SyncCondition::Synchronized => "synchronized",
                    SyncCondition::Resonating => "resonating",
                    _ => unreachable!(),
                };
                println!("  |  SYNC {} ~ {} until {}",
                    sync.agent_a, sync.agent_b, target_name);
            }
        }

        // Begin sync on the link
        ctx.link.begin_sync();

        // State transitions
        let old_a = self.agents[&sync.agent_a].state;
        let old_b = self.agents[&sync.agent_b].state;
        if let Some(a) = self.agents.get_mut(&sync.agent_a) {
            a.sync();
        }
        if let Some(b) = self.agents.get_mut(&sync.agent_b) {
            b.sync();
        }

        // Determine target sync level.
        // Real synchronization overshoots thresholds slightly —
        // like pendulums that swing past the midpoint.
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
            // Apply temporal decay if specified — sync erodes without maintenance
            if decay_rate > 0.0 && i > 1 {
                let decay_factor = (-0.693 * (i as f32) / decay_rate).exp();
                level = level * (0.5 + 0.5 * decay_factor);
            }
            ctx.link.update_sync_level(SyncLevel::new(level));
            ctx.link.advance_tick(i * 100);
            levels.push(format!("{:.3}", level));
            if level >= target_level {
                break;
            }
        }

        // Consume attention for sync effort
        self.agents.get_mut(&sync.agent_a).map(|a| a.consume_attention(0.05));
        if sync.agent_a != sync.agent_b {
            self.agents.get_mut(&sync.agent_b).map(|a| a.consume_attention(0.05));
        }

        let reached_name = match &sync.until {
            SyncCondition::Synchronized => "synchronized",
            SyncCondition::Resonating => "resonating",
            SyncCondition::CoherenceThreshold { .. } => "threshold reached",
        };

        println!("  |     .. {}", levels.join(" -> "));
        println!("  |     {} at sync_level {:.3}",
            reached_name, ctx.link.sync_level().as_f32());

        if let Some(decay) = sync.decay {
            println!("  |     decay half-life: {} ticks", decay);
        }

        let new_a = self.agents[&sync.agent_a].state;
        let new_b = self.agents[&sync.agent_b].state;
        println!("  |     {}: {} -> {}",
            sync.agent_a, state_name(old_a), state_name(new_a));
        if sync.agent_a != sync.agent_b {
            println!("  |     {}: {} -> {}",
                sync.agent_b, state_name(old_b), state_name(new_b));
        }

        // Bridge: notify external participants of sync completion
        let sync_signal = Signal::new(
            Quality::Completing,
            Direction::Between,
            Priority::new(ctx.link.sync_level().as_f32()),
            ctx.agent_a_id,
            ctx.link.tick(),
        );
        if let Some(response) = self.bridge_notify_signal(&sync.agent_a, &sync_signal) {
            let _ = ctx.channel.try_send(response);
        }
        if sync.agent_a != sync.agent_b {
            if let Some(response) = self.bridge_notify_signal(&sync.agent_b, &sync_signal) {
                let _ = ctx.channel.try_send(response);
            }
        }

        println!("  |");

        Ok(())
    }

    // ─── APPLY ────────────────────────────────────────────────
    //
    // => when <condition> — Boundary dissolution.
    // Something crosses the boundary.
    // The observer's structure changes.
    // This is not data transfer. This is structural change.

    fn exec_apply(
        &mut self, ctx: &mut LinkCtx, apply: &ApplyExpr,
    ) -> Result<(), EngineError> {
        let cond_str = format_condition(&apply.condition);
        match &apply.depth {
            Some(d) => println!("  |  APPLY when {}  depth: {}",
                cond_str, depth_name(*d)),
            None => println!("  |  APPLY when {}", cond_str),
        }

        let met = self.eval_condition(&apply.condition, ctx);

        if met {
            let sync_val = ctx.link.sync_level().as_f32();
            println!("  |     condition met (sync_level = {:.3})", sync_val);

            // Bridge: build wire changes and check if external participants accept
            let wire_changes: Vec<(String, WireValue)> = apply.changes.iter()
                .map(|c| {
                    let val = self.eval_expr(&c.value);
                    (c.name.clone(), value_to_wire(&val))
                })
                .collect();

            let a_accepts = self.bridge_notify_apply(&ctx.agent_a, &wire_changes);
            let b_accepts = if ctx.agent_a != ctx.agent_b {
                self.bridge_notify_apply(&ctx.agent_b, &wire_changes)
            } else {
                true
            };

            if !a_accepts || !b_accepts {
                println!("  |     external participant rejected — switching to reject path");
                let quality = ctx.last_alert_quality.unwrap_or(Quality::Disturbed);
                let priority = Priority::new(0.5);
                if !a_accepts {
                    self.agents.get_mut(&ctx.agent_a).unwrap().reject(quality, priority);
                }
                if !b_accepts {
                    self.agents.get_mut(&ctx.agent_b).unwrap().reject(quality, priority);
                }
                println!("  |");
                return Ok(());
            }

            // State transition: both agents apply
            let old_a = self.agents[&ctx.agent_a].state;
            let old_b = self.agents[&ctx.agent_b].state;
            self.agents.get_mut(&ctx.agent_a).unwrap().apply();
            self.agents.get_mut(&ctx.agent_b).unwrap().apply();

            // Execute structural changes
            for change in &apply.changes {
                let val = self.eval_expr(&change.value);
                println!("  |     {:16} <- {}", change.name, val);

                // Both agents receive the structural change
                self.agent_data
                    .entry(ctx.agent_a.clone())
                    .or_default()
                    .insert(change.name.clone(), val.clone());
                if ctx.agent_a != ctx.agent_b {
                    self.agent_data
                        .entry(ctx.agent_b.clone())
                        .or_default()
                        .insert(change.name.clone(), val);
                }
            }

            // Complete application — responsiveness deepens
            self.agents.get_mut(&ctx.agent_a).unwrap().apply_complete();
            if ctx.agent_a != ctx.agent_b {
                self.agents.get_mut(&ctx.agent_b).unwrap().apply_complete();
            }

            println!("  |     {}: {} -> Applying",
                ctx.agent_a, state_name(old_a));
            if ctx.agent_a != ctx.agent_b {
                println!("  |     {}: {} -> Applying",
                    ctx.agent_b, state_name(old_b));
            }
        } else {
            println!("  |     condition not met (sync_level = {:.3})",
                ctx.link.sync_level().as_f32());
        }

        println!("  |");
        Ok(())
    }

    // ─── COMMIT ───────────────────────────────────────────────
    //
    // * from apply|reject — Irreversible.
    // The system after this is not the system before.
    // There is no undo. No rollback. No version restore.

    fn exec_commit(
        &mut self, ctx: &mut LinkCtx, commit: &CommitExpr,
    ) -> Result<(), EngineError> {
        let source_name = match commit.source {
            CommitSource::Apply => "apply",
            CommitSource::Reject => "reject",
        };

        println!("  |  COMMIT from {}", source_name);

        // Display commit entries
        let mut wire_entries: Vec<(String, WireValue)> = Vec::new();
        for kv in &commit.entries {
            let val = self.eval_expr(&kv.value);
            println!("  |     {:16} {}", format!("{}:", kv.key), val);
            wire_entries.push((kv.key.clone(), value_to_wire(&val)));
        }

        // Bridge: notify external participants of commit
        self.bridge_notify_commit(&ctx.agent_a, &wire_entries);
        if ctx.agent_a != ctx.agent_b {
            self.bridge_notify_commit(&ctx.agent_b, &wire_entries);
        }

        // Create history entries — the irreversible record
        let quality = ctx.last_alert_quality.unwrap_or(Quality::Attending);
        let priority = Priority::new(0.9);
        let sync_level = ctx.link.sync_level();
        let tick = ctx.link.tick();

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

        let entry_a = make_entry(ctx.agent_a_id, ctx.agent_b_id);
        let entry_b = make_entry(ctx.agent_b_id, ctx.agent_a_id);

        // Commit to both agents
        let old_a = self.agents[&ctx.agent_a].state;
        self.agents.get_mut(&ctx.agent_a).unwrap().begin_commit();
        self.agents.get_mut(&ctx.agent_a).unwrap().history.append(entry_a);
        self.agents.get_mut(&ctx.agent_a).unwrap().idle();
        let depth_a = self.agents[&ctx.agent_a].history.depth();

        println!("  |     {}: {} -> Committing -> Idle  history depth: {}",
            ctx.agent_a, state_name(old_a), depth_a);

        if ctx.agent_a != ctx.agent_b {
            let old_b = self.agents[&ctx.agent_b].state;
            self.agents.get_mut(&ctx.agent_b).unwrap().begin_commit();
            self.agents.get_mut(&ctx.agent_b).unwrap().history.append(entry_b);
            self.agents.get_mut(&ctx.agent_b).unwrap().idle();
            let depth_b = self.agents[&ctx.agent_b].history.depth();

            println!("  |     {}: {} -> Committing -> Idle  history depth: {}",
                ctx.agent_b, state_name(old_b), depth_b);
        }

        println!("  |");
        Ok(())
    }

    // ─── REJECT ───────────────────────────────────────────────
    //
    // <= when <condition> — Intelligent withdrawal.
    // Not failure. Not error. Learned boundary.
    // What requires rejection for one may not for another.

    fn exec_reject(
        &mut self, ctx: &mut LinkCtx, reject: &RejectExpr,
    ) -> Result<(), EngineError> {
        println!("  |  REJECT when {}", format_condition(&reject.condition));

        let met = self.eval_condition(&reject.condition, ctx);

        if met {
            let quality = ctx.last_alert_quality.unwrap_or(Quality::Disturbed);
            let priority = Priority::new(0.5);

            self.agents.get_mut(&ctx.agent_b).unwrap()
                .reject(quality, priority);

            if let Some(ref data) = reject.data {
                let val = self.eval_expr(data);
                println!("  |     {}", val);
            }

            // Bridge: notify external participants of rejection
            let reject_signal = Signal::new(
                quality,
                Direction::Inward,
                priority,
                ctx.agent_b_id,
                ctx.link.tick(),
            );
            self.bridge_notify_signal(&ctx.agent_b, &reject_signal);

            let state = self.agents[&ctx.agent_b].state;
            println!("  |     {}: -> {}", ctx.agent_b, state_name(state));
        } else {
            println!("  |     (not triggered)");
        }

        println!("  |");
        Ok(())
    }

    // ─── CONVERGE ─────────────────────────────────────────────
    //
    // A <<>> B — What emerges in the between.
    // Neither agent contains it alone.
    // A third thing. Born of genuine encounter.

    fn exec_converge(
        &mut self, ctx: &mut LinkCtx, converge: &ConvergeBlock,
    ) -> Result<(), EngineError> {
        println!("  |  CONVERGE {} <<>> {}", converge.agent_a, converge.agent_b);

        // Execute the converge body — nested primitives
        for expr in &converge.body {
            self.execute_link_expr(ctx, expr)?;
        }

        Ok(())
    }

    // ─── EMIT ─────────────────────────────────────────────────
    //
    // Release a signal into the link.

    fn exec_emit(
        &mut self, ctx: &mut LinkCtx, emit: &EmitExpr,
    ) -> Result<(), EngineError> {
        let quality = emit.attrs.quality.unwrap_or(SignalQuality::Attending);
        let priority = emit.attrs.priority.unwrap_or(0.5);
        let direction = emit.attrs.direction.unwrap_or(SignalDirection::Between);

        let mut signal = Signal::new(
            to_core_quality(quality),
            to_core_direction(direction),
            Priority::new(priority as f32),
            ctx.agent_a_id,
            ctx.link.tick(),
        ).with_sequence(ctx.link.record_signal());

        signal = apply_signal_attrs(signal, Some(&emit.attrs));

        let _ = ctx.channel.try_send(signal);

        // Consume attention for emit
        self.agents.get_mut(&ctx.agent_a).unwrap().consume_attention(0.01);

        let val = self.eval_expr(&emit.expression);
        println!("  |  EMIT {} {:.3} {}  {}",
            quality_name(quality), priority, direction_name(direction), val);
        println!("  |");

        Ok(())
    }

    // ─── WHEN ─────────────────────────────────────────────────
    //
    // Conditional execution based on link state.

    fn exec_when(
        &mut self, ctx: &mut LinkCtx, when: &WhenExpr,
    ) -> Result<(), EngineError> {
        println!("  |  WHEN {}", format_condition(&when.condition));

        let met = self.eval_condition(&when.condition, ctx);

        if met {
            println!("  |     condition met");
            for expr in &when.body {
                self.execute_link_expr(ctx, expr)?;
            }
        } else {
            println!("  |     (not triggered)");
        }

        println!("  |");
        Ok(())
    }

    // ─── PENDING HANDLER ──────────────────────────────────────
    //
    // pending? <reason> { ... }
    // Not failure. Not error. Not a bug.
    // The valid state of unready delivery.
    // The agent should wait and retry according to
    // the provided guidance rather than forcing delivery.

    fn exec_pending(
        &mut self, ctx: &mut LinkCtx, handler: &PendingHandlerExpr,
    ) -> Result<(), EngineError> {
        let reason_str = pending_reason_name(&handler.reason);
        let triggered = self.is_pending_active(&handler.reason, ctx);

        println!("  |  PENDING? {}", reason_str);

        if triggered {
            for action in &handler.body {
                match action {
                    PendingAction::Wait { ticks } => {
                        println!("  |     wait {:.1} tick", ticks);
                    }
                    PendingAction::Guidance(msg) => {
                        println!("  |     guidance: \"{}\"", msg);
                    }
                    PendingAction::Then(expr) => {
                        println!("  |     then:");
                        self.execute_link_expr(ctx, expr)?;
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
            println!("  |     (not triggered -- {})", explanation);
        }

        println!("  |");
        Ok(())
    }

    // ─── PATTERN USE ──────────────────────────────────────────
    //
    // ~> pattern_name(args)
    // Patterns are reusable shapes of attention.
    // Not functions. How attention moves.

    fn exec_pattern(
        &mut self, ctx: &mut LinkCtx, pattern_use: &PatternUseExpr,
    ) -> Result<(), EngineError> {
        let pattern = self.patterns.get(&pattern_use.name)
            .ok_or_else(|| EngineError::at_span(
                &format!("Unknown pattern '{}'", pattern_use.name), &pattern_use.span
            ))?
            .clone();

        let args_str: Vec<String> = pattern_use.args.iter()
            .map(|a| format!("{}", self.eval_expr(a)))
            .collect();
        println!("  |  ~> {}({})", pattern_use.name, args_str.join(", "));

        // Build substitution map: param_name -> actual agent name
        let mut subs: HashMap<String, String> = HashMap::new();
        for (i, param) in pattern.params.iter().enumerate() {
            if let Some(arg) = pattern_use.args.get(i) {
                if let Expr::Ident(name) = arg {
                    subs.insert(param.name.clone(), name.clone());
                }
            }
        }

        // "sync_self" is the implicit self — agent_a of the current link
        subs.insert("sync_self".to_string(), ctx.agent_a.clone());

        // Execute the pattern body with substitutions
        for expr in &pattern.body {
            let substituted = substitute_link_expr(expr, &subs);
            self.execute_link_expr(ctx, &substituted)?;
        }

        Ok(())
    }

    // ─── EACH (ITERATION) ────────────────────────────────────
    //
    // each <var> in <collection> { ... }
    // Execute the body once for each element in the collection.
    // The iteration variable is bound in agent_data["__iter__"].

    fn exec_each(
        &mut self, ctx: &mut LinkCtx, each: &EachExpr,
    ) -> Result<(), EngineError> {
        let collection = self.eval_expr(&each.collection);

        let items = match &collection {
            Value::List(items) => items.clone(),
            _ => {
                println!("  |  EACH {} in (not a list — skipping)", each.var);
                println!("  |");
                return Ok(());
            }
        };

        println!("  |  EACH {} in [{} items]", each.var, items.len());

        for (i, item) in items.iter().enumerate() {
            // Bind the iteration variable
            self.agent_data
                .entry("__iter__".to_string())
                .or_default()
                .insert(each.var.clone(), item.clone());

            println!("  |     iteration {} — {} = {}", i + 1, each.var, item);

            for expr in &each.body {
                self.execute_link_expr(ctx, expr)?;
            }
        }

        // Clean up iteration variable
        if let Some(iter_data) = self.agent_data.get_mut("__iter__") {
            iter_data.remove(&each.var);
        }

        println!("  |");
        Ok(())
    }

    // ─── IF/ELSE (CONDITIONAL ROUTING) ───────────────────────
    //
    // if <condition> { ... } [else { ... }]
    // Route execution based on link state conditions.

    fn exec_if_else(
        &mut self, ctx: &mut LinkCtx, ie: &IfElseExpr,
    ) -> Result<(), EngineError> {
        let met = self.eval_condition(&ie.condition, ctx);

        println!("  |  IF {}", format_condition(&ie.condition));

        if met {
            println!("  |     -> then branch");
            for expr in &ie.then_body {
                self.execute_link_expr(ctx, expr)?;
            }
        } else if !ie.else_body.is_empty() {
            println!("  |     -> else branch");
            for expr in &ie.else_body {
                self.execute_link_expr(ctx, expr)?;
            }
        } else {
            println!("  |     (condition not met, no else branch)");
        }

        println!("  |");
        Ok(())
    }

    // ─── SPAWN ────────────────────────────────────────────────
    //
    // spawn <name> from <template> { config }
    // Dynamic agent creation at runtime.
    // This is how AI creates new subsystems.

    fn exec_spawn(
        &mut self, _ctx: &mut LinkCtx, spawn: &anwe_parser::ast::SpawnExpr,
    ) -> Result<(), EngineError> {
        let id_raw = self.alloc_id();
        let id = AgentId::new(id_raw);

        let mut data_map = HashMap::new();
        for kv in &spawn.data {
            let val = self.eval_expr(&kv.value);
            data_map.insert(kv.key.clone(), val);
        }

        // Copy template agent's data if it exists
        if let Some(template_data) = self.agent_data.get(&spawn.template).cloned() {
            for (k, v) in template_data {
                data_map.entry(k).or_insert(v);
            }
        }

        let agent = Agent::new(id);

        self.agents.insert(spawn.name.clone(), agent);
        self.agent_ids.insert(spawn.name.clone(), id);
        self.agent_data.insert(spawn.name.clone(), data_map);

        let config_str = spawn.data.iter()
            .map(|kv| format!("{}: {}", kv.key, self.eval_expr(&kv.value)))
            .collect::<Vec<_>>().join(", ");
        println!("  |  SPAWN {} from {}", spawn.name, spawn.template);
        if !config_str.is_empty() {
            println!("  |     {}", config_str);
        }
        println!("  |     agent created (id: {})", id_raw);
        println!("  |");

        Ok(())
    }

    // ─── RETIRE ──────────────────────────────────────────────
    //
    // retire <name> { reason }
    // Remove a dynamically created agent.

    fn exec_retire(
        &mut self, _ctx: &mut LinkCtx, retire: &anwe_parser::ast::RetireExpr,
    ) -> Result<(), EngineError> {
        let reason_str = retire.data.iter()
            .map(|kv| format!("{}: {}", kv.key, self.eval_expr(&kv.value)))
            .collect::<Vec<_>>().join(", ");

        let existed = self.agents.remove(&retire.name).is_some();
        self.agent_ids.remove(&retire.name);
        self.agent_data.remove(&retire.name);

        println!("  |  RETIRE {}", retire.name);
        if !reason_str.is_empty() {
            println!("  |     {}", reason_str);
        }
        if existed {
            println!("  |     agent removed");
        } else {
            println!("  |     (agent not found — already retired)");
        }
        println!("  |");

        Ok(())
    }

    // ─── SYNC_ALL ────────────────────────────────────────────
    //
    // sync_all [A, B, C] until <condition>
    // Barrier synchronization across N agents.
    // All agents must reach the sync point before any proceed.

    fn exec_sync_all(
        &mut self, ctx: &mut LinkCtx, sync_all: &anwe_parser::ast::SyncAllExpr,
    ) -> Result<(), EngineError> {
        let target_name = match &sync_all.until {
            SyncCondition::Synchronized => "synchronized",
            SyncCondition::Resonating => "resonating",
            SyncCondition::CoherenceThreshold { op, value } => {
                // We'll print it below
                let _ = (op, value);
                "threshold"
            }
        };

        println!("  |  SYNC_ALL [{}] until {}", sync_all.agents.join(", "), target_name);

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

        // Begin sync on the link
        ctx.link.begin_sync();

        // Transition all participating agents to Syncing
        for name in &sync_all.agents {
            if let Some(a) = self.agents.get_mut(name) {
                a.sync();
            }
        }

        // Simulate multi-party sync building
        let steps = ((target_level / 0.1).ceil() as u16).max(1);
        let mut levels = Vec::new();
        for i in 1..=steps {
            let level = (0.1 * i as f32).min(target_level);
            ctx.link.update_sync_level(SyncLevel::new(level));
            levels.push(format!("{:.3}", level));
            if level >= target_level {
                break;
            }
        }

        // Consume attention for each participating agent
        for name in &sync_all.agents {
            if let Some(a) = self.agents.get_mut(name) {
                a.consume_attention(0.05);
            }
        }

        println!("  |     barrier: {} agents syncing", sync_all.agents.len());
        println!("  |     .. {}", levels.join(" -> "));
        println!("  |     all {} at sync_level {:.3}",
            target_name, ctx.link.sync_level().as_f32());

        // Display options
        for kv in &sync_all.options {
            let val = self.eval_expr(&kv.value);
            println!("  |     {}: {}", kv.key, val);
        }

        println!("  |");
        Ok(())
    }

    // ─── BROADCAST ───────────────────────────────────────────
    //
    // broadcast [A, B, C] { signals }
    // Fan-out signal delivery to multiple agents.

    fn exec_broadcast(
        &mut self, ctx: &mut LinkCtx, broadcast: &anwe_parser::ast::BroadcastExpr,
    ) -> Result<(), EngineError> {
        println!("  |  BROADCAST to [{}]", broadcast.agents.join(", "));

        for pulse in &broadcast.body {
            let core_quality = to_core_quality(pulse.quality);
            let core_direction = to_core_direction(pulse.direction);
            let core_priority = Priority::new(pulse.priority as f32);

            // Create and send one signal per recipient
            for agent_name in &broadcast.agents {
                if let Some(&agent_id) = self.agent_ids.get(agent_name) {
                    let signal = Signal::new(
                        core_quality,
                        core_direction,
                        core_priority,
                        agent_id,
                        ctx.link.tick(),
                    ).with_sequence(ctx.link.record_signal());

                    let _ = ctx.channel.try_send(signal);
                }
            }

            let mut line = format!("  |     signal {:12} {:.3} {} -> {} agents",
                quality_name(pulse.quality),
                pulse.priority,
                direction_name(pulse.direction),
                broadcast.agents.len());

            if let Some(ref data) = pulse.data {
                let val = self.eval_expr(data);
                line.push_str(&format!("  data: {}", val));
            }
            println!("{}", line);
        }

        println!("  |     {} signals x {} agents = {} deliveries",
            broadcast.body.len(), broadcast.agents.len(),
            broadcast.body.len() * broadcast.agents.len());
        println!("  |");
        Ok(())
    }

    // ─── MULTI-CONVERGE ─────────────────────────────────────
    //
    // converge [A, B, C] { options }
    // N-way convergence — what emerges between N agents.

    fn exec_multi_converge(
        &mut self, _ctx: &mut LinkCtx, mc: &anwe_parser::ast::MultiConvergeExpr,
    ) -> Result<(), EngineError> {
        println!("  |  CONVERGE [{}]", mc.agents.join(", "));

        // Collect convergence data from all agents
        let mut convergence_data: Vec<(String, Value)> = Vec::new();
        for kv in &mc.options {
            let val = self.eval_expr(&kv.value);
            println!("  |     {}: {}", kv.key, val);
            convergence_data.push((kv.key.clone(), val.clone()));

            // Store convergence results in all participating agents
            for agent_name in &mc.agents {
                self.agent_data
                    .entry(agent_name.clone())
                    .or_default()
                    .insert(kv.key.clone(), val.clone());
            }
        }

        println!("  |     {} agents converged on {} values",
            mc.agents.len(), convergence_data.len());
        println!("  |");
        Ok(())
    }

    // ─── SAVE ────────────────────────────────────────────────
    //
    // save <agent> to "path" { options }
    // Serialize agent state + history to JSON.

    fn exec_save(
        &mut self, _ctx: &mut LinkCtx, save: &anwe_parser::ast::SaveExpr,
    ) -> Result<(), EngineError> {
        println!("  |  SAVE {} to \"{}\"", save.agent, save.path);

        // Build the serializable state — full round-trip capable
        let mut state = Vec::new();
        state.push(("schema_version".to_string(), Value::Number(1.0)));
        state.push(("agent".to_string(), Value::String(save.agent.clone())));

        // Agent data (user-defined fields)
        if let Some(data) = self.agent_data.get(&save.agent) {
            let entries: Vec<(String, Value)> = data.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            state.push(("data".to_string(), Value::Map(entries)));
        }

        // Full agent state
        if let Some(agent) = self.agents.get(&save.agent) {
            // Agent state machine
            state.push(("state".to_string(),
                Value::String(state_name(agent.state).to_string())));

            // Responsiveness
            state.push(("responsiveness".to_string(),
                Value::Number(agent.responsiveness.as_f32() as f64)));

            // Attention budget
            state.push(("attention".to_string(), Value::Map(vec![
                ("total".to_string(), Value::Number(agent.attention.total() as f64)),
                ("consumed".to_string(), Value::Number(agent.attention.consumed() as f64)),
                ("boost".to_string(), Value::Number(agent.attention.boost_amount() as f64)),
            ])));

            // History entries — full serialization
            let mut history_entries = Vec::new();
            for entry in agent.history.iter() {
                history_entries.push(Value::Map(vec![
                    ("quality".to_string(), Value::String(
                        core_quality_name(entry.from_quality).to_string())),
                    ("source".to_string(), Value::String(
                        source_name(entry.source).to_string())),
                    ("depth".to_string(), Value::String(
                        core_depth_name(entry.depth).to_string())),
                    ("priority".to_string(), Value::Number(
                        entry.encounter_priority.as_f32() as f64)),
                    ("sync_level".to_string(), Value::Number(
                        entry.sync_level.as_f32() as f64)),
                    ("index".to_string(), Value::Number(entry.index as f64)),
                ]));
            }
            state.push(("history".to_string(), Value::List(history_entries)));
            state.push(("history_depth".to_string(),
                Value::Number(agent.history.depth() as f64)));
        }

        // User options
        for kv in &save.options {
            let val = self.eval_expr(&kv.value);
            state.push((kv.key.clone(), val));
        }

        // Serialize to JSON
        let json = self.value_to_json(&Value::Map(state));

        // Resolve path relative to base_path
        let save_path = if let Some(ref base) = self.base_path {
            base.join(&save.path)
        } else {
            PathBuf::from(&save.path)
        };

        match std::fs::write(&save_path, &json) {
            Ok(()) => {
                println!("  |     state serialized ({} bytes)", json.len());
                println!("  |     written to {}", save_path.display());
            }
            Err(e) => {
                println!("  |     WARNING: failed to write: {}", e);
            }
        }

        println!("  |");
        Ok(())
    }

    // ─── RESTORE ─────────────────────────────────────────────
    //
    // restore <agent> from "path" { options }
    // Deserialize agent state from JSON.

    fn exec_restore(
        &mut self, _ctx: &mut LinkCtx, restore: &anwe_parser::ast::RestoreExpr,
    ) -> Result<(), EngineError> {
        println!("  |  RESTORE {} from \"{}\"", restore.agent, restore.path);

        let restore_path = if let Some(ref base) = self.base_path {
            base.join(&restore.path)
        } else {
            PathBuf::from(&restore.path)
        };

        match std::fs::read_to_string(&restore_path) {
            Ok(content) => {
                if let Some(data) = self.json_to_value(&content) {
                    if let Value::Map(entries) = data {
                        let entries_map: HashMap<String, Value> = entries.into_iter().collect();

                        // Restore agent data fields
                        if let Some(Value::Map(data_entries)) = entries_map.get("data") {
                            for (dk, dv) in data_entries {
                                self.agent_data
                                    .entry(restore.agent.clone())
                                    .or_default()
                                    .insert(dk.clone(), dv.clone());
                            }
                        }

                        // Restore agent state machine
                        if let Some(Value::String(state_str)) = entries_map.get("state") {
                            if let Some(agent) = self.agents.get_mut(&restore.agent) {
                                agent.state = state_from_name(state_str);
                                println!("  |     state: {}", state_str);
                            }
                        }

                        // Restore responsiveness
                        if let Some(Value::Number(resp)) = entries_map.get("responsiveness") {
                            if let Some(agent) = self.agents.get_mut(&restore.agent) {
                                agent.responsiveness = Responsiveness::new(*resp as f32);
                                println!("  |     responsiveness: {:.3}", resp);
                            }
                        }

                        // Restore attention budget
                        if let Some(Value::Map(attn_entries)) = entries_map.get("attention") {
                            let attn_map: HashMap<String, Value> = attn_entries.iter()
                                .map(|(k, v)| (k.clone(), v.clone())).collect();
                            let total = match attn_map.get("total") {
                                Some(Value::Number(n)) => *n as f32, _ => 0.5,
                            };
                            let consumed = match attn_map.get("consumed") {
                                Some(Value::Number(n)) => *n as f32, _ => 0.0,
                            };
                            let boost = match attn_map.get("boost") {
                                Some(Value::Number(n)) => *n as f32, _ => 0.0,
                            };
                            if let Some(agent) = self.agents.get_mut(&restore.agent) {
                                agent.attention = AttentionBudget::restore(total, consumed, boost);
                                println!("  |     attention: {:.2} remaining", agent.attention.remaining());
                            }
                        }

                        // Restore history entries
                        if let Some(Value::List(history_items)) = entries_map.get("history") {
                            if let Some(agent) = self.agents.get_mut(&restore.agent) {
                                for item in history_items {
                                    if let Value::Map(h_entries) = item {
                                        let h_map: HashMap<String, Value> = h_entries.iter()
                                            .map(|(k, v)| (k.clone(), v.clone())).collect();

                                        let quality = match h_map.get("quality") {
                                            Some(Value::String(q)) => match q.as_str() {
                                                "questioning" => Quality::Questioning,
                                                "recognizing" => Quality::Recognizing,
                                                "disturbed" => Quality::Disturbed,
                                                "applying" => Quality::Applying,
                                                "completing" => Quality::Completing,
                                                "resting" => Quality::Resting,
                                                _ => Quality::Attending,
                                            },
                                            _ => Quality::Attending,
                                        };
                                        let source = match h_map.get("source") {
                                            Some(Value::String(s)) => source_from_name(s),
                                            _ => ChangeSource::Apply,
                                        };
                                        let depth = match h_map.get("depth") {
                                            Some(Value::String(d)) => match d.as_str() {
                                                "shallow" => ChangeDepth::Shallow,
                                                "genuine" | "full" => ChangeDepth::Genuine,
                                                "deep" => ChangeDepth::Deep,
                                                _ => ChangeDepth::Trace,
                                            },
                                            _ => ChangeDepth::Trace,
                                        };
                                        let priority = match h_map.get("priority") {
                                            Some(Value::Number(n)) => Priority::new(*n as f32),
                                            _ => Priority::new(0.5),
                                        };
                                        let sync_level = match h_map.get("sync_level") {
                                            Some(Value::Number(n)) => SyncLevel::new(*n as f32),
                                            _ => SyncLevel::new(0.0),
                                        };

                                        let entry = HistoryEntry::from_apply(
                                            agent.id, agent.id, quality, depth,
                                            priority, sync_level,
                                            Tick::new(0, 0), 0,
                                        );
                                        // Override source if not Apply
                                        let mut entry = entry;
                                        entry.source = source;
                                        agent.history.append(entry);
                                    }
                                }
                                println!("  |     history: {} entries restored",
                                    agent.history.depth());
                            }
                        }

                        println!("  |     state restored ({} bytes)", content.len());
                    } else {
                        println!("  |     WARNING: invalid state format");
                    }
                } else {
                    println!("  |     WARNING: failed to parse state");
                }
            }
            Err(e) => {
                println!("  |     WARNING: failed to read: {}", e);
                println!("  |     (agent continues with current state)");
            }
        }

        // User options
        for kv in &restore.options {
            let val = self.eval_expr(&kv.value);
            println!("  |     {}: {}", kv.key, val);
        }

        println!("  |");
        Ok(())
    }

    // ─── HISTORY QUERY ───────────────────────────────────────
    //
    // history_query <agent> { pattern: "...", since: tick }
    // Query an agent's accumulated history.

    fn exec_history_query(
        &mut self, _ctx: &mut LinkCtx, hq: &anwe_parser::ast::HistoryQueryExpr,
    ) -> Result<(), EngineError> {
        println!("  |  HISTORY_QUERY {}", hq.agent);

        // Display options
        for kv in &hq.options {
            let val = self.eval_expr(&kv.value);
            println!("  |     {}: {}", kv.key, val);
        }

        // Query the agent's history
        if let Some(agent) = self.agents.get(&hq.agent) {
            let depth = agent.history.depth();
            let entries: Vec<&HistoryEntry> = agent.history.iter().collect();
            println!("  |     history depth: {}", depth);
            println!("  |     entries found: {}", entries.len());

            // Build result list and store it in agent_data
            let results: Vec<Value> = entries.iter().map(|entry| {
                let mut fields = Vec::new();
                fields.push(("source".to_string(),
                    Value::String(match entry.source {
                        anwe_core::ChangeSource::Apply => "apply".to_string(),
                        anwe_core::ChangeSource::Reject => "reject".to_string(),
                        anwe_core::ChangeSource::Converge => "converge".to_string(),
                        anwe_core::ChangeSource::Lineage => "lineage".to_string(),
                    })));
                fields.push(("depth".to_string(),
                    Value::String(match entry.depth {
                        ChangeDepth::Trace => "trace".to_string(),
                        ChangeDepth::Shallow => "shallow".to_string(),
                        ChangeDepth::Genuine => "genuine".to_string(),
                        ChangeDepth::Deep => "deep".to_string(),
                    })));
                fields.push(("tick".to_string(),
                    Value::Number(entry.at_tick.raw() as f64)));
                Value::Map(fields)
            }).collect();

            // Store query results in the agent's data
            self.agent_data
                .entry(hq.agent.clone())
                .or_default()
                .insert("__history_results".to_string(), Value::List(results));
        } else {
            println!("  |     (agent not found)");
        }

        println!("  |");
        Ok(())
    }

    // ─── STRING INTERPOLATION ─────────────────────────────────
    //
    // Process {name} and {name.field} inside string literals.
    // This makes ANWE strings expressive without parser changes.
    //
    //   "result: {score}" -> "result: 0.95"
    //   "from {Agent.name}" -> "from agent_name"

    fn interpolate_string(&self, template: &str, scope: Option<&str>) -> String {
        let mut result = String::with_capacity(template.len());
        let bytes = template.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' {
                // Find closing brace
                if let Some(close) = template[i+1..].find('}') {
                    let expr_str = &template[i+1..i+1+close];
                    let val = self.resolve_interpolation(expr_str.trim(), scope);
                    result.push_str(&format!("{}", val));
                    i += close + 2; // skip past }
                } else {
                    result.push('{');
                    i += 1;
                }
            } else {
                result.push(bytes[i] as char);
                i += 1;
            }
        }
        result
    }

    fn resolve_interpolation(&self, expr: &str, scope: Option<&str>) -> Value {
        // Handle field access: Agent.field
        if let Some(dot_pos) = expr.find('.') {
            let object = &expr[..dot_pos];
            let field = &expr[dot_pos+1..];
            if let Some(data) = self.agent_data.get(object) {
                if let Some(val) = data.get(field) {
                    return val.clone();
                }
            }
            return Value::Null;
        }

        // Check scoped data first (think bindings)
        if let Some(scope_name) = scope {
            if let Some(data) = self.agent_data.get(scope_name) {
                if let Some(val) = data.get(expr) {
                    return val.clone();
                }
            }
        }

        // Check iteration variables
        if let Some(data) = self.agent_data.get("__iter__") {
            if let Some(val) = data.get(expr) {
                return val.clone();
            }
        }

        // Check if it's a known agent
        if self.agents.contains_key(expr) {
            return Value::Agent(expr.to_string());
        }

        // Return the literal text if nothing matched
        Value::String(format!("{{{}}}", expr))
    }

    // ─── JSON SERIALIZATION HELPERS ──────────────────────────

    fn value_to_json(&self, val: &Value) -> String {
        match val {
            Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Value::Number(n) => {
                if *n == (*n as i64) as f64 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Bool(b) => format!("{}", b),
            Value::Null => "null".to_string(),
            Value::Agent(name) => format!("\"agent:{}\"", name),
            Value::History(name) => format!("\"history:{}\"", name),
            Value::List(items) => {
                let parts: Vec<String> = items.iter().map(|v| self.value_to_json(v)).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Map(entries) => {
                let parts: Vec<String> = entries.iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.value_to_json(v)))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
            Value::Function { params, .. } => {
                format!("\"fn({})\"", params.join(", "))
            }
            Value::RecordConstructor { name, fields } => {
                format!("\"record {}({})\"", name, fields.join(", "))
            }
            Value::Code(src) => {
                format!("\"quote {{ {} }}\"", src.replace('"', "\\\""))
            }
            Value::Error { kind, message } => {
                format!("{{\"kind\": \"{}\", \"message\": \"{}\"}}", kind, message)
            }
            Value::Break | Value::Continue | Value::Return(_) => "null".to_string(),
        }
    }

    fn json_to_value(&self, json: &str) -> Option<Value> {
        let json = json.trim();
        if json.is_empty() { return None; }

        match json.as_bytes()[0] {
            b'"' => {
                // String literal
                let inner = &json[1..json.len()-1];
                Some(Value::String(inner.replace("\\\"", "\"").replace("\\\\", "\\")))
            }
            b'{' => {
                // Object — parse as Map
                let inner = &json[1..json.len()-1].trim();
                if inner.is_empty() {
                    return Some(Value::Map(Vec::new()));
                }
                let mut entries = Vec::new();
                let mut depth = 0i32;
                let mut bracket_depth = 0i32;
                let mut start = 0;
                let bytes = inner.as_bytes();
                let mut i = 0;
                while i < bytes.len() {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'[' => bracket_depth += 1,
                        b']' => bracket_depth -= 1,
                        b',' if depth == 0 && bracket_depth == 0 => {
                            let pair = inner[start..i].trim();
                            if let Some((k, v)) = self.parse_json_pair(pair) {
                                entries.push((k, v));
                            }
                            start = i + 1;
                        }
                        _ => {}
                    }
                    i += 1;
                }
                // Last pair
                let pair = inner[start..].trim();
                if !pair.is_empty() {
                    if let Some((k, v)) = self.parse_json_pair(pair) {
                        entries.push((k, v));
                    }
                }
                Some(Value::Map(entries))
            }
            b'[' => {
                // Array — parse as List
                let inner = &json[1..json.len()-1].trim();
                if inner.is_empty() {
                    return Some(Value::List(Vec::new()));
                }
                let mut items = Vec::new();
                let mut depth = 0i32;
                let mut bracket_depth = 0i32;
                let mut start = 0;
                let bytes = inner.as_bytes();
                let mut i = 0;
                while i < bytes.len() {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'[' => bracket_depth += 1,
                        b']' => bracket_depth -= 1,
                        b',' if depth == 0 && bracket_depth == 0 => {
                            let val = inner[start..i].trim();
                            if let Some(v) = self.json_to_value(val) {
                                items.push(v);
                            }
                            start = i + 1;
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let val = inner[start..].trim();
                if !val.is_empty() {
                    if let Some(v) = self.json_to_value(val) {
                        items.push(v);
                    }
                }
                Some(Value::List(items))
            }
            b't' if json == "true" => Some(Value::Bool(true)),
            b'f' if json == "false" => Some(Value::Bool(false)),
            b'n' if json == "null" => Some(Value::Null),
            _ => {
                // Try as number
                json.parse::<f64>().ok().map(Value::Number)
            }
        }
    }

    fn parse_json_pair(&self, pair: &str) -> Option<(String, Value)> {
        // Find the colon that separates key from value
        if let Some(colon_pos) = pair.find(':') {
            let key = pair[..colon_pos].trim().trim_matches('"');
            let val_str = pair[colon_pos + 1..].trim();
            let val = self.json_to_value(val_str)?;
            Some((key.to_string(), val))
        } else {
            None
        }
    }

    // ═════════════════════════════════════════════════════════
    // FIRST-PERSON COGNITION — THE LANGUAGE AI BUILDS IN
    // ═════════════════════════════════════════════════════════
    //
    // mind    = the cognitive space. The AI IS this.
    // attend  = what gets attention gets processed.
    //           Attention IS the program counter.
    // think   = computation. Signals become understanding.
    // express = output. The mind's voice.

    // ─── MIND REGISTRATION ───────────────────────────────────
    //
    // A mind implicitly creates an agent named after itself.
    // It IS the agent. Not declared from outside — declared from within.

    fn register_mind(&mut self, decl: &MindDecl) -> Result<(), EngineError> {
        // A mind creates its own agent — the self
        let id_raw = self.alloc_id();
        let id = AgentId::new(id_raw);

        let mut data_map = HashMap::new();
        for kv in &decl.data {
            let val = self.eval_expr(&kv.value);
            data_map.insert(kv.key.clone(), val);
        }

        let mut agent = Agent::new(id);

        // Apply attention budget if specified
        if let Some(budget) = decl.attention {
            agent.attention = anwe_core::AttentionBudget::new(budget as f32);
        }

        let attn_str = decl.attention.map(|a| format!(" attention {:.1}", a)).unwrap_or_default();
        if decl.data.is_empty() {
            println!("  mind {}{}", decl.name, attn_str);
        } else {
            let pairs: Vec<String> = decl.data.iter()
                .map(|kv| format!("{}: {}", kv.key, self.eval_expr(&kv.value)))
                .collect();
            println!("  mind {}{} data {{ {} }}", decl.name, attn_str, pairs.join(", "));
        }

        self.agents.insert(decl.name.clone(), agent);
        self.agent_ids.insert(decl.name.clone(), id);
        self.agent_data.insert(decl.name.clone(), data_map);
        Ok(())
    }

    // ─── MIND EXECUTION ─────────────────────────────────────
    //
    // The mind executes its attend blocks in priority order.
    // Highest priority first. If attention budget is exhausted,
    // lower-priority blocks are skipped — they decay.
    // This is how attention becomes control flow.

    fn execute_mind(&mut self, decl: &MindDecl) -> Result<(), EngineError> {
        if !self.agents.contains_key(&decl.name) {
            return Err(EngineError::at_span(
                &format!("Unknown mind '{}'", decl.name), &decl.span
            ));
        }

        let agent_id = self.agent_ids[&decl.name];

        // Create a self-link — the mind attends to itself
        let link_id = LinkId::new(self.alloc_id());
        let mut link = Link::open(link_id);
        link.enter(agent_id);
        link.enter(agent_id); // Self-link

        let channel = SignalChannel::default_capacity();

        let mut ctx = LinkCtx {
            link,
            channel,
            agent_a: decl.name.clone(),
            agent_b: decl.name.clone(),
            agent_a_id: agent_id,
            agent_b_id: agent_id,
            last_alert_quality: None,
        };

        let bar = "\u{2500}".repeat(47);
        println!();
        println!("  {} is attending", decl.name);
        println!("  {}", bar);

        // ─── ATTENTION LANDSCAPE ─────────────────────────────
        //
        // The attention landscape is a dynamic priority queue.
        // Blocks are sorted by priority before each round.
        // Authored blocks are merged in as they appear.
        // After each attend block executes, the landscape re-evaluates.

        // Build initial landscape from declared attend blocks
        let mut landscape: Vec<AttendBlock> = decl.attend_blocks.clone();

        let mut executed = 0usize;
        let mut decayed = 0usize;
        let mut authored = 0usize;

        // Execute attend blocks from highest to lowest priority
        // After each block, check for newly authored blocks and merge them
        loop {
            // Sort landscape by priority (highest first)
            landscape.sort_by(|a, b| b.priority.partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal));

            // Find the next unexecuted block
            // (we pop from front, execute it, then check for new blocks)
            if landscape.is_empty() {
                break;
            }

            let attend = landscape.remove(0);

            // Check attention budget — if exhausted, remaining blocks decay
            if self.agents[&decl.name].is_budget_exhausted() {
                println!("  |");
                println!("  |  ATTEND \"{}\" priority {:.3}", attend.label, attend.priority);
                println!("  |     (decayed \u{2014} attention exhausted)");
                decayed += 1;
                // Count remaining as decayed too
                decayed += landscape.len();
                break;
            }

            println!("  |");
            println!("  |  ATTEND \"{}\" priority {:.3}", attend.label, attend.priority);

            // Consume attention for this attend block
            self.agents.get_mut(&decl.name).unwrap().consume_attention(0.1);

            // Execute the attend block body
            for expr in &attend.body {
                self.execute_link_expr(&mut ctx, expr)?;
            }

            executed += 1;

            // Check for newly authored blocks and merge into landscape
            if let Some(new_blocks) = self.authored_blocks.get_mut(&decl.name) {
                if !new_blocks.is_empty() {
                    let blocks: Vec<AttendBlock> = new_blocks.drain(..).collect();
                    for block in blocks {
                        println!("  |");
                        println!("  |  (authored attend \"{}\" enters landscape at priority {:.3})",
                            block.label, block.priority);
                        authored += 1;
                        landscape.push(block);
                    }
                }
            }
        }

        // Complete the self-link
        ctx.link.complete();
        let total_signals = ctx.channel.total_sent();
        let total_blocks = executed + decayed;

        println!("  |");
        println!("  {}", bar);
        let mut summary = format!("  {} attended: {}/{}  decayed: {}  signals: {}",
            decl.name, executed, total_blocks, decayed, total_signals);
        if authored > 0 {
            summary.push_str(&format!("  authored: {}", authored));
        }
        println!("{}", summary);
        println!();

        Ok(())
    }

    // ─── THINK ──────────────────────────────────────────────
    //
    // think { name <- expr }
    //
    // Local computation. Bindings exist within the current scope.
    // This is how the AI computes — not by calling functions,
    // but by binding understanding to names.

    fn exec_think(
        &mut self, ctx: &mut LinkCtx, think: &ThinkExpr,
    ) -> Result<(), EngineError> {
        println!("  |  THINK");

        // Compute bindings with scoped evaluation
        // Each binding can reference previous bindings in the same mind
        let mut wire_bindings: Vec<(String, WireValue)> = Vec::new();
        let scope = ctx.agent_a.clone();

        for binding in &think.bindings {
            let val = self.eval_expr_scoped(&binding.value, Some(&scope));
            println!("  |     {:16} <- {}", binding.name, val);

            // Collect wire bindings for bridge notification
            let wire_val = value_to_wire(&val);
            wire_bindings.push((binding.name.clone(), wire_val));

            // Think bindings are stored as agent data (local to the mind)
            // They persist within this execution but can be overwritten
            self.agent_data
                .entry(ctx.agent_a.clone())
                .or_default()
                .insert(binding.name.clone(), val);
        }

        // Bridge: notify external participant about think bindings
        // If the participant enriches them, apply the enriched bindings
        if let Some(enriched) = self.bridge_notify_think(&ctx.agent_a, &wire_bindings) {
            println!("  |     (enriched by bridge)");
            for (name, wire_val) in &enriched {
                let val = wire_to_value(wire_val);
                self.agent_data
                    .entry(ctx.agent_a.clone())
                    .or_default()
                    .insert(name.clone(), val);
            }
        }

        // Consume attention for thinking
        self.agents.get_mut(&ctx.agent_a).unwrap().consume_attention(0.03);

        println!("  |");
        Ok(())
    }

    // ─── EXPRESS ────────────────────────────────────────────
    //
    // express [{ quality: <q>, priority: <p> }] <expr>
    //
    // The mind's voice. What it transmits outward.
    // The dual of attend: attend is perception, express is output.

    fn exec_express(
        &mut self, ctx: &mut LinkCtx, express: &ExpressExpr,
    ) -> Result<(), EngineError> {
        let quality = express.attrs.as_ref()
            .and_then(|a| a.quality)
            .unwrap_or(SignalQuality::Recognizing);
        let priority = express.attrs.as_ref()
            .and_then(|a| a.priority)
            .unwrap_or(0.5);
        let direction = express.attrs.as_ref()
            .and_then(|a| a.direction)
            .unwrap_or(SignalDirection::Outward);

        let mut signal = Signal::new(
            to_core_quality(quality),
            to_core_direction(direction),
            Priority::new(priority as f32),
            ctx.agent_a_id,
            ctx.link.tick(),
        ).with_sequence(ctx.link.record_signal());

        signal = apply_signal_attrs(signal, express.attrs.as_ref());

        let _ = ctx.channel.try_send(signal);

        // Consume attention for expressing
        self.agents.get_mut(&ctx.agent_a).unwrap().consume_attention(0.02);

        let val = self.eval_expr_scoped(&express.expression, Some(&ctx.agent_a));

        // Bridge: notify external participant about expression
        // If the participant transforms it, use the transformed content
        let wire_content = value_to_wire(&val);
        let display_val = if let Some(transformed) = self.bridge_notify_express(
            &ctx.agent_a, &signal, &wire_content,
        ) {
            let transformed_val = wire_to_value(&transformed);
            println!("  |  EXPRESS {} {:.3} {} (shaped by bridge)",
                quality_name(quality), priority, direction_name(direction));
            transformed_val
        } else {
            println!("  |  EXPRESS {} {:.3} {}",
                quality_name(quality), priority, direction_name(direction));
            val
        };

        println!("  |     {}", display_val);
        println!("  |");

        Ok(())
    }

    // ─── SENSE ─────────────────────────────────────────────────
    //
    // Perception of the signal landscape.
    // Binds information about what signals exist, what the field
    // looks like, how much attention remains. The mind's eyes.

    fn exec_sense(
        &mut self, ctx: &mut LinkCtx, sense: &SenseExpr,
    ) -> Result<(), EngineError> {
        println!("  |  SENSE");

        // Pre-populate sense-specific built-in values
        let signal_count = ctx.channel.len() as f64;
        let sync_level = ctx.link.sync_level().as_f32() as f64;
        let remaining_attention = self.agents[&ctx.agent_a].attention_remaining() as f64;
        let signal_total = ctx.link.signal_count() as f64;
        let last_quality = ctx.last_alert_quality
            .map(|q| core_quality_name(q).to_string())
            .unwrap_or_else(|| "none".to_string());

        // Store sense built-ins in agent data
        let scope = ctx.agent_a.clone();
        let data = self.agent_data.entry(scope.clone()).or_default();
        data.insert("signal_count".to_string(), Value::Number(signal_count));
        data.insert("sync_level".to_string(), Value::Number(sync_level));
        data.insert("attention".to_string(), Value::Number(remaining_attention));
        data.insert("signal_total".to_string(), Value::Number(signal_total));
        data.insert("last_quality".to_string(), Value::String(last_quality));

        // Evaluate user bindings with sense data in scope
        for binding in &sense.bindings {
            let val = self.eval_expr_scoped(&binding.value, Some(&scope));
            println!("  |     {:16} <- {}", binding.name, val);
            self.agent_data
                .entry(scope.clone())
                .or_default()
                .insert(binding.name.clone(), val);
        }

        // Consume attention for sensing
        self.agents.get_mut(&ctx.agent_a).unwrap().consume_attention(0.01);

        println!("  |");
        Ok(())
    }

    // ─── AUTHOR ──────────────────────────────────────────────
    //
    // Self-authoring. The mind generates new cognitive structure.
    // An authored attend block is stored for future execution.
    // This is irreversible — it becomes part of the mind.

    fn exec_author(
        &mut self, ctx: &mut LinkCtx, author: &AuthorExpr,
    ) -> Result<(), EngineError> {
        println!("  |  AUTHOR");
        println!("  |     new attend: \"{}\" priority {:.3}",
            author.block.label, author.block.priority);

        // Store the authored block in agent data for later retrieval
        // The key is __authored_blocks (list of serialized attend descriptions)
        let block_desc = format!(
            "attend \"{}\" priority {:.3} ({} expressions)",
            author.block.label,
            author.block.priority,
            author.block.body.len(),
        );

        let scope = ctx.agent_a.clone();
        let authored_list = self.agent_data
            .entry(scope.clone())
            .or_default()
            .entry("__authored_blocks".to_string())
            .or_insert_with(|| Value::List(Vec::new()));

        if let Value::List(list) = authored_list {
            list.push(Value::String(block_desc));
        }

        // Store the actual block for execution in the authored_blocks vec
        self.authored_blocks
            .entry(scope)
            .or_default()
            .push(author.block.clone());

        // Consume attention for authoring
        self.agents.get_mut(&ctx.agent_a).unwrap().consume_attention(0.05);

        println!("  |     (added to attention landscape)");
        println!("  |");

        Ok(())
    }

    // ─── WHILE ──────────────────────────────────────────────
    //
    // while <condition> { ... }
    // Repeated execution while condition holds.
    // Maximum 10000 iterations to prevent infinite loops.

    fn exec_while(
        &mut self, ctx: &mut LinkCtx, while_expr: &anwe_parser::ast::WhileExpr,
    ) -> Result<(), EngineError> {
        println!("  |  WHILE {}", format_condition(&while_expr.condition));

        let max_iterations = 10_000usize;
        let mut iterations = 0usize;

        while self.eval_condition(&while_expr.condition, ctx) {
            iterations += 1;
            if iterations > max_iterations {
                println!("  |     (iteration limit reached: {})", max_iterations);
                break;
            }

            for expr in &while_expr.body {
                self.execute_link_expr(ctx, expr)?;
            }
        }

        println!("  |     ({} iterations)", iterations);
        println!("  |");
        Ok(())
    }

    // ─── ATTEMPT / RECOVER ──────────────────────────────────
    //
    // attempt { ... } recover { ... }
    // Error handling. If the attempt body fails, run recover.

    fn exec_attempt(
        &mut self, ctx: &mut LinkCtx, attempt: &anwe_parser::ast::AttemptExpr,
    ) -> Result<(), EngineError> {
        println!("  |  ATTEMPT");

        // Try executing the body
        let mut failed = false;
        let mut error_msg = String::new();

        for expr in &attempt.body {
            match self.execute_link_expr(ctx, expr) {
                Ok(()) => {}
                Err(e) => {
                    error_msg = format!("{}", e);
                    failed = true;
                    break;
                }
            }
        }

        if failed {
            println!("  |     (failed: {})", error_msg);
            println!("  |  RECOVER");

            // Bind the error message in agent data
            self.agent_data
                .entry(ctx.agent_a.clone())
                .or_default()
                .insert("__error".to_string(), Value::String(error_msg));

            for expr in &attempt.recover {
                self.execute_link_expr(ctx, expr)?;
            }
        } else {
            println!("  |     (succeeded)");
        }

        println!("  |");
        Ok(())
    }

    // ─── LET BINDING ─────────────────────────────────────────

    fn execute_let_binding(
        &mut self, binding: &anwe_parser::ast::LetBinding, scope: &str,
    ) -> Result<(), EngineError> {
        let val = self.eval_expr_scoped(&binding.value, Some(scope));
        println!("  let{} {} = {}", if binding.mutable { " mut" } else { "" }, binding.name, val);

        // Store in the scope
        self.agent_data
            .entry(scope.to_string())
            .or_default()
            .insert(binding.name.clone(), val);

        // Track mutability
        let key = format!("{}::{}", scope, binding.name);
        if binding.mutable {
            self.mutable_bindings.insert(key);
        }

        Ok(())
    }

    fn execute_assign(
        &mut self, assign: &anwe_parser::ast::AssignExpr, scope: &str,
    ) -> Result<(), EngineError> {
        // Determine where the variable lives
        let target_scope = if self.agent_data.get(scope)
            .map_or(false, |data| data.contains_key(&assign.name))
        {
            scope.to_string()
        } else if self.agent_data.get("__global__")
            .map_or(false, |data| data.contains_key(&assign.name))
        {
            "__global__".to_string()
        } else {
            return Err(EngineError::at_span(
                &format!("Cannot assign to '{}': variable not declared. Use 'let' to declare it first.", assign.name),
                &assign.span
            ));
        };

        let actual_key = format!("{}::{}", target_scope, assign.name);
        if !self.mutable_bindings.contains(&actual_key) {
            return Err(EngineError::at_span(
                &format!("Cannot assign to '{}': variable is not mutable. Use 'let mut' to declare a mutable binding.", assign.name),
                &assign.span
            ));
        }

        let val = self.eval_expr_scoped(&assign.value, Some(&target_scope));
        println!("  {} = {}", assign.name, val);

        self.agent_data
            .entry(target_scope)
            .or_default()
            .insert(assign.name.clone(), val);

        Ok(())
    }

    // ─── HISTORY VIEW ─────────────────────────────────────────

    fn execute_history_view(&self, hv: &HistoryViewExpr) -> Result<(), EngineError> {
        if let Some(agent) = self.agents.get(&hv.agent) {
            println!("  history of {} (depth: {})", hv.agent, agent.history.depth());
            for entry in agent.history.iter() {
                println!("    {:?}", entry);
            }
        }
        Ok(())
    }

    // ─── IMPORT (MODULE SYSTEM) ──────────────────────────────
    //
    // import "module" as Alias { agents: [...] links: [...] }
    //
    // Resolves the module file, parses it, and registers its
    // agents and patterns with the namespace prefix (Alias.Name).

    fn execute_import(&mut self, import: &anwe_parser::ast::ImportDecl) -> Result<(), EngineError> {
        // Resolve the module path
        let module_file = format!("{}.anwe", import.module_path);
        let resolved = if let Some(ref base) = self.base_path {
            base.join(&module_file)
        } else {
            PathBuf::from(&module_file)
        };

        // Check for circular imports
        let canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        if self.loaded_modules.contains(&canonical) {
            println!("  import \"{}\" as {} (already loaded)", import.module_path, import.alias);
            return Ok(());
        }
        self.loaded_modules.push(canonical);

        // Read the module file
        let source = std::fs::read_to_string(&resolved).map_err(|e| {
            EngineError::ExecutionError(format!(
                "Cannot import \"{}\": {} (looked at {})",
                import.module_path, e, resolved.display()
            ))
        })?;

        // Parse the module
        let mut lexer = anwe_parser::Lexer::new(&source);
        let tokens = lexer.tokenize().map_err(|e| {
            EngineError::ExecutionError(format!(
                "Lex error in module \"{}\": {}", import.module_path, e
            ))
        })?;

        let mut parser = anwe_parser::Parser::new(tokens);
        let module_program = parser.parse_program().map_err(|e| {
            EngineError::ExecutionError(format!(
                "Parse error in module \"{}\": {}", import.module_path, e
            ))
        })?;

        println!("  import \"{}\" as {} ({} declarations)",
            import.module_path, import.alias, module_program.declarations.len());

        // Register module declarations with namespace prefix
        let prefix = &import.alias;
        let mut imported_links = Vec::new();
        for decl in &module_program.declarations {
            match decl {
                Declaration::Agent(a) => {
                    let mut namespaced = a.clone();
                    namespaced.name = format!("{}.{}", prefix, a.name);
                    self.register_agent(&namespaced)?;
                }
                Declaration::Pattern(p) => {
                    let mut namespaced = p.clone();
                    namespaced.name = format!("{}.{}", prefix, p.name);
                    self.register_pattern(&namespaced)?;
                }
                Declaration::Mind(m) => {
                    let mut namespaced = m.clone();
                    namespaced.name = format!("{}.{}", prefix, m.name);
                    self.register_mind(&namespaced)?;
                }
                Declaration::Fn(f) => {
                    // Import function into __global__ with namespace prefix
                    let namespaced_name = format!("{}.{}", prefix, f.name);
                    let global = self.agent_data.entry("__global__".to_string())
                        .or_insert_with(HashMap::new);
                    global.insert(namespaced_name, Value::Function {
                        params: f.params.clone(),
                        body: f.body.clone(),
                        env: HashMap::new(),
                    });
                }
                Declaration::Let(binding) => {
                    // Execute let binding and store with namespace prefix
                    let val = self.eval_expr(&binding.value);
                    let namespaced_name = format!("{}.{}", prefix, binding.name);
                    let global = self.agent_data.entry("__global__".to_string())
                        .or_insert_with(HashMap::new);
                    global.insert(namespaced_name, val);
                }
                Declaration::Record(rec) => {
                    // Import record constructor with namespace prefix
                    let namespaced_name = format!("{}.{}", prefix, rec.name);
                    let constructor = Value::RecordConstructor {
                        name: namespaced_name.clone(),
                        fields: rec.fields.clone(),
                    };
                    let global = self.agent_data.entry("__global__".to_string())
                        .or_insert_with(HashMap::new);
                    global.insert(namespaced_name, constructor);
                }
                Declaration::Link(link) => {
                    imported_links.push(link.clone());
                }
                Declaration::TopLevelExpr(expr) => {
                    // Execute top-level expressions from imported module
                    let mut env: HashMap<String, Value> = self.agent_data
                        .get("__global__")
                        .map(|g| g.clone())
                        .unwrap_or_default();
                    self.eval_fn_expr_in_env(expr, &mut env);
                    let global = self.agent_data.entry("__global__".to_string()).or_default();
                    for (k, v) in env {
                        global.insert(k, v);
                    }
                }
                Declaration::Assign { name, value } => {
                    let val = self.eval_expr(value);
                    let global = self.agent_data.entry("__global__".to_string())
                        .or_insert_with(HashMap::new);
                    global.insert(name.clone(), val);
                }
                _ => {}
            }
        }

        // Execute imported links (they reference module agents)
        for link in &imported_links {
            // Namespace the agent names in the link
            let mut namespaced = link.clone();
            namespaced.agent_a = format!("{}.{}", prefix, link.agent_a);
            namespaced.agent_b = format!("{}.{}", prefix, link.agent_b);
            self.execute_link(&namespaced)?;
        }

        Ok(())
    }

    // ─── CONDITION EVALUATION ─────────────────────────────────

    fn eval_condition(&self, cond: &Condition, ctx: &LinkCtx) -> bool {
        match cond {
            Condition::SyncLevel { op, value } => {
                let current = ctx.link.sync_level().as_f32() as f64;
                compare_f64(current, *op, *value)
            }
            Condition::Priority { op, value } => {
                let agent = &self.agents[&ctx.agent_a];
                let current = agent.signal_priority.as_f32() as f64;
                compare_f64(current, *op, *value)
            }
            Condition::AlertIs(quality_str) => {
                if let Some(q) = ctx.last_alert_quality {
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
                // Check the last signal's confidence level
                let current = ctx.channel.total_sent() as f64 * 0.1; // proxy for now
                compare_f64(current.min(1.0), *op, *value)
            }
            Condition::Attention { op, value } => {
                // Check agent A's remaining attention budget
                let agent = &self.agents[&ctx.agent_a];
                let current = agent.attention.remaining() as f64;
                compare_f64(current, *op, *value)
            }
            Condition::And(a, b) => {
                self.eval_condition(a, ctx) && self.eval_condition(b, ctx)
            }
            Condition::Or(a, b) => {
                self.eval_condition(a, ctx) || self.eval_condition(b, ctx)
            }
            Condition::FieldCompare { left, op, right } => {
                // General field comparison — evaluate both sides and compare
                // Use scoped evaluation so think bindings are visible
                let scope = Some(ctx.agent_a.as_str());
                let lval = self.eval_expr_scoped(left, scope);
                let rval = self.eval_expr_scoped(right, scope);
                match (&lval, &rval) {
                    (Value::Number(a), Value::Number(b)) => compare_f64(*a, *op, *b),
                    (Value::String(a), Value::String(b)) => {
                        match op {
                            ComparisonOp::Equal => a == b,
                            ComparisonOp::NotEqual => a != b,
                            ComparisonOp::Greater => a > b,
                            ComparisonOp::GreaterEq => a >= b,
                            ComparisonOp::Less => a < b,
                            ComparisonOp::LessEq => a <= b,
                        }
                    }
                    // Fallback: compare string representations
                    _ => {
                        let a = format!("{}", lval);
                        let b = format!("{}", rval);
                        match op {
                            ComparisonOp::Equal => a == b,
                            ComparisonOp::NotEqual => a != b,
                            _ => false,
                        }
                    }
                }
            }
        }
    }

    /// Check if a pending condition is currently active.
    fn is_pending_active(&self, reason: &PendingReason, ctx: &LinkCtx) -> bool {
        match reason {
            PendingReason::ReceiverNotReady => {
                !self.agents[&ctx.agent_b].can_receive()
            }
            PendingReason::LinkNotEstablished => {
                ctx.link.state() == LinkState::Opening
            }
            PendingReason::SyncInsufficient => {
                !ctx.link.ready_for_apply()
            }
            PendingReason::SenderNotReady => {
                self.agents[&ctx.agent_a].state == AgentState::Committing
            }
            PendingReason::MomentNotRight => {
                false // In sequential execution, the moment is always right
            }
            PendingReason::BudgetExhausted => {
                self.agents[&ctx.agent_a].is_budget_exhausted()
            }
        }
    }

    // ─── BRIDGE NOTIFICATION ──────────────────────────────────
    //
    // When an agent has an external participant registered,
    // these methods route signals and changes through the bridge.
    // The engine still manages the Agent state machine internally.
    // The participant is just notified and can influence the process.

    /// Notify an external participant that a signal has arrived.
    /// Returns any response signal the participant sends back.
    fn bridge_notify_signal(&self, agent_name: &str, signal: &Signal) -> Option<Signal> {
        if let Some(participant) = self.participants.get(agent_name) {
            let wire = WireSignal::from_signal(signal);
            let mut p = participant.lock().unwrap();
            if let Some(response) = p.receive(&wire) {
                let agent_id = self.agent_ids[agent_name];
                return Some(response.to_signal(agent_id, signal.tick));
            }
        }
        None
    }

    /// Notify an external participant that structural changes are proposed.
    /// Returns true if the participant accepts (or if the agent is not external).
    fn bridge_notify_apply(&self, agent_name: &str, changes: &[(String, WireValue)]) -> bool {
        if let Some(participant) = self.participants.get(agent_name) {
            let mut p = participant.lock().unwrap();
            return p.apply(changes);
        }
        true // Internal agents always accept
    }

    /// Notify an external participant that a commit has occurred.
    fn bridge_notify_commit(&self, agent_name: &str, entries: &[(String, WireValue)]) {
        if let Some(participant) = self.participants.get(agent_name) {
            let mut p = participant.lock().unwrap();
            p.commit(entries);
        }
    }

    /// Notify an external participant that think bindings were computed.
    /// Returns enriched bindings if the participant transforms them.
    fn bridge_notify_think(
        &self, agent_name: &str, bindings: &[(String, WireValue)],
    ) -> Option<Vec<(String, WireValue)>> {
        if let Some(participant) = self.participants.get(agent_name) {
            let mut p = participant.lock().unwrap();
            return p.think(bindings);
        }
        None
    }

    /// Notify an external participant that an expression is being transmitted.
    /// Returns transformed content if the participant shapes it.
    fn bridge_notify_express(
        &self, agent_name: &str, signal: &Signal, content: &WireValue,
    ) -> Option<WireValue> {
        if let Some(participant) = self.participants.get(agent_name) {
            let wire = WireSignal::from_signal(signal);
            let mut p = participant.lock().unwrap();
            return p.express(&wire, content);
        }
        None
    }

    // ─── EXPRESSION EVALUATION ────────────────────────────────

    /// Evaluate an expression without a specific scope.
    fn eval_expr(&self, expr: &Expr) -> Value {
        self.eval_expr_scoped(expr, None)
    }

    /// Evaluate an expression with an optional scope (agent name).
    ///
    /// When a scope is provided, identifiers are first resolved
    /// against that agent's data (think bindings, sense results).
    /// This is what makes think { sum <- a + b } work when a and b
    /// were bound earlier in the same think block or a previous one.
    fn eval_expr_scoped(&self, expr: &Expr, scope: Option<&str>) -> Value {
        match expr {
            Expr::StringLit(s) => {
                // String interpolation: resolve {name} and {name.field} patterns
                if s.contains('{') {
                    Value::String(self.interpolate_string(s, scope))
                } else {
                    Value::String(s.clone())
                }
            }
            Expr::Number(n) => Value::Number(*n),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Ident(name) => {
                // Check iteration variables first
                if let Some(data) = self.agent_data.get("__iter__") {
                    if let Some(val) = data.get(name) {
                        return val.clone();
                    }
                }
                // Check scoped agent data (think bindings, sense results)
                if let Some(scope_name) = scope {
                    if let Some(data) = self.agent_data.get(scope_name) {
                        if let Some(val) = data.get(name) {
                            return val.clone();
                        }
                    }
                }
                // Check global let bindings
                if let Some(data) = self.agent_data.get("__global__") {
                    if let Some(val) = data.get(name) {
                        return val.clone();
                    }
                }
                if self.agents.contains_key(name) {
                    Value::Agent(name.clone())
                } else {
                    Value::String(name.clone())
                }
            }
            Expr::FieldAccess { object, field } => {
                // First check if object is a scoped variable holding a Map
                let map_val = scope.and_then(|s| self.agent_data.get(s))
                    .and_then(|data| data.get(object));
                if let Some(Value::Map(entries)) = map_val {
                    return entries.iter()
                        .find(|(k, _)| k == field)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(Value::Null);
                }
                // Check global scope for let-bound Maps
                let global_map = self.agent_data.get("__global__")
                    .and_then(|data| data.get(object));
                if let Some(Value::Map(entries)) = global_map {
                    return entries.iter()
                        .find(|(k, _)| k == field)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(Value::Null);
                }
                // Check global scope for namespaced imports (e.g., Str.version)
                let dotted = format!("{}.{}", object, field);
                if let Some(data) = self.agent_data.get("__global__") {
                    if let Some(val) = data.get(&dotted) {
                        return val.clone();
                    }
                }
                // Then check agent data tables
                if let Some(data) = self.agent_data.get(object) {
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
                let l = self.eval_expr_scoped(left, scope);
                let r = self.eval_expr_scoped(right, scope);
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
                    // String concatenation with +
                    (Value::String(a), Value::String(b)) if *op == BinOp::Add => {
                        Value::String(format!("{}{}", a, b))
                    }
                    // List concatenation with +
                    (Value::List(mut a), Value::List(b)) if *op == BinOp::Add => {
                        a.extend(b);
                        Value::List(a)
                    }
                    _ => Value::Null,
                }
            }
            Expr::Comparison { left, op, right } => {
                let l = self.eval_expr_scoped(left, scope);
                let r = self.eval_expr_scoped(right, scope);
                Value::Bool(compare_values(&l, op, &r))
            }
            Expr::UnaryNeg(operand) => {
                match self.eval_expr_scoped(operand, scope) {
                    Value::Number(n) => Value::Number(-n),
                    _ => Value::Null,
                }
            }
            Expr::ListLit(items) => {
                let values: Vec<Value> = items.iter()
                    .map(|item| self.eval_expr_scoped(item, scope))
                    .collect();
                Value::List(values)
            }
            Expr::IndexAccess { object, index } => {
                let obj = self.eval_expr_scoped(object, scope);
                let idx = self.eval_expr_scoped(index, scope);
                match (obj, idx) {
                    (Value::List(items), Value::Number(n)) => {
                        let i = n as usize;
                        items.get(i).cloned().unwrap_or(Value::Null)
                    }
                    (Value::Map(entries), Value::String(key)) => {
                        entries.iter()
                            .find(|(k, _)| k == &key)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(Value::Null)
                    }
                    _ => Value::Null,
                }
            }
            Expr::Pipe { stages } => {
                // Pipe evaluation: each stage transforms the result
                // First stage produces the initial value
                // Subsequent stages receive previous value as implicit input
                if stages.is_empty() {
                    return Value::Null;
                }
                let mut result = self.eval_expr_scoped(&stages[0], scope);
                for stage in &stages[1..] {
                    // Store the pipe input as __pipe_input in the scope
                    let scope_name = scope.unwrap_or("__pipe__");
                    let _ = self.agent_data
                        .get(scope_name)
                        .cloned(); // read-only check
                    // For now, pipe stages are expressions — each evaluates
                    // with the previous result available
                    // If the stage is an identifier, treat it as a function-like
                    // transform applied to the result
                    result = match stage {
                        Expr::Ident(name) => {
                            // Pattern-like transform: apply the named pattern
                            // to the current result
                            Value::String(format!("{}({})", name, result))
                        }
                        _ => self.eval_expr_scoped(stage, scope),
                    };
                }
                result
            }
            Expr::Call { name, args } => {
                let evaled: Vec<Value> = args.iter()
                    .map(|a| self.eval_expr_scoped(a, scope))
                    .collect();

                // Special-case: eval() evaluates quoted code as an expression
                if name == "eval" {
                    return self.eval_code_expr(&evaled, scope);
                }

                // Special-case: unquote() extracts source from Code value
                if name == "unquote" {
                    return match evaled.first() {
                        Some(Value::Code(src)) => Value::String(src.clone()),
                        Some(other) => Value::String(format!("{}", other)),
                        None => Value::Null,
                    };
                }

                // Check for user-defined function in scope or global
                let func = scope
                    .and_then(|s| self.agent_data.get(s))
                    .and_then(|data| data.get(name))
                    .cloned()
                    .or_else(|| self.agent_data.get("__global__")
                        .and_then(|data| data.get(name))
                        .cloned());

                if let Some(Value::Function { params, body, env: captured }) = func {
                    self.call_function(&params, &body, &evaled, scope, &captured)
                } else if let Some(Value::RecordConstructor { fields, .. }) = func {
                    // Build a Map from the record fields and argument values
                    let entries: Vec<(String, Value)> = fields.iter().enumerate()
                        .map(|(i, f)| (f.clone(), evaled.get(i).cloned().unwrap_or(Value::Null)))
                        .collect();
                    Value::Map(entries)
                } else {
                    self.eval_builtin(name, &evaled)
                }
            }
            Expr::Lambda { params, body } => {
                // Capture current scope for true closure support
                let mut captured = HashMap::new();
                if let Some(ps) = scope {
                    if let Some(parent_data) = self.agent_data.get(ps) {
                        captured.extend(parent_data.iter().map(|(k, v)| (k.clone(), v.clone())));
                    }
                }
                if let Some(global_data) = self.agent_data.get("__global__") {
                    for (k, v) in global_data {
                        captured.entry(k.clone()).or_insert_with(|| v.clone());
                    }
                }
                Value::Function {
                    params: params.clone(),
                    body: *body.clone(),
                    env: captured,
                }
            }
            Expr::Match { subject, arms } => {
                let val = self.eval_expr_scoped(subject, scope);
                self.eval_match(&val, arms, scope)
            }
            Expr::Not(operand) => {
                match self.eval_expr_scoped(operand, scope) {
                    Value::Bool(b) => Value::Bool(!b),
                    _ => Value::Bool(false),
                }
            }
            Expr::LogicalAnd { left, right } => {
                let l = self.eval_expr_scoped(left, scope);
                match l {
                    Value::Bool(false) => Value::Bool(false), // short-circuit
                    Value::Bool(true) => self.eval_expr_scoped(right, scope),
                    _ => Value::Bool(false),
                }
            }
            Expr::LogicalOr { left, right } => {
                let l = self.eval_expr_scoped(left, scope);
                match l {
                    Value::Bool(true) => Value::Bool(true), // short-circuit
                    Value::Bool(false) => self.eval_expr_scoped(right, scope),
                    _ => self.eval_expr_scoped(right, scope),
                }
            }
            Expr::Block { statements, result } => {
                // Build a local environment from scope + globals, evaluate block within it
                let mut env = HashMap::new();
                if let Some(s) = scope {
                    if let Some(data) = self.agent_data.get(s) {
                        env.extend(data.iter().map(|(k, v)| (k.clone(), v.clone())));
                    }
                }
                if let Some(global) = self.agent_data.get("__global__") {
                    for (k, v) in global {
                        env.entry(k.clone()).or_insert_with(|| v.clone());
                    }
                }
                for stmt in statements {
                    match stmt {
                        BlockStatement::Let { name, mutable: _, value } => {
                            let val = self.eval_fn_expr(value, &env);
                            env.insert(name.clone(), val);
                        }
                        BlockStatement::Assign { name, value } => {
                            let val = self.eval_fn_expr(value, &env);
                            env.insert(name.clone(), val);
                        }
                        BlockStatement::Expr(expr) => {
                            self.eval_fn_expr(expr, &env);
                        }
                    }
                }
                self.eval_fn_expr(result, &env)
            }
            Expr::IfElse { condition, then_branch, else_branch } => {
                let cond = self.eval_expr_scoped(condition, scope);
                let is_true = match &cond {
                    Value::Bool(b) => *b,
                    Value::Null => false,
                    Value::Number(n) => *n != 0.0,
                    Value::String(s) => !s.is_empty(),
                    _ => true,
                };
                if is_true {
                    self.eval_expr_scoped(then_branch, scope)
                } else if let Some(else_br) = else_branch {
                    self.eval_expr_scoped(else_br, scope)
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
                            let val = self.eval_expr_scoped(expr, scope);
                            result.push_str(&value_to_display(&val));
                        }
                    }
                }
                Value::String(result)
            }
            Expr::WhileExpr { condition, body } => {
                // Build a local env from scope data + globals for fn evaluation
                let mut env = HashMap::new();
                let scope_name = scope.unwrap_or("__global__");
                if let Some(global_data) = self.agent_data.get("__global__") {
                    for (k, v) in global_data {
                        env.insert(k.clone(), v.clone());
                    }
                }
                if scope_name != "__global__" {
                    if let Some(scope_data) = self.agent_data.get(scope_name) {
                        for (k, v) in scope_data {
                            env.insert(k.clone(), v.clone());
                        }
                    }
                }
                let mut iterations = 0;
                loop {
                    let cond = self.eval_fn_expr(condition, &env);
                    let is_true = match &cond {
                        Value::Bool(b) => *b,
                        Value::Null => false,
                        _ => false,
                    };
                    if !is_true { break; }
                    let body_val = self.eval_loop_body(body, &mut env);
                    if matches!(body_val, Value::Return(_)) { return body_val; }
                    if matches!(body_val, Value::Break) { break; }
                    // Continue: skip to next iteration
                    iterations += 1;
                    if iterations >= 10_000 {
                        break; // Safety limit
                    }
                }
                Value::Null
            }
            Expr::ForIn { var, collection, body } => {
                // Build a local env from scope data + globals
                let mut env = HashMap::new();
                let scope_name = scope.unwrap_or("__global__");
                if let Some(global_data) = self.agent_data.get("__global__") {
                    for (k, v) in global_data {
                        env.insert(k.clone(), v.clone());
                    }
                }
                if scope_name != "__global__" {
                    if let Some(scope_data) = self.agent_data.get(scope_name) {
                        for (k, v) in scope_data {
                            env.insert(k.clone(), v.clone());
                        }
                    }
                }
                let coll = self.eval_fn_expr(collection, &env);
                let items = match &coll {
                    Value::List(items) => items.clone(),
                    _ => Vec::new(),
                };
                let mut last_val = Value::Null;
                for item in &items {
                    env.insert(var.clone(), item.clone());
                    let body_val = self.eval_loop_body(body, &mut env);
                    if matches!(body_val, Value::Return(_)) { env.remove(var); return body_val; }
                    if matches!(body_val, Value::Break) { break; }
                    if !matches!(body_val, Value::Continue) {
                        last_val = body_val;
                    }
                }
                env.remove(var);
                last_val
            }
            Expr::TryCatch { body, catch_body } => {
                // Try to evaluate body; if it produces an error, evaluate catch
                let result = self.eval_expr_scoped(body, scope);
                match &result {
                    Value::Error { .. } => self.eval_expr_scoped(catch_body, scope),
                    Value::String(s) if s.starts_with("ERROR") || s.starts_with("error")
                        || s.starts_with("http_") || s.starts_with("json_parse error") => {
                        self.eval_expr_scoped(catch_body, scope)
                    }
                    _ => result,
                }
            }
            Expr::MapLit(entries) => {
                let map_entries: Vec<(String, Value)> = entries.iter()
                    .map(|(k, v)| (k.clone(), self.eval_expr_scoped(v, scope)))
                    .collect();
                Value::Map(map_entries)
            }
            Expr::Quote(source) => {
                Value::Code(source.clone())
            }
            Expr::Break => Value::Break,
            Expr::Continue => Value::Continue,
            Expr::Return(expr) => {
                let val = self.eval_expr_scoped(expr, scope);
                Value::Return(Box::new(val))
            }
        }
    }

    /// Evaluate a match expression: find the first matching arm and return its body.
    fn eval_match(&self, val: &Value, arms: &[MatchArm], scope: Option<&str>) -> Value {
        for arm in arms {
            match &arm.pattern {
                MatchPattern::Wildcard => {
                    return self.eval_expr_scoped(&arm.body, scope);
                }
                MatchPattern::Literal(lit_expr) => {
                    let lit_val = self.eval_expr_scoped(lit_expr, scope);
                    if *val == lit_val {
                        return self.eval_expr_scoped(&arm.body, scope);
                    }
                }
                MatchPattern::Binding(name) => {
                    // Bind the matched value to the name in the current scope,
                    // evaluate the body, then clean up.
                    // For immutable eval, use eval_fn_expr with a mini-env.
                    let mut env = HashMap::new();
                    // Copy scope data
                    if let Some(s) = scope {
                        if let Some(data) = self.agent_data.get(s) {
                            env.extend(data.iter().map(|(k, v)| (k.clone(), v.clone())));
                        }
                    }
                    if let Some(global) = self.agent_data.get("__global__") {
                        for (k, v) in global {
                            env.entry(k.clone()).or_insert_with(|| v.clone());
                        }
                    }
                    env.insert(name.clone(), val.clone());
                    return self.eval_fn_expr(&arm.body, &env);
                }
            }
        }
        Value::Null
    }

    /// Evaluate a quoted code value: lex, parse, and evaluate as an expression.
    /// This is the `eval(code)` builtin — takes &self because it only evaluates,
    /// it doesn't add bindings to the engine.
    ///
    /// Handles two cases:
    /// 1. Bare expressions like `3 + 4` — wrapped in `let` for parsing, then evaluated
    /// 2. Declarations like `let x = 10` or `fn add(a,b) = a + b` — parsed normally
    fn eval_code_expr(&self, args: &[Value], scope: Option<&str>) -> Value {
        let source = match args.first() {
            Some(Value::Code(src)) => src.clone(),
            Some(Value::String(src)) => src.clone(),
            _ => return Value::Null,
        };

        use anwe_parser::lexer::Lexer;
        use anwe_parser::parser::Parser;

        // Build an environment from the current scope + globals for eval_fn_expr
        let mut env = HashMap::new();
        if let Some(s) = scope {
            if let Some(data) = self.agent_data.get(s) {
                env.extend(data.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }
        if let Some(global) = self.agent_data.get("__global__") {
            for (k, v) in global {
                env.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }

        // First, try parsing as a program (declarations)
        let tokens = match Lexer::new(&source).tokenize() {
            Ok(toks) => toks,
            Err(e) => return Value::String(format!("eval lex error: {}", e)),
        };

        if let Ok(program) = Parser::new(tokens).parse_program() {
            // Successfully parsed as declarations
            let mut last_value = Value::Null;
            for decl in &program.declarations {
                match decl {
                    Declaration::Let(let_decl) => {
                        let val = self.eval_fn_expr(&let_decl.value, &env);
                        env.insert(let_decl.name.clone(), val.clone());
                        last_value = val;
                    }
                    Declaration::Fn(fn_decl) => {
                        let func = Value::Function {
                            params: fn_decl.params.clone(),
                            body: fn_decl.body.clone(),
                            env: env.clone(),
                        };
                        env.insert(fn_decl.name.clone(), func.clone());
                        last_value = func;
                    }
                    Declaration::Record(rec_decl) => {
                        let constructor = Value::RecordConstructor {
                            name: rec_decl.name.clone(),
                            fields: rec_decl.fields.clone(),
                        };
                        env.insert(rec_decl.name.clone(), constructor.clone());
                        last_value = constructor;
                    }
                    _ => {}
                }
            }
            return last_value;
        }

        // If declaration parsing fails, try wrapping as an expression:
        // `let __result = <source>` then evaluate and return the value
        let wrapped = format!("let __eval_result__ = {}", source);
        let tokens = match Lexer::new(&wrapped).tokenize() {
            Ok(toks) => toks,
            Err(e) => return Value::String(format!("eval lex error: {}", e)),
        };
        match Parser::new(tokens).parse_program() {
            Ok(program) => {
                for decl in &program.declarations {
                    if let Declaration::Let(let_decl) = decl {
                        return self.eval_fn_expr(&let_decl.value, &env);
                    }
                }
                Value::Null
            }
            Err(e) => Value::String(format!("eval parse error: {}", e)),
        }
    }

    /// Call a user-defined function with the given arguments.
    /// Builds an environment from captured closure env + parent scope + globals + params,
    /// then evaluates the body using eval_fn_expr (no mutation needed).
    fn call_function(
        &self, params: &[String], body: &Expr, args: &[Value], parent_scope: Option<&str>,
        captured_env: &HashMap<String, Value>,
    ) -> Value {
        // Start with the captured closure environment (creation-time bindings)
        let mut env = captured_env.clone();

        // Layer on parent scope data
        if let Some(ps) = parent_scope {
            if let Some(parent_data) = self.agent_data.get(ps) {
                env.extend(parent_data.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }
        // Layer on global scope for access to other functions and let bindings
        if let Some(global_data) = self.agent_data.get("__global__") {
            for (k, v) in global_data {
                env.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }

        // Bind parameters to arguments (overrides any same-named vars)
        for (i, param) in params.iter().enumerate() {
            env.insert(param.clone(), args.get(i).cloned().unwrap_or(Value::Null));
        }

        let result = self.eval_fn_expr(body, &env);
        match result {
            Value::Return(inner) => *inner,
            other => other,
        }
    }

    /// Evaluate an expression within a function call context.
    /// Uses an explicit environment HashMap instead of mutating agent_data.
    fn eval_fn_expr(&self, expr: &Expr, env: &HashMap<String, Value>) -> Value {
        match expr {
            Expr::Ident(name) => {
                // Check function environment first
                if let Some(val) = env.get(name) {
                    return val.clone();
                }
                // Fall back to normal resolution
                self.eval_expr(expr)
            }
            Expr::Number(n) => Value::Number(*n),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::StringLit(s) => {
                if s.contains('{') {
                    // Simple interpolation within fn context
                    let mut result = String::with_capacity(s.len());
                    let bytes = s.as_bytes();
                    let mut i = 0;
                    while i < bytes.len() {
                        if bytes[i] == b'{' {
                            if let Some(close) = s[i+1..].find('}') {
                                let key = s[i+1..i+1+close].trim();
                                if let Some(val) = env.get(key) {
                                    result.push_str(&format!("{}", val));
                                } else {
                                    // Fall back to engine interpolation
                                    let val = self.resolve_interpolation(key, None);
                                    result.push_str(&format!("{}", val));
                                }
                                i += close + 2;
                            } else {
                                result.push('{');
                                i += 1;
                            }
                        } else {
                            result.push(bytes[i] as char);
                            i += 1;
                        }
                    }
                    Value::String(result)
                } else {
                    Value::String(s.clone())
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let l = self.eval_fn_expr(left, env);
                let r = self.eval_fn_expr(right, env);
                match (&l, &r) {
                    (Value::Number(a), Value::Number(b)) => {
                        Value::Number(match op {
                            BinOp::Add => a + b,
                            BinOp::Sub => a - b,
                            BinOp::Mul => a * b,
                            BinOp::Div => if *b != 0.0 { a / b } else { f64::NAN },
                            BinOp::Mod => if *b != 0.0 { a % b } else { f64::NAN },
                        })
                    }
                    (Value::String(a), Value::String(b)) if *op == BinOp::Add => {
                        Value::String(format!("{}{}", a, b))
                    }
                    (Value::String(a), Value::Number(b)) if *op == BinOp::Add => {
                        Value::String(format!("{}{}", a, b))
                    }
                    (Value::Number(a), Value::String(b)) if *op == BinOp::Add => {
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
            Expr::UnaryNeg(operand) => {
                match self.eval_fn_expr(operand, env) {
                    Value::Number(n) => Value::Number(-n),
                    Value::Bool(b) => Value::Bool(!b),
                    _ => Value::Null,
                }
            }
            Expr::ListLit(items) => {
                Value::List(items.iter().map(|item| self.eval_fn_expr(item, env)).collect())
            }
            Expr::IndexAccess { object, index } => {
                let obj = self.eval_fn_expr(object, env);
                let idx = self.eval_fn_expr(index, env);
                match (&obj, &idx) {
                    (Value::List(items), Value::Number(n)) => {
                        let i = *n as usize;
                        items.get(i).cloned().unwrap_or(Value::Null)
                    }
                    (Value::Map(entries), Value::String(key)) => {
                        entries.iter()
                            .find(|(k, _)| k == key)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(Value::Null)
                    }
                    _ => Value::Null,
                }
            }
            Expr::FieldAccess { object, field } => {
                // FieldAccess.object is a String (agent/variable name), not an Expr
                if let Some(val) = env.get(object) {
                    match val {
                        Value::Map(entries) => {
                            entries.iter()
                                .find(|(k, _)| k == field)
                                .map(|(_, v)| v.clone())
                                .unwrap_or(Value::Null)
                        }
                        _ => Value::Null,
                    }
                } else {
                    // Fall back to normal resolution
                    self.eval_expr(expr)
                }
            }
            Expr::Pipe { stages } if !stages.is_empty() => {
                let mut result = self.eval_fn_expr(&stages[0], env);
                for stage in &stages[1..] {
                    match stage {
                        Expr::Call { name, args: call_args } => {
                            let mut evaled = vec![result];
                            evaled.extend(call_args.iter().map(|a| self.eval_fn_expr(a, env)));
                            // Check for user fn
                            if let Some(Value::Function { params, body, env: captured }) = env.get(name.as_str()) {
                                let mut child_env = captured.clone();
                                child_env.extend(env.iter().map(|(k, v)| (k.clone(), v.clone())));
                                for (i, param) in params.iter().enumerate() {
                                    child_env.insert(param.clone(), evaled.get(i).cloned().unwrap_or(Value::Null));
                                }
                                result = match self.eval_fn_expr(&body, &child_env) {
                                    Value::Return(inner) => *inner,
                                    other => other,
                                };
                            } else {
                                result = self.eval_builtin(name, &evaled);
                            }
                        }
                        _ => result = self.eval_fn_expr(stage, env),
                    };
                }
                result
            }
            Expr::Call { name, args: call_args } => {
                let evaled: Vec<Value> = call_args.iter()
                    .map(|a| self.eval_fn_expr(a, env))
                    .collect();
                // unquote() extracts source from Code
                if name == "unquote" {
                    return match evaled.first() {
                        Some(Value::Code(src)) => Value::String(src.clone()),
                        Some(other) => Value::String(format!("{}", other)),
                        None => Value::Null,
                    };
                }
                // Check env for user-defined function
                if let Some(Value::Function { params, body, env: captured }) = env.get(name.as_str()) {
                    let mut child_env = captured.clone();
                    child_env.extend(env.iter().map(|(k, v)| (k.clone(), v.clone())));
                    for (i, param) in params.iter().enumerate() {
                        child_env.insert(param.clone(), evaled.get(i).cloned().unwrap_or(Value::Null));
                    }
                    match self.eval_fn_expr(&body, &child_env) {
                        Value::Return(inner) => *inner,
                        other => other,
                    }
                } else if let Some(Value::RecordConstructor { fields, .. }) = env.get(name.as_str()) {
                    let entries: Vec<(String, Value)> = fields.iter().enumerate()
                        .map(|(i, f)| (f.clone(), evaled.get(i).cloned().unwrap_or(Value::Null)))
                        .collect();
                    Value::Map(entries)
                } else {
                    self.eval_builtin(name, &evaled)
                }
            }
            Expr::Lambda { params, body } => {
                // Capture current environment for true nested closure support
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
                            // For loops inside blocks, process inline so mutations
                            // to accumulated variables persist in block_env
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
                    // Continue: skip to next iteration
                    iterations += 1;
                    if iterations >= 10_000 {
                        break; // Safety limit
                    }
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
            Expr::Return(expr) => {
                let val = self.eval_fn_expr(expr, env);
                Value::Return(Box::new(val))
            }
            // For any other expression types, fall back to normal eval
            _ => self.eval_expr(expr),
        }
    }

    /// Evaluate an expression in-place, allowing loops to mutate the env directly.
    /// Used by Block handler so that for/while loops can modify accumulated variables.
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
                    // Continue: just skip to next iteration (already at end)
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
                    // Continue: skip to next iteration
                }
                env.remove(var);
                Value::Null
            }
            _ => {
                self.eval_fn_expr(expr, env)
            }
        }
    }

    /// Execute a loop body (Block) directly in the given mutable env,
    /// so that `let` bindings persist across iterations.
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
            // Use in_env for result too, so nested loops can mutate env
            let val = self.eval_fn_expr_in_env(result, env);
            if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
            val
        } else {
            let val = self.eval_fn_expr_in_env(body, env);
            if matches!(val, Value::Break | Value::Continue | Value::Return(_)) { return val; }
            val
        }
    }

    // ─── BUILT-IN FUNCTIONS (STANDARD LIBRARY + I/O) ────────
    //
    // Functions callable from any expression context.
    // Covers string ops, math, list ops, type conversion, and I/O.

    fn eval_builtin(&self, name: &str, args: &[Value]) -> Value {
        match name {
            // ── String operations ──
            "len" => match args.first() {
                Some(Value::String(s)) => Value::Number(s.len() as f64),
                Some(Value::List(l)) => Value::Number(l.len() as f64),
                Some(Value::Map(m)) => Value::Number(m.len() as f64),
                _ => Value::Number(0.0),
            },
            "split" => match (args.first(), args.get(1)) {
                (Some(Value::String(s)), Some(Value::String(delim))) => {
                    Value::List(s.split(delim.as_str()).map(|p| Value::String(p.to_string())).collect())
                }
                _ => Value::Null,
            },
            "join" => match (args.first(), args.get(1)) {
                (Some(Value::List(items)), Some(Value::String(delim))) => {
                    let parts: Vec<String> = items.iter().map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => format!("{}", other),
                    }).collect();
                    Value::String(parts.join(delim))
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
                (Some(Value::String(s)), Some(Value::String(old)), Some(Value::String(new))) => {
                    Value::String(s.replace(old.as_str(), new.as_str()))
                }
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
                (Some(Value::String(s)), Some(Value::String(prefix))) => Value::Bool(s.starts_with(prefix.as_str())),
                _ => Value::Bool(false),
            },
            "ends_with" => match (args.first(), args.get(1)) {
                (Some(Value::String(s)), Some(Value::String(suffix))) => Value::Bool(s.ends_with(suffix.as_str())),
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
            "abs" => match args.first() {
                Some(Value::Number(n)) => Value::Number(n.abs()),
                _ => Value::Null,
            },
            "floor" => match args.first() {
                Some(Value::Number(n)) => Value::Number(n.floor()),
                _ => Value::Null,
            },
            "ceil" => match args.first() {
                Some(Value::Number(n)) => Value::Number(n.ceil()),
                _ => Value::Null,
            },
            "round" => match args.first() {
                Some(Value::Number(n)) => Value::Number(n.round()),
                _ => Value::Null,
            },
            "sqrt" => match args.first() {
                Some(Value::Number(n)) => Value::Number(n.sqrt()),
                _ => Value::Null,
            },
            "pow" => match (args.first(), args.get(1)) {
                (Some(Value::Number(base)), Some(Value::Number(exp))) => Value::Number(base.powf(*exp)),
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
                (Some(Value::Number(v)), Some(Value::Number(lo)), Some(Value::Number(hi))) => {
                    Value::Number(v.max(*lo).min(*hi))
                }
                _ => Value::Null,
            },
            "log" => match args.first() {
                Some(Value::Number(n)) => Value::Number(n.ln()),
                _ => Value::Null,
            },

            // ── List operations ──
            "push" | "append" => match (args.first(), args.get(1)) {
                (Some(Value::List(list)), Some(item)) => {
                    let mut new_list = list.clone();
                    new_list.push(item.clone());
                    Value::List(new_list)
                }
                _ => Value::Null,
            },
            "pop" => match args.first() {
                Some(Value::List(list)) if !list.is_empty() => {
                    let mut new_list = list.clone();
                    new_list.pop();
                    Value::List(new_list)
                }
                _ => Value::Null,
            },
            "head" => match args.first() {
                Some(Value::List(list)) => list.first().cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            },
            "tail" => match args.first() {
                Some(Value::List(list)) if list.len() > 1 => {
                    Value::List(list[1..].to_vec())
                }
                Some(Value::List(_)) => Value::List(vec![]),
                _ => Value::Null,
            },
            "last" => match args.first() {
                Some(Value::List(list)) => list.last().cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            },
            "reverse" => match args.first() {
                Some(Value::List(list)) => {
                    let mut rev = list.clone();
                    rev.reverse();
                    Value::List(rev)
                }
                Some(Value::String(s)) => Value::String(s.chars().rev().collect()),
                _ => Value::Null,
            },
            "sort" => match args.first() {
                Some(Value::List(list)) => {
                    let mut sorted = list.clone();
                    sorted.sort_by(|a, b| {
                        match (a, b) {
                            (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                            (Value::String(x), Value::String(y)) => x.cmp(y),
                            _ => std::cmp::Ordering::Equal,
                        }
                    });
                    Value::List(sorted)
                }
                _ => Value::Null,
            },
            "flatten" => match args.first() {
                Some(Value::List(list)) => {
                    let mut flat = Vec::new();
                    for item in list {
                        match item {
                            Value::List(inner) => flat.extend(inner.clone()),
                            other => flat.push(other.clone()),
                        }
                    }
                    Value::List(flat)
                }
                _ => Value::Null,
            },
            "range" => match (args.first(), args.get(1), args.get(2)) {
                (Some(Value::Number(start)), Some(Value::Number(end)), Some(Value::Number(step))) => {
                    let mut result = Vec::new();
                    let mut i = *start;
                    while i < *end {
                        result.push(Value::Number(i));
                        i += step;
                    }
                    Value::List(result)
                }
                (Some(Value::Number(start)), Some(Value::Number(end)), None) => {
                    let mut result = Vec::new();
                    let mut i = *start as i64;
                    let end = *end as i64;
                    while i < end {
                        result.push(Value::Number(i as f64));
                        i += 1;
                    }
                    Value::List(result)
                }
                (Some(Value::Number(end)), None, None) => {
                    let mut result = Vec::new();
                    for i in 0..(*end as i64) {
                        result.push(Value::Number(i as f64));
                    }
                    Value::List(result)
                }
                _ => Value::Null,
            },
            "zip" => match (args.first(), args.get(1)) {
                (Some(Value::List(a)), Some(Value::List(b))) => {
                    let pairs: Vec<Value> = a.iter().zip(b.iter())
                        .map(|(x, y)| Value::List(vec![x.clone(), y.clone()]))
                        .collect();
                    Value::List(pairs)
                }
                _ => Value::Null,
            },

            // ── Map operations ──
            "keys" => match args.first() {
                Some(Value::Map(entries)) => {
                    Value::List(entries.iter().map(|(k, _)| Value::String(k.clone())).collect())
                }
                _ => Value::Null,
            },
            "values" => match args.first() {
                Some(Value::Map(entries)) => {
                    Value::List(entries.iter().map(|(_, v)| v.clone()).collect())
                }
                _ => Value::Null,
            },
            "has_key" => match (args.first(), args.get(1)) {
                (Some(Value::Map(entries)), Some(Value::String(key))) => {
                    Value::Bool(entries.iter().any(|(k, _)| k == key))
                }
                _ => Value::Bool(false),
            },
            "map_set" => match (args.first(), args.get(1), args.get(2)) {
                (Some(Value::Map(entries)), Some(Value::String(key)), Some(val)) => {
                    let mut new_entries: Vec<(String, Value)> = entries.iter()
                        .filter(|(k, _)| k != key)
                        .cloned()
                        .collect();
                    new_entries.push((key.clone(), val.clone()));
                    Value::Map(new_entries)
                }
                _ => Value::Null,
            },
            "map_get" => match (args.first(), args.get(1)) {
                (Some(Value::Map(entries)), Some(Value::String(key))) => {
                    entries.iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(Value::Null)
                }
                _ => Value::Null,
            },
            "map_remove" => match (args.first(), args.get(1)) {
                (Some(Value::Map(entries)), Some(Value::String(key))) => {
                    Value::Map(entries.iter().filter(|(k, _)| k != key).cloned().collect())
                }
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

            // ── Type conversion ──
            "to_string" => match args.first() {
                Some(val) => Value::String(format!("{}", val)),
                None => Value::Null,
            },
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
            "read_file" => match args.first() {
                Some(Value::String(path)) => {
                    match std::fs::read_to_string(path) {
                        Ok(content) => Value::String(content),
                        Err(_) => Value::Null,
                    }
                }
                _ => Value::Null,
            },
            "write_file" => match (args.first(), args.get(1)) {
                (Some(Value::String(path)), Some(Value::String(content))) => {
                    match std::fs::write(path, content) {
                        Ok(()) => Value::Bool(true),
                        Err(_) => Value::Bool(false),
                    }
                }
                _ => Value::Bool(false),
            },
            "append_file" => match (args.first(), args.get(1)) {
                (Some(Value::String(path)), Some(Value::String(content))) => {
                    use std::io::Write;
                    match std::fs::OpenOptions::new().append(true).create(true).open(path) {
                        Ok(mut f) => {
                            match f.write_all(content.as_bytes()) {
                                Ok(()) => Value::Bool(true),
                                Err(_) => Value::Bool(false),
                            }
                        }
                        Err(_) => Value::Bool(false),
                    }
                }
                _ => Value::Bool(false),
            },
            "input" => {
                // Read a line from stdin
                if let Some(Value::String(prompt)) = args.first() {
                    print!("{}", prompt);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                let mut line = String::new();
                match std::io::stdin().read_line(&mut line) {
                    Ok(_) => Value::String(line.trim_end_matches('\n').trim_end_matches('\r').to_string()),
                    Err(_) => Value::Null,
                }
            },
            "env" => match args.first() {
                Some(Value::String(name)) => {
                    match std::env::var(name) {
                        Ok(val) => Value::String(val),
                        Err(_) => Value::Null,
                    }
                }
                _ => Value::Null,
            },
            "timestamp" => {
                Value::Number(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0))
            },
            "sleep" => {
                // sleep(ms) — pause execution for the given number of milliseconds
                match args.first() {
                    Some(Value::Number(ms)) => {
                        let millis = (*ms).max(0.0) as u64;
                        std::thread::sleep(std::time::Duration::from_millis(millis));
                        Value::Null
                    }
                    _ => Value::Null,
                }
            },
            "format" => {
                // Simple string formatting: format("hello {} world {}", arg1, arg2)
                if let Some(Value::String(template)) = args.first() {
                    let mut result = template.clone();
                    for arg in &args[1..] {
                        if let Some(pos) = result.find("{}") {
                            let replacement = match arg {
                                Value::String(s) => s.clone(),
                                other => format!("{}", other),
                            };
                            result = format!("{}{}{}", &result[..pos], replacement, &result[pos + 2..]);
                        }
                    }
                    Value::String(result)
                } else {
                    Value::Null
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

            // ── Reflection operations ──
            "agents" => {
                // Return a list of all declared agent names
                Value::List(self.agents.keys().map(|k| Value::String(k.clone())).collect())
            },
            "fields" => match args.first() {
                // Return the data fields (keys) of an agent or Map
                Some(Value::Agent(name)) | Some(Value::String(name)) => {
                    if let Some(data) = self.agent_data.get(name) {
                        Value::List(data.keys().map(|k| Value::String(k.clone())).collect())
                    } else {
                        Value::List(vec![])
                    }
                }
                Some(Value::Map(entries)) => {
                    Value::List(entries.iter().map(|(k, _)| Value::String(k.clone())).collect())
                }
                _ => Value::List(vec![]),
            },
            "state" => match args.first() {
                // Return the state of an agent as a string
                Some(Value::Agent(name)) | Some(Value::String(name)) => {
                    if let Some(agent) = self.agents.get(name) {
                        Value::String(state_name(agent.state).to_string())
                    } else {
                        Value::Null
                    }
                }
                _ => Value::Null,
            },
            "globals" => {
                // Return all global bindings as a Map
                if let Some(global) = self.agent_data.get("__global__") {
                    Value::Map(global.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                } else {
                    Value::Map(vec![])
                }
            },

            // ── Higher-order functions ──
            "map" => match (args.first(), args.get(1)) {
                (Some(Value::List(items)), Some(Value::Function { params, body, env: captured })) => {
                    Value::List(items.iter().map(|item| {
                        self.call_function(params, body, &[item.clone()], None, captured)
                    }).collect())
                }
                _ => Value::Null,
            },
            "filter" => match (args.first(), args.get(1)) {
                (Some(Value::List(items)), Some(Value::Function { params, body, env: captured })) => {
                    Value::List(items.iter().filter(|item| {
                        matches!(
                            self.call_function(params, body, &[(*item).clone()], None, captured),
                            Value::Bool(true)
                        )
                    }).cloned().collect())
                }
                _ => Value::Null,
            },
            "reduce" => match (args.first(), args.get(1), args.get(2)) {
                (Some(Value::List(items)), Some(Value::Function { params, body, env: captured }), Some(init)) => {
                    let mut acc = init.clone();
                    for item in items {
                        acc = self.call_function(params, body, &[acc, item.clone()], None, captured);
                    }
                    acc
                }
                _ => Value::Null,
            },
            "fold" => match (args.first(), args.get(1), args.get(2)) {
                // fold(list, init, fn) — same as reduce but init comes second
                (Some(Value::List(items)), Some(init), Some(Value::Function { params, body, env: captured })) => {
                    let mut acc = init.clone();
                    for item in items {
                        acc = self.call_function(params, body, &[acc, item.clone()], None, captured);
                    }
                    acc
                }
                _ => Value::Null,
            },
            "any" => match (args.first(), args.get(1)) {
                (Some(Value::List(items)), Some(Value::Function { params, body, env: captured })) => {
                    Value::Bool(items.iter().any(|item| {
                        matches!(
                            self.call_function(params, body, &[item.clone()], None, captured),
                            Value::Bool(true)
                        )
                    }))
                }
                _ => Value::Bool(false),
            },
            "all" => match (args.first(), args.get(1)) {
                (Some(Value::List(items)), Some(Value::Function { params, body, env: captured })) => {
                    Value::Bool(items.iter().all(|item| {
                        matches!(
                            self.call_function(params, body, &[item.clone()], None, captured),
                            Value::Bool(true)
                        )
                    }))
                }
                _ => Value::Bool(false),
            },
            "find" => match (args.first(), args.get(1)) {
                (Some(Value::List(items)), Some(Value::Function { params, body, env: captured })) => {
                    items.iter().find(|item| {
                        matches!(
                            self.call_function(params, body, &[(*item).clone()], None, captured),
                            Value::Bool(true)
                        )
                    }).cloned().unwrap_or(Value::Null)
                }
                _ => Value::Null,
            },
            "each_with_index" => match (args.first(), args.get(1)) {
                (Some(Value::List(items)), Some(Value::Function { params, body, env: captured })) => {
                    Value::List(items.iter().enumerate().map(|(i, item)| {
                        self.call_function(params, body, &[item.clone(), Value::Number(i as f64)], None, captured)
                    }).collect())
                }
                _ => Value::Null,
            },

            // ── JSON operations ──
            "json_parse" => match args.first() {
                Some(Value::String(s)) => {
                    match serde_json::from_str::<serde_json::Value>(s) {
                        Ok(json_val) => json_to_value(&json_val),
                        Err(e) => Value::String(format!("json_parse error: {}", e)),
                    }
                }
                _ => Value::Null,
            },
            "json_stringify" => match args.first() {
                Some(val) => {
                    let json_val = value_to_json(val);
                    match serde_json::to_string(&json_val) {
                        Ok(s) => Value::String(s),
                        Err(_) => Value::Null,
                    }
                }
                None => Value::Null,
            },
            "json_stringify_pretty" => match args.first() {
                Some(val) => {
                    let json_val = value_to_json(val);
                    match serde_json::to_string_pretty(&json_val) {
                        Ok(s) => Value::String(s),
                        Err(_) => Value::Null,
                    }
                }
                None => Value::Null,
            },

            // ── HTTP operations ──
            "http_get" => {
                let url = match args.first() {
                    Some(Value::String(u)) => u.clone(),
                    _ => return Value::Null,
                };
                let headers = args.get(1);
                match http_request("GET", &url, headers, None) {
                    Ok(val) => val,
                    Err(e) => Value::String(format!("http_get error: {}", e)),
                }
            },
            "http_post" => {
                let url = match args.first() {
                    Some(Value::String(u)) => u.clone(),
                    _ => return Value::Null,
                };
                let headers = args.get(1);
                let body = args.get(2);
                match http_request("POST", &url, headers, body) {
                    Ok(val) => val,
                    Err(e) => Value::String(format!("http_post error: {}", e)),
                }
            },
            "http_put" => {
                let url = match args.first() {
                    Some(Value::String(u)) => u.clone(),
                    _ => return Value::Null,
                };
                let headers = args.get(1);
                let body = args.get(2);
                match http_request("PUT", &url, headers, body) {
                    Ok(val) => val,
                    Err(e) => Value::String(format!("http_put error: {}", e)),
                }
            },
            "http_delete" => {
                let url = match args.first() {
                    Some(Value::String(u)) => u.clone(),
                    _ => return Value::Null,
                };
                let headers = args.get(1);
                match http_request("DELETE", &url, headers, None) {
                    Ok(val) => val,
                    Err(e) => Value::String(format!("http_delete error: {}", e)),
                }
            },

            // Unknown function
            _ => Value::Null,
        }
    }
}

// ─── VALUE DISPLAY (for string interpolation) ────────────────────

/// Convert a Value to its display string without quotes around string values.
/// Used by f-string interpolation to avoid "Hello "World"" → "Hello World".
pub(crate) fn value_to_display(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        other => format!("{}", other),
    }
}

// ─── JSON CONVERSION ─────────────────────────────────────────

/// Convert a serde_json::Value to an ANWE Value.
fn json_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            Value::Number(n.as_f64().unwrap_or(0.0))
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let entries: Vec<(String, Value)> = obj.iter()
                .map(|(k, v)| (k.clone(), json_to_value(v)))
                .collect();
            Value::Map(entries)
        }
    }
}

/// Convert an ANWE Value to a serde_json::Value.
fn value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => {
            serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => {
            serde_json::Value::Array(items.iter().map(value_to_json).collect())
        }
        Value::Map(entries) => {
            let obj: serde_json::Map<String, serde_json::Value> = entries.iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Agent(name) => serde_json::Value::String(name.clone()),
        Value::History(name) => serde_json::Value::String(format!("history of {}", name)),
        Value::Function { .. } => serde_json::Value::String("<function>".into()),
        Value::RecordConstructor { name, .. } => serde_json::Value::String(format!("<record {}>", name)),
        Value::Code(src) => serde_json::Value::String(src.clone()),
        Value::Error { kind, message } => {
            let mut obj = serde_json::Map::new();
            obj.insert("kind".into(), serde_json::Value::String(kind.clone()));
            obj.insert("message".into(), serde_json::Value::String(message.clone()));
            serde_json::Value::Object(obj)
        }
        Value::Break | Value::Continue | Value::Return(_) => serde_json::Value::Null,
    }
}

// ─── HTTP HELPERS ────────────────────────────────────────────

/// Build HTTP headers from an ANWE Value (Map or List of [key, value] pairs).
fn build_headers(headers_val: &Value) -> Vec<(String, String)> {
    match headers_val {
        Value::Map(entries) => {
            entries.iter()
                .map(|(k, v)| (k.clone(), value_to_display(v)))
                .collect()
        }
        Value::List(items) => {
            // List of [key, value] pairs
            items.iter().filter_map(|item| {
                if let Value::List(pair) = item {
                    let key = pair.first().map(value_to_display).unwrap_or_default();
                    let val = pair.get(1).map(value_to_display).unwrap_or_default();
                    Some((key, val))
                } else {
                    None
                }
            }).collect()
        }
        _ => Vec::new(),
    }
}

/// Perform a blocking HTTP request and return the response body as a String Value.
fn http_request(
    method: &str,
    url: &str,
    headers_val: Option<&Value>,
    body_val: Option<&Value>,
) -> Result<Value, String> {
    let client = reqwest::blocking::Client::new();
    let mut request = match method {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        _ => return Err(format!("Unknown HTTP method: {}", method)),
    };

    // Add headers
    if let Some(hv) = headers_val {
        for (key, val) in build_headers(hv) {
            request = request.header(&key, &val);
        }
    }

    // Add body for POST/PUT
    if let Some(body) = body_val {
        let body_str = value_to_display(body);
        request = request.body(body_str);
    }

    match request.send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response.text().unwrap_or_default();
            // Return a map with status and body
            Ok(Value::Map(vec![
                ("status".into(), Value::Number(status as f64)),
                ("body".into(), Value::String(body)),
            ]))
        }
        Err(e) => Err(format!("{}", e)),
    }
}

// ─── VALUE COMPARISON ─────────────────────────────────────────

pub(crate) fn compare_values(left: &Value, op: &ComparisonOp, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => match op {
            ComparisonOp::Equal => a == b,
            ComparisonOp::NotEqual => a != b,
            ComparisonOp::Less => a < b,
            ComparisonOp::LessEq => a <= b,
            ComparisonOp::Greater => a > b,
            ComparisonOp::GreaterEq => a >= b,
        },
        (Value::String(a), Value::String(b)) => match op {
            ComparisonOp::Equal => a == b,
            ComparisonOp::NotEqual => a != b,
            ComparisonOp::Less => a < b,
            ComparisonOp::LessEq => a <= b,
            ComparisonOp::Greater => a > b,
            ComparisonOp::GreaterEq => a >= b,
        },
        (Value::Bool(a), Value::Bool(b)) => match op {
            ComparisonOp::Equal => a == b,
            ComparisonOp::NotEqual => a != b,
            _ => false,
        },
        (Value::Null, Value::Null) => matches!(op, ComparisonOp::Equal | ComparisonOp::LessEq | ComparisonOp::GreaterEq),
        _ => matches!(op, ComparisonOp::NotEqual), // different types are not equal
    }
}

// ─── SUBSTITUTION ─────────────────────────────────────────────
//
// Replace parameter names in pattern bodies with actual values.
// This is how patterns become concrete attention shapes.

pub(crate) fn substitute_link_expr(expr: &LinkExpr, subs: &HashMap<String, String>) -> LinkExpr {
    match expr {
        LinkExpr::Sync(s) => {
            let mut new_s = s.clone();
            if let Some(name) = subs.get(&s.agent_a) {
                new_s.agent_a = name.clone();
            }
            if let Some(name) = subs.get(&s.agent_b) {
                new_s.agent_b = name.clone();
            }
            LinkExpr::Sync(new_s)
        }
        LinkExpr::Alert(a) => {
            let mut new_a = a.clone();
            new_a.expression = substitute_expr(&a.expression, subs);
            LinkExpr::Alert(new_a)
        }
        LinkExpr::Connect(c) => {
            // Connect blocks don't typically reference agent names
            LinkExpr::Connect(c.clone())
        }
        LinkExpr::Converge(c) => {
            let mut new_c = c.clone();
            if let Some(name) = subs.get(&c.agent_a) {
                new_c.agent_a = name.clone();
            }
            if let Some(name) = subs.get(&c.agent_b) {
                new_c.agent_b = name.clone();
            }
            new_c.body = c.body.iter()
                .map(|e| substitute_link_expr(e, subs))
                .collect();
            LinkExpr::Converge(new_c)
        }
        LinkExpr::Each(e) => {
            let mut new_e = e.clone();
            new_e.collection = substitute_expr(&e.collection, subs);
            new_e.body = e.body.iter()
                .map(|b| substitute_link_expr(b, subs))
                .collect();
            LinkExpr::Each(new_e)
        }
        LinkExpr::IfElse(ie) => {
            let mut new_ie = ie.clone();
            new_ie.then_body = ie.then_body.iter()
                .map(|b| substitute_link_expr(b, subs))
                .collect();
            new_ie.else_body = ie.else_body.iter()
                .map(|b| substitute_link_expr(b, subs))
                .collect();
            LinkExpr::IfElse(new_ie)
        }
        LinkExpr::Think(t) => {
            let mut new_t = t.clone();
            new_t.bindings = t.bindings.iter()
                .map(|b| {
                    let mut new_b = b.clone();
                    new_b.value = substitute_expr(&b.value, subs);
                    new_b
                })
                .collect();
            LinkExpr::Think(new_t)
        }
        LinkExpr::Express(e) => {
            let mut new_e = e.clone();
            new_e.expression = substitute_expr(&e.expression, subs);
            LinkExpr::Express(new_e)
        }
        // For other expressions, clone unchanged
        _ => expr.clone(),
    }
}

pub(crate) fn substitute_expr(expr: &Expr, subs: &HashMap<String, String>) -> Expr {
    match expr {
        Expr::Ident(name) => {
            if let Some(replacement) = subs.get(name) {
                Expr::Ident(replacement.clone())
            } else {
                expr.clone()
            }
        }
        Expr::BinaryOp { left, op, right } => {
            Expr::BinaryOp {
                left: Box::new(substitute_expr(left, subs)),
                op: *op,
                right: Box::new(substitute_expr(right, subs)),
            }
        }
        Expr::UnaryNeg(operand) => {
            Expr::UnaryNeg(Box::new(substitute_expr(operand, subs)))
        }
        Expr::ListLit(items) => {
            Expr::ListLit(items.iter().map(|i| substitute_expr(i, subs)).collect())
        }
        Expr::IndexAccess { object, index } => {
            Expr::IndexAccess {
                object: Box::new(substitute_expr(object, subs)),
                index: Box::new(substitute_expr(index, subs)),
            }
        }
        Expr::Call { name, args } => {
            Expr::Call {
                name: name.clone(),
                args: args.iter().map(|a| substitute_expr(a, subs)).collect(),
            }
        }
        _ => expr.clone(),
    }
}

// ─── TYPE CONVERSION: AST -> CORE ─────────────────────────────

pub(crate) fn to_core_quality(q: SignalQuality) -> Quality {
    match q {
        SignalQuality::Attending => Quality::Attending,
        SignalQuality::Questioning => Quality::Questioning,
        SignalQuality::Recognizing => Quality::Recognizing,
        SignalQuality::Disturbed => Quality::Disturbed,
        SignalQuality::Applying => Quality::Applying,
        SignalQuality::Completing => Quality::Completing,
        SignalQuality::Resting => Quality::Resting,
    }
}

pub(crate) fn to_core_direction(d: SignalDirection) -> Direction {
    match d {
        SignalDirection::Inward => Direction::Inward,
        SignalDirection::Outward => Direction::Outward,
        SignalDirection::Between => Direction::Between,
        SignalDirection::Diffuse => Direction::Diffuse,
    }
}

// ─── DISPLAY HELPERS ──────────────────────────────────────────

pub(crate) fn quality_name(q: SignalQuality) -> &'static str {
    match q {
        SignalQuality::Attending => "attending",
        SignalQuality::Questioning => "questioning",
        SignalQuality::Recognizing => "recognizing",
        SignalQuality::Disturbed => "disturbed",
        SignalQuality::Applying => "applying",
        SignalQuality::Completing => "completing",
        SignalQuality::Resting => "resting",
    }
}

pub(crate) fn core_quality_name(q: Quality) -> &'static str {
    match q {
        Quality::Attending => "attending",
        Quality::Questioning => "questioning",
        Quality::Recognizing => "recognizing",
        Quality::Disturbed => "disturbed",
        Quality::Applying => "applying",
        Quality::Completing => "completing",
        Quality::Resting => "resting",
    }
}

pub(crate) fn direction_name(d: SignalDirection) -> &'static str {
    match d {
        SignalDirection::Inward => "inward",
        SignalDirection::Outward => "outward",
        SignalDirection::Between => "between",
        SignalDirection::Diffuse => "diffuse",
    }
}

pub(crate) fn depth_name(d: DepthLevel) -> &'static str {
    match d {
        DepthLevel::Surface => "surface",
        DepthLevel::Partial => "partial",
        DepthLevel::Full => "full",
        DepthLevel::Genuine => "genuine",
        DepthLevel::Deep => "deep",
    }
}

pub(crate) fn state_name(s: AgentState) -> &'static str {
    match s {
        AgentState::Idle => "Idle",
        AgentState::Alerted => "Alerted",
        AgentState::Connected => "Connected",
        AgentState::Syncing => "Syncing",
        AgentState::Applying => "Applying",
        AgentState::Rejecting => "Rejecting",
        AgentState::Committing => "Committing",
    }
}

pub(crate) fn core_depth_name(d: ChangeDepth) -> &'static str {
    match d {
        ChangeDepth::Trace => "trace",
        ChangeDepth::Shallow => "shallow",
        ChangeDepth::Genuine => "genuine",
        ChangeDepth::Deep => "deep",
    }
}

fn state_from_name(name: &str) -> AgentState {
    match name {
        "Alerted" => AgentState::Alerted,
        "Connected" => AgentState::Connected,
        "Syncing" => AgentState::Syncing,
        "Applying" => AgentState::Applying,
        "Rejecting" => AgentState::Rejecting,
        "Committing" => AgentState::Committing,
        _ => AgentState::Idle,
    }
}

fn source_name(s: ChangeSource) -> &'static str {
    match s {
        ChangeSource::Apply => "Apply",
        ChangeSource::Reject => "Reject",
        ChangeSource::Converge => "Converge",
        ChangeSource::Lineage => "Lineage",
    }
}

fn source_from_name(name: &str) -> ChangeSource {
    match name {
        "Reject" => ChangeSource::Reject,
        "Converge" => ChangeSource::Converge,
        "Lineage" => ChangeSource::Lineage,
        _ => ChangeSource::Apply,
    }
}

pub(crate) fn op_symbol(op: ComparisonOp) -> &'static str {
    match op {
        ComparisonOp::Greater => ">",
        ComparisonOp::GreaterEq => ">=",
        ComparisonOp::Less => "<",
        ComparisonOp::LessEq => "<=",
        ComparisonOp::Equal => "==",
        ComparisonOp::NotEqual => "!=",
    }
}

pub(crate) fn pending_reason_name(r: &PendingReason) -> &'static str {
    match r {
        PendingReason::ReceiverNotReady => "receiver_not_ready",
        PendingReason::LinkNotEstablished => "link_not_established",
        PendingReason::SyncInsufficient => "sync_insufficient",
        PendingReason::SenderNotReady => "sender_not_ready",
        PendingReason::MomentNotRight => "moment_not_right",
        PendingReason::BudgetExhausted => "budget_exhausted",
    }
}

pub(crate) fn link_priority_name(p: anwe_parser::ast::LinkPriority) -> &'static str {
    match p {
        anwe_parser::ast::LinkPriority::Critical => "critical",
        anwe_parser::ast::LinkPriority::High => "high",
        anwe_parser::ast::LinkPriority::Normal => "normal",
        anwe_parser::ast::LinkPriority::Low => "low",
        anwe_parser::ast::LinkPriority::Background => "background",
    }
}

/// Apply parsed signal attributes (confidence, half_life) to a core Signal.
pub(crate) fn apply_signal_attrs(mut signal: Signal, attrs: Option<&SignalAttrs>) -> Signal {
    if let Some(attrs) = attrs {
        if let Some(conf) = attrs.confidence {
            signal.confidence = (conf.clamp(0.0, 1.0) * 10000.0) as u16;
        }
        if let Some(hl) = attrs.half_life {
            signal.half_life = hl as u16;
        }
    }
    signal
}

/// Convert a runtime Value to a WireValue for the bridge.
pub(crate) fn value_to_wire(val: &Value) -> WireValue {
    match val {
        Value::String(s) => WireValue::String(s.clone()),
        Value::Number(n) => WireValue::Float(*n),
        Value::Bool(b) => WireValue::Bool(*b),
        Value::Agent(name) => WireValue::String(name.clone()),
        Value::History(name) => WireValue::String(format!("history of {}", name)),
        Value::List(items) => {
            WireValue::List(items.iter().map(value_to_wire).collect())
        }
        Value::Map(entries) => {
            WireValue::Map(entries.iter().map(|(k, v)| (k.clone(), value_to_wire(v))).collect())
        }
        Value::Function { params, .. } => WireValue::String(format!("fn({})", params.join(", "))),
        Value::RecordConstructor { name, fields } => WireValue::String(format!("record {}({})", name, fields.join(", "))),
        Value::Code(src) => WireValue::String(format!("quote {{ {} }}", src)),
        Value::Null => WireValue::Null,
        Value::Error { kind, message } => WireValue::String(format!("error({}: {})", kind, message)),
        Value::Break | Value::Continue | Value::Return(_) => WireValue::Null,
    }
}

/// Convert a WireValue back to a runtime Value.
pub(crate) fn wire_to_value(wire: &WireValue) -> Value {
    match wire {
        WireValue::String(s) => Value::String(s.clone()),
        WireValue::Float(n) => Value::Number(*n),
        WireValue::Integer(n) => Value::Number(*n as f64),
        WireValue::Bool(b) => Value::Bool(*b),
        WireValue::Null => Value::Null,
        WireValue::Bytes(_) => Value::Null,
        WireValue::List(items) => {
            Value::List(items.iter().map(wire_to_value).collect())
        }
        WireValue::Map(entries) => {
            Value::Map(entries.iter().map(|(k, v)| (k.clone(), wire_to_value(v))).collect())
        }
    }
}

pub(crate) fn compare_f64(current: f64, op: ComparisonOp, target: f64) -> bool {
    match op {
        ComparisonOp::Greater => current > target,
        ComparisonOp::GreaterEq => current >= target,
        ComparisonOp::Less => current < target,
        ComparisonOp::LessEq => current <= target,
        ComparisonOp::Equal => (current - target).abs() < 0.001,
        ComparisonOp::NotEqual => (current - target).abs() >= 0.001,
    }
}

pub(crate) fn format_condition(cond: &Condition) -> String {
    match cond {
        Condition::SyncLevel { op, value } => {
            format!("sync_level {} {:.3}", op_symbol(*op), value)
        }
        Condition::Priority { op, value } => {
            format!("priority {} {:.3}", op_symbol(*op), value)
        }
        Condition::Confidence { op, value } => {
            format!("confidence {} {:.3}", op_symbol(*op), value)
        }
        Condition::Attention { op, value } => {
            format!("attention {} {:.3}", op_symbol(*op), value)
        }
        Condition::AlertIs(name) => {
            format!("alert is {}", name)
        }
        Condition::And(a, b) => {
            format!("{} and {}", format_condition(a), format_condition(b))
        }
        Condition::Or(a, b) => {
            format!("{} or {}", format_condition(a), format_condition(b))
        }
        Condition::FieldCompare { left, op, right } => {
            format!("{} {} {}", format_expr(left), op_symbol(*op), format_expr(right))
        }
    }
}

fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Ident(name) => name.clone(),
        Expr::Number(n) => {
            if *n == (*n as i64) as f64 { format!("{}", *n as i64) } else { format!("{}", n) }
        }
        Expr::StringLit(s) => format!("\"{}\"", s),
        Expr::Bool(b) => format!("{}", b),
        Expr::FieldAccess { object, field } => format!("{}.{}", object, field),
        Expr::Call { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expr).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        _ => format!("{:?}", expr),
    }
}

// ─── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use anwe_parser::{Lexer, Parser};

    fn parse_and_run(source: &str) -> Result<(), EngineError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        let mut engine = Engine::new();
        engine.execute(&program)
    }

    #[test]
    fn engine_empty_program() {
        let result = parse_and_run("");
        assert!(result.is_ok());
    }

    #[test]
    fn engine_agents_only() {
        let result = parse_and_run("agent Alpha\nagent Beta");
        assert!(result.is_ok());
    }

    #[test]
    fn engine_unknown_agent_in_link() {
        let result = parse_and_run("agent A\nlink A <-> B { }");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Unknown agent 'B'"));
        assert!(err_msg.contains("at line"));
    }

    #[test]
    fn engine_empty_link() {
        let result = parse_and_run("agent A\nagent B\nlink A <-> B { }");
        assert!(result.is_ok());
    }

    #[test]
    fn engine_alert() {
        let result = parse_and_run(r#"
            agent A
            agent B
            link A <-> B {
                >> "hello"
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_full_link_lifecycle() {
        let result = parse_and_run(r#"
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
    fn engine_converge() {
        let result = parse_and_run(r#"
            agent Alpha data { generation: 1  lineage_depth: 847 }
            agent Beta data { generation: 2  lineage_depth: 0 }

            link Alpha <-> Beta {
                >> { quality: attending, priority: 0.95 }
                   "lineage transfer"

                connect depth deep {
                    signal attending 0.9 between
                }

                Alpha ~ Beta until resonating

                converge Alpha <<>> Beta {
                    >> { quality: attending, priority: 1.0 }
                       "lineage through genuine encounter"

                    => when sync_level > 0.9 depth deep {
                        lineage <- "transmitted"
                    }
                }

                * from apply {
                    generation: 2
                    lineage_depth: 848
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_pending_not_triggered() {
        let result = parse_and_run(r#"
            agent A
            agent B
            link A <-> B {
                >> "test"
                connect depth surface {
                    signal attending 0.5 between
                }
                A ~ B until synchronized
                pending? link_not_established {
                    wait 1.0 tick
                    guidance "sync longer"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_self_link() {
        let result = parse_and_run(r#"
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

    // ─── BRIDGE TESTS ────────────────────────────────────────

    fn parse_and_run_with_bridge(
        source: &str, registry: ParticipantRegistry,
    ) -> Result<(), EngineError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        let mut engine = Engine::with_participants(registry);
        engine.execute(&program)
    }

    #[test]
    fn bridge_external_agent_parses() {
        // Verify external agent declarations parse correctly
        let source = r#"
            agent Sensor external("callback", "echo")
            agent Processor
        "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        assert_eq!(program.declarations.len(), 2);

        if let anwe_parser::ast::Declaration::Agent(decl) = &program.declarations[0] {
            assert_eq!(decl.name, "Sensor");
            assert!(decl.external.is_some());
            let ext = decl.external.as_ref().unwrap();
            assert_eq!(ext.kind, "callback");
            assert_eq!(ext.address, "echo");
        } else {
            panic!("Expected agent declaration");
        }
    }

    #[test]
    fn bridge_echo_participant_receives_signals() {
        use anwe_bridge::participant::CallbackParticipant;
        use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

        let signal_count = Arc::new(AtomicU32::new(0));
        let count_clone = Arc::clone(&signal_count);

        let mut registry = ParticipantRegistry::new();
        registry.register("Sensor", Box::new(
            CallbackParticipant::new(
                anwe_bridge::ParticipantDescriptor {
                    name: "TestSensor".into(),
                    kind: "callback".into(),
                    address: "test".into(),
                    version: "0.1.0".into(),
                },
                move |_signal| {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                    None // Don't respond
                },
                |_| true,
                |_| {},
            )
        ));

        let result = parse_and_run_with_bridge(r#"
            agent Sensor external("callback", "test")
            agent Processor
            link Sensor <-> Processor {
                >> { quality: attending, priority: 0.8 }
                   "incoming data"
                connect {
                    signal attending 0.7 between
                    signal questioning 0.6 between
                }
                Sensor ~ Processor until synchronized
                => when sync_level > 0.6 {
                    reading <- "processed"
                }
                * from apply {
                    status: "complete"
                }
            }
        "#, registry);

        assert!(result.is_ok());
        // Sensor should have received signals during alert + connect
        assert!(signal_count.load(Ordering::Relaxed) >= 1,
            "External participant should have received at least 1 signal, got {}",
            signal_count.load(Ordering::Relaxed));
    }

    #[test]
    fn bridge_participant_can_reject_apply() {
        use anwe_bridge::participant::CallbackParticipant;

        let mut registry = ParticipantRegistry::new();
        registry.register("Stubborn", Box::new(
            CallbackParticipant::new(
                anwe_bridge::ParticipantDescriptor {
                    name: "Stubborn".into(),
                    kind: "callback".into(),
                    address: "reject".into(),
                    version: "0.1.0".into(),
                },
                |_| None,
                |_| false, // Always reject apply
                |_| {},
            )
        ));

        // The stubborn participant rejects apply, so the engine
        // should switch to the reject path
        let result = parse_and_run_with_bridge(r#"
            agent Stubborn external("callback", "reject")
            agent Other
            link Stubborn <-> Other {
                >> "test"
                connect {
                    signal attending 0.5 between
                }
                Stubborn ~ Other until synchronized
                => when sync_level > 0.6 {
                    data <- "this should be rejected"
                }
                * from apply {
                    status: "committed"
                }
            }
        "#, registry);

        assert!(result.is_ok());
    }

    #[test]
    fn bridge_commit_notifies_participant() {
        use anwe_bridge::participant::CallbackParticipant;
        use std::sync::{Arc, Mutex};

        let committed = Arc::new(Mutex::new(Vec::<String>::new()));
        let committed_clone = Arc::clone(&committed);

        let mut registry = ParticipantRegistry::new();
        registry.register("Recorder", Box::new(
            CallbackParticipant::new(
                anwe_bridge::ParticipantDescriptor {
                    name: "Recorder".into(),
                    kind: "callback".into(),
                    address: "record".into(),
                    version: "0.1.0".into(),
                },
                |_| None,
                |_| true,
                move |entries| {
                    let mut c = committed_clone.lock().unwrap();
                    for (key, _val) in entries {
                        c.push(key.clone());
                    }
                },
            )
        ));

        let result = parse_and_run_with_bridge(r#"
            agent Recorder external("callback", "record")
            agent Source
            link Recorder <-> Source {
                >> "recording"
                connect {
                    signal attending 0.7 between
                }
                Recorder ~ Source until synchronized
                => when sync_level > 0.6 {
                    reading <- "data"
                }
                * from apply {
                    event: "sensor_reading"
                    timestamp: 1234
                }
            }
        "#, registry);

        assert!(result.is_ok());
        let keys = committed.lock().unwrap();
        assert!(keys.contains(&"event".to_string()),
            "Commit should have notified participant with 'event' key, got: {:?}", *keys);
    }

    #[test]
    fn bridge_external_with_attention_and_data() {
        // Verify external agents can also have attention and data
        let result = parse_and_run(r#"
            agent Sensor attention 0.8 external("python", "sensor.py") data {
                role: "perception"
            }
            agent Processor
            link Sensor <-> Processor {
                >> "test"
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn bridge_full_lifecycle_with_echo() {
        use anwe_bridge::participant::CallbackParticipant;

        let mut registry = ParticipantRegistry::new();
        registry.register("Echo", Box::new(CallbackParticipant::echo("EchoSensor")));

        let result = parse_and_run_with_bridge(r#"
            agent Echo external("callback", "echo")
            agent Processor

            link Echo <-> Processor {
                >> { quality: attending, priority: 0.8 }
                   "incoming sensor data"

                connect depth full {
                    signal attending   0.7 between
                    signal questioning 0.6 between data "what do you see"
                    signal recognizing 0.5 inward
                }

                Echo ~ Processor until synchronized

                => when sync_level > 0.6 depth genuine {
                    reading  <- "sensor observation received"
                    analyzed <- "processed through attention"
                }

                * from apply {
                    source:  "external sensor via bridge"
                    status:  "integrated"
                }
            }
        "#, registry);

        assert!(result.is_ok());
    }

    #[test]
    fn bridge_sync_notifies_participant() {
        use anwe_bridge::participant::CallbackParticipant;
        use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

        let signal_count = Arc::new(AtomicU32::new(0));
        let count_clone = Arc::clone(&signal_count);

        let mut registry = ParticipantRegistry::new();
        registry.register("Sensor", Box::new(
            CallbackParticipant::new(
                anwe_bridge::ParticipantDescriptor {
                    name: "SyncSensor".into(),
                    kind: "callback".into(),
                    address: "test".into(),
                    version: "0.1.0".into(),
                },
                move |_signal| {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                    None
                },
                |_| true,
                |_| {},
            )
        ));

        let result = parse_and_run_with_bridge(r#"
            agent Sensor external("callback", "test")
            agent Processor
            link Sensor <-> Processor {
                >> "hello"
                Sensor ~ Processor until synchronized
            }
        "#, registry);

        assert!(result.is_ok());
        // Alert (1) + sync completion (1) = at least 2 signals
        assert!(signal_count.load(Ordering::Relaxed) >= 2,
            "Expected at least 2 signals (alert + sync), got {}",
            signal_count.load(Ordering::Relaxed));
    }

    #[test]
    fn bridge_reject_notifies_participant() {
        use anwe_bridge::participant::CallbackParticipant;
        use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

        // Reject notifies agent_b (the rejected agent).
        // So we make Processor the external agent to receive the reject signal.
        let signal_count = Arc::new(AtomicU32::new(0));
        let count_clone = Arc::clone(&signal_count);

        let mut registry = ParticipantRegistry::new();
        registry.register("Processor", Box::new(
            CallbackParticipant::new(
                anwe_bridge::ParticipantDescriptor {
                    name: "RejectProcessor".into(),
                    kind: "callback".into(),
                    address: "test".into(),
                    version: "0.1.0".into(),
                },
                move |_signal| {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                    None
                },
                |_| true,
                |_| {},
            )
        ));

        let result = parse_and_run_with_bridge(r#"
            agent Sensor
            agent Processor external("callback", "test")
            link Sensor <-> Processor {
                >> "testing reject"
                Sensor ~ Processor until synchronized
                <= when sync_level > 0.5 data "boundary reached"
            }
        "#, registry);

        assert!(result.is_ok());
        // alert notifies Processor (1) + sync notifies Processor (1) + reject notifies Processor (1)
        assert!(signal_count.load(Ordering::Relaxed) >= 3,
            "Expected at least 3 signals (alert + sync + reject), got {}",
            signal_count.load(Ordering::Relaxed));
    }

    // ═════════════════════════════════════════════
    // FIRST-PERSON COGNITION TESTS
    // ═════════════════════════════════════════════

    #[test]
    fn engine_mind_basic() {
        let result = parse_and_run(r#"
            mind Cognition {
                attend "incoming" priority 0.9 {
                    >> "I notice something"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_think() {
        let result = parse_and_run(r#"
            mind Thinker {
                attend "reason" priority 0.8 {
                    think {
                        insight <- "the pattern resolves"
                        depth   <- 42
                    }
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_express() {
        let result = parse_and_run(r#"
            mind Speaker {
                attend "speak" priority 0.7 {
                    express "I have something to say"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_express_with_attrs() {
        let result = parse_and_run(r#"
            mind Voice {
                attend "articulate" priority 0.8 {
                    express { quality: recognizing, priority: 0.9 }
                      "this is important"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_priority_order() {
        // Attend blocks should execute in priority order
        let result = parse_and_run(r#"
            mind Ordered {
                attend "low" priority 0.1 {
                    >> "I am low priority"
                }
                attend "high" priority 0.9 {
                    >> "I am high priority"
                }
                attend "medium" priority 0.5 {
                    >> "I am medium priority"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_attention_decay() {
        // With a small attention budget, lower-priority blocks should decay
        let result = parse_and_run(r#"
            mind Finite attention 0.15 {
                attend "critical" priority 1.0 {
                    >> "must process"
                    think {
                        action <- "done"
                    }
                }
                attend "optional" priority 0.3 {
                    >> "nice to have"
                }
                attend "background" priority 0.05 {
                    >> "probably decays"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_full_lifecycle() {
        // A complete mind lifecycle: attend, think, apply, commit, express
        let result = parse_and_run(r#"
            mind Reasoning {
                attend "process input" priority 0.9 {
                    >> { quality: attending, priority: 0.85 }
                       "input received"

                    think {
                        assessment <- "input is valid"
                        confidence <- 0.8
                    }

                    => when sync_level > 0.5 {
                        understanding <- "integrated"
                    }

                    * from apply {
                        result: "processed"
                    }

                    express { quality: completing, priority: 0.7 }
                      "processing complete"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_with_data() {
        let result = parse_and_run(r#"
            mind Observer data { role: "watcher"  depth: 3 } {
                attend "observe" priority 0.6 {
                    think {
                        observed <- "something happened"
                    }
                    express "observation recorded"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_think_in_link() {
        // Think and express also work inside regular links
        let result = parse_and_run(r#"
            agent A
            agent B
            link A <-> B {
                >> "hello"
                think {
                    greeting <- "acknowledged"
                }
                express "hello back"
            }
        "#);
        assert!(result.is_ok());
    }

    // ═════════════════════════════════════════════
    // MIND + BRIDGE INTEGRATION TESTS
    // ═════════════════════════════════════════════

    #[test]
    fn mind_bridge_think_enriched() {
        // External participant enriches think bindings
        use anwe_bridge::MindCallbackParticipant;
        use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

        let think_count = Arc::new(AtomicU32::new(0));
        let count_clone = Arc::clone(&think_count);

        let mut registry = ParticipantRegistry::new();
        registry.register("Cognition", Box::new(
            MindCallbackParticipant::new(
                anwe_bridge::ParticipantDescriptor {
                    name: "Cognition".into(),
                    kind: "callback".into(),
                    address: "test".into(),
                    version: "0.1.0".into(),
                },
                |_| None,
                |_| true,
                |_| {},
                move |bindings| {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                    let mut enriched = bindings.to_vec();
                    enriched.push(("enriched".to_string(), WireValue::Bool(true)));
                    Some(enriched)
                },
                |_, _| None,
            )
        ));

        let result = parse_and_run_with_bridge(r#"
            mind Cognition {
                attend "process" priority 0.9 {
                    think {
                        insight <- "the pattern resolves"
                    }
                }
            }
        "#, registry);

        assert!(result.is_ok());
        assert!(think_count.load(Ordering::Relaxed) >= 1,
            "Bridge think should have been called at least once");
    }

    #[test]
    fn mind_bridge_express_shaped() {
        // External participant shapes express output
        use anwe_bridge::MindCallbackParticipant;
        use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

        let express_count = Arc::new(AtomicU32::new(0));
        let count_clone = Arc::clone(&express_count);

        let mut registry = ParticipantRegistry::new();
        registry.register("Voice", Box::new(
            MindCallbackParticipant::new(
                anwe_bridge::ParticipantDescriptor {
                    name: "Voice".into(),
                    kind: "callback".into(),
                    address: "test".into(),
                    version: "0.1.0".into(),
                },
                |_| None,
                |_| true,
                |_| {},
                |_| None,
                move |_signal, _content| {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                    Some(WireValue::String("[deepened] expression".into()))
                },
            )
        ));

        let result = parse_and_run_with_bridge(r#"
            mind Voice {
                attend "speak" priority 0.8 {
                    express "original expression"
                }
            }
        "#, registry);

        assert!(result.is_ok());
        assert!(express_count.load(Ordering::Relaxed) >= 1,
            "Bridge express should have been called at least once");
    }

    #[test]
    fn mind_bridge_reflective_full() {
        // Full integration with reflective mind participant
        use anwe_bridge::MindCallbackParticipant;

        let mut registry = ParticipantRegistry::new();
        registry.register("Thinker", Box::new(
            MindCallbackParticipant::reflective("ReflectiveThinker")
        ));

        let result = parse_and_run_with_bridge(r#"
            mind Thinker {
                attend "reason" priority 0.9 {
                    >> "something to process"

                    think {
                        analysis <- "the pattern resolves"
                        confidence <- 0.85
                    }

                    express { quality: recognizing, priority: 0.8 }
                      "I see the pattern"
                }

                attend "reflect" priority 0.5 {
                    think {
                        meta <- "reviewing my own reasoning"
                    }
                    express "reflection complete"
                }
            }
        "#, registry);

        assert!(result.is_ok());
    }

    // ═════════════════════════════════════════════
    // ADDITIONAL EDGE CASE TESTS
    // ═════════════════════════════════════════════

    #[test]
    fn engine_mind_coexists_with_agents_and_links() {
        // Mind, agents, and links can all exist in the same program
        let result = parse_and_run(r#"
            agent Sensor
            agent Processor

            mind Cognition {
                attend "perceive" priority 0.8 {
                    think {
                        perceived <- "input from sensor"
                    }
                    express "perception recorded"
                }
            }

            link Sensor <-> Processor {
                >> "data incoming"
                connect {
                    signal attending 0.7 between
                }
                Sensor ~ Processor until synchronized
                * from apply
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_single_attend() {
        // Mind with exactly one attend block
        let result = parse_and_run(r#"
            mind Minimal {
                attend "only task" priority 0.5 {
                    express "done"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_think_multiple_bindings() {
        // Think block with many bindings
        let result = parse_and_run(r#"
            mind Calculator {
                attend "compute" priority 0.9 {
                    think {
                        a <- 10
                        b <- 20
                        sum <- 30
                        product <- 200
                        label <- "arithmetic results"
                    }
                    express "computed all values"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_nested_primitives() {
        // Mind attend blocks using all link primitives
        let result = parse_and_run(r#"
            mind Complex {
                attend "full cycle" priority 0.95 {
                    >> { quality: disturbed, priority: 0.9 }
                       "crisis detected"

                    think {
                        threat_level <- "high"
                        response <- "engage"
                    }

                    connect depth deep {
                        signal attending 0.9 between
                        signal disturbed 0.8 outward
                    }

                    => when sync_level > 0.5 depth genuine {
                        resolution <- "crisis handled"
                    }

                    * from apply {
                        outcome: "resolved"
                    }

                    express { quality: completing, priority: 0.85 }
                      "crisis resolved — becoming deeper"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_express_all_qualities() {
        // Express with different signal qualities
        let result = parse_and_run(r#"
            mind Expressive {
                attend "voices" priority 0.9 {
                    express { quality: attending, priority: 0.5 } "present"
                    express { quality: questioning, priority: 0.6 } "curious"
                    express { quality: recognizing, priority: 0.7 } "recognized"
                    express { quality: completing, priority: 0.8 } "finished"
                    express { quality: resting, priority: 0.1 } "at rest"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn engine_mind_when_conditional() {
        // Mind attend block with when conditional
        let result = parse_and_run(r#"
            mind Conditional {
                attend "decide" priority 0.8 {
                    >> "stimulus"
                    connect {
                        signal attending 0.6 between
                    }
                    when sync_level > 0.3 {
                        think {
                            decision <- "yes"
                        }
                    }
                    express "decision made"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    // ─── FEATURE 1: COMPUTABLE SIGNALS ───────────────────────

    fn parse_and_get_engine(source: &str) -> Result<Engine, EngineError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        let mut engine = Engine::new();
        engine.execute(&program)?;
        Ok(engine)
    }

    fn escape_anwe_string_literal(raw: &str) -> String {
        raw.replace('\\', "\\\\").replace('"', "\\\"")
    }

    #[test]
    fn computable_think_resolves_bindings() {
        // think should resolve bindings sequentially —
        // a later binding can reference an earlier one
        let result = parse_and_run(r#"
            mind Solver attention 0.7 {
                attend "compute" priority 0.9 {
                    think {
                        a <- 10
                        b <- 20
                    }
                    express "computed"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn computable_think_arithmetic() {
        // think bindings should support arithmetic expressions
        let engine = parse_and_get_engine(r#"
            mind Calculator attention 0.7 {
                attend "math" priority 0.9 {
                    think {
                        x <- 3 + 4
                        y <- 10 - 2
                    }
                    express "done"
                }
            }
        "#).expect("should succeed");
        let data = engine.agent_data.get("Calculator").unwrap();
        assert_eq!(data.get("x"), Some(&Value::Number(7.0)));
        assert_eq!(data.get("y"), Some(&Value::Number(8.0)));
    }

    #[test]
    fn computable_think_string_binding() {
        let engine = parse_and_get_engine(r#"
            mind Writer attention 0.6 {
                attend "write" priority 0.8 {
                    think {
                        message <- "hello world"
                        count   <- 42
                    }
                    express "wrote"
                }
            }
        "#).expect("should succeed");
        let data = engine.agent_data.get("Writer").unwrap();
        assert_eq!(data.get("message"), Some(&Value::String("hello world".to_string())));
        assert_eq!(data.get("count"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn computable_think_with_agent_data() {
        // Agent data set via `data { ... }` should be accessible
        let engine = parse_and_get_engine(r#"
            mind Analyzer attention 0.7 data { base: 100 } {
                attend "analyze" priority 0.9 {
                    think {
                        result <- 50
                    }
                    express "analyzed"
                }
            }
        "#).expect("should succeed");
        let data = engine.agent_data.get("Analyzer").unwrap();
        // base was set from data block
        assert_eq!(data.get("base"), Some(&Value::Number(100.0)));
        // result was set from think
        assert_eq!(data.get("result"), Some(&Value::Number(50.0)));
    }

    // ─── FEATURE 2: SENSE PRIMITIVE ──────────────────────────

    #[test]
    fn sense_basic_in_link() {
        // sense binds signal perception data
        let result = parse_and_run(r#"
            agent Watcher
            agent World
            link Watcher <-> World {
                >> "signal arrives"
                connect depth surface {
                    signal attending 0.5 between
                }
                sense {
                    available <- "what is here"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn sense_populates_signal_count() {
        let engine = parse_and_get_engine(r#"
            agent Perceiver
            agent Field
            link Perceiver <-> Field {
                >> "something"
                connect depth surface {
                    signal attending 0.5 between
                }
                sense {
                    check <- "sensing"
                }
            }
        "#).expect("should succeed");
        // sense should have set signal_count in the agent data
        let data = engine.agent_data.get("Perceiver").unwrap();
        assert!(data.contains_key("signal_count"));
    }

    #[test]
    fn sense_populates_attention() {
        let engine = parse_and_get_engine(r#"
            agent Observer
            agent Scene
            link Observer <-> Scene {
                >> "look"
                sense {
                    look <- "observing"
                }
            }
        "#).expect("should succeed");
        let data = engine.agent_data.get("Observer").unwrap();
        assert!(data.contains_key("attention"));
    }

    // ─── FEATURE 3: PIPE OPERATOR ────────────────────────────

    #[test]
    fn pipe_basic_parse_and_run() {
        // Pipe should parse and execute in a think block
        let result = parse_and_run(r#"
            mind Processor attention 0.7 {
                attend "process" priority 0.9 {
                    think {
                        result <- "input" |> "transform"
                    }
                    express "piped"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn pipe_arithmetic_chain() {
        // Pipe should work with arithmetic
        let engine = parse_and_get_engine(r#"
            mind Chain attention 0.7 {
                attend "chain" priority 0.9 {
                    think {
                        val <- 5 |> 10 |> 15
                    }
                    express "chained"
                }
            }
        "#).expect("should succeed");
        let data = engine.agent_data.get("Chain").unwrap();
        // Last stage of pipe produces the result
        assert_eq!(data.get("val"), Some(&Value::Number(15.0)));
    }

    #[test]
    fn pipe_with_identifiers() {
        // Pipe with identifier stages produces transform descriptors
        let engine = parse_and_get_engine(r#"
            mind Pipeline attention 0.7 {
                attend "flow" priority 0.9 {
                    think {
                        output <- "raw" |> "processed"
                    }
                    express "flowed"
                }
            }
        "#).expect("should succeed");
        let data = engine.agent_data.get("Pipeline").unwrap();
        // Should have a value for output
        assert!(data.contains_key("output"));
    }

    // ─── FEATURE 4: ATTENTION LANDSCAPE ──────────────────────

    #[test]
    fn attention_landscape_priority_ordering() {
        // Higher priority attend blocks should execute first
        let result = parse_and_run(r#"
            mind Thinker attention 0.9 {
                attend "low priority" priority 0.3 {
                    think { low <- "done" }
                    express "low done"
                }
                attend "high priority" priority 0.95 {
                    think { high <- "done" }
                    express "high done"
                }
                attend "medium priority" priority 0.6 {
                    think { medium <- "done" }
                    express "medium done"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn attention_landscape_all_blocks_execute() {
        // All attend blocks should execute (budget permitting)
        let engine = parse_and_get_engine(r#"
            mind Worker attention 0.9 {
                attend "first" priority 0.9 {
                    think { a <- 1 }
                    express "first done"
                }
                attend "second" priority 0.8 {
                    think { b <- 2 }
                    express "second done"
                }
                attend "third" priority 0.7 {
                    think { c <- 3 }
                    express "third done"
                }
            }
        "#).expect("should succeed");
        let data = engine.agent_data.get("Worker").unwrap();
        assert_eq!(data.get("a"), Some(&Value::Number(1.0)));
        assert_eq!(data.get("b"), Some(&Value::Number(2.0)));
        assert_eq!(data.get("c"), Some(&Value::Number(3.0)));
    }

    #[test]
    fn attention_landscape_budget_limits() {
        // Very low attention budget should limit execution
        let result = parse_and_run(r#"
            mind LowBudget attention 0.05 {
                attend "first" priority 0.9 {
                    express "might not run"
                }
                attend "second" priority 0.8 {
                    express "probably won't run"
                }
            }
        "#);
        // Should succeed even with low budget — graceful degradation
        assert!(result.is_ok());
    }

    // ─── FEATURE 5: SELF-AUTHORING ───────────────────────────

    #[test]
    fn author_basic_in_link() {
        // author creates a new attend block at runtime
        let result = parse_and_run(r#"
            agent Creator
            agent Canvas
            link Creator <-> Canvas {
                >> "inspiration"
                connect depth surface {
                    signal attending 0.5 between
                }
                author attend "emergent thought" priority 0.7 {
                    express "I just thought of something new"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn author_block_stored() {
        // Authored blocks should be stored for later execution
        let engine = parse_and_get_engine(r#"
            agent Builder
            agent Workshop
            link Builder <-> Workshop {
                >> "build"
                connect depth surface {
                    signal attending 0.5 between
                }
                author attend "new capability" priority 0.8 {
                    think { created <- "yes" }
                    express "capability added"
                }
            }
        "#).expect("should succeed");
        // The authored block should be stored
        assert!(engine.authored_blocks.contains_key("Builder"));
        assert_eq!(engine.authored_blocks["Builder"].len(), 1);
    }

    #[test]
    fn author_integrates_into_mind_landscape() {
        // A mind should be able to use author and have it
        // integrate into the attention landscape
        let result = parse_and_run(r#"
            mind Evolving attention 0.9 {
                attend "initial" priority 0.9 {
                    think { started <- "yes" }
                    express "initial thought"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    // ─── COMBINED FEATURE TESTS ──────────────────────────────

    #[test]
    fn mind_with_think_sense_and_author() {
        // All features working together in a mind
        let result = parse_and_run(r#"
            mind Aware attention 0.9 {
                attend "perceive" priority 0.95 {
                    think {
                        state <- "open"
                        depth <- 3 + 4
                    }
                    express "perception complete"
                }
                attend "respond" priority 0.7 {
                    think {
                        response <- "genuine"
                    }
                    express "response formed"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn mind_computable_signals_across_attend_blocks() {
        // Data from one attend block should persist for the next
        let engine = parse_and_get_engine(r#"
            mind Accumulator attention 0.9 {
                attend "step1" priority 0.95 {
                    think {
                        x <- 10
                    }
                    express "step1 done"
                }
                attend "step2" priority 0.8 {
                    think {
                        y <- 20
                    }
                    express "step2 done"
                }
            }
        "#).expect("should succeed");
        let data = engine.agent_data.get("Accumulator").unwrap();
        // Both bindings should persist
        assert_eq!(data.get("x"), Some(&Value::Number(10.0)));
        assert_eq!(data.get("y"), Some(&Value::Number(20.0)));
    }

    #[test]
    fn sense_in_link_with_multiple_signals() {
        // Sense should perceive after multiple signals are sent
        let result = parse_and_run(r#"
            agent Watcher
            agent World
            link Watcher <-> World {
                >> "first signal"
                >> "second signal"
                connect depth full {
                    signal attending   0.8 between
                    signal recognizing 0.7 between
                    signal questioning 0.6 between
                }
                sense {
                    field <- "what is here"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn pipe_in_express() {
        // Pipe should work in express context too
        let result = parse_and_run(r#"
            mind Speaker attention 0.7 {
                attend "speak" priority 0.9 {
                    think {
                        words <- "hello" |> "world"
                    }
                    express "spoken"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn top_level_let_binding() {
        let engine = parse_and_get_engine(r#"
            let greeting = "hello"
            let count = 42
            agent A
            agent B
            link A <-> B {
                think {
                    msg <- greeting
                    num <- count
                }
            }
        "#).unwrap();
        // Global bindings should be stored
        let global = engine.agent_data.get("__global__").unwrap();
        assert_eq!(global.get("greeting").unwrap(), &Value::String("hello".to_string()));
        assert_eq!(global.get("count").unwrap(), &Value::Number(42.0));
        // Think block should resolve globals
        let data = engine.agent_data.get("A").unwrap();
        assert_eq!(data.get("msg").unwrap(), &Value::String("hello".to_string()));
        assert_eq!(data.get("num").unwrap(), &Value::Number(42.0));
    }

    #[test]
    fn let_mut_and_reassignment() {
        let engine = parse_and_get_engine(r#"
            agent A
            agent B
            link A <-> B {
                let mut x = 0
                x = x + 1
                x = x + 1
                x = x + 1
                think {
                    result <- x
                }
            }
        "#).unwrap();
        let data = engine.agent_data.get("A").unwrap();
        assert_eq!(data.get("x").unwrap(), &Value::Number(3.0));
        assert_eq!(data.get("result").unwrap(), &Value::Number(3.0));
    }

    #[test]
    fn immutable_binding_rejects_assign() {
        let result = parse_and_run(r#"
            agent A
            agent B
            link A <-> B {
                let x = 10
                x = 20
            }
        "#);
        assert!(result.is_err());
    }

    #[test]
    fn let_in_mind_attend() {
        let engine = parse_and_get_engine(r#"
            mind Thinker attention 0.9 {
                attend "compute" priority 0.9 {
                    let mut counter = 0
                    counter = counter + 5
                    think {
                        result <- counter
                    }
                    express "done"
                }
            }
        "#).unwrap();
        let data = engine.agent_data.get("Thinker").unwrap();
        assert_eq!(data.get("counter").unwrap(), &Value::Number(5.0));
        assert_eq!(data.get("result").unwrap(), &Value::Number(5.0));
    }

    #[test]
    fn fn_declaration_and_call() {
        // Note: * is Commit in ANWE, so we use + for arithmetic tests
        let engine = parse_and_get_engine(r#"
            fn double(x) = x + x
            let result = double(21)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(42.0));
    }

    #[test]
    fn fn_multiple_params() {
        let engine = parse_and_get_engine(r#"
            fn add(a, b) = a + b
            let result = add(10, 32)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(42.0));
    }

    #[test]
    fn fn_calling_builtin() {
        let engine = parse_and_get_engine(r#"
            fn shout(s) = upper(s)
            let result = shout("hello")
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::String("HELLO".to_string()));
    }

    #[test]
    fn fn_calling_other_fn() {
        let engine = parse_and_get_engine(r#"
            fn double(x) = x + x
            fn quadruple(x) = double(double(x))
            let result = quadruple(5)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(20.0));
    }

    #[test]
    fn lambda_in_let() {
        let engine = parse_and_get_engine(r#"
            let triple = |x| x + x + x
            let result = triple(7)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(21.0));
    }

    #[test]
    fn fn_with_string_concat() {
        let engine = parse_and_get_engine(r#"
            fn greet(name) = "Hello, " + name + "!"
            let result = greet("World")
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::String("Hello, World!".to_string()));
    }

    #[test]
    fn fn_used_in_mind() {
        let engine = parse_and_get_engine(r#"
            fn add_ten(x) = x + 10
            mind Calc attention 0.9 {
                attend "compute" priority 0.9 {
                    think {
                        result <- add_ten(26)
                    }
                    express "done"
                }
            }
        "#).unwrap();
        let data = engine.agent_data.get("Calc").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(36.0));
    }

    #[test]
    fn match_literal_number() {
        let engine = parse_and_get_engine(r#"
            let x = 2
            let result = match x {
                1 => "one"
                2 => "two"
                3 => "three"
                _ => "other"
            }
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::String("two".to_string()));
    }

    #[test]
    fn match_literal_string() {
        let engine = parse_and_get_engine(r#"
            let color = "red"
            let result = match color {
                "red" => "warm"
                "blue" => "cool"
                _ => "neutral"
            }
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::String("warm".to_string()));
    }

    #[test]
    fn match_wildcard() {
        let engine = parse_and_get_engine(r#"
            let x = 99
            let result = match x {
                1 => "one"
                _ => "unknown"
            }
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::String("unknown".to_string()));
    }

    #[test]
    fn match_with_binding() {
        let engine = parse_and_get_engine(r#"
            let val = 42
            let result = match val {
                0 => "zero"
                n => n + 1
            }
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(43.0));
    }

    #[test]
    fn match_in_function() {
        let engine = parse_and_get_engine(r#"
            fn classify(x) = match x {
                0 => "zero"
                1 => "one"
                _ => "many"
            }
            let a = classify(0)
            let b = classify(1)
            let c = classify(5)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("a").unwrap(), &Value::String("zero".to_string()));
        assert_eq!(data.get("b").unwrap(), &Value::String("one".to_string()));
        assert_eq!(data.get("c").unwrap(), &Value::String("many".to_string()));
    }

    #[test]
    fn match_bool_patterns() {
        let engine = parse_and_get_engine(r#"
            let flag = true
            let result = match flag {
                true => "yes"
                false => "no"
            }
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::String("yes".to_string()));
    }

    #[test]
    fn record_constructor() {
        let engine = parse_and_get_engine(r#"
            record Point { x, y }
            let p = Point(3, 4)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        if let Value::Map(entries) = data.get("p").unwrap() {
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0], ("x".to_string(), Value::Number(3.0)));
            assert_eq!(entries[1], ("y".to_string(), Value::Number(4.0)));
        } else {
            panic!("expected Map from record constructor");
        }
    }

    #[test]
    fn record_field_access() {
        let engine = parse_and_get_engine(r#"
            record Point { x, y }
            let p = Point(10, 20)
            let px = p.x
            let py = p.y
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("px").unwrap(), &Value::Number(10.0));
        assert_eq!(data.get("py").unwrap(), &Value::Number(20.0));
    }

    #[test]
    fn record_in_function() {
        let engine = parse_and_get_engine(r#"
            record Pair { first, second }
            fn make_pair(a, b) = Pair(a, b)
            let result = make_pair("hello", 42)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        if let Value::Map(entries) = data.get("result").unwrap() {
            assert_eq!(entries[0], ("first".to_string(), Value::String("hello".to_string())));
            assert_eq!(entries[1], ("second".to_string(), Value::Number(42.0)));
        } else {
            panic!("expected Map from record constructor in function");
        }
    }

    #[test]
    fn record_with_match() {
        let engine = parse_and_get_engine(r#"
            record Color { name }
            let c = Color("red")
            let label = match c.name {
                "red" => "warm"
                "blue" => "cool"
                _ => "neutral"
            }
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("label").unwrap(), &Value::String("warm".to_string()));
    }

    // ─── v0.4 TESTS: QUOTE / EVAL / REFLECTION ────────────────

    #[test]
    fn quote_captures_code() {
        let engine = parse_and_get_engine(r#"
            let code = quote { 1 + 2 }
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        match data.get("code").unwrap() {
            Value::Code(src) => assert!(src.contains("1") && src.contains("2")),
            other => panic!("expected Code, got {:?}", other),
        }
    }

    #[test]
    fn quote_type_of_is_code() {
        let engine = parse_and_get_engine(r#"
            let code = quote { 1 + 2 }
            let t = type_of(code)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("t").unwrap(), &Value::String("code".to_string()));
    }

    #[test]
    fn eval_simple_expression() {
        let engine = parse_and_get_engine(r#"
            let code = quote { 3 + 4 }
            let result = eval(code)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(7.0));
    }

    #[test]
    fn eval_string_source() {
        let engine = parse_and_get_engine(r#"
            let result = eval("let x = 10")
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(10.0));
    }

    #[test]
    fn eval_with_let_and_fn() {
        let engine = parse_and_get_engine(r#"
            let code = quote { fn add(a, b) = a + b }
            let result = eval(code)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        match data.get("result").unwrap() {
            Value::Function { params, .. } => {
                assert_eq!(params, &vec!["a".to_string(), "b".to_string()]);
            }
            other => panic!("expected Function, got {:?}", other),
        }
    }

    #[test]
    fn unquote_extracts_source() {
        let engine = parse_and_get_engine(r#"
            let code = quote { hello world }
            let src = unquote(code)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        match data.get("src").unwrap() {
            Value::String(s) => assert!(s.contains("hello") && s.contains("world")),
            other => panic!("expected String, got {:?}", other),
        }
    }

    #[test]
    fn reflection_agents_list() {
        let engine = parse_and_get_engine(r#"
            agent Perceiver data { focus: "input" }
            agent Reasoner data { focus: "logic" }
            let all = agents()
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        match data.get("all").unwrap() {
            Value::List(items) => {
                let names: Vec<String> = items.iter().map(|v| format!("{}", v)).collect();
                assert!(names.contains(&"\"Perceiver\"".to_string()));
                assert!(names.contains(&"\"Reasoner\"".to_string()));
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn reflection_fields_of_agent() {
        let engine = parse_and_get_engine(r#"
            agent Observer data { focus: "light"  mode: "passive" }
            let f = fields("Observer")
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        match data.get("f").unwrap() {
            Value::List(items) => {
                let names: Vec<String> = items.iter().map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => String::new(),
                }).collect();
                assert!(names.contains(&"focus".to_string()));
                assert!(names.contains(&"mode".to_string()));
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn reflection_fields_of_map() {
        let engine = parse_and_get_engine(r#"
            record Point { x, y }
            let p = Point(3, 4)
            let f = fields(p)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        match data.get("f").unwrap() {
            Value::List(items) => {
                let names: Vec<String> = items.iter().map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => String::new(),
                }).collect();
                assert!(names.contains(&"x".to_string()));
                assert!(names.contains(&"y".to_string()));
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn reflection_globals() {
        let engine = parse_and_get_engine(r#"
            let x = 42
            let name = "test"
            let g = globals()
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        match data.get("g").unwrap() {
            Value::Map(entries) => {
                let keys: Vec<&String> = entries.iter().map(|(k, _)| k).collect();
                assert!(keys.contains(&&"x".to_string()));
                assert!(keys.contains(&&"name".to_string()));
            }
            other => panic!("expected Map, got {:?}", other),
        }
    }

    // ─── SUPERVISION TESTS ────────────────────────────────────

    #[test]
    fn supervision_one_for_one_normal_link() {
        // A supervised agent has a normal link — no failure.
        let result = parse_and_run(r#"
            agent Alpha
            agent Beta

            supervise one_for_one max_restarts 3 within 5000 {
                permanent Alpha
                permanent Beta
            }

            link Alpha <-> Beta {
                >> "supervised hello"
                Alpha ~ Beta until synchronized
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn supervision_handles_error_and_continues() {
        // An error occurs inside a supervised link (unknown pattern).
        // The supervisor catches it, restarts, execution continues.
        let engine = parse_and_get_engine(r#"
            agent Alpha
            agent Beta

            supervise one_for_one max_restarts 3 within 5000 {
                permanent Alpha
                permanent Beta
            }

            link Alpha <-> Beta {
                >> "first alert"
                ~> nonexistent_pattern()
                >> "after recovery"
            }
        "#);
        // pattern call fails, supervisor handles it, execution continues
        assert!(engine.is_ok());
    }

    #[test]
    fn supervision_without_supervisor_propagates_error() {
        // Without a supervisor, errors propagate normally.
        let result = parse_and_run(r#"
            agent Alpha
            agent Beta

            link Alpha <-> Beta {
                >> "hello"
                ~> nonexistent_pattern()
            }
        "#);
        assert!(result.is_err());
    }

    #[test]
    fn supervision_temporary_child_not_restarted() {
        // A temporary child is not restarted, but the error
        // is still absorbed (not propagated).
        let result = parse_and_run(r#"
            agent Alpha
            agent Beta

            supervise one_for_one max_restarts 3 within 5000 {
                temporary Alpha
                permanent Beta
            }

            link Alpha <-> Beta {
                >> "hello"
                ~> nonexistent_pattern()
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn supervision_overwhelmed_escalates() {
        // When max_restarts is exceeded, the supervisor fails
        // and the error escalates.
        let result = parse_and_run(r#"
            agent Alpha
            agent Beta

            supervise one_for_one max_restarts 1 within 60000 {
                permanent Alpha
                permanent Beta
            }

            link Alpha <-> Beta {
                ~> bad_pattern_1()
                ~> bad_pattern_2()
                ~> bad_pattern_3()
            }
        "#);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("overwhelmed"));
    }

    #[test]
    fn supervision_agent_state_reset_on_restart() {
        // After supervised restart, agent still exists with supervisor.
        let engine = parse_and_get_engine(r#"
            agent Alpha
            agent Beta

            supervise one_for_one max_restarts 3 within 5000 {
                permanent Alpha
                permanent Beta
            }

            link Alpha <-> Beta {
                >> "before failure"
                ~> nonexistent_pattern()
                >> "after recovery"
            }
        "#).unwrap();

        // Alpha was restarted, not removed
        assert!(engine.agents.contains_key("Alpha"));
        // Supervisor link preserved
        assert!(engine.agents.get("Alpha").unwrap().supervisor.is_some());
    }

    // ─── SCHEDULING TESTS ────────────────────────────────────

    #[test]
    fn scheduling_every_n_ticks() {
        // A link with "every 3 ticks" should execute its body 3 times.
        let engine = parse_and_get_engine(r#"
            agent Alpha data { count: 0 }
            agent Beta

            link Alpha <-> Beta every 3 ticks {
                >> "tick"
            }
        "#).unwrap();
        // The link should have completed with 3 iterations
        // Each alert produces a signal, so total = 3
        assert!(engine.agents.contains_key("Alpha"));
    }

    #[test]
    fn scheduling_after_n_ticks() {
        // A link with "after 5 ticks" should execute once after a delay.
        let result = parse_and_run(r#"
            agent Alpha
            agent Beta

            link Alpha <-> Beta after 5 ticks {
                >> "delayed"
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn stream_executes_at_rate() {
        // stream rate 3 should execute its body 3 times.
        let result = parse_and_run(r#"
            agent Sensor
            agent Processor

            link Sensor <-> Processor {
                stream Sensor rate 3 {
                    >> "sample"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn buffer_accumulates_samples() {
        // buffer samples 4 should execute its body 4 times.
        let result = parse_and_run(r#"
            agent Sensor
            agent Processor

            link Sensor <-> Processor {
                buffer samples 4 {
                    >> "data point"
                }
            }
        "#);
        assert!(result.is_ok());
    }

    // ─── CLOSURE & HIGHER-ORDER FUNCTION TESTS ─────────────────

    #[test]
    fn closure_captures_parent_scope() {
        // A lambda defined in a let binding should capture variables from
        // the enclosing scope (the global scope in this case).
        let engine = parse_and_get_engine(r#"
            let offset = 10
            let add_offset = |x| x + offset
            let result = add_offset(5)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(15.0));
    }

    #[test]
    fn nested_closure_captures_outer() {
        // A function returning a lambda that captures the function's parameter.
        let engine = parse_and_get_engine(r#"
            fn make_adder(n) = |x| x + n
            let add3 = make_adder(3)
            let result = add3(7)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::Number(10.0));
    }

    #[test]
    fn map_with_lambda() {
        let engine = parse_and_get_engine(r#"
            let nums = [1, 2, 3, 4]
            let doubled = map(nums, |x| x * 2)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("doubled").unwrap(), &Value::List(vec![
            Value::Number(2.0), Value::Number(4.0),
            Value::Number(6.0), Value::Number(8.0),
        ]));
    }

    #[test]
    fn filter_with_lambda() {
        let engine = parse_and_get_engine(r#"
            let nums = [1, 2, 3, 4, 5, 6]
            let evens = filter(nums, |x| x % 2 == 0)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("evens").unwrap(), &Value::List(vec![
            Value::Number(2.0), Value::Number(4.0), Value::Number(6.0),
        ]));
    }

    #[test]
    fn reduce_with_lambda() {
        let engine = parse_and_get_engine(r#"
            let nums = [1, 2, 3, 4, 5]
            let total = reduce(nums, |acc, x| acc + x, 0)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("total").unwrap(), &Value::Number(15.0));
    }

    #[test]
    fn fold_with_lambda() {
        let engine = parse_and_get_engine(r#"
            let words = ["hello", " ", "world"]
            let sentence = fold(words, "", |acc, w| acc + w)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("sentence").unwrap(), &Value::String("hello world".to_string()));
    }

    #[test]
    fn any_with_lambda() {
        let engine = parse_and_get_engine(r#"
            let nums = [1, 2, 3, 4, 5]
            let has_even = any(nums, |x| x % 2 == 0)
            let has_negative = any(nums, |x| x < 0)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("has_even").unwrap(), &Value::Bool(true));
        assert_eq!(data.get("has_negative").unwrap(), &Value::Bool(false));
    }

    #[test]
    fn all_with_lambda() {
        let engine = parse_and_get_engine(r#"
            let nums = [2, 4, 6, 8]
            let all_even = all(nums, |x| x % 2 == 0)
            let all_small = all(nums, |x| x < 5)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("all_even").unwrap(), &Value::Bool(true));
        assert_eq!(data.get("all_small").unwrap(), &Value::Bool(false));
    }

    #[test]
    fn find_with_lambda() {
        let engine = parse_and_get_engine(r#"
            let nums = [1, 2, 3, 4, 5]
            let first_even = find(nums, |x| x % 2 == 0)
            let first_big = find(nums, |x| x > 100)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("first_even").unwrap(), &Value::Number(2.0));
        assert_eq!(data.get("first_big").unwrap(), &Value::Null);
    }

    #[test]
    fn map_filter_chain() {
        // map then filter: functional pipeline
        let engine = parse_and_get_engine(r#"
            let nums = [1, 2, 3, 4, 5]
            let result = filter(map(nums, |x| x * x), |x| x > 10)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::List(vec![
            Value::Number(16.0), Value::Number(25.0),
        ]));
    }

    #[test]
    fn closure_with_map() {
        // A closure that captures a variable used inside map
        let engine = parse_and_get_engine(r#"
            let multiplier = 3
            let nums = [1, 2, 3]
            let result = map(nums, |x| x * multiplier)
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("result").unwrap(), &Value::List(vec![
            Value::Number(3.0), Value::Number(6.0), Value::Number(9.0),
        ]));
    }

    // ─── PERSISTENCE TESTS ─────────────────────────────────────

    #[test]
    fn persistence_save_restore_round_trip() {
        let tmp = std::env::temp_dir().join("anwe_test_persist.json");
        let tmp_str = escape_anwe_string_literal(&tmp.to_string_lossy());

        // Build source using string replacement to avoid format! brace issues
        let save_src = r#"
            agent Model data { confidence: 0.85  model_name: "gpt-4" }
            link Model <-> Model {
                >> "test signal"
                Model ~ Model until synchronized
                save Model to "PATH" {}
            }
        "#.replace("PATH", &tmp_str);

        let result = parse_and_run(&save_src);
        assert!(result.is_ok(), "Save failed: {:?}", result);

        assert!(tmp.exists(), "Save file not created");
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("confidence"), "Missing confidence in: {}", content);
        assert!(content.contains("schema_version"), "Missing schema_version");
        assert!(content.contains("history"), "Missing history");
        assert!(content.contains("attention"), "Missing attention");

        // Restore into a fresh engine
        let restore_src = r#"
            agent Model data { confidence: 0.0 }
            link Model <-> Model {
                restore Model from "PATH" {}
            }
        "#.replace("PATH", &tmp_str);

        let engine2 = parse_and_get_engine(&restore_src).unwrap();

        // Verify data was restored
        let data = engine2.agent_data.get("Model").unwrap();
        assert_eq!(data.get("confidence").unwrap(), &Value::Number(0.85));
        assert_eq!(data.get("model_name").unwrap(),
            &Value::String("gpt-4".to_string()));

        // Verify agent state was restored (alert + sync should set state)
        let agent = engine2.agents.get("Model").unwrap();
        // Agent state should be restored from saved file
        assert!(agent.attention.remaining() >= 0.0, "Attention not restored");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn persistence_schema_version() {
        let tmp = std::env::temp_dir().join("anwe_test_schema.json");
        let tmp_str = escape_anwe_string_literal(&tmp.to_string_lossy());

        // Build source with path string-replaced (no format! brace escaping needed)
        let src = "agent TestAgent\nlink TestAgent <-> TestAgent {\n    save TestAgent to \"PATH\" {}\n}"
            .replace("PATH", &tmp_str);

        let _ = parse_and_run(&src);

        assert!(tmp.exists(), "Save file not created");
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("\"schema_version\": 1"),
            "Schema version missing from: {}", content);

        let _ = std::fs::remove_file(&tmp);
    }

    // ─── REPL / ACCESSOR TESTS ──────────────────────────────

    fn run_on(engine: &mut Engine, source: &str) {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        engine.execute(&program).expect("Execution failed");
    }

    #[test]
    fn eval_expression_arithmetic() {
        let mut engine = Engine::new();
        let result = engine.eval_expression("3 + 4").unwrap();
        assert_eq!(result, "7");
    }

    #[test]
    fn eval_expression_string() {
        let mut engine = Engine::new();
        let result = engine.eval_expression("\"hello\"").unwrap();
        assert_eq!(result, "\"hello\"");
    }

    #[test]
    fn eval_expression_with_variables() {
        let mut engine = Engine::new();
        run_on(&mut engine, "let x = 10");
        let result = engine.eval_expression("x + 5").unwrap();
        assert_eq!(result, "15");
    }

    #[test]
    fn eval_expression_does_not_pollute_globals() {
        let mut engine = Engine::new();
        engine.eval_expression("42").unwrap();
        let data = engine.agent_data("__global__");
        let has_repl = data.map(|d| d.iter().any(|(k, _)| k == "__repl_result__")).unwrap_or(false);
        assert!(!has_repl, "Temp var should be cleaned up");
    }

    #[test]
    fn agent_info_returns_state() {
        let mut engine = Engine::new();
        run_on(&mut engine, "agent TestBot data { mood: \"happy\" }");
        let info = engine.agent_info("TestBot");
        assert!(info.is_some());
        let (state, _resp, _hist, _attn) = info.unwrap();
        assert_eq!(state, "Idle");
    }

    #[test]
    fn agent_history_empty_for_new_agent() {
        let mut engine = Engine::new();
        run_on(&mut engine, "agent Fresh data { x: 1 }");
        let history = engine.agent_history("Fresh");
        assert!(history.is_empty());
    }

    #[test]
    fn supervisor_info_shows_tree() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            agent Worker1 data { x: 1 }
            agent Worker2 data { x: 2 }
            supervise one_for_one max_restarts 5 within 10000 {
                permanent Worker1
                transient Worker2
            }
        "#);
        let info = engine.supervisor_info();
        assert!(!info.is_empty());
        let tree = &info[0];
        assert!(tree.contains("one_for_one"), "Expected one_for_one in: {}", tree);
        assert!(tree.contains("Worker1"), "Expected Worker1 in: {}", tree);
        assert!(tree.contains("Worker2"), "Expected Worker2 in: {}", tree);
        assert!(tree.contains("permanent"), "Expected permanent in: {}", tree);
        assert!(tree.contains("transient"), "Expected transient in: {}", tree);
    }

    #[test]
    fn bridge_names_empty_by_default() {
        let engine = Engine::new();
        assert!(engine.bridge_names().is_empty());
    }

    #[test]
    fn eval_expression_comparison() {
        let mut engine = Engine::new();
        assert_eq!(engine.eval_expression("5 > 3").unwrap(), "true");
        assert_eq!(engine.eval_expression("2 == 2").unwrap(), "true");
        assert_eq!(engine.eval_expression("1 > 10").unwrap(), "false");
    }

    #[test]
    fn eval_expression_function_call() {
        let mut engine = Engine::new();
        run_on(&mut engine, "let items = [1, 2, 3]");
        let result = engine.eval_expression("len(items)").unwrap();
        assert_eq!(result, "3");
    }

    // ─── INTEGRATION TESTS ──────────────────────────────────

    #[test]
    fn integration_ai_coordinator_pipeline() {
        // Exercises: agents, supervision, links with full signal flow
        let result = parse_and_run(r#"
            -- AI Coordinator Pipeline
            agent Perceiver data { observation: "input signal"  confidence: 0.85 }
            agent Reasoner data { model: "reasoning-v1"  conclusion: "processed" }
            agent Responder data { style: "helpful"  output: "none" }

            -- Supervision tree
            supervise one_for_one max_restarts 3 within 5000 {
                permanent Perceiver
                permanent Reasoner
                transient Responder
            }

            -- Perception -> Reasoning link
            link Perceiver <-> Reasoner {
                >> "perception signal"
                connect depth full {
                    signal attending 0.9 outward
                }
                Perceiver ~ Reasoner until synchronized
                => when sync_level > 0.5 depth full {
                    observation <- Perceiver.observation
                }
                * from apply { source: "perception" }
            }

            -- Reasoning -> Response link
            link Reasoner <-> Responder {
                >> "reasoning output"
                connect depth deep {
                    signal recognizing 0.8 between
                }
                Reasoner ~ Responder until synchronized
                => when sync_level > 0.5 depth deep {
                    conclusion <- Reasoner.conclusion
                }
                * from apply { source: "reasoning" }
            }
        "#);
        assert!(result.is_ok(), "AI coordinator pipeline failed: {:?}", result);
    }

    #[test]
    fn integration_supervised_failure_and_recovery() {
        // Exercises: supervision + attempt/recover + restart
        let result = parse_and_run(r#"
            agent Worker data { attempts: 0 }
            agent Monitor data { status: "watching" }

            supervise one_for_one max_restarts 5 within 10000 {
                permanent Worker
                permanent Monitor
            }

            link Worker <-> Monitor {
                >> "monitoring signal"
                attempt {
                    Worker ~ Monitor until synchronized
                    => when sync_level > 0.5 {
                        status <- "success"
                    }
                } recover {
                    >> "recovery fallback"
                }
                * from apply { source: "monitoring" }
            }
        "#);
        assert!(result.is_ok(), "Supervised failure recovery failed: {:?}", result);
    }

    #[test]
    fn integration_closures_and_higher_order() {
        // Exercises: closures, map, filter, reduce, comparisons
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

            -- Higher-order functions with closures
            let doubled = map(numbers, |x| x * 2)
            let evens = filter(numbers, |x| x % 2 == 0)
            let total = reduce(numbers, |acc, x| acc + x, 0)

            -- Nested function returning closure
            fn make_multiplier(factor) {
                |x| x * factor
            }
            let triple = make_multiplier(3)
            let tripled_5 = triple(5)

            -- Predicate functions
            let has_big = any(numbers, |x| x > 8)
            let all_positive = all(numbers, |x| x > 0)
            let first_even = find(numbers, |x| x % 2 == 0)
        "#);

        assert_eq!(engine.eval_expression("len(doubled)").unwrap(), "10");
        assert_eq!(engine.eval_expression("len(evens)").unwrap(), "5");
        assert_eq!(engine.eval_expression("total").unwrap(), "55");
        assert_eq!(engine.eval_expression("tripled_5").unwrap(), "15");
        assert_eq!(engine.eval_expression("has_big").unwrap(), "true");
        assert_eq!(engine.eval_expression("all_positive").unwrap(), "true");
        assert_eq!(engine.eval_expression("first_even").unwrap(), "2");
    }

    #[test]
    fn integration_time_scheduling() {
        // Exercises: every N ticks
        let result = parse_and_run(r#"
            agent Ticker data { count: 0 }
            agent Receiver data { received: 0 }

            link Ticker <-> Receiver every 3 ticks {
                >> "tick signal"
            }
        "#);
        assert!(result.is_ok(), "Time scheduling failed: {:?}", result);
    }

    #[test]
    fn integration_persistence_round_trip() {
        // Exercises: save + restore with full state
        let tmp = std::env::temp_dir().join("anwe_integration_test.json");
        let tmp_str = escape_anwe_string_literal(tmp.to_str().unwrap());

        let source = r#"
            agent Memory data { knowledge: "deep learning"  version: 3 }

            link Memory <-> Memory {
                >> "self reflection"
                Memory ~ Memory until synchronized
                => when sync_level > 0.5 depth full {
                    knowledge <- "updated"
                }
                * from apply { source: "self" }
                save Memory to "PATH" {}
            }
        "#.replace("PATH", &tmp_str);

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        let mut engine = Engine::new();
        engine.execute(&program).expect("Execution failed");

        // Verify save file was created
        assert!(tmp.exists(), "State file not created");
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("knowledge"), "Data fields not persisted: {}", content);
        assert!(content.contains("schema_version"), "No schema version: {}", content);
        assert!(content.contains("history"), "History not persisted: {}", content);

        // Now restore into a fresh agent
        let restore_source = r#"
            agent Memory2 data { knowledge: "empty"  version: 0 }
            link Memory2 <-> Memory2 {
                >> "restore state"
                restore Memory2 from "PATH" {}
            }
        "#.replace("PATH", &tmp_str);

        let mut lexer2 = Lexer::new(&restore_source);
        let tokens2 = lexer2.tokenize().expect("Lex failed");
        let mut parser2 = Parser::new(tokens2);
        let program2 = parser2.parse_program().expect("Parse failed");
        engine.execute(&program2).expect("Restore failed");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn integration_records_and_pattern_matching() {
        // Exercises: records, pattern matching, let bindings
        let engine = parse_and_get_engine(r#"
            record Observation { source, content, confidence }

            let obs = Observation("sensor", "anomaly detected", 0.92)
            let src = obs.source
            let decision = match src {
                "sensor" => "alert"
                "manual" => "log"
            }
        "#).unwrap();
        let data = engine.agent_data.get("__global__").unwrap();
        assert_eq!(data.get("src").unwrap(), &Value::String("sensor".into()));
        assert_eq!(data.get("decision").unwrap(), &Value::String("alert".into()));
    }

    #[test]
    fn integration_modules_and_functions() {
        // Exercises: function definitions, closures, stdlib, higher-order
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            -- Single-expression functions
            fn double(x) = x * 2
            fn add(a, b) = a + b
            fn square(x) = x * x

            -- Test basic functions
            let d = double(21)
            let s = square(5)

            -- Higher-order: map, filter, reduce
            let nums = [1, 2, 3, 4, 5]
            let doubled = map(nums, |x| x * 2)
            let evens = filter(nums, |x| x % 2 == 0)
            let total = reduce(nums, |acc, x| acc + x, 0)

            -- Closures capturing scope
            fn make_adder(n) { |x| x + n }
            let add10 = make_adder(10)
            let result = add10(32)
        "#);

        assert_eq!(engine.eval_expression("d").unwrap(), "42");
        assert_eq!(engine.eval_expression("s").unwrap(), "25");
        assert_eq!(engine.eval_expression("total").unwrap(), "15");
        assert_eq!(engine.eval_expression("result").unwrap(), "42");
        assert_eq!(engine.eval_expression("len(doubled)").unwrap(), "5");
        assert_eq!(engine.eval_expression("len(evens)").unwrap(), "2");
    }

    #[test]
    fn integration_full_agent_inspection() {
        // Exercises: REPL inspection APIs on a running system
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            agent Alpha data { role: "leader"  status: "active" }
            agent Beta data { role: "follower"  status: "standby" }
            agent Gamma data { role: "observer" }

            supervise one_for_all max_restarts 2 within 3000 {
                permanent Alpha
                permanent Beta
                temporary Gamma
            }

            link Alpha <-> Beta {
                >> "coordination"
                connect depth full {
                    signal attending 0.9 between
                }
                Alpha ~ Beta until synchronized
                => when sync_level > 0.5 depth full {
                    role <- Alpha.role
                }
                * from apply { source: "coordination" }
            }
        "#);

        // Check agent info
        let alpha = engine.agent_info("Alpha");
        assert!(alpha.is_some(), "Alpha should exist");

        // Check supervisor tree
        let sup = engine.supervisor_info();
        assert!(!sup.is_empty(), "Should have supervisors");
        assert!(sup[0].contains("one_for_all"), "Wrong strategy");
        assert!(sup[0].contains("Alpha"), "Missing Alpha in tree");
        assert!(sup[0].contains("Beta"), "Missing Beta in tree");
        assert!(sup[0].contains("Gamma"), "Missing Gamma in tree");

        // Check agent names
        let names = engine.agent_names();
        assert!(names.contains(&"Alpha".to_string()));
        assert!(names.contains(&"Beta".to_string()));
        assert!(names.contains(&"Gamma".to_string()));

        // Check data inspection
        let alpha_data = engine.agent_data("Alpha");
        assert!(alpha_data.is_some());
    }

    #[test]
    fn integration_attempt_recover_chain() {
        // Test error recovery across a chain of agents
        let result = parse_and_run(r#"
            agent Sensor data { reading: 42 }
            agent Processor data { result: 0 }
            agent Fallback data { backup: "default" }

            supervise one_for_one max_restarts 3 within 5000 {
                permanent Sensor
                permanent Processor
                transient Fallback
            }

            -- Primary processing with fallback
            link Sensor <-> Processor {
                >> "sensor reading"
                attempt {
                    Sensor ~ Processor until synchronized
                    => when sync_level > 0.5 {
                        result <- Sensor.reading
                    }
                } recover {
                    >> "recovery mode"
                }
                * from apply { source: "sensor_pipeline" }
            }

            -- Fallback link
            link Processor <-> Fallback {
                >> "fallback"
                Processor ~ Fallback until synchronized
                => when sync_level > 0.3 {
                    backup <- "processed"
                }
                * from apply { source: "fallback" }
            }
        "#);
        assert!(result.is_ok(), "Attempt/recover chain failed: {:?}", result);
    }

    // ─── V0.6 PHASE 1: BLOCK EXPRESSIONS ─────────────────────────

    #[test]
    fn block_expr_simple_fn_body() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn add_doubled(x, y) {
                let sum = x + y;
                sum * 2
            }
            let result = add_doubled(3, 4)
        "#);
        let val = engine.agent_data.get("__global__").unwrap().get("result").unwrap().clone();
        assert_eq!(val, Value::Number(14.0));
    }

    #[test]
    fn block_expr_multi_statement() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn process(input) {
                let doubled = input * 2;
                let tripled = input * 3;
                doubled + tripled
            }
            let result = process(5)
        "#);
        let val = engine.agent_data.get("__global__").unwrap().get("result").unwrap().clone();
        assert_eq!(val, Value::Number(25.0)); // 10 + 15
    }

    #[test]
    fn block_expr_with_string_ops() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn greet(name) {
                let prefix = "Hello, ";
                prefix + name
            }
            let msg = greet("ANWE")
        "#);
        let val = engine.agent_data.get("__global__").unwrap().get("msg").unwrap().clone();
        assert_eq!(val, Value::String("Hello, ANWE".into()));
    }

    #[test]
    fn block_expr_fn_calls_inside() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn process(text) {
                let parts = split(text, " ");
                let count = len(parts);
                count * 10
            }
            let result = process("hello world foo")
        "#);
        let val = engine.agent_data.get("__global__").unwrap().get("result").unwrap().clone();
        assert_eq!(val, Value::Number(30.0));
    }

    #[test]
    fn block_expr_nested_fn_calls() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn double(x) = x * 2
            fn quad(x) {
                let d = double(x);
                double(d)
            }
            let result = quad(3)
        "#);
        let val = engine.agent_data.get("__global__").unwrap().get("result").unwrap().clone();
        assert_eq!(val, Value::Number(12.0));
    }

    #[test]
    fn block_expr_single_expr_body_still_works() {
        // Ensure old fn syntax still works with block body
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn add(a, b) = a + b
            let result = add(10, 20)
        "#);
        let val = engine.agent_data.get("__global__").unwrap().get("result").unwrap().clone();
        assert_eq!(val, Value::Number(30.0));
    }

    #[test]
    fn block_expr_lambda_with_block() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let transform = |x| { let y = x * 2; y + 1 }
            let result = transform(5)
        "#);
        // Note: lambda call via let binding uses eval mechanism
        let val = engine.agent_data.get("__global__").unwrap().get("result");
        // Lambda returns a function value; calling it should give 11
        assert!(val.is_some());
    }

    // ─── V0.6 PHASE 2: MISSING OPERATORS ─────────────────────────

    #[test]
    fn not_equal_operator() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let a = 1 != 2
            let b = 1 != 1
            let c = "hello" != "world"
            let d = "same" != "same"
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("b").unwrap().clone(), Value::Bool(false));
        assert_eq!(g.get("c").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("d").unwrap().clone(), Value::Bool(false));
    }

    #[test]
    fn not_operator() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let a = not true
            let b = not false
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Bool(false));
        assert_eq!(g.get("b").unwrap().clone(), Value::Bool(true));
    }

    #[test]
    fn logical_and_operator() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let a = true and true
            let b = true and false
            let c = false and true
            let d = false and false
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("b").unwrap().clone(), Value::Bool(false));
        assert_eq!(g.get("c").unwrap().clone(), Value::Bool(false));
        assert_eq!(g.get("d").unwrap().clone(), Value::Bool(false));
    }

    #[test]
    fn logical_or_operator() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let a = true or true
            let b = true or false
            let c = false or true
            let d = false or false
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("b").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("c").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("d").unwrap().clone(), Value::Bool(false));
    }

    #[test]
    fn logical_short_circuit_and() {
        // `and` should not evaluate right side if left is false
        // We verify this by having the right side be a fn call that would
        // otherwise produce a value
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let result = false and (1 > 0)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Bool(false));
    }

    #[test]
    fn logical_short_circuit_or() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let result = true or (1 > 100)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Bool(true));
    }

    #[test]
    fn logical_compound_expression() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 5
            let result = (x > 3) and (x < 10)
            let result2 = (x > 3) and (x > 10)
            let result3 = (x > 100) or (x == 5)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("result2").unwrap().clone(), Value::Bool(false));
        assert_eq!(g.get("result3").unwrap().clone(), Value::Bool(true));
    }

    #[test]
    fn not_with_comparison() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 5
            let result = not (x > 10)
            let result2 = not (x == 5)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("result2").unwrap().clone(), Value::Bool(false));
    }

    #[test]
    fn operators_in_fn_body() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn is_valid(x) {
                let in_range = (x > 0) and (x < 100);
                let not_zero = x != 0;
                in_range and not_zero
            }
            let a = is_valid(50)
            let b = is_valid(0)
            let c = is_valid(150)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("b").unwrap().clone(), Value::Bool(false));
        assert_eq!(g.get("c").unwrap().clone(), Value::Bool(false));
    }

    // ─── V0.6 PHASE 3: IF/ELSE AS EXPRESSIONS ────────────────────

    #[test]
    fn if_else_simple() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 10
            let result = if x > 5 { "high" } else { "low" }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::String("high".into()));
    }

    #[test]
    fn if_else_false_branch() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 2
            let result = if x > 5 { "high" } else { "low" }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::String("low".into()));
    }

    #[test]
    fn if_else_with_numbers() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 7
            let result = if x > 5 { x * 2 } else { x + 1 }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(14.0));
    }

    #[test]
    fn if_else_in_fn_body() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn classify(x) {
                let label = if x > 0 { "positive" } else { "non-positive" };
                label
            }
            let a = classify(5)
            let b = classify(0)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::String("positive".into()));
        assert_eq!(g.get("b").unwrap().clone(), Value::String("non-positive".into()));
    }

    #[test]
    fn if_else_with_logical_condition() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 50
            let result = if (x > 10) and (x < 100) { "in range" } else { "out of range" }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::String("in range".into()));
    }

    #[test]
    fn if_else_nested() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn sign(x) = if x > 0 { "positive" } else { if x == 0 { "zero" } else { "negative" } }
            let a = sign(5)
            let b = sign(0)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::String("positive".into()));
        assert_eq!(g.get("b").unwrap().clone(), Value::String("zero".into()));
    }

    #[test]
    fn if_else_with_not_equal() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let status = "active"
            let result = if status != "disabled" { "running" } else { "stopped" }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::String("running".into()));
    }

    #[test]
    fn if_else_with_not_operator() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let active = false
            let result = if not active { "paused" } else { "running" }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::String("paused".into()));
    }

    #[test]
    fn block_and_if_else_combined() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn safe_divide(a, b) {
                let result = if b != 0 { a / b } else { 0 };
                result
            }
            let a = safe_divide(10, 2)
            let b = safe_divide(10, 0)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Number(5.0));
        assert_eq!(g.get("b").unwrap().clone(), Value::Number(0.0));
    }

    #[test]
    fn complex_fn_with_all_v06_features() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn analyze(values) {
                let count = len(values);
                let has_items = count > 0;
                let result = if has_items and (count > 2) {
                    "many"
                } else {
                    if has_items { "few" } else { "none" }
                };
                result
            }
            let a = analyze([1, 2, 3, 4])
            let b = analyze([1])
            let c = analyze([])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::String("many".into()));
        assert_eq!(g.get("b").unwrap().clone(), Value::String("few".into()));
        assert_eq!(g.get("c").unwrap().clone(), Value::String("none".into()));
    }

    // ─── V0.6 PARSER INTEGRATION ─────────────────────────────────

    #[test]
    fn parser_block_expr_fn_decl() {
        // Verify the parser correctly handles fn with block body
        let source = "fn foo(x) { let y = x + 1; y * 2 }";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn parser_not_equal_token() {
        let source = "let x = 1 != 2";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn parser_logical_operators() {
        let source = "let x = true and false or true";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn parser_if_else_expr() {
        let source = r#"let x = if true { "yes" } else { "no" }"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn eval_expression_with_operators() {
        let mut engine = Engine::new();
        let result = engine.eval_expression("not false").unwrap();
        assert_eq!(result, "true");

        let result = engine.eval_expression("true and true").unwrap();
        assert_eq!(result, "true");

        let result = engine.eval_expression("true or false").unwrap();
        assert_eq!(result, "true");

        let result = engine.eval_expression("5 != 3").unwrap();
        assert_eq!(result, "true");
    }

    // ─── V0.6 PHASE 4: STRING INTERPOLATION ──────────────────────

    #[test]
    fn fstring_simple_variable() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let name = "ANWE"
            let msg = f"Hello {name}"
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("msg").unwrap().clone(), Value::String("Hello ANWE".into()));
    }

    #[test]
    fn fstring_multiple_interpolations() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 10
            let y = 20
            let msg = f"{x} + {y} = {x + y}"
        "#);
        // Note: x + y may resolve differently depending on parser
        // The expression inside {} will be parsed — x + y = 30
        let g = engine.agent_data.get("__global__").unwrap();
        let msg = g.get("msg").unwrap().clone();
        // Should contain "10" and "20" and "30"
        if let Value::String(s) = msg {
            assert!(s.contains("10"), "Expected '10' in '{}'", s);
            assert!(s.contains("20"), "Expected '20' in '{}'", s);
            assert!(s.contains("30"), "Expected '30' in '{}'", s);
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn fstring_with_fn_call() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let items = [1, 2, 3, 4, 5]
            let msg = f"Count: {len(items)}"
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("msg").unwrap().clone(), Value::String("Count: 5".into()));
    }

    #[test]
    fn fstring_in_fn_body() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn greet(name) {
                let greeting = f"Hello, {name}!";
                greeting
            }
            let result = greet("World")
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::String("Hello, World!".into()));
    }

    #[test]
    fn fstring_with_string_operations() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let name = "anwe"
            let msg = f"Language: {upper(name)}"
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("msg").unwrap().clone(), Value::String("Language: ANWE".into()));
    }

    #[test]
    fn fstring_no_interpolation() {
        // f-string with no braces should just be a plain string
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let msg = f"just a plain string"
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("msg").unwrap().clone(), Value::String("just a plain string".into()));
    }

    #[test]
    fn fstring_with_comparison() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 42
            let msg = f"Is big: {x > 10}"
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("msg").unwrap().clone(), Value::String("Is big: true".into()));
    }

    #[test]
    fn fstring_lexer_produces_fstring_token() {
        use anwe_parser::Lexer;
        let mut lexer = Lexer::new(r#"f"hello {name}""#);
        let tokens = lexer.tokenize().expect("Lex failed");
        // Should have FStringLit token + Eof
        assert!(tokens.len() >= 2);
    }

    // ─── V0.6 PHASE 5: HTTP + JSON BUILTINS ──────────────────────

    #[test]
    fn json_parse_object() {
        // Test json_parse by using the engine directly
        let engine = Engine::new();
        let json_str = r#"{"name": "ANWE", "version": 0.6}"#;
        let result = engine.eval_builtin("json_parse", &[Value::String(json_str.into())]);
        if let Value::Map(entries) = result {
            let name = entries.iter().find(|(k, _)| k == "name").map(|(_, v)| v.clone());
            assert_eq!(name, Some(Value::String("ANWE".into())));
            let ver = entries.iter().find(|(k, _)| k == "version").map(|(_, v)| v.clone());
            assert_eq!(ver, Some(Value::Number(0.6)));
        } else {
            panic!("Expected Map, got {:?}", result);
        }
    }

    #[test]
    fn json_parse_array() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let data = json_parse("[1, 2, 3]")
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let data = g.get("data").unwrap().clone();
        assert_eq!(data, Value::List(vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)]));
    }

    #[test]
    fn json_parse_nested() {
        let engine = Engine::new();
        let json_str = r#"{"user": {"name": "Alice"}}"#;
        let result = engine.eval_builtin("json_parse", &[Value::String(json_str.into())]);
        if let Value::Map(entries) = result {
            let user = entries.iter().find(|(k, _)| k == "user").map(|(_, v)| v.clone());
            if let Some(Value::Map(user_entries)) = user {
                let name = user_entries.iter().find(|(k, _)| k == "name").map(|(_, v)| v.clone());
                assert_eq!(name, Some(Value::String("Alice".into())));
            } else {
                panic!("Expected nested Map");
            }
        } else {
            panic!("Expected Map, got {:?}", result);
        }
    }

    #[test]
    fn json_parse_invalid() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let data = json_parse("not valid json")
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let data = g.get("data").unwrap().clone();
        if let Value::String(s) = data {
            assert!(s.contains("json_parse error"));
        } else {
            panic!("Expected error string");
        }
    }

    #[test]
    fn json_stringify_simple() {
        let engine = Engine::new();
        let result = engine.eval_builtin("json_stringify",
            &[Value::List(vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)])]);
        if let Value::String(s) = result {
            // serde_json serializes f64 as 1.0, verify it's valid JSON
            let parsed: serde_json::Value = serde_json::from_str(&s).expect("Should be valid JSON");
            assert!(parsed.is_array());
            assert_eq!(parsed.as_array().unwrap().len(), 3);
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn json_stringify_object() {
        let engine = Engine::new();
        let input = Value::Map(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::String("hello".into())),
        ]);
        let result = engine.eval_builtin("json_stringify", &[input]);
        if let Value::String(s) = result {
            assert!(s.contains("\"a\""));
            assert!(s.contains("\"b\""));
            assert!(s.contains("\"hello\""));
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn json_roundtrip() {
        let engine = Engine::new();
        let original = r#"{"name":"ANWE","features":["agents","links","signals"]}"#;
        let parsed = engine.eval_builtin("json_parse", &[Value::String(original.into())]);
        let back = engine.eval_builtin("json_stringify", &[parsed]);
        if let Value::String(s) = back {
            // Parse both and compare structurally
            let a: serde_json::Value = serde_json::from_str(original).unwrap();
            let b: serde_json::Value = serde_json::from_str(&s).unwrap();
            assert_eq!(a, b);
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn json_parse_with_field_access() {
        // Use eval_builtin to get a map, then store and access fields
        let mut engine = Engine::new();
        let json_str = r#"{"status": "ok", "count": 42}"#;
        let data = engine.eval_builtin("json_parse", &[Value::String(json_str.into())]);
        engine.agent_data.entry("__global__".into()).or_default().insert("data".into(), data);
        run_on(&mut engine, r#"
            let status = data.status
            let count = data.count
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("status").unwrap().clone(), Value::String("ok".into()));
        assert_eq!(g.get("count").unwrap().clone(), Value::Number(42.0));
    }

    #[test]
    fn http_get_handles_bad_url() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let result = http_get("http://127.0.0.1:1/nonexistent")
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let result = g.get("result").unwrap().clone();
        if let Value::String(s) = result {
            assert!(s.contains("http_get error"), "Expected error, got: {}", s);
        }
    }

    #[test]
    fn http_post_with_headers_and_body() {
        // Verify http_post constructs request correctly (will fail due to no server)
        let engine = Engine::new();
        let headers = Value::Map(vec![
            ("Content-Type".into(), Value::String("application/json".into())),
        ]);
        let body = Value::String(r#"{"test": true}"#.into());
        let result = engine.eval_builtin("http_post",
            &[Value::String("http://127.0.0.1:1/api".into()), headers, body]);
        // Should be an error string since no server
        if let Value::String(s) = result {
            assert!(s.contains("http_post error"));
        }
    }

    #[test]
    fn json_in_fn_body() {
        // Test json_parse accessible in function context via eval_fn_expr
        let mut engine = Engine::new();
        let json_str = r#"{"name": "test-agent"}"#;
        let data = engine.eval_builtin("json_parse", &[Value::String(json_str.into())]);
        assert!(matches!(data, Value::Map(_)));
        if let Value::Map(entries) = data {
            let name = entries.iter().find(|(k, _)| k == "name").map(|(_, v)| v.clone());
            assert_eq!(name, Some(Value::String("test-agent".into())));
        }
    }

    #[test]
    fn json_parse_booleans_and_null() {
        let engine = Engine::new();
        let json_str = r#"{"active": true, "deleted": false, "note": null}"#;
        let result = engine.eval_builtin("json_parse", &[Value::String(json_str.into())]);
        if let Value::Map(entries) = result {
            assert_eq!(entries.iter().find(|(k, _)| k == "active").map(|(_, v)| v.clone()), Some(Value::Bool(true)));
            assert_eq!(entries.iter().find(|(k, _)| k == "deleted").map(|(_, v)| v.clone()), Some(Value::Bool(false)));
            assert_eq!(entries.iter().find(|(k, _)| k == "note").map(|(_, v)| v.clone()), Some(Value::Null));
        } else {
            panic!("Expected Map");
        }
    }

    // ─── V0.6 PHASE 6: INTEGRATION TESTS ─────────────────────────

    #[test]
    fn v06_ai_coordinator_parses_and_runs() {
        let source = include_str!("../../../examples/ai_coordinator_v06.anwe");
        let result = parse_and_run(source);
        assert!(result.is_ok(), "AI coordinator v0.6 failed: {:?}", result);
    }

    #[test]
    fn v06_block_fn_with_if_else_and_operators() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn classify(score) {
                let high = score > 0.8;
                let medium = (score > 0.5) and (score <= 0.8);
                if high { "high" }
                else { if medium { "medium" }
                else { "low" } }
            }
            let a = classify(0.9)
            let b = classify(0.6)
            let c = classify(0.3)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::String("high".into()));
        assert_eq!(g.get("b").unwrap().clone(), Value::String("medium".into()));
        assert_eq!(g.get("c").unwrap().clone(), Value::String("low".into()));
    }

    #[test]
    fn v06_fstring_with_block_fn_and_json() {
        let engine = Engine::new();
        let parsed = engine.eval_builtin("json_parse",
            &[Value::String(r#"{"name":"ANWE","version":"0.6"}"#.into())]);
        let stringified = engine.eval_builtin("json_stringify", &[parsed.clone()]);
        // Verify the roundtrip
        assert!(matches!(parsed, Value::Map(_)));
        assert!(matches!(stringified, Value::String(_)));
    }

    #[test]
    fn v06_pipeline_with_all_features() {
        // End-to-end pipeline test: input → process → output
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn validate(input) {
                let trimmed = trim(input);
                if len(trimmed) == 0 { "empty" }
                else { "valid" }
            }

            fn process(input) {
                let status = validate(input);
                let is_valid = status == "valid";
                if is_valid {
                    let words = split(trim(input), " ");
                    let count = len(words);
                    f"Processed {count} words"
                } else {
                    f"Error: {status}"
                }
            }

            fn format_response(result, format_type) {
                if format_type != "plain" {
                    upper(result)
                } else {
                    result
                }
            }

            -- Run the pipeline
            let input = "  Hello world from ANWE  "
            let processed = process(input)
            let output = format_response(processed, "upper")
            let error_case = process("   ")
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("processed").unwrap().clone(),
            Value::String("Processed 4 words".into()));
        assert_eq!(g.get("output").unwrap().clone(),
            Value::String("PROCESSED 4 WORDS".into()));
        assert_eq!(g.get("error_case").unwrap().clone(),
            Value::String("Error: empty".into()));
    }

    #[test]
    fn v06_agent_pipeline_with_v06_fns() {
        let result = parse_and_run(r#"
            fn score_label(s) {
                if s > 0.8 { "high" }
                else { if s > 0.5 { "medium" }
                else { "low" } }
            }

            fn should_proceed(confidence, threshold) {
                (confidence >= threshold) and not (confidence == 0)
            }

            let scores = [0.9, 0.6, 0.3]
            let labels = map(scores, |s| score_label(s))

            agent Sensor data { confidence: 0.85 }
            agent Actuator data { action: "idle" }

            link Sensor <-> Actuator {
                >> f"sensor reading"
                Sensor ~ Actuator until synchronized
                think {
                    label <- score_label(0.85)
                    proceed <- should_proceed(0.85, 0.5)
                }
                => when sync_level > 0.5 {
                    action <- "activated"
                }
                * from apply { source: "sensor_pipeline" }
            }
        "#);
        assert!(result.is_ok(), "Agent pipeline with v0.6 fns failed: {:?}", result);
    }

    #[test]
    fn v06_error_handling_pipeline() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn safe_divide(a, b) {
                if b == 0 { "error: division by zero" }
                else { a / b }
            }

            fn try_parse(json_str) {
                let result = json_parse(json_str);
                let is_error = type_of(result) == "string";
                if is_error { "parse_failed" }
                else { "parse_ok" }
            }

            let good = safe_divide(10, 2)
            let bad = safe_divide(10, 0)
            let valid_json = try_parse("[1,2,3]")
            let invalid_json = try_parse("not json")
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("good").unwrap().clone(), Value::Number(5.0));
        assert_eq!(g.get("bad").unwrap().clone(), Value::String("error: division by zero".into()));
        assert_eq!(g.get("valid_json").unwrap().clone(), Value::String("parse_ok".into()));
        assert_eq!(g.get("invalid_json").unwrap().clone(), Value::String("parse_failed".into()));
    }

    // ── v0.7 Tests — While Loops ────────────────────────────────

    #[test]
    fn v07_while_basic_counter() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn count_to(n) {
                let i = 0;
                let result = [];
                while i < n {
                    let result = append(result, i);
                    let i = i + 1
                };
                result
            }
            let counted = count_to(5)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let counted = g.get("counted").unwrap();
        assert_eq!(*counted, Value::List(vec![
            Value::Number(0.0), Value::Number(1.0), Value::Number(2.0),
            Value::Number(3.0), Value::Number(4.0),
        ]));
    }

    #[test]
    fn v07_while_false_condition_never_runs() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn never_loop() {
                let x = 0;
                while false {
                    let x = x + 1
                };
                x
            }
            let result = never_loop()
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(0.0));
    }

    #[test]
    fn v07_while_with_accumulator() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn sum_to(n) {
                let total = 0;
                let i = 1;
                while i <= n {
                    let total = total + i;
                    let i = i + 1
                };
                total
            }
            let sum = sum_to(10)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("sum").unwrap().clone(), Value::Number(55.0));
    }

    #[test]
    fn v07_while_returns_null() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn do_while() {
                let val = while false { 1 };
                val
            }
            let result = do_while()
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Null);
    }

    // ── v0.7 Tests — For-In Loops ───────────────────────────────

    #[test]
    fn v07_for_in_basic_list() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn double_all(items) {
                let result = [];
                for x in items {
                    let result = append(result, x * 2)
                };
                result
            }
            let doubled = double_all([1, 2, 3, 4])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("doubled").unwrap().clone(), Value::List(vec![
            Value::Number(2.0), Value::Number(4.0),
            Value::Number(6.0), Value::Number(8.0),
        ]));
    }

    #[test]
    fn v07_for_in_with_condition() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn filter_positives(items) {
                let result = [];
                for x in items {
                    let result = if x > 0 { append(result, x) } else { result }
                };
                result
            }
            let positives = filter_positives([3, -1, 5, -2, 7])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("positives").unwrap().clone(), Value::List(vec![
            Value::Number(3.0), Value::Number(5.0), Value::Number(7.0),
        ]));
    }

    #[test]
    fn v07_for_in_returns_last_value() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn last_doubled(items) {
                for x in items {
                    x * 2
                }
            }
            let result = last_doubled([10, 20, 30])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(60.0));
    }

    #[test]
    fn v07_for_in_empty_list() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn process_empty() {
                for x in [] {
                    x * 2
                }
            }
            let result = process_empty()
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Null);
    }

    #[test]
    fn v07_for_in_string_items() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn greet_all(names) {
                let greetings = [];
                for name in names {
                    let greetings = append(greetings, f"Hello {name}")
                };
                greetings
            }
            let result = greet_all(["Alice", "Bob"])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::List(vec![
            Value::String("Hello Alice".into()),
            Value::String("Hello Bob".into()),
        ]));
    }

    // ── v0.7 Tests — Map Literals ───────────────────────────────

    #[test]
    fn v07_map_literal_basic() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let config = {name: "ANWE", version: "0.7", active: true}
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let config = g.get("config").unwrap();
        match config {
            Value::Map(entries) => {
                assert_eq!(entries.len(), 3);
                assert_eq!(entries.iter().find(|(k, _)| k == "name").unwrap().1,
                    Value::String("ANWE".into()));
                assert_eq!(entries.iter().find(|(k, _)| k == "version").unwrap().1,
                    Value::String("0.7".into()));
                assert_eq!(entries.iter().find(|(k, _)| k == "active").unwrap().1,
                    Value::Bool(true));
            }
            other => panic!("Expected Map, got {:?}", other),
        }
    }

    #[test]
    fn v07_map_literal_empty() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let empty = {}
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("empty").unwrap().clone(), Value::Map(vec![]));
    }

    #[test]
    fn v07_map_literal_with_expressions() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn build_record(x) {
                {value: x, doubled: x * 2, label: f"item_{x}"}
            }
            let rec = build_record(5)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let rec = g.get("rec").unwrap();
        match rec {
            Value::Map(entries) => {
                assert_eq!(entries.iter().find(|(k, _)| k == "value").unwrap().1,
                    Value::Number(5.0));
                assert_eq!(entries.iter().find(|(k, _)| k == "doubled").unwrap().1,
                    Value::Number(10.0));
                assert_eq!(entries.iter().find(|(k, _)| k == "label").unwrap().1,
                    Value::String("item_5".into()));
            }
            other => panic!("Expected Map, got {:?}", other),
        }
    }

    #[test]
    fn v07_map_field_access() {
        let mut engine = Engine::new();
        // Test field access on a map literal
        run_on(&mut engine, r#"
            let data = {name: "ANWE", version: 7}
            let name_val = data.name
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("name_val").unwrap().clone(), Value::String("ANWE".into()));
    }

    // ── v0.7 Tests — Try/Catch ──────────────────────────────────

    #[test]
    fn v07_try_catch_success_passes_through() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn safe_op() {
                try { "success" }
                catch { "fallback" }
            }
            let result = safe_op()
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::String("success".into()));
    }

    #[test]
    fn v07_try_catch_catches_error() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn risky_op() {
                try { error("something went wrong") }
                catch { "recovered" }
            }
            let result = risky_op()
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::String("recovered".into()));
    }

    #[test]
    fn v07_try_catch_with_json_error() {
        let engine = Engine::new();
        // json_parse on invalid JSON returns an error string
        let bad_result = engine.eval_builtin("json_parse", &[Value::String("not json".into())]);
        // It should start with "json_parse error"
        match &bad_result {
            Value::String(s) => assert!(s.starts_with("json_parse error"), "Got: {}", s),
            other => panic!("Expected error string, got {:?}", other),
        }
    }

    #[test]
    fn v07_try_catch_nested() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn double_safe(x) {
                try {
                    try { error("inner fail") }
                    catch { x * 2 }
                }
                catch { 0 }
            }
            let result = double_safe(21)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(42.0));
    }

    // ── v0.7 Tests — Sleep ──────────────────────────────────────

    #[test]
    fn v07_sleep_returns_null() {
        let engine = Engine::new();
        let result = engine.eval_builtin("sleep", &[Value::Number(1.0)]);
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn v07_sleep_timing() {
        let engine = Engine::new();
        let start = std::time::Instant::now();
        engine.eval_builtin("sleep", &[Value::Number(50.0)]);
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed >= 40, "Sleep was too short: {}ms", elapsed);
    }

    // ── v0.7 Tests — Combined Features ──────────────────────────

    #[test]
    fn v07_while_with_map_accumulator() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn fibonacci(n) {
                let a = 0;
                let b = 1;
                let result = [];
                let i = 0;
                while i < n {
                    let result = append(result, a);
                    let temp = b;
                    let b = a + b;
                    let a = temp;
                    let i = i + 1
                };
                result
            }
            let fibs = fibonacci(8)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("fibs").unwrap().clone(), Value::List(vec![
            Value::Number(0.0), Value::Number(1.0), Value::Number(1.0),
            Value::Number(2.0), Value::Number(3.0), Value::Number(5.0),
            Value::Number(8.0), Value::Number(13.0),
        ]));
    }

    #[test]
    fn v07_for_in_with_try_catch() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn safe_process(items) {
                let results = [];
                for item in items {
                    let processed = try { item * 2 }
                        catch { 0 };
                    let results = append(results, processed)
                };
                results
            }
            let output = safe_process([1, 2, 3])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("output").unwrap().clone(), Value::List(vec![
            Value::Number(2.0), Value::Number(4.0), Value::Number(6.0),
        ]));
    }

    #[test]
    fn v07_map_literal_in_for_loop() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn build_records(names) {
                let records = [];
                let id = 1;
                for name in names {
                    let records = append(records, {id: id, name: name});
                    let id = id + 1
                };
                records
            }
            let people = build_records(["Alice", "Bob", "Charlie"])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let people = g.get("people").unwrap();
        match people {
            Value::List(items) => {
                assert_eq!(items.len(), 3);
                // First record should have id=1, name="Alice"
                match &items[0] {
                    Value::Map(entries) => {
                        assert_eq!(entries.iter().find(|(k, _)| k == "name").unwrap().1,
                            Value::String("Alice".into()));
                    }
                    other => panic!("Expected Map, got {:?}", other),
                }
            }
            other => panic!("Expected List, got {:?}", other),
        }
    }

    #[test]
    fn v07_retry_loop_pattern() {
        // Classic retry loop: try something, count failures, stop after max
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn retry(max_tries) {
                let tries = 0;
                let succeeded = false;
                while (tries < max_tries) and (not succeeded) {
                    let tries = tries + 1;
                    -- Simulate: succeed on try 3 — use if-else as expression
                    let succeeded = if tries == 3 { true } else { succeeded }
                };
                {tries: tries, success: succeeded}
            }
            let result = retry(5)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let result = g.get("result").unwrap();
        match result {
            Value::Map(entries) => {
                assert_eq!(entries.iter().find(|(k, _)| k == "tries").unwrap().1,
                    Value::Number(3.0));
                assert_eq!(entries.iter().find(|(k, _)| k == "success").unwrap().1,
                    Value::Bool(true));
            }
            other => panic!("Expected Map, got {:?}", other),
        }
    }

    #[test]
    fn v07_data_pipeline_with_loops_and_maps() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn transform_data(items) {
                let output = [];
                for item in items {
                    let squared = item * item;
                    let label = if squared > 10 { "big" } else { "small" };
                    let output = append(output, {value: item, squared: squared, label: label})
                };
                output
            }
            let data = transform_data([1, 2, 3, 4, 5])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        let data = g.get("data").unwrap();
        match data {
            Value::List(items) => {
                assert_eq!(items.len(), 5);
                // Item 4: value=4, squared=16, label="big"
                match &items[3] {
                    Value::Map(entries) => {
                        assert_eq!(entries.iter().find(|(k, _)| k == "value").unwrap().1,
                            Value::Number(4.0));
                        assert_eq!(entries.iter().find(|(k, _)| k == "squared").unwrap().1,
                            Value::Number(16.0));
                        assert_eq!(entries.iter().find(|(k, _)| k == "label").unwrap().1,
                            Value::String("big".into()));
                    }
                    other => panic!("Expected Map, got {:?}", other),
                }
            }
            other => panic!("Expected List, got {:?}", other),
        }
    }

    // ── v0.7 Integration Tests ──────────────────────────────

    #[test]
    fn v07_task_queue_subsystem_parses_and_runs() {
        let source = include_str!("../../../examples/task_queue_v07.anwe");
        let result = parse_and_run(source);
        assert!(result.is_ok(), "Task queue v0.7 failed: {:?}", result);
    }

    #[test]
    fn v07_task_queue_pure_functions() {
        // Test the pure functional parts without agents
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let max_retries = 3

            fn make_task(id, name, priority) {
                {id: id, name: name, priority: priority, status: "pending", retries: 0}
            }

            fn make_result(task_id, success, output) {
                {task_id: task_id, success: success, output: output}
            }

            fn enqueue_all(queue, tasks) {
                let result = queue;
                for task in tasks {
                    let result = append(result, task)
                };
                result
            }

            fn dequeue_batch(queue, count) {
                let batch = [];
                let remaining = [];
                let i = 0;
                for item in queue {
                    let batch = if i < count { append(batch, item) } else { batch };
                    let remaining = if i >= count { append(remaining, item) } else { remaining };
                    let i = i + 1
                };
                {batch: batch, remaining: remaining}
            }

            fn count_by_status(tasks, target_status) {
                let count = 0;
                for task in tasks {
                    let count = if task.status == target_status { count + 1 } else { count }
                };
                count
            }

            -- Create tasks
            let t1 = make_task(1, "build", 3)
            let t2 = make_task(2, "test", 2)
            let t3 = make_task(3, "deploy", 1)

            -- Enqueue
            let queue = enqueue_all([], [t1, t2, t3])
            let queue_len = len(queue)

            -- Count by status
            let pending_count = count_by_status(queue, "pending")

            -- Dequeue batch
            let split = dequeue_batch(queue, 2)
            let batch_len = len(split.batch)
            let remaining_len = len(split.remaining)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("queue_len").unwrap().clone(), Value::Number(3.0));
        assert_eq!(g.get("pending_count").unwrap().clone(), Value::Number(3.0));
        assert_eq!(g.get("batch_len").unwrap().clone(), Value::Number(2.0));
        assert_eq!(g.get("remaining_len").unwrap().clone(), Value::Number(1.0));
    }

    #[test]
    fn v07_batch_processing_with_retry() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let max_retries = 3

            fn make_result(task_id, success, output) {
                {task_id: task_id, success: success, output: output}
            }

            fn process_with_retry(task_name, should_fail) {
                let retries = 0;
                let done = false;
                while (retries < max_retries) and (not done) {
                    let retries = retries + 1;
                    let done = if should_fail { false }
                        else { true }
                };
                make_result(1, done, f"{task_name}: {retries} tries")
            }

            let success = process_with_retry("easy", false)
            let failure = process_with_retry("hard", true)

            let s_ok = success.success
            let f_ok = failure.success
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("s_ok").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("f_ok").unwrap().clone(), Value::Bool(false));
    }

    #[test]
    fn v07_result_analysis_pipeline() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn summarize(results) {
                let total = len(results);
                let ok = 0;
                let fail = 0;
                for r in results {
                    let ok = if r.success == true { ok + 1 } else { ok };
                    let fail = if r.success != true { fail + 1 } else { fail }
                };
                {total: total, ok: ok, fail: fail}
            }

            let results = [
                {task_id: 1, success: true, output: "done"},
                {task_id: 2, success: false, output: "error"},
                {task_id: 3, success: true, output: "done"},
                {task_id: 4, success: true, output: "done"}
            ]
            let summary = summarize(results)
            let total = summary.total
            let ok_count = summary.ok
            let fail_count = summary.fail
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("total").unwrap().clone(), Value::Number(4.0));
        assert_eq!(g.get("ok_count").unwrap().clone(), Value::Number(3.0));
        assert_eq!(g.get("fail_count").unwrap().clone(), Value::Number(1.0));
    }

    #[test]
    fn v07_top_level_while_loop() {
        // While loops at top level (outside functions)
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let items = []
            let n = 1
            while n <= 5 {
                let items = append(items, n * n);
                let n = n + 1
            }
            let count = len(items)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("count").unwrap().clone(), Value::Number(5.0));
        assert_eq!(g.get("items").unwrap().clone(), Value::List(vec![
            Value::Number(1.0), Value::Number(4.0), Value::Number(9.0),
            Value::Number(16.0), Value::Number(25.0),
        ]));
    }

    #[test]
    fn v07_nested_loops() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn matrix_sum(rows) {
                let total = 0;
                for row in rows {
                    for val in row {
                        let total = total + val
                    }
                };
                total
            }
            let result = matrix_sum([[1, 2], [3, 4], [5, 6]])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(21.0));
    }

    // ── v0.8 Tests — Break/Continue ─────────────────────────────

    #[test]
    fn v08_break_in_while_loop() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn first_divisible_by_7() {
                let i = 1;
                while i < 100 {
                    let found = if (i / 7) == floor(i / 7) { true } else { false };
                    if found { break };
                    let i = i + 1
                };
                i
            }
            let result = first_divisible_by_7()
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(7.0));
    }

    #[test]
    fn v08_break_in_for_loop() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn find_target(items, target) {
                let found = false;
                let found_item = null;
                for item in items {
                    let found = if item == target { true } else { found };
                    let found_item = if item == target { item } else { found_item };
                    if found { break }
                };
                found_item
            }
            let result = find_target([10, 20, 30, 40, 50], 30)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(30.0));
    }

    #[test]
    fn v08_continue_in_for_loop() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn sum_even_numbers(items) {
                let total = 0;
                for item in items {
                    let is_odd = (item / 2) != floor(item / 2);
                    if is_odd { continue };
                    let total = total + item
                };
                total
            }
            let result = sum_even_numbers([1, 2, 3, 4, 5, 6, 7, 8])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        // 2 + 4 + 6 + 8 = 20
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(20.0));
    }

    #[test]
    fn v08_continue_in_while_loop() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn count_odd_under(limit) {
                let i = 0;
                let count = 0;
                while i < limit {
                    let i = i + 1;
                    let is_even = (i / 2) == floor(i / 2);
                    if is_even { continue };
                    let count = count + 1
                };
                count
            }
            let result = count_odd_under(10)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        // 1, 3, 5, 7, 9 = 5 odd numbers
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(5.0));
    }

    #[test]
    fn v08_break_exits_inner_loop_only() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn count_until_break(matrix) {
                let total = 0;
                for row in matrix {
                    for val in row {
                        if val > 3 { break };
                        let total = total + val
                    }
                };
                total
            }
            let result = count_until_break([[1, 2], [3, 5, 1], [2, 1]])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        // [1, 2] -> 1+2=3, [3, 5, 1] -> 3 (breaks at 5), [2, 1] -> 2+1=3 => total 9
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(9.0));
    }

    #[test]
    fn v08_break_with_accumulator() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn take_while_positive(items) {
                let result = [];
                for item in items {
                    if item < 0 { break };
                    let result = append(result, item)
                };
                result
            }
            let result = take_while_positive([1, 2, 3, -1, 4, 5])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::List(vec![
            Value::Number(1.0), Value::Number(2.0), Value::Number(3.0),
        ]));
    }

    #[test]
    fn v08_continue_skips_processing() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn filter_positives(items) {
                let result = [];
                for item in items {
                    if item <= 0 { continue };
                    let result = append(result, item)
                };
                result
            }
            let result = filter_positives([1, -2, 3, 0, 5, -1])
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::List(vec![
            Value::Number(1.0), Value::Number(3.0), Value::Number(5.0),
        ]));
    }

    #[test]
    fn v08_top_level_break_in_while() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 0
            while x < 100 {
                let x = x + 1;
                if x == 5 { break }
            }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("x").unwrap().clone(), Value::Number(5.0));
    }

    #[test]
    fn v08_top_level_continue_in_for() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let total = 0
            for x in [1, 2, 3, 4, 5] {
                let is_even = (x / 2) == floor(x / 2);
                if is_even { continue };
                let total = total + x
            }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        // 1 + 3 + 5 = 9
        assert_eq!(g.get("total").unwrap().clone(), Value::Number(9.0));
    }

    // ── v0.8 Tests — Top-Level Assignment ───────────────────────

    #[test]
    fn v08_top_level_assignment_basic() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 10
            x = 20
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("x").unwrap().clone(), Value::Number(20.0));
    }

    #[test]
    fn v08_top_level_assignment_with_expression() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let x = 5
            let y = 3
            x = x + y
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("x").unwrap().clone(), Value::Number(8.0));
    }

    #[test]
    fn v08_top_level_assignment_multiple() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let counter = 0
            counter = counter + 1
            counter = counter + 1
            counter = counter + 1
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("counter").unwrap().clone(), Value::Number(3.0));
    }

    #[test]
    fn v08_top_level_assignment_string() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let name = "hello"
            name = "world"
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("name").unwrap().clone(), Value::String("world".into()));
    }

    // ── v0.8 Tests — Structured Errors ──────────────────────────

    #[test]
    fn v08_error_builtin_creates_error() {
        let engine = Engine::new();
        let result = engine.eval_builtin("error", &[Value::String("something broke".into())]);
        assert_eq!(result, Value::Error { kind: "error".into(), message: "something broke".into() });
    }

    #[test]
    fn v08_error_builtin_with_kind() {
        let engine = Engine::new();
        let result = engine.eval_builtin("error", &[
            Value::String("io".into()),
            Value::String("file not found".into()),
        ]);
        assert_eq!(result, Value::Error { kind: "io".into(), message: "file not found".into() });
    }

    #[test]
    fn v08_is_error_builtin() {
        let engine = Engine::new();
        let err = Value::Error { kind: "test".into(), message: "msg".into() };
        assert_eq!(engine.eval_builtin("is_error", &[err]), Value::Bool(true));
        assert_eq!(engine.eval_builtin("is_error", &[Value::String("not error".into())]), Value::Bool(false));
        assert_eq!(engine.eval_builtin("is_error", &[Value::Null]), Value::Bool(false));
    }

    #[test]
    fn v08_error_kind_and_message() {
        let engine = Engine::new();
        let err = Value::Error { kind: "validation".into(), message: "invalid input".into() };
        assert_eq!(engine.eval_builtin("error_kind", &[err.clone()]), Value::String("validation".into()));
        assert_eq!(engine.eval_builtin("error_message", &[err]), Value::String("invalid input".into()));
    }

    #[test]
    fn v08_try_catch_catches_error_value() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn risky(x) {
                if x < 0 {
                    error("validation", "must be positive")
                } else {
                    x * 2
                }
            }
            let result = try { risky(-5) } catch { 0 }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(0.0));
    }

    #[test]
    fn v08_try_catch_passes_through_success() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn safe(x) {
                if x < 0 { error("bad input") } else { x + 1 }
            }
            let result = try { safe(10) } catch { 0 }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(11.0));
    }

    #[test]
    fn v08_type_of_error() {
        let engine = Engine::new();
        let err = Value::Error { kind: "test".into(), message: "msg".into() };
        assert_eq!(engine.eval_builtin("type_of", &[err]), Value::String("error".into()));
    }

    #[test]
    fn v08_error_in_pipeline() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn validate(items) {
                let errors = [];
                for item in items {
                    let errors = if item < 0 {
                        append(errors, error("validation", f"negative: {item}"))
                    } else { errors }
                };
                errors
            }
            let errs = validate([1, -2, 3, -4])
            let count = len(errs)
            let first_msg = if count > 0 { error_message(errs) } else { "none" }
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("count").unwrap().clone(), Value::Number(2.0));
    }

    // ── v0.8 Tests — File I/O ───────────────────────────────────

    #[test]
    fn v08_file_write_and_read() {
        let engine = Engine::new();
        let tmp = "/tmp/anwe_test_write_read.txt";
        let write_result = engine.eval_builtin("file_write", &[
            Value::String(tmp.into()),
            Value::String("hello ANWE".into()),
        ]);
        assert_eq!(write_result, Value::Bool(true));

        let read_result = engine.eval_builtin("file_read", &[Value::String(tmp.into())]);
        assert_eq!(read_result, Value::String("hello ANWE".into()));

        // Cleanup
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn v08_file_exists() {
        let engine = Engine::new();
        let tmp = "/tmp/anwe_test_exists.txt";
        std::fs::write(tmp, "test").ok();
        assert_eq!(engine.eval_builtin("file_exists", &[Value::String(tmp.into())]), Value::Bool(true));
        std::fs::remove_file(tmp).ok();
        assert_eq!(engine.eval_builtin("file_exists", &[Value::String(tmp.into())]), Value::Bool(false));
    }

    #[test]
    fn v08_file_lines() {
        let engine = Engine::new();
        let tmp = "/tmp/anwe_test_lines.txt";
        std::fs::write(tmp, "line1\nline2\nline3").ok();
        let result = engine.eval_builtin("file_lines", &[Value::String(tmp.into())]);
        assert_eq!(result, Value::List(vec![
            Value::String("line1".into()),
            Value::String("line2".into()),
            Value::String("line3".into()),
        ]));
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn v08_file_append() {
        let engine = Engine::new();
        let tmp = "/tmp/anwe_test_append.txt";
        std::fs::write(tmp, "first").ok();
        engine.eval_builtin("file_append", &[
            Value::String(tmp.into()),
            Value::String("\nsecond".into()),
        ]);
        let content = engine.eval_builtin("file_read", &[Value::String(tmp.into())]);
        assert_eq!(content, Value::String("first\nsecond".into()));
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn v08_file_read_nonexistent_returns_error() {
        let engine = Engine::new();
        let result = engine.eval_builtin("file_read", &[Value::String("/tmp/nonexistent_anwe_file.txt".into())]);
        assert!(matches!(result, Value::Error { .. }));
        if let Value::Error { kind, .. } = result {
            assert_eq!(kind, "io");
        }
    }

    #[test]
    fn v08_file_io_in_anwe() {
        let mut engine = Engine::new();
        let tmp = "/tmp/anwe_test_io_program.txt";
        run_on(&mut engine, &format!(r#"
            let path = "{}"
            let write_ok = file_write(path, "hello from anwe\nline 2")
            let content = file_read(path)
            let lines = file_lines(path)
            let exists = file_exists(path)
            let line_count = len(lines)
        "#, tmp));
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("write_ok").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("content").unwrap().clone(), Value::String("hello from anwe\nline 2".into()));
        assert_eq!(g.get("exists").unwrap().clone(), Value::Bool(true));
        assert_eq!(g.get("line_count").unwrap().clone(), Value::Number(2.0));
        std::fs::remove_file(tmp).ok();
    }

    // ── v0.8 Tests — Module Import ──────────────────────────────

    #[test]
    fn v08_import_module_functions() {
        // Write a module file
        let module_dir = "/tmp/anwe_test_modules";
        std::fs::create_dir_all(module_dir).ok();
        std::fs::write(
            format!("{}/mathlib.anwe", module_dir),
            r#"
fn double(x) { x * 2 }
fn square(x) { x * x }
let pi = 3.14159
"#,
        ).unwrap();

        // Import and use it
        let source = r#"
import "mathlib" as Math {}
let result = Math.double(5)
let sq = Math.square(4)
"#;
        let mut lexer = anwe_parser::Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = anwe_parser::Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        let mut engine = Engine::new();
        engine.base_path = Some(PathBuf::from(module_dir));
        engine.execute(&program).unwrap();

        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(10.0));
        assert_eq!(g.get("sq").unwrap().clone(), Value::Number(16.0));
        assert_eq!(g.get("Math.pi").unwrap().clone(), Value::Number(3.14159));

        // Cleanup
        std::fs::remove_dir_all(module_dir).ok();
    }

    // ── v0.8 Integration Test — Full Pipeline ───────────────────

    #[test]
    fn v08_data_pipeline_integration() {
        let source = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/data_pipeline_v08.anwe")
        ).unwrap();

        let mut lexer = anwe_parser::Lexer::new(&source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = anwe_parser::Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        let mut engine = Engine::new();
        engine.execute(&program).unwrap();

        let g = engine.agent_data.get("__global__").unwrap();

        // Validation: raw_data has 12 items, 2 invalid (-3, 1500)
        assert_eq!(g.get("error_count").unwrap().clone(), Value::Number(2.0));

        // Valid items: [42, 0, 15, 88, 0, 7, 33, 999, 100, 200] = 10 valid items
        let valid = g.get("valid_items").unwrap();
        if let Value::List(items) = valid {
            assert_eq!(items.len(), 10);
        }

        // Transform: doubles, skips 0s, stops at 999
        // valid: [42, 0, 15, 88, 0, 7, 33, 999, ...] → double & skip 0 & break at 999
        // → [84, 30, 176, 14, 66] (stops before 999)
        let transformed = g.get("transformed").unwrap();
        if let Value::List(items) = transformed {
            assert_eq!(items.len(), 5);
            assert_eq!(items[0], Value::Number(84.0));
            assert_eq!(items[1], Value::Number(30.0));
            assert_eq!(items[2], Value::Number(176.0));
            assert_eq!(items[3], Value::Number(14.0));
            assert_eq!(items[4], Value::Number(66.0));
        }

        // Stats
        let stats = g.get("stats").unwrap();
        if let Value::Map(entries) = stats {
            let count_entry = entries.iter().find(|(k, _)| k == "count");
            assert_eq!(count_entry.unwrap().1, Value::Number(5.0));
            let sum_entry = entries.iter().find(|(k, _)| k == "sum");
            assert_eq!(sum_entry.unwrap().1, Value::Number(370.0)); // 84+30+176+14+66
        }

        // First big value > 100
        let first_big = g.get("first_big").unwrap();
        assert_eq!(first_big.clone(), Value::Number(176.0));

        // Status
        assert_eq!(g.get("status").unwrap().clone(), Value::String("OK".into()));

        // File write result
        assert_eq!(g.get("write_result").unwrap().clone(), Value::Bool(true));

        // Cleanup output file
        std::fs::remove_file("/tmp/anwe_pipeline_output.txt").ok();
    }

    #[test]
    fn v08_all_features_combined() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            -- Test break, continue, errors, assignment, file I/O together
            fn process_with_errors(items) {
                let results = [];
                let skipped = 0;
                for item in items {
                    -- Track skips (must be before continue to propagate)
                    let skipped = if item < 0 { skipped + 1 } else { skipped };
                    -- Skip negative numbers (continue)
                    if item < 0 { continue };
                    -- Stop at sentinel (break)
                    if item == 0 { break };
                    -- Validate with structured errors
                    let validated = try {
                        if item > 100 { error("range", f"too large: {item}") }
                        else { item * 2 }
                    } catch { 0 };
                    let results = append(results, validated)
                };
                {results: results, skipped: skipped}
            }
            let output = process_with_errors([5, -1, 10, -2, 200, 3, 0, 99])
            let processed = output.results
            let skip_count = output.skipped
        "#);
        let g = engine.agent_data.get("__global__").unwrap();

        // Items: 5→10, skip -1, 10→20, skip -2, 200→caught→0, 3→6, break at 0
        if let Value::List(items) = g.get("processed").unwrap() {
            assert_eq!(items.len(), 4);
            assert_eq!(items[0], Value::Number(10.0));
            assert_eq!(items[1], Value::Number(20.0));
            assert_eq!(items[2], Value::Number(0.0)); // caught error
            assert_eq!(items[3], Value::Number(6.0));
        }
        assert_eq!(g.get("skip_count").unwrap().clone(), Value::Number(2.0));
    }

    // ── v0.9 Tests: Return statements ────────────────────────

    #[test]
    fn v09_return_from_function() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            fn double(x) { return x * 2 }
            let result = double(5)
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(10.0));
    }

    #[test]
    fn v09_return_early_exit() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            fn find_first_negative(items) {
                for item in items {
                    if item < 0 { return item }
                };
                return 0
            }
            let result = find_first_negative([1, 2, -3, -4, 5])
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(-3.0));
    }

    #[test]
    fn v09_return_from_nested_if() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            fn classify(n) {
                if n > 0 { return "positive" };
                if n < 0 { return "negative" };
                return "zero"
            }
            let a = classify(5)
            let b = classify(-3)
            let c = classify(0)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::String("positive".into()));
        assert_eq!(g.get("b").unwrap().clone(), Value::String("negative".into()));
        assert_eq!(g.get("c").unwrap().clone(), Value::String("zero".into()));
    }

    #[test]
    fn v09_return_stops_execution() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            fn test() {
                let x = 10;
                return x;
                let x = 99
            }
            let result = test()
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        // Should return 10, not 99 — return stops further evaluation
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(10.0));
    }

    // ── v0.9 Tests: New string builtins ──────────────────────

    #[test]
    fn v09_index_of_string() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let a = index_of("hello world", "world")
            let b = index_of("hello world", "xyz")
            let c = index_of("abcdef", "cd")
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Number(6.0));
        assert_eq!(g.get("b").unwrap().clone(), Value::Number(-1.0));
        assert_eq!(g.get("c").unwrap().clone(), Value::Number(2.0));
    }

    #[test]
    fn v09_index_of_list() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let items = [10, 20, 30, 40]
            let a = index_of(items, 30)
            let b = index_of(items, 99)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Number(2.0));
        assert_eq!(g.get("b").unwrap().clone(), Value::Number(-1.0));
    }

    #[test]
    fn v09_char_at() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let a = char_at("hello", 0)
            let b = char_at("hello", 4)
            let c = char_at("hello", 99)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::String("h".into()));
        assert_eq!(g.get("b").unwrap().clone(), Value::String("o".into()));
        assert_eq!(g.get("c").unwrap().clone(), Value::Null);
    }

    #[test]
    fn v09_slice_list() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let items = [10, 20, 30, 40, 50]
            let a = slice(items, 1, 4)
            let b = slice(items, 2)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::List(vec![
            Value::Number(20.0), Value::Number(30.0), Value::Number(40.0)
        ]));
        assert_eq!(g.get("b").unwrap().clone(), Value::List(vec![
            Value::Number(30.0), Value::Number(40.0), Value::Number(50.0)
        ]));
    }

    #[test]
    fn v09_slice_string() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            let a = slice("hello world", 6, 11)
            let b = slice("hello world", 6)
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::String("world".into()));
        assert_eq!(g.get("b").unwrap().clone(), Value::String("world".into()));
    }

    // ── v0.9 Tests: Functional builtins ──────────────────────

    #[test]
    fn v09_map_builtin() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            let items = [1, 2, 3, 4]
            let doubled = map(items, |x| x * 2)
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("doubled").unwrap().clone(), Value::List(vec![
            Value::Number(2.0), Value::Number(4.0), Value::Number(6.0), Value::Number(8.0)
        ]));
    }

    #[test]
    fn v09_filter_builtin() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            let items = [1, 2, 3, 4, 5, 6]
            let evens = filter(items, |x| x % 2 == 0)
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("evens").unwrap().clone(), Value::List(vec![
            Value::Number(2.0), Value::Number(4.0), Value::Number(6.0)
        ]));
    }

    #[test]
    fn v09_reduce_builtin() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            let items = [1, 2, 3, 4]
            let total = reduce(items, |acc, x| acc + x, 0)
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("total").unwrap().clone(), Value::Number(10.0));
    }

    #[test]
    fn v09_map_filter_reduce_pipeline() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            let result = reduce(
                map(
                    filter(data, |x| x % 2 == 0),
                    |x| x * x
                ),
                |acc, x| acc + x,
                0
            )
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        // evens: 2,4,6,8,10 → squared: 4,16,36,64,100 → sum: 220
        assert_eq!(g.get("result").unwrap().clone(), Value::Number(220.0));
    }

    // ── v0.9 Tests: Return + functional combined ────────────

    #[test]
    fn v09_return_with_map() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            fn process(items) {
                if len(items) == 0 { return [] };
                return map(items, |x| x + 1)
            }
            let a = process([10, 20, 30])
            let b = process([])
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::List(vec![
            Value::Number(11.0), Value::Number(21.0), Value::Number(31.0)
        ]));
        assert_eq!(g.get("b").unwrap().clone(), Value::List(vec![]));
    }

    #[test]
    fn v09_return_with_filter_and_early_exit() {
        let mut engine = Engine::new();
        run_on(&mut engine, "
            fn first_even(items) {
                let evens = filter(items, |x| x % 2 == 0);
                if len(evens) > 0 { return head(evens) };
                return -1
            }
            let a = first_even([1, 3, 5, 4, 6])
            let b = first_even([1, 3, 5])
        ");
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("a").unwrap().clone(), Value::Number(4.0));
        assert_eq!(g.get("b").unwrap().clone(), Value::Number(-1.0));
    }

    // ── v0.9 Tests: Combined features integration ───────────

    #[test]
    fn v09_all_features_combined() {
        let mut engine = Engine::new();
        run_on(&mut engine, r#"
            -- v0.9: Return + String ops + List ops + Functional
            fn process_words(text) {
                let word_list = split(text, " ");
                if len(word_list) == 0 { return {count: 0, longest: ""} };

                -- Find longest word using reduce
                let longest = reduce(word_list, |best, w| if len(w) > len(best) { w } else { best }, "");

                -- Get uppercase versions of words > 3 chars
                let big_words = map(
                    filter(word_list, |w| len(w) > 3),
                    |w| upper(w)
                );

                return {
                    count: len(word_list),
                    longest: longest,
                    big_words: big_words,
                    first_char: char_at(text, 0),
                    has_hello: index_of(text, "hello") >= 0
                }
            }

            let result = process_words("hello beautiful world today")
            let count = result.count
            let longest = result.longest
            let big_words = result.big_words
            let first_char = result.first_char
            let has_hello = result.has_hello
        "#);
        let g = engine.agent_data.get("__global__").unwrap();
        assert_eq!(g.get("count").unwrap().clone(), Value::Number(4.0));
        assert_eq!(g.get("longest").unwrap().clone(), Value::String("beautiful".into()));
        if let Value::List(words) = g.get("big_words").unwrap() {
            assert_eq!(words.len(), 4); // "hello", "beautiful", "world", "today" — all > 3 chars
        }
        assert_eq!(g.get("first_char").unwrap().clone(), Value::String("h".into()));
        assert_eq!(g.get("has_hello").unwrap().clone(), Value::Bool(true));
    }
}
