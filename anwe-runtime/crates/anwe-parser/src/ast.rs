// ─────────────────────────────────────────────────────────────
// ANWE v0.1 — ABSTRACT SYNTAX TREE
//
// The tree representation of an Anwe program.
// Each node represents a concept native to Anwe:
//   - Agents (minds that attend)
//   - Links (shared spaces)
//   - The seven primitives
//   - Signal expressions
//   - Patterns (reusable attention shapes)
//   - Pending handlers
//   - Supervision trees
//
// This AST is what the parser produces and what the
// runtime consumes. It is the bridge between syntax
// and execution.
// ─────────────────────────────────────────────────────────────

use crate::token::Span;

/// A complete Anwe program.
#[derive(Debug, Clone)]
pub struct Program {
    pub declarations: Vec<Declaration>,
    pub span: Span,
}

/// Top-level declarations in an Anwe program.
#[derive(Debug, Clone)]
pub enum Declaration {
    Agent(AgentDecl),
    Link(LinkDecl),
    Pattern(PatternDecl),
    HistoryView(HistoryViewExpr),
    Supervise(SuperviseDecl),
    Import(ImportDecl),
    /// A first-person cognitive space. The AI IS the mind.
    /// Contains attend blocks that execute in priority order.
    Mind(MindDecl),
    /// Variable binding: let name = expr, let mut name = expr
    Let(LetBinding),
    /// Function declaration: fn name(params) { body }
    Fn(FnDecl),
    /// Record type: record Name { field1, field2, ... }
    Record(RecordDecl),
    /// Top-level expression (while loops, for-in loops, etc.)
    TopLevelExpr(Expr),
    /// Top-level assignment: name = expr (reassignment of existing variable)
    Assign { name: String, value: Expr },
}

// ─── AGENT ───────────────────────────────────────

/// An agent declaration: a mind that can attend.
#[derive(Debug, Clone)]
pub struct AgentDecl {
    pub name: String,
    /// Optional attention budget (0.0 to 1.0)
    pub attention: Option<f64>,
    /// Optional carried history from prior encounters
    pub data: Vec<KeyValue>,
    /// Optional external source — bridges to outside participants.
    /// When present, this agent's signals are routed through the bridge
    /// to whatever is on the other side.
    pub external: Option<ExternalSource>,
    pub span: Span,
}

/// External source declaration for bridge participants.
///
/// Syntax: external("kind", "address")
///
/// The kind and address are transport-level hints.
/// The runtime does not interpret them — they are passed
/// to the participant registry for lookup and routing.
#[derive(Debug, Clone)]
pub struct ExternalSource {
    /// Transport kind (e.g., "python", "grpc", "wasm", "callback")
    pub kind: String,
    /// Transport-specific address
    pub address: String,
    pub span: Span,
}

// ─── LINK ───────────────────────────────────────

