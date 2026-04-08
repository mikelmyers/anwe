// ─────────────────────────────────────────────────────────────
// ANWE v0.1 — TOKENS
//
// The lexical atoms of the Anwe language.
// Each token carries its meaning visually:
//   <->    bidirectional (connect)
//   >>     alert (attention called)
//   =>     apply (boundary crossed)
//   <=     reject (withdrawal)
//   <<>>   converge (the between)
//   ~      sync (synchronize)
//   *      commit (irreversible)
//   <-     structural change
//   ~>     pattern flow
//   ?      pending query
// ─────────────────────────────────────────────────────────────

use std::fmt;

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset in source
    pub start: usize,
    /// Byte offset of end
    pub end: usize,
    /// Line number (1-based)
    pub line: u32,
    /// Column number (1-based)
    pub column: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, column: u32) -> Self {
        Span { start, end, line, column }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Token kinds — every lexical element of Anwe.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ─── Operators (the soul of Anwe) ───
    /// <-> bidirectional connect
    BiDir,
    /// >> alert (attention called)
    Alert,
    /// => apply (boundary crossed)
    Apply,
    /// <= reject (withdrawal)
    Reject,
    /// <<>> converge (the between)
    Converge,
    /// ~ sync (synchronize)
    Sync,
    /// * commit (irreversible)
    Commit,
    /// <- structural change (was changed by)
    StructChange,
    /// ~> pattern flow
    PatternFlow,

    // ─── Arithmetic ───
    /// +
    Plus,
    /// - (subtraction / negation)
    Minus,
    /// /
    Slash,
    /// %
    Percent,

    // ─── Delimiters ───
    /// {
    LBrace,
    /// }
    RBrace,
    /// (
    LParen,
    /// )
    RParen,
    /// [
    LBracket,
    /// ]
    RBracket,
    /// :
    Colon,
    /// ,
    Comma,
    /// .
    Dot,
    /// ?
    Question,
    /// ;
    Semicolon,

    // ─── Comparison ───
    /// >
    Greater,
    /// >=
    GreaterEq,
    /// <
    Less,
    /// <=  (context-dependent: reject vs comparison)
    LessEq,
    /// ==
    EqualEqual,
    /// !=
    BangEqual,

    // ─── Keywords ───
    /// agent
    KwAgent,
    /// link
    KwLink,
    /// connect
    KwConnect,
    /// sync (as keyword, not ~ operator)
    KwSync,
    /// apply (as keyword)
    KwApply,
    /// commit (as keyword)
    KwCommit,
    /// reject (as keyword)
    KwReject,
    /// converge (as keyword)
    KwConverge,
    /// pattern
    KwPattern,
    /// when
    KwWhen,
    /// until
    KwUntil,
    /// pending
    KwPending,
    /// data
    KwData,
    /// trace
    KwTrace,
    /// from
    KwFrom,
    /// depth
    KwDepth,
    /// history
    KwHistory,
    /// of
    KwOf,
    /// since
    KwSince,
    /// emit
    KwEmit,
    /// then
    KwThen,
    /// wait
    KwWait,
    /// tick (unit of time)
    KwTick,
    /// guidance
    KwGuidance,
    /// and
    KwAnd,
    /// or
    KwOr,
    /// each (iteration)
    KwEach,
    /// in (iteration)
    KwIn,
    /// if (conditional routing)
    KwIf,
    /// else (conditional routing)
    KwElse,

    // ─── Signal qualities (keywords) ───
    /// attending
    QualAttending,
    /// questioning
    QualQuestioning,
    /// recognizing
    QualRecognizing,
    /// disturbed
    QualDisturbed,
    /// applying
    QualApplying,
    /// completing
    QualCompleting,
    /// resting
    QualResting,

    // ─── Directions (keywords) ───
    /// inward
    DirInward,
    /// outward
    DirOutward,
    /// between
    DirBetween,
    /// diffuse
    DirDiffuse,

    // ─── Depth levels (keywords) ───
    /// surface
    DepthSurface,
    /// partial
    DepthPartial,
    /// full
    DepthFull,
    /// genuine
    DepthGenuine,
    /// deep
    DepthDeep,

    // ─── Sync conditions (keywords) ───
    /// synchronized
    SyncSynchronized,
    /// resonating
    SyncResonating,

    // ─── Pending reasons (keywords) ───
    /// receiver_not_ready
    NyReceiverNotReady,
    /// link_not_established
    NyLinkNotEstablished,
    /// sync_insufficient
    NySyncInsufficient,
    /// sender_not_ready
    NySenderNotReady,
    /// moment_not_right
    NyMomentNotRight,

    // ─── Link state keywords ───
    /// quality
    KwQuality,
    /// priority
    KwPriority,
    /// direction
    KwDirection,
    /// duration
    KwDuration,
    /// sync_level
    KwSyncLevel,
    /// alert (as keyword for conditions)
    KwAlert,
    /// is
    KwIs,
    /// absence
    KwAbsence,
    /// against
    KwAgainst,
    /// signal (as keyword)
    KwSignal,
    /// true
    KwTrue,
    /// false
    KwFalse,

    // ─── Cognitive keywords ───
    /// attention (agent budget)
    KwAttention,
    /// confidence (signal confidence)
    KwConfidence,
    /// half_life (temporal decay)
    KwHalfLife,
    /// decay (sync decay)
    KwDecay,
    /// supervise (supervision tree)
    KwSupervise,
    /// permanent (child restart)
    KwPermanent,
    /// transient (child restart)
    KwTransient,
    /// temporary (child restart)
    KwTemporary,
    /// one_for_one (restart strategy)
    KwOneForOne,
    /// one_for_all (restart strategy)
    KwOneForAll,
    /// rest_for_one (restart strategy)
    KwRestForOne,
    /// max_restarts
    KwMaxRestarts,
    /// within (time window)
    KwWithin,

    // ─── Priority levels ───
    /// critical
    PriCritical,
    /// high
    PriHigh,
    /// normal
    PriNormal,
    /// low
    PriLow,
    /// background
    PriBackground,

    // ─── Additional pending reasons ───
    /// budget_exhausted
    NyBudgetExhausted,

    // ─── Bridge keywords ───
    /// external (bridge to outside participants)
    KwExternal,
    /// bridge (network-transparent bridge)
    KwBridge,

    // ─── First-person cognition (the language AI builds in) ───
    /// mind — a first-person cognitive space. The AI IS the mind.
    KwMind,
    /// attend — what gets attention gets processed. The fundamental operation.
    KwAttend,
    /// think — computation block. Where signals become understanding.
    KwThink,
    /// express — output. What the mind transmits outward.
    KwExpress,
    /// sense — perception. What signals are available to attend to.
    KwSense,
    /// author — self-authoring. Mind generates new attend blocks at runtime.
    KwAuthor,
    /// |> pipe — chain transforms. Flow attention through stages.
    Pipe,
    /// while (loop)
    KwWhile,
    /// for (loop over collection)
    KwFor,
    /// break (exit loop)
    KwBreak,
    /// continue (skip to next iteration)
    KwContinue,
    /// attempt (error handling in links)
    KwAttempt,
    /// recover (error handling in links)
    KwRecover,
    /// try (expression-level error handling)
    KwTry,
    /// catch (expression-level error handling)
    KwCatch,

    // ─── Variable binding keywords ───
    /// let (variable binding)
    KwLet,
    /// mut (mutable binding modifier)
    KwMut,
    /// fn (function declaration)
    KwFn,
    /// return (explicit return from function)
    KwReturn,
    /// = (assignment operator)
    Assign,
    /// | (pipe character, used for lambda |x| expr)
    Bar,
    /// match (pattern matching)
    KwMatch,
    /// _ (wildcard / underscore)
    Underscore,
    /// record (user-defined type)
    KwRecord,
    /// quote (capture code as data)
    KwQuote,

    // ─── Phase 6-9 keywords ───
    /// spawn (dynamic agent creation)
    KwSpawn,
    /// retire (dynamic agent removal)
    KwRetire,
    /// sync_all (multi-party synchronization)
    KwSyncAll,
    /// broadcast (send signal to multiple agents)
    KwBroadcast,
    /// stream (continuous data stream)
    KwStream,
    /// every (periodic link execution)
    KwEvery,
    /// after (delayed one-shot link execution)
    KwAfter,
    /// rate (stream data rate)
    KwRate,
    /// ticks (time unit)
    KwTicks,
    /// import (module import)
    KwImport,
    /// as (alias in import)
    KwAs,
    /// save (serialize agent state)
    KwSave,
    /// restore (deserialize agent state)
    KwRestore,
    /// to (destination for save)
    KwTo,
    /// on_failure_of (failure cascade handler)
    KwOnFailureOf,
    /// history_query (episodic memory query)
    KwHistoryQuery,
    /// align (temporal alignment)
    KwAlign,
    /// continuous (continuous link execution)
    KwContinuous,
    /// timeout (sync timeout)
    KwTimeout,
    /// buffer (stream buffer)
    KwBuffer,
    /// samples (buffer sample count)
    KwSamples,
    /// reading (stream reading variable)
    KwReading,
    /// not (boolean negation)
    KwNot,

    // ─── Literals ───
    /// A number literal (integer or float)
    Number(f64),
    /// A string literal "..."
    StringLit(String),
    /// An f-string literal f"... {expr} ..."
    FStringLit(String),
    /// An identifier
    Ident(String),

    // ─── Special ───
    /// End of file
    Eof,
}

/// A token with its source location.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }

    pub fn is_eof(&self) -> bool {
        matches!(self.kind, TokenKind::Eof)
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TokenKind::BiDir => write!(f, "<->"),
            TokenKind::Alert => write!(f, ">>"),
            TokenKind::Apply => write!(f, "=>"),
            TokenKind::Reject => write!(f, "<="),
            TokenKind::Converge => write!(f, "<<>>"),
            TokenKind::Sync => write!(f, "~"),
            TokenKind::Commit => write!(f, "*"),
            TokenKind::StructChange => write!(f, "<-"),
            TokenKind::PatternFlow => write!(f, "~>"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Assign => write!(f, "="),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Underscore => write!(f, "_"),
            TokenKind::Number(n) => write!(f, "{}", n),
            TokenKind::StringLit(s) => write!(f, "\"{}\"", s),
            TokenKind::FStringLit(s) => write!(f, "f\"{}\"", s),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Eof => write!(f, ""),
            // Keywords display as their source text
            TokenKind::KwAgent => write!(f, "agent"),
            TokenKind::KwLink => write!(f, "link"),
            TokenKind::KwConnect => write!(f, "connect"),
            TokenKind::KwSync => write!(f, "sync"),
            TokenKind::KwApply => write!(f, "apply"),
            TokenKind::KwCommit => write!(f, "commit"),
            TokenKind::KwReject => write!(f, "reject"),
            TokenKind::KwConverge => write!(f, "converge"),
            TokenKind::KwPattern => write!(f, "pattern"),
            TokenKind::KwWhen => write!(f, "when"),
            TokenKind::KwUntil => write!(f, "until"),
            TokenKind::KwPending => write!(f, "pending"),
            TokenKind::KwData => write!(f, "data"),
            TokenKind::KwTrace => write!(f, "trace"),
            TokenKind::KwFrom => write!(f, "from"),
            TokenKind::KwDepth => write!(f, "depth"),
            TokenKind::KwHistory => write!(f, "history"),
            TokenKind::KwOf => write!(f, "of"),
            TokenKind::KwSince => write!(f, "since"),
            TokenKind::KwEmit => write!(f, "emit"),
            TokenKind::KwThen => write!(f, "then"),
            TokenKind::KwWait => write!(f, "wait"),
            TokenKind::KwTick => write!(f, "tick"),
            TokenKind::KwGuidance => write!(f, "guidance"),
            TokenKind::KwAnd => write!(f, "and"),
            TokenKind::KwOr => write!(f, "or"),
            TokenKind::KwEach => write!(f, "each"),
            TokenKind::KwIn => write!(f, "in"),
            TokenKind::KwIf => write!(f, "if"),
            TokenKind::KwElse => write!(f, "else"),
            TokenKind::KwSignal => write!(f, "signal"),
            TokenKind::KwTrue => write!(f, "true"),
            TokenKind::KwFalse => write!(f, "false"),
            TokenKind::KwLet => write!(f, "let"),
            TokenKind::KwMut => write!(f, "mut"),
            TokenKind::KwFn => write!(f, "fn"),
            TokenKind::KwReturn => write!(f, "return"),
            TokenKind::KwMatch => write!(f, "match"),
            TokenKind::KwRecord => write!(f, "record"),
            TokenKind::KwQuote => write!(f, "quote"),
            TokenKind::KwImport => write!(f, "import"),
            TokenKind::KwAs => write!(f, "as"),
            TokenKind::KwMind => write!(f, "mind"),
            TokenKind::KwAttend => write!(f, "attend"),
            TokenKind::KwThink => write!(f, "think"),
            TokenKind::KwExpress => write!(f, "express"),
            TokenKind::KwSense => write!(f, "sense"),
            TokenKind::KwAuthor => write!(f, "author"),
            TokenKind::KwWhile => write!(f, "while"),
            TokenKind::KwFor => write!(f, "for"),
            TokenKind::KwBreak => write!(f, "break"),
            TokenKind::KwContinue => write!(f, "continue"),
            TokenKind::KwAttempt => write!(f, "attempt"),
            TokenKind::KwRecover => write!(f, "recover"),
            TokenKind::KwTry => write!(f, "try"),
            TokenKind::KwCatch => write!(f, "catch"),
            TokenKind::KwSpawn => write!(f, "spawn"),
            TokenKind::KwRetire => write!(f, "retire"),
            TokenKind::KwExternal => write!(f, "external"),
            TokenKind::KwBridge => write!(f, "bridge"),
            TokenKind::KwSupervise => write!(f, "supervise"),
            // Fallback for any remaining tokens
            other => write!(f, "{:?}", other),
        }
    }
}