/// A link: shared space between agents.
/// Everything inside runs simultaneously.
#[derive(Debug, Clone)]
pub struct LinkDecl {
    /// First agent in the <-> relationship
    pub agent_a: String,
    /// Second agent in the <-> relationship
    pub agent_b: String,
    /// Scheduling priority for this link
    pub priority: Option<LinkPriority>,
    /// Optional schedule: every N ticks, after N ticks, or continuous
    pub schedule: Option<LinkSchedule>,
    /// Optional failure cascade: on_failure_of <agent>
    pub on_failure_of: Option<String>,
    /// The body: primitives that run within this link
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

/// Scheduling modifier for links.
#[derive(Debug, Clone)]
pub enum LinkSchedule {
    /// Execute every N ticks (periodic)
    Every { ticks: f64 },
    /// Execute after N ticks (delayed one-shot)
    After { ticks: f64 },
    /// Continuous stream processing mode
    Continuous,
}

/// Priority level for link scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

/// Expressions that can appear inside a link body.
#[derive(Debug, Clone)]
pub enum LinkExpr {
    Alert(AlertExpr),
    Connect(ConnectBlock),
    Sync(SyncExpr),
    Apply(ApplyExpr),
    Commit(CommitExpr),
    Reject(RejectExpr),
    Converge(ConvergeBlock),
    Emit(EmitExpr),
    When(WhenExpr),
    PendingHandler(PendingHandlerExpr),
    PatternUse(PatternUseExpr),
    /// Iteration: each <var> in <collection> { ... }
    Each(EachExpr),
    /// Conditional routing: if <condition> { ... } [else { ... }]
    IfElse(IfElseExpr),
    /// Dynamic agent creation: spawn <name> from <template> { ... }
    Spawn(SpawnExpr),
    /// Dynamic agent removal: retire <name> { ... }
    Retire(RetireExpr),
    /// Multi-party sync: sync_all [agents] until <condition> { ... }
    SyncAll(SyncAllExpr),
    /// Broadcast signal to multiple agents: broadcast [agents] { ... }
    Broadcast(BroadcastExpr),
    /// Multi-agent convergence: converge [agents] { ... }
    MultiConverge(MultiConvergeExpr),
    /// Continuous data stream: stream <source> rate <N> { ... }
    Stream(StreamExpr),
    /// Save agent state: save <agent> to <path> { ... }
    Save(SaveExpr),
    /// Restore agent state: restore <agent> from <path> { ... }
    Restore(RestoreExpr),
    /// Episodic memory query: history_query <agent> { ... }
    HistoryQueryBlock(HistoryQueryExpr),
    /// Temporal alignment: align [agents] to <ref> { ... }
    Align(AlignExpr),
    /// Buffer in stream: buffer samples <N> { ... }
    Buffer(BufferExpr),
    /// Think: local computation block. Where signals become understanding.
    /// Produces local bindings that don't persist unless applied.
    Think(ThinkExpr),
    /// Express: transmit a signal outward. First-person output.
    Express(ExpressExpr),
    /// Sense: perception. Bind information about available signals.
    Sense(SenseExpr),
    /// Author: self-authoring. Mind generates new attend blocks at runtime.
    Author(AuthorExpr),
    /// While loop: while <condition> { ... }
    While(WhileExpr),
    /// Error handling: attempt { ... } recover { ... }
    Attempt(AttemptExpr),
    /// Variable binding inside a link/attend body: let name = expr
    Let(LetBinding),
    /// Assignment to a mutable variable: name = expr
    Assign(AssignExpr),
}

// ─── ALERT ────────────────────────────────────

/// Alert: >> — what calls attention.
#[derive(Debug, Clone)]
pub struct AlertExpr {
    /// Optional signal attributes (quality, priority, etc.)
    pub attrs: Option<SignalAttrs>,
    /// What the alert carries
    pub expression: Expr,
    pub span: Span,
}

// ─── CONNECT ─────────────────────────────────────

/// Connect block: sustained bidirectional presence.
#[derive(Debug, Clone)]
pub struct ConnectBlock {
    /// Optional depth level
    pub depth: Option<DepthLevel>,
    /// Signal expressions within connection
    pub pulses: Vec<SignalExpr>,
    pub span: Span,
}

/// A signal expression within a connect block.
#[derive(Debug, Clone)]
pub struct SignalExpr {
    pub quality: SignalQuality,
    pub priority: f64,
    pub direction: SignalDirection,
    /// Optional: what this signal carries
    pub data: Option<Expr>,
    /// Optional: what this signal leaves behind
    pub trace: Vec<KeyValue>,
    pub span: Span,
}

// ─── SYNC ─────────────────────────────────────

/// Sync: A ~ B until <condition> [decay N] [timeout N { ... }]
#[derive(Debug, Clone)]
pub struct SyncExpr {
    pub agent_a: String,
    pub agent_b: String,
    pub until: SyncCondition,
    /// Optional decay half-life in ticks. Sync level fades
    /// if agents don't maintain contact.
    pub decay: Option<u32>,
    /// Optional timeout with recovery options.
    pub timeout: Option<SyncTimeout>,
    pub span: Span,
}

/// Timeout configuration for sync operations.
#[derive(Debug, Clone)]
pub struct SyncTimeout {
    pub timeout_ticks: f64,
    pub options: Vec<KeyValue>,
    pub span: Span,
}

/// Synchronization condition for sync.
#[derive(Debug, Clone)]
pub enum SyncCondition {
    Synchronized,
    Resonating,
    CoherenceThreshold { op: ComparisonOp, value: f64 },
}

// ─── APPLY ───────────────────────────────────

/// Apply: => when <condition> — boundary dissolution.
#[derive(Debug, Clone)]
pub struct ApplyExpr {
    pub condition: Condition,
    pub depth: Option<DepthLevel>,
    /// Structural changes that occur
    pub changes: Vec<StructuralChange>,
    pub span: Span,
}

/// A structural change: identifier <- expression
/// "This changed in me because of the encounter"
#[derive(Debug, Clone)]
pub struct StructuralChange {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

// ─── COMMIT ──────────────────────────────────

/// Commit: * from <source> — irreversible change.
#[derive(Debug, Clone)]
pub struct CommitExpr {
    pub source: CommitSource,
    pub entries: Vec<KeyValue>,
    pub span: Span,
}

/// Source of commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitSource {
    Apply,
    Reject,
}

// ─── REJECT ───────────────────────────────────

/// Reject: <= when <condition> — intelligent withdrawal.
#[derive(Debug, Clone)]
pub struct RejectExpr {
    pub condition: Condition,
    pub data: Option<Expr>,
    pub span: Span,
}

// ─── CONVERGE ──────────────────────────────────

/// Converge: A <<>> B — what emerges in the between.
#[derive(Debug, Clone)]
pub struct ConvergeBlock {
    pub agent_a: String,
    pub agent_b: String,
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

// ─── EMIT ────────────────────────────────────

/// Emit: release a signal into the link.
#[derive(Debug, Clone)]
pub struct EmitExpr {
    pub attrs: SignalAttrs,
    pub expression: Expr,
    pub span: Span,
}

// ─── WHEN ────────────────────────────────────

/// When: conditional on link state.
#[derive(Debug, Clone)]
pub struct WhenExpr {
    pub condition: Condition,
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

// ─── PENDING HANDLER ─────────────────────────

/// Pending handler: pending? <reason> { ... }
#[derive(Debug, Clone)]
pub struct PendingHandlerExpr {
    pub reason: PendingReason,
    pub body: Vec<PendingAction>,
    pub span: Span,
}

/// Actions within a pending handler.
#[derive(Debug, Clone)]
pub enum PendingAction {
    Wait { ticks: f64 },
    Guidance(String),
    Then(LinkExpr),
}

// ─── PATTERN ─────────────────────────────────

/// Pattern declaration: reusable attention shapes.
#[derive(Debug, Clone)]
pub struct PatternDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

/// Pattern parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ref: Option<String>,
}

/// Pattern use: ~> <name>(<args>)
#[derive(Debug, Clone)]
pub struct PatternUseExpr {
    pub name: String,
    pub args: Vec<Expr>,
    pub span: Span,
}

// ─── HISTORY VIEW ───────────────────────────

/// History view: history of <agent>
#[derive(Debug, Clone)]
pub struct HistoryViewExpr {
    pub agent: String,
    pub since: Option<Box<Expr>>,
    pub depth: Option<DepthLevel>,
    pub span: Span,
}

// ─── SUPERVISION ────────────────────────────

/// Supervision tree declaration.
///
/// supervise <strategy> [max_restarts N within T] {
///     permanent Agent1
///     transient Agent2
///     temporary Agent3
/// }
#[derive(Debug, Clone)]
pub struct SuperviseDecl {
    pub strategy: SuperviseStrategy,
    pub max_restarts: Option<u32>,
    pub time_window: Option<u32>,
    pub children: Vec<SupervisedChild>,
    pub span: Span,
}

/// Restart strategy for supervision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuperviseStrategy {
    OneForOne,
    OneForAll,
    RestForOne,
}

/// A child in a supervision tree.
#[derive(Debug, Clone)]
pub struct SupervisedChild {
    pub restart: ChildRestartType,
    pub agent: String,
    pub span: Span,
}

/// How a supervised child should be restarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildRestartType {
    Permanent,
    Transient,
    Temporary,
}

// ─── SHARED TYPES ────────────────────────────

/// Signal quality — matches the seven qualities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalQuality {
    Attending,
    Questioning,
    Recognizing,
    Disturbed,
    Applying,
    Completing,
    Resting,
}

/// Signal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDirection {
    Inward,
    Outward,
    Between,
    Diffuse,
}

/// Depth levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthLevel {
    Surface,
    Partial,
    Full,
    Genuine,
    Deep,
}

/// Comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Equal,
    NotEqual,
}

/// Pending reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingReason {
    ReceiverNotReady,
    LinkNotEstablished,
    SyncInsufficient,
    SenderNotReady,
    MomentNotRight,
    BudgetExhausted,
}

/// A condition for when/apply/reject.
#[derive(Debug, Clone)]
pub enum Condition {
    SyncLevel { op: ComparisonOp, value: f64 },
    Priority { op: ComparisonOp, value: f64 },
    Confidence { op: ComparisonOp, value: f64 },
    Attention { op: ComparisonOp, value: f64 },
    AlertIs(String),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    /// General field comparison: Agent.field op expr
    /// Used for conditions like PrimaryModel.circuit_breaker == "open"
    FieldCompare { left: Expr, op: ComparisonOp, right: Expr },
}

/// Signal attributes block.
#[derive(Debug, Clone)]
pub struct SignalAttrs {
    pub quality: Option<SignalQuality>,
    pub priority: Option<f64>,
    pub direction: Option<SignalDirection>,
    pub duration: Option<f64>,
    pub confidence: Option<f64>,
    pub half_life: Option<f64>,
}