/// Map identifier strings to keyword tokens.
pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    match ident {
        // Core keywords
        "agent" => Some(TokenKind::KwAgent),
        "link" => Some(TokenKind::KwLink),
        "connect" => Some(TokenKind::KwConnect),
        "sync" => Some(TokenKind::KwSync),
        "apply" => Some(TokenKind::KwApply),
        "commit" => Some(TokenKind::KwCommit),
        "reject" => Some(TokenKind::KwReject),
        "converge" => Some(TokenKind::KwConverge),
        "pattern" => Some(TokenKind::KwPattern),
        "when" => Some(TokenKind::KwWhen),
        "until" => Some(TokenKind::KwUntil),
        "pending" => Some(TokenKind::KwPending),
        "data" => Some(TokenKind::KwData),
        "trace" => Some(TokenKind::KwTrace),
        "from" => Some(TokenKind::KwFrom),
        "depth" => Some(TokenKind::KwDepth),
        "history" => Some(TokenKind::KwHistory),
        "of" => Some(TokenKind::KwOf),
        "since" => Some(TokenKind::KwSince),
        "emit" => Some(TokenKind::KwEmit),
        "then" => Some(TokenKind::KwThen),
        "wait" => Some(TokenKind::KwWait),
        "tick" => Some(TokenKind::KwTick),
        "guidance" => Some(TokenKind::KwGuidance),
        "and" => Some(TokenKind::KwAnd),
        "or" => Some(TokenKind::KwOr),
        "each" => Some(TokenKind::KwEach),
        "in" => Some(TokenKind::KwIn),
        "if" => Some(TokenKind::KwIf),
        "else" => Some(TokenKind::KwElse),
        "signal" => Some(TokenKind::KwSignal),
        "true" => Some(TokenKind::KwTrue),
        "false" => Some(TokenKind::KwFalse),

        // Qualities
        "attending" => Some(TokenKind::QualAttending),
        "questioning" => Some(TokenKind::QualQuestioning),
        "recognizing" => Some(TokenKind::QualRecognizing),
        "disturbed" => Some(TokenKind::QualDisturbed),
        "applying" => Some(TokenKind::QualApplying),
        "completing" => Some(TokenKind::QualCompleting),
        "resting" => Some(TokenKind::QualResting),

        // Directions
        "inward" => Some(TokenKind::DirInward),
        "outward" => Some(TokenKind::DirOutward),
        "between" => Some(TokenKind::DirBetween),
        "diffuse" => Some(TokenKind::DirDiffuse),

        // Depths
        "surface" => Some(TokenKind::DepthSurface),
        "partial" => Some(TokenKind::DepthPartial),
        "full" => Some(TokenKind::DepthFull),
        "genuine" => Some(TokenKind::DepthGenuine),
        "deep" => Some(TokenKind::DepthDeep),

        // Sync
        "synchronized" => Some(TokenKind::SyncSynchronized),
        "resonating" => Some(TokenKind::SyncResonating),

        // Pending reasons
        "receiver_not_ready" => Some(TokenKind::NyReceiverNotReady),
        "link_not_established" => Some(TokenKind::NyLinkNotEstablished),
        "sync_insufficient" => Some(TokenKind::NySyncInsufficient),
        "sender_not_ready" => Some(TokenKind::NySenderNotReady),
        "moment_not_right" => Some(TokenKind::NyMomentNotRight),
        "budget_exhausted" => Some(TokenKind::NyBudgetExhausted),

        // Link state
        "quality" => Some(TokenKind::KwQuality),
        "priority" => Some(TokenKind::KwPriority),
        "direction" => Some(TokenKind::KwDirection),
        "duration" => Some(TokenKind::KwDuration),
        "sync_level" => Some(TokenKind::KwSyncLevel),
        "alert" => Some(TokenKind::KwAlert),
        "is" => Some(TokenKind::KwIs),
        "absence" => Some(TokenKind::KwAbsence),
        "against" => Some(TokenKind::KwAgainst),

        // Cognitive keywords
        "attention" => Some(TokenKind::KwAttention),
        "confidence" => Some(TokenKind::KwConfidence),
        "half_life" => Some(TokenKind::KwHalfLife),
        "decay" => Some(TokenKind::KwDecay),
        "supervise" => Some(TokenKind::KwSupervise),
        "permanent" => Some(TokenKind::KwPermanent),
        "transient" => Some(TokenKind::KwTransient),
        "temporary" => Some(TokenKind::KwTemporary),
        "one_for_one" => Some(TokenKind::KwOneForOne),
        "one_for_all" => Some(TokenKind::KwOneForAll),
        "rest_for_one" => Some(TokenKind::KwRestForOne),
        "max_restarts" => Some(TokenKind::KwMaxRestarts),
        "within" => Some(TokenKind::KwWithin),

        // Priority levels
        "critical" => Some(TokenKind::PriCritical),
        "high" => Some(TokenKind::PriHigh),
        "normal" => Some(TokenKind::PriNormal),
        "low" => Some(TokenKind::PriLow),
        "background" => Some(TokenKind::PriBackground),

        // First-person cognition
        "mind" => Some(TokenKind::KwMind),
        "attend" => Some(TokenKind::KwAttend),
        "think" => Some(TokenKind::KwThink),
        "express" => Some(TokenKind::KwExpress),
        "sense" => Some(TokenKind::KwSense),
        "author" => Some(TokenKind::KwAuthor),
        "while" => Some(TokenKind::KwWhile),
        "for" => Some(TokenKind::KwFor),
        "break" => Some(TokenKind::KwBreak),
        "continue" => Some(TokenKind::KwContinue),
        "attempt" => Some(TokenKind::KwAttempt),
        "recover" => Some(TokenKind::KwRecover),
        "try" => Some(TokenKind::KwTry),
        "catch" => Some(TokenKind::KwCatch),

        // Bridge
        "external" => Some(TokenKind::KwExternal),
        "bridge" => Some(TokenKind::KwBridge),

        // Variable bindings and functions
        "let" => Some(TokenKind::KwLet),
        "mut" => Some(TokenKind::KwMut),
        "fn" => Some(TokenKind::KwFn),
        "return" => Some(TokenKind::KwReturn),
        "match" => Some(TokenKind::KwMatch),
        "_" => Some(TokenKind::Underscore),
        "record" => Some(TokenKind::KwRecord),
        "quote" => Some(TokenKind::KwQuote),

        // Phase 6-9 keywords
        "spawn" => Some(TokenKind::KwSpawn),
        "retire" => Some(TokenKind::KwRetire),
        "sync_all" => Some(TokenKind::KwSyncAll),
        "broadcast" => Some(TokenKind::KwBroadcast),
        "stream" => Some(TokenKind::KwStream),
        "every" => Some(TokenKind::KwEvery),
        "after" => Some(TokenKind::KwAfter),
        "rate" => Some(TokenKind::KwRate),
        "ticks" => Some(TokenKind::KwTicks),
        "import" => Some(TokenKind::KwImport),
        "as" => Some(TokenKind::KwAs),
        "save" => Some(TokenKind::KwSave),
        "restore" => Some(TokenKind::KwRestore),
        "to" => Some(TokenKind::KwTo),
        "on_failure_of" => Some(TokenKind::KwOnFailureOf),
        "history_query" => Some(TokenKind::KwHistoryQuery),
        "align" => Some(TokenKind::KwAlign),
        "continuous" => Some(TokenKind::KwContinuous),
        "timeout" => Some(TokenKind::KwTimeout),
        "buffer" => Some(TokenKind::KwBuffer),
        "samples" => Some(TokenKind::KwSamples),
        "reading" => Some(TokenKind::KwReading),
        "not" => Some(TokenKind::KwNot),

        _ => None,
    }
}