/// Key-value pair for data/trace/commit entries.
#[derive(Debug, Clone)]
pub struct KeyValue {
    pub key: String,
    pub value: Expr,
    pub span: Span,
}

/// General expression.
#[derive(Debug, Clone)]
pub enum Expr {
    StringLit(String),
    Number(f64),
    Bool(bool),
    Ident(String),
    FieldAccess { object: String, field: String },
    HistoryOf(Box<HistoryViewExpr>),
    /// Binary operation: left op right
    BinaryOp { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    /// Unary operation: -expr
    UnaryNeg(Box<Expr>),
    /// Boolean negation: not expr
    Not(Box<Expr>),
    /// Logical AND: left and right (short-circuit)
    LogicalAnd { left: Box<Expr>, right: Box<Expr> },
    /// Logical OR: left or right (short-circuit)
    LogicalOr { left: Box<Expr>, right: Box<Expr> },
    /// List literal: [expr, expr, ...]
    ListLit(Vec<Expr>),
    /// Index access: expr[index]
    IndexAccess { object: Box<Expr>, index: Box<Expr> },
    /// Pipe: value |> transform |> transform
    /// Chains transformations. Each stage receives the result of the previous.
    Pipe { stages: Vec<Expr> },
    /// Function call: name(arg1, arg2, ...)
    /// Used for standard library functions, user-defined functions, and I/O operations.
    Call { name: String, args: Vec<Expr> },
    /// Lambda expression: |params| body_expr
    Lambda { params: Vec<String>, body: Box<Expr> },
    /// Match expression: match expr { pattern => result, ... }
    Match { subject: Box<Expr>, arms: Vec<MatchArm> },
    /// Comparison expression: left op right → Bool
    Comparison { left: Box<Expr>, op: ComparisonOp, right: Box<Expr> },
    /// Quoted code: quote { code } — captures source text as a Value
    Quote(String),
    /// Block expression: { stmt; stmt; expr } — evaluates statements, returns last expr
    Block { statements: Vec<BlockStatement>, result: Box<Expr> },
    /// If/else expression: if cond { expr } else { expr }
    IfElse { condition: Box<Expr>, then_branch: Box<Expr>, else_branch: Option<Box<Expr>> },
    /// Interpolated string: f"Hello {name}, you have {len(items)} items"
    InterpolatedString { parts: Vec<StringPart> },
    /// While loop expression: while cond { body } — returns null, used for side effects
    WhileExpr { condition: Box<Expr>, body: Box<Expr> },
    /// For-in loop expression: for item in collection { body }
    ForIn { var: String, collection: Box<Expr>, body: Box<Expr> },
    /// Try/catch expression: try { expr } catch { fallback }
    TryCatch { body: Box<Expr>, catch_body: Box<Expr> },
    /// Map literal: {key: value, key2: value2}
    MapLit(Vec<(String, Expr)>),
    /// Break: exit the enclosing loop
    Break,
    /// Continue: skip to next iteration of enclosing loop
    Continue,
    /// Return: early return from function with a value
    Return(Box<Expr>),
}

/// A part of an interpolated string.
#[derive(Debug, Clone)]
pub enum StringPart {
    /// Literal text segment
    Literal(String),
    /// An expression to be evaluated and converted to string
    Expression(Expr),
}

/// A statement inside a block expression.
#[derive(Debug, Clone)]
pub enum BlockStatement {
    /// let name = expr, let mut name = expr
    Let { name: String, mutable: bool, value: Expr },
    /// name = expr (reassignment of mutable variable)
    Assign { name: String, value: Expr },
    /// Expression evaluated for side effects (e.g. print("hi"))
    Expr(Expr),
}

/// A single arm in a match expression.
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Expr,
}

/// Patterns for match arms.
#[derive(Debug, Clone)]
pub enum MatchPattern {
    /// Match a literal value
    Literal(Expr),
    /// Wildcard: matches anything
    Wildcard,
    /// Bind to a variable name (captures the value)
    Binding(String),
}

/// Binary operators for arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

// ─── EACH (ITERATION) ───────────────────────

/// Each: iteration over a collection.
///
/// each <var> in <collection_expr> { <link_expr>* }
#[derive(Debug, Clone)]
pub struct EachExpr {
    pub var: String,
    pub collection: Expr,
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

// ─── IF/ELSE (CONDITIONAL ROUTING) ──────────

/// If/else: conditional routing.
///
/// if <condition> { <link_expr>* } [else { <link_expr>* }]
#[derive(Debug, Clone)]
pub struct IfElseExpr {
    pub condition: Condition,
    pub then_body: Vec<LinkExpr>,
    pub else_body: Vec<LinkExpr>,
    pub span: Span,
}

// ─── IMPORT (MODULE SYSTEM) ─────────────────

/// Import declaration: import "module" as Name { agents: [...] links: [...] }
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub module_path: String,
    pub alias: String,
    pub entries: Vec<KeyValue>,
    pub span: Span,
}

// ─── SPAWN / RETIRE (DYNAMIC AGENTS) ──────

/// Spawn: create a new agent at runtime from a template.
///
/// spawn <name> from <template> { key: value ... }
#[derive(Debug, Clone)]
pub struct SpawnExpr {
    pub name: String,
    pub template: String,
    pub data: Vec<KeyValue>,
    pub span: Span,
}

/// Retire: remove a dynamically created agent.
///
/// retire <name> { key: value ... }
#[derive(Debug, Clone)]
pub struct RetireExpr {
    pub name: String,
    pub data: Vec<KeyValue>,
    pub span: Span,
}

// ─── SYNC_ALL (MULTI-PARTY SYNC) ──────────

/// Multi-party synchronization: sync_all [agents] until <condition> { ... }
#[derive(Debug, Clone)]
pub struct SyncAllExpr {
    pub agents: Vec<String>,
    pub until: SyncCondition,
    pub options: Vec<KeyValue>,
    pub span: Span,
}

// ─── BROADCAST ─────────────────────────────

/// Broadcast signal to multiple agents: broadcast [agents] { signal ... }
#[derive(Debug, Clone)]
pub struct BroadcastExpr {
    pub agents: Vec<String>,
    pub body: Vec<SignalExpr>,
    pub span: Span,
}

// ─── MULTI-AGENT CONVERGE ──────────────────

/// Multi-agent convergence: converge [agents] { key: value ... }
#[derive(Debug, Clone)]
pub struct MultiConvergeExpr {
    pub agents: Vec<String>,
    pub options: Vec<KeyValue>,
    pub span: Span,
}

// ─── STREAM ────────────────────────────────

/// Stream processing: stream <source> rate <N> { <link_expr>* }
#[derive(Debug, Clone)]
pub struct StreamExpr {
    pub source: String,
    pub rate: f64,
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

// ─── SAVE / RESTORE ────────────────────────

/// Save agent state: save <agent> to <path> { key: value ... }
#[derive(Debug, Clone)]
pub struct SaveExpr {
    pub agent: String,
    pub path: String,
    pub options: Vec<KeyValue>,
    pub span: Span,
}

/// Restore agent state: restore <agent> from <path> { key: value ... }
#[derive(Debug, Clone)]
pub struct RestoreExpr {
    pub agent: String,
    pub path: String,
    pub options: Vec<KeyValue>,
    pub span: Span,
}

// ─── HISTORY QUERY ─────────────────────────

/// Episodic memory query: history_query <agent> { key: value ... }
#[derive(Debug, Clone)]
pub struct HistoryQueryExpr {
    pub agent: String,
    pub options: Vec<KeyValue>,
    pub span: Span,
}

// ─── ALIGN ─────────────────────────────────

/// Temporal alignment: align [agents] to <reference> { key: value ... }
#[derive(Debug, Clone)]
pub struct AlignExpr {
    pub agents: Vec<String>,
    pub reference: Expr,
    pub options: Vec<KeyValue>,
    pub span: Span,
}

// ─── BUFFER ────────────────────────────────

/// Buffer in stream: buffer samples <N> { <link_expr>* }
#[derive(Debug, Clone)]
pub struct BufferExpr {
    pub samples: f64,
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

// ─── WHILE (LOOP) ───────────────────────────

/// While loop: execute body while condition holds.
///
/// while <condition> { <link_expr>* }
///
/// The condition is re-evaluated each iteration.
/// Maximum iteration limit prevents infinite loops.
#[derive(Debug, Clone)]
pub struct WhileExpr {
    pub condition: Condition,
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

// ─── ATTEMPT / RECOVER (ERROR HANDLING) ─────

/// Error handling: attempt body, recover on failure.
///
/// attempt { <link_expr>* } recover { <link_expr>* }
///
/// If any expression in the attempt body produces an
/// engine error, execution switches to the recover body.
/// The error message is bound as "__error" in the scope.
#[derive(Debug, Clone)]
pub struct AttemptExpr {
    pub body: Vec<LinkExpr>,
    pub recover: Vec<LinkExpr>,
    pub span: Span,
}

// ═══════════════════════════════════════════════════
// FIRST-PERSON COGNITION — THE LANGUAGE AI BUILDS IN
// ═══════════════════════════════════════════════════
//
// These constructs flip ANWE from third-person choreography
// to first-person cognition. The AI IS the mind writing code.
//
// mind     = the cognitive space (replaces agent + self-link)
// attend   = what gets attention gets processed (attention IS control flow)
// think    = computation (where signals become understanding)
// express  = output (what the mind transmits outward)

// ─── MIND ──────────────────────────────────

/// A first-person cognitive space.
///
/// The AI IS the mind. It doesn't declare agents from outside.
/// It declares what it attends to, how it thinks, what it expresses.
///
/// Syntax:
///   mind <name> [data { ... }] {
///     attend "label" priority <N> { ... }
///     attend "label" priority <N> { ... }
///   }
///
/// Attend blocks execute in priority order (highest first).
/// Lower-priority blocks may never execute if higher-priority
/// ones consume all available attention. This is natural —
/// not failure, but the finite reality of cognition.
#[derive(Debug, Clone)]
pub struct MindDecl {
    pub name: String,
    /// Optional attention budget (0.0 to 1.0). None = default budget.
    pub attention: Option<f64>,
    /// Optional data the mind carries
    pub data: Vec<KeyValue>,
    /// Attend blocks — executed in priority order
    pub attend_blocks: Vec<AttendBlock>,
    pub span: Span,
}

// ─── ATTEND ────────────────────────────────

/// An attend block: what the mind pays attention to.
///
/// Attention IS control flow. What gets attended to gets processed.
/// What doesn't get attended to decays.
///
/// Syntax:
///   attend "description" [priority <N>] {
///     <link_expr>*
///   }
///
/// The priority (0.0 to 1.0) determines execution order.
/// Higher priority attend blocks run first.
/// If attention budget is exhausted, lower-priority blocks
/// are skipped — they decay, like unattended thoughts.
#[derive(Debug, Clone)]
pub struct AttendBlock {
    /// Human-readable description of what's being attended to
    pub label: String,
    /// Priority determines execution order (0.0 to 1.0)
    pub priority: f64,
    /// The body: what happens when this is attended to
    pub body: Vec<LinkExpr>,
    pub span: Span,
}

// ─── THINK ─────────────────────────────────

/// Think: local computation block.
///
/// Where signals become understanding. Produces local bindings
/// that exist within the current attend block's scope.
/// Bindings don't persist to agent state unless explicitly
/// applied and committed.
///
/// Syntax:
///   think {
///     <name> <- <expr>
///     <name> <- <expr>
///   }
///
/// This is how the AI computes. Not by calling functions.
/// By binding understanding to names through encounter
/// with the data.
#[derive(Debug, Clone)]
pub struct ThinkExpr {
    /// Local bindings: name <- expression
    pub bindings: Vec<StructuralChange>,
    pub span: Span,
}

// ─── EXPRESS ───────────────────────────────

/// Express: transmit understanding outward.
///
/// The dual of attend. Attend is perception. Express is voice.
/// What the mind produces. Its output to the world.
///
/// Syntax:
///   express [{ quality: <q>, priority: <p> }] <expr>
///
/// An express without attributes uses quality: recognizing,
/// priority: 0.5 as defaults — the mind recognizes something
/// and shares it at normal priority.
#[derive(Debug, Clone)]
pub struct ExpressExpr {
    /// Optional signal attributes for the expressed signal
    pub attrs: Option<SignalAttrs>,
    /// What is being expressed
    pub expression: Expr,
    pub span: Span,
}

// ─── SENSE ────────────────────────────────

/// Sense: perception of the signal landscape.
///
/// Populates bindings with information about what signals
/// are available, their qualities, priorities, and counts.
/// This is how a mind perceives its environment before
/// deciding what to attend to.
///
/// Syntax:
///   sense {
///     <name> <- <expr>
///   }
///
/// Available built-in identifiers within sense:
///   signal_count   — number of signals in the channel
///   max_priority   — highest priority signal available
///   qualities      — list of distinct signal qualities
///   sync_level     — current synchronization level
///   attention      — remaining attention budget
#[derive(Debug, Clone)]
pub struct SenseExpr {
    /// Bindings populated from the signal landscape
    pub bindings: Vec<StructuralChange>,
    pub span: Span,
}

// ─── AUTHOR ───────────────────────────────

/// Author: self-authoring of new attend blocks.
///
/// The mind generates new cognitive structure at runtime.
/// This is irreversible — authored attend blocks become
/// part of the mind's structure.
///
/// Syntax:
///   author attend "label" priority <N> {
///     <link_expr>*
///   }
///
/// The authored attend block is added to the mind's
/// attention queue and may execute in the current or
/// future cycles depending on priority and remaining
/// attention budget.
#[derive(Debug, Clone)]
pub struct AuthorExpr {
    /// The attend block being authored
    pub block: AttendBlock,
    pub span: Span,
}

// ─── LET BINDING ─────────────────────────────

/// Variable binding: let name = expr, let mut name = expr
///
/// `let` is declaration. `let mut` is explicit about change.
/// This aligns with Anwe's philosophy: change is intentional
/// and visible, never accidental.
///
/// Syntax:
///   let <name> = <expr>
///   let mut <name> = <expr>
///
/// Works both at top-level (program scope) and inside
/// link/attend bodies (local scope).
#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub mutable: bool,
    pub value: Expr,
    pub span: Span,
}

// ─── ASSIGN ──────────────────────────────────

/// Assignment to a mutable variable: name = expr
///
/// Only valid for variables declared with `let mut`.
/// Attempting to assign to an immutable binding is an error.
///
/// Syntax:
///   <name> = <expr>
#[derive(Debug, Clone)]
pub struct AssignExpr {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

// ─── FN DECLARATION ──────────────────────────

/// Function declaration: fn name(params) { body }
///
/// Functions are computational tools that exist alongside
/// the seven primitives. They are not attention shapes.
/// They are not patterns. They are values that can be
/// passed, stored, and called.
///
/// Syntax:
///   fn name(param1, param2, ...) { expr }
///
/// The body is a single expression (the return value).
/// Multiple statements are expressed as blocks in the future.
#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Expr,
    pub span: Span,
}

/// Record type declaration: record Name { field1, field2, ... }
/// Records are Maps with named fields and a constructor function.
#[derive(Debug, Clone)]
pub struct RecordDecl {
    pub name: String,
    pub fields: Vec<String>,
    pub span: Span,
}
