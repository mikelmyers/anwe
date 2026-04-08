// ─────────────────────────────────────────────────────────────
// ANWE v0.1 — PARSER
//
// Recursive descent parser for the Anwe language.
// Transforms a token stream into an AST.
//
// Hand-written for clarity and error quality.
// Anwe error messages should guide, not blame —
// like pending guides the agent.
// ─────────────────────────────────────────────────────────────

use crate::ast::*;
use crate::token::{Span, Token, TokenKind};

/// Parse error with guidance.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    /// Guidance: what the parser thinks you meant to write.
    /// Like pending guidance — what to do next.
    pub guidance: Option<String>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at {}: {}", self.span, self.message)?;
        if let Some(ref guidance) = self.guidance {
            write!(f, "\n  guidance: {}", guidance)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// The Anwe parser.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Create a parser from a token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    /// Parse the entire program.
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let start_span = self.current_span();
        let mut declarations = Vec::new();

        while !self.is_at_end() {
            declarations.push(self.parse_declaration()?);
        }

        Ok(Program {
            declarations,
            span: start_span,
        })
    }

    /// Parse a top-level declaration.
    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        match self.current_kind() {
            TokenKind::KwAgent => Ok(Declaration::Agent(self.parse_agent_decl()?)),
            TokenKind::KwLink => Ok(Declaration::Link(self.parse_link_decl()?)),
            TokenKind::KwPattern => Ok(Declaration::Pattern(self.parse_pattern_decl()?)),
            TokenKind::KwHistory => Ok(Declaration::HistoryView(self.parse_history_view()?)),
            TokenKind::KwSupervise => Ok(Declaration::Supervise(self.parse_supervise_decl()?)),
            TokenKind::KwImport => Ok(Declaration::Import(self.parse_import_decl()?)),
            TokenKind::KwMind => Ok(Declaration::Mind(self.parse_mind_decl()?)),
            TokenKind::KwLet => Ok(Declaration::Let(self.parse_let_binding()?)),
            TokenKind::KwFn => Ok(Declaration::Fn(self.parse_fn_decl()?)),
            TokenKind::KwRecord => Ok(Declaration::Record(self.parse_record_decl()?)),
            TokenKind::KwWhile => Ok(Declaration::TopLevelExpr(self.parse_while_expr()?)),
            TokenKind::KwFor => Ok(Declaration::TopLevelExpr(self.parse_for_in_expr()?)),
            // Top-level assignment: name = expr
            TokenKind::Ident(_) if self.peek_is(TokenKind::Assign) => {
                let name = self.expect_ident()?;
                self.expect(TokenKind::Assign)?;
                let value = self.parse_expr()?;
                Ok(Declaration::Assign { name, value })
            }
            _ => Err(self.error(
                format!("Expected 'agent', 'link', 'mind', 'let', 'fn', 'record', 'pattern', 'supervise', 'import', 'while', 'for', or identifier assignment, found {:?}", self.current_kind()),
                Some("An Anwe program is made of agent declarations, link declarations, mind declarations, let/fn/record bindings, patterns, imports, and supervision trees.".into()),
            )),
        }
    }

    // ─── AGENT ───────────────────────────────

    fn parse_agent_decl(&mut self) -> Result<AgentDecl, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwAgent)?;
        let name = self.expect_ident()?;

        // Optional: attention budget
        let attention = if self.check(TokenKind::KwAttention) {
            self.advance();
            Some(self.expect_number()?)
        } else {
            None
        };

        // Optional: external/bridge source — bridge to outside participant
        // Syntax: external("kind", "address") or bridge("kind", "address")
        let external = if self.check(TokenKind::KwExternal) || self.check(TokenKind::KwBridge) {
            let ext_span = self.current_span();
            self.advance();
            self.expect(TokenKind::LParen)?;
            let kind = self.expect_string()?;
            self.expect(TokenKind::Comma)?;
            let address = self.expect_string()?;
            self.expect(TokenKind::RParen)?;
            Some(ExternalSource { kind, address, span: ext_span })
        } else {
            None
        };

        let mut data = Vec::new();
        if self.check(TokenKind::KwData) {
            self.advance();
            self.expect(TokenKind::LBrace)?;
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                data.push(self.parse_key_value()?);
            }
            self.expect(TokenKind::RBrace)?;
        }

        Ok(AgentDecl { name, attention, data, external, span })
    }

    // ─── LINK ───────────────────────────────────

    fn parse_link_decl(&mut self) -> Result<LinkDecl, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwLink)?;
        let agent_a = self.parse_dotted_name()?;
        self.expect(TokenKind::BiDir)?;
        let agent_b = self.parse_dotted_name()?;

        // Optional: priority level
        let priority = if self.check(TokenKind::KwPriority) {
            self.advance();
            Some(self.parse_link_priority()?)
        } else {
            self.try_parse_link_priority()
        };

        // Optional: schedule modifier (every N ticks, after N ticks, continuous)
        let schedule = if self.check(TokenKind::KwEvery) {
            self.advance();
            let ticks = self.expect_number()?;
            self.expect(TokenKind::KwTicks)?;
            Some(LinkSchedule::Every { ticks })
        } else if self.check(TokenKind::KwAfter) {
            self.advance();
            let ticks = self.expect_number()?;
            self.expect(TokenKind::KwTicks)?;
            Some(LinkSchedule::After { ticks })
        } else if self.check(TokenKind::KwContinuous) {
            self.advance();
            Some(LinkSchedule::Continuous)
        } else {
            None
        };

        // Optional: on_failure_of <agent>
        let on_failure_of = if self.check(TokenKind::KwOnFailureOf) {
            self.advance();
            Some(self.parse_dotted_name()?)
        } else {
            None
        };

        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(LinkDecl { agent_a, agent_b, priority, schedule, on_failure_of, body, span })
    }

    /// Parse a dotted name like "Safety.InputFilter" or plain "Agent".
    fn parse_dotted_name(&mut self) -> Result<String, ParseError> {
        let mut name = self.expect_ident()?;
        while self.check(TokenKind::Dot) {
            self.advance();
            let part = self.expect_ident()?;
            name = format!("{}.{}", name, part);
        }
        Ok(name)
    }

    fn parse_link_priority(&mut self) -> Result<LinkPriority, ParseError> {
        match self.current_kind() {
            TokenKind::PriCritical => { self.advance(); Ok(LinkPriority::Critical) }
            TokenKind::PriHigh => { self.advance(); Ok(LinkPriority::High) }
            TokenKind::PriNormal => { self.advance(); Ok(LinkPriority::Normal) }
            TokenKind::PriLow => { self.advance(); Ok(LinkPriority::Low) }
            TokenKind::PriBackground => { self.advance(); Ok(LinkPriority::Background) }
            _ => Err(self.error(
                "Expected priority level: critical, high, normal, low, background".into(),
                None,
            )),
        }
    }

    fn try_parse_link_priority(&mut self) -> Option<LinkPriority> {
        match self.current_kind() {
            TokenKind::PriCritical => { self.advance(); Some(LinkPriority::Critical) }
            TokenKind::PriHigh => { self.advance(); Some(LinkPriority::High) }
            TokenKind::PriNormal => { self.advance(); Some(LinkPriority::Normal) }
            TokenKind::PriLow => { self.advance(); Some(LinkPriority::Low) }
            TokenKind::PriBackground => { self.advance(); Some(LinkPriority::Background) }
            _ => None,
        }
    }

    /// Parse a link body expression.
    fn parse_link_expr(&mut self) -> Result<LinkExpr, ParseError> {
        match self.current_kind() {
            TokenKind::Alert => Ok(LinkExpr::Alert(self.parse_alert()?)),
            TokenKind::KwConnect => Ok(LinkExpr::Connect(self.parse_connect()?)),
            TokenKind::Apply => Ok(LinkExpr::Apply(self.parse_apply()?)),
            TokenKind::Commit => Ok(LinkExpr::Commit(self.parse_commit()?)),
            TokenKind::KwConverge => {
                // Disambiguate: converge [agents] vs converge A <<>> B
                if self.peek_is(TokenKind::LBracket) {
                    Ok(LinkExpr::MultiConverge(self.parse_multi_converge()?))
                } else {
                    Ok(LinkExpr::Converge(self.parse_converge()?))
                }
            }
            TokenKind::KwEmit => Ok(LinkExpr::Emit(self.parse_emit()?)),
            TokenKind::KwWhen => Ok(LinkExpr::When(self.parse_when()?)),
            TokenKind::KwPending => Ok(LinkExpr::PendingHandler(self.parse_pending_handler()?)),
            TokenKind::PatternFlow => Ok(LinkExpr::PatternUse(self.parse_pattern_use()?)),
            // Let binding: let name = expr
            TokenKind::KwLet => Ok(LinkExpr::Let(self.parse_let_binding()?)),
            // Sync starts with an identifier (agent name) followed by ~ (possibly after dots)
            TokenKind::Ident(_) if self.is_sync_ahead() => {
                Ok(LinkExpr::Sync(self.parse_sync()?))
            }
            // Assignment: name = expr (mutable rebinding)
            TokenKind::Ident(_) if self.peek_is(TokenKind::Assign) => {
                Ok(LinkExpr::Assign(self.parse_assign()?))
            }
            // Reject: <= when ...
            TokenKind::LessEq => Ok(LinkExpr::Reject(self.parse_reject()?)),
            // Each: each <var> in <expr> { ... }
            TokenKind::KwEach => Ok(LinkExpr::Each(self.parse_each()?)),
            // If/else: if <condition> { ... } [else { ... }]
            TokenKind::KwIf => Ok(LinkExpr::IfElse(self.parse_if_else()?)),
            // Phase 6-9 extensions
            TokenKind::KwSpawn => Ok(LinkExpr::Spawn(self.parse_spawn()?)),
            TokenKind::KwRetire => Ok(LinkExpr::Retire(self.parse_retire()?)),
            TokenKind::KwSyncAll => Ok(LinkExpr::SyncAll(self.parse_sync_all()?)),
            TokenKind::KwBroadcast => Ok(LinkExpr::Broadcast(self.parse_broadcast()?)),
            TokenKind::KwStream => Ok(LinkExpr::Stream(self.parse_stream()?)),
            TokenKind::KwSave => Ok(LinkExpr::Save(self.parse_save()?)),
            TokenKind::KwRestore => Ok(LinkExpr::Restore(self.parse_restore()?)),
            TokenKind::KwHistoryQuery => Ok(LinkExpr::HistoryQueryBlock(self.parse_history_query()?)),
            TokenKind::KwAlign => Ok(LinkExpr::Align(self.parse_align()?)),
            TokenKind::KwBuffer => Ok(LinkExpr::Buffer(self.parse_buffer()?)),
            // First-person cognition primitives
            TokenKind::KwThink => Ok(LinkExpr::Think(self.parse_think()?)),
            TokenKind::KwExpress => Ok(LinkExpr::Express(self.parse_express()?)),
            TokenKind::KwSense => Ok(LinkExpr::Sense(self.parse_sense()?)),
            TokenKind::KwAuthor => Ok(LinkExpr::Author(self.parse_author()?)),
            // While loop: while <condition> { ... }
            TokenKind::KwWhile => Ok(LinkExpr::While(self.parse_while()?)),
            // Error handling: attempt { ... } recover { ... }
            TokenKind::KwAttempt => Ok(LinkExpr::Attempt(self.parse_attempt()?)),
            _ => Err(self.error(
                format!("Expected a link expression (>>, connect, =>, *, <=, let, think, express, sense, author, each, if, while, attempt, etc.), found {:?}", self.current_kind()),
                Some("Inside a link or attend block, you can use: >> (alert), connect, ~ (sync), => (apply), * (commit), <= (reject), converge, when, pending?, ~> (pattern), each, if, while, attempt, think, express, sense, author, let, spawn, retire, sync_all, broadcast, stream, save, restore, history_query, align, buffer.".into()),
            )),
        }
    }

    // ─── ALERT ────────────────────────────────

    fn parse_alert(&mut self) -> Result<AlertExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Alert)?; // >>

        let attrs = if self.check(TokenKind::LBrace) {
            Some(self.parse_signal_attrs()?)
        } else {
            None
        };

        let expression = self.parse_expr()?;

        Ok(AlertExpr { attrs, expression, span })
    }

    // ─── CONNECT ─────────────────────────────────

    fn parse_connect(&mut self) -> Result<ConnectBlock, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwConnect)?;

        let depth = if self.check(TokenKind::KwDepth) {
            self.advance();
            Some(self.parse_depth_level()?)
        } else if self.is_depth_level() {
            // Allow "connect deep {" shorthand
            Some(self.parse_depth_level()?)
        } else {
            None
        };

        self.expect(TokenKind::LBrace)?;

        let mut pulses = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            pulses.push(self.parse_signal_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(ConnectBlock { depth, pulses, span })
    }

    fn parse_signal_expr(&mut self) -> Result<SignalExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwSignal)?;

        let quality = self.parse_signal_quality()?;
        let priority = self.expect_number()?;
        let direction = self.parse_signal_direction()?;

        let data = if self.check(TokenKind::KwData) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        let mut trace = Vec::new();
        if self.check(TokenKind::KwTrace) {
            self.advance();
            self.expect(TokenKind::LBrace)?;
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                trace.push(self.parse_key_value()?);
            }
            self.expect(TokenKind::RBrace)?;
        }

        Ok(SignalExpr { quality, priority, direction, data, trace, span })
    }

    // ─── SYNC ─────────────────────────────────

    fn parse_sync(&mut self) -> Result<SyncExpr, ParseError> {
        let span = self.current_span();
        let agent_a = self.parse_dotted_name()?;
        self.expect(TokenKind::Sync)?; // ~
        let agent_b = self.parse_dotted_name()?;
        self.expect(TokenKind::KwUntil)?;
        let until = self.parse_sync_condition()?;

        // Optional: decay half-life
        let decay = if self.check(TokenKind::KwDecay) {
            self.advance();
            Some(self.expect_number()? as u32)
        } else {
            None
        };

        // Optional: timeout N { ... }
        let timeout = if self.check(TokenKind::KwTimeout) {
            let timeout_span = self.current_span();
            self.advance();
            let timeout_ticks = self.expect_number()?;
            let mut options = Vec::new();
            if self.check(TokenKind::LBrace) {
                self.advance();
                while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                    options.push(self.parse_key_value()?);
                }
                self.expect(TokenKind::RBrace)?;
            }
            Some(SyncTimeout { timeout_ticks, options, span: timeout_span })
        } else {
            None
        };

        Ok(SyncExpr { agent_a, agent_b, until, decay, timeout, span })
    }

    fn parse_sync_condition(&mut self) -> Result<SyncCondition, ParseError> {
        match self.current_kind() {
            TokenKind::SyncSynchronized => {
                self.advance();
                Ok(SyncCondition::Synchronized)
            }
            TokenKind::SyncResonating => {
                self.advance();
                Ok(SyncCondition::Resonating)
            }
            TokenKind::KwSyncLevel => {
                self.advance();
                let op = self.parse_comparison_op()?;
                let value = self.expect_number()?;
                Ok(SyncCondition::CoherenceThreshold { op, value })
            }
            _ => Err(self.error(
                "Expected sync condition: 'synchronized', 'resonating', or 'sync_level > N'".into(),
                None,
            )),
        }
    }

    // ─── APPLY ───────────────────────────────

    fn parse_apply(&mut self) -> Result<ApplyExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Apply)?; // =>
        self.expect(TokenKind::KwWhen)?;
        let condition = self.parse_condition()?;

        let depth = if self.check(TokenKind::KwDepth) {
            self.advance();
            Some(self.parse_depth_level()?)
        } else if self.is_depth_level() {
            Some(self.parse_depth_level()?)
        } else {
            None
        };

        let mut changes = Vec::new();
        if self.check(TokenKind::LBrace) {
            self.advance();
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                changes.push(self.parse_structural_change()?);
            }
            self.expect(TokenKind::RBrace)?;
        }

        Ok(ApplyExpr { condition, depth, changes, span })
    }

    fn parse_structural_change(&mut self) -> Result<StructuralChange, ParseError> {
        let span = self.current_span();
        let name = self.expect_ident()?;
        self.expect(TokenKind::StructChange)?; // <-
        let value = self.parse_expr()?;
        Ok(StructuralChange { name, value, span })
    }

    // ─── COMMIT ──────────────────────────────

    fn parse_commit(&mut self) -> Result<CommitExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Commit)?; // *
        self.expect(TokenKind::KwFrom)?;

        let source = match self.current_kind() {
            TokenKind::KwApply => { self.advance(); CommitSource::Apply }
            TokenKind::KwReject => { self.advance(); CommitSource::Reject }
            _ => return Err(self.error(
                "Expected 'apply' or 'reject' after '* from'".into(),
                Some("Commit always follows apply or reject. Always.".into()),
            )),
        };

        let mut entries = Vec::new();
        if self.check(TokenKind::LBrace) {
            self.advance();
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                entries.push(self.parse_key_value()?);
            }
            self.expect(TokenKind::RBrace)?;
        }

        Ok(CommitExpr { source, entries, span })
    }

    // ─── REJECT ───────────────────────────────

    fn parse_reject(&mut self) -> Result<RejectExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::LessEq)?; // <=
        self.expect(TokenKind::KwWhen)?;
        let condition = self.parse_condition()?;

        let data = if self.check(TokenKind::KwData) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(RejectExpr { condition, data, span })
    }

    // ─── CONVERGE ──────────────────────────────

    fn parse_converge(&mut self) -> Result<ConvergeBlock, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwConverge)?;
        let agent_a = self.expect_ident()?;
        self.expect(TokenKind::Converge)?; // <<>>
        let agent_b = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(ConvergeBlock { agent_a, agent_b, body, span })
    }

    // ─── EMIT ────────────────────────────────

    fn parse_emit(&mut self) -> Result<EmitExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwEmit)?;
        let attrs = self.parse_signal_attrs()?;
        let expression = self.parse_expr()?;
        Ok(EmitExpr { attrs, expression, span })
    }

    // ─── WHEN ────────────────────────────────

    fn parse_when(&mut self) -> Result<WhenExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwWhen)?;
        let condition = self.parse_condition()?;
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(WhenExpr { condition, body, span })
    }

    // ─── EACH (ITERATION) ────────────────────

    fn parse_each(&mut self) -> Result<EachExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwEach)?;
        let var = self.expect_ident()?;
        self.expect(TokenKind::KwIn)?;
        let collection = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(EachExpr { var, collection, body, span })
    }

    // ─── IF/ELSE (CONDITIONAL ROUTING) ──────

    fn parse_if_else(&mut self) -> Result<IfElseExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwIf)?;
        let condition = self.parse_condition()?;
        self.expect(TokenKind::LBrace)?;

        let mut then_body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            then_body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        let mut else_body = Vec::new();
        if self.check(TokenKind::KwElse) {
            self.advance();
            self.expect(TokenKind::LBrace)?;
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                else_body.push(self.parse_link_expr()?);
            }
            self.expect(TokenKind::RBrace)?;
        }

        Ok(IfElseExpr { condition, then_body, else_body, span })
    }

    // ─── WHILE ──────────────────────────────
    //
    // while <condition> { <link_expr>* }

    fn parse_while(&mut self) -> Result<WhileExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwWhile)?;
        let condition = self.parse_condition()?;
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(WhileExpr { condition, body, span })
    }

    // ─── ATTEMPT / RECOVER ──────────────────
    //
    // attempt { <link_expr>* } recover { <link_expr>* }

    fn parse_attempt(&mut self) -> Result<AttemptExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwAttempt)?;
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        self.expect(TokenKind::KwRecover)?;
        self.expect(TokenKind::LBrace)?;

        let mut recover = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            recover.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(AttemptExpr { body, recover, span })
    }

    // ─── IMPORT ──────────────────────────────

    fn parse_import_decl(&mut self) -> Result<ImportDecl, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwImport)?;
        let module_path = self.expect_string()?;
        self.expect(TokenKind::KwAs)?;
        let alias = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut entries = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            entries.push(self.parse_key_value()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ImportDecl { module_path, alias, entries, span })
    }

    // ═══════════════════════════════════════════
    // FIRST-PERSON COGNITION
    // The language AI builds in.
    // ═══════════════════════════════════════════

    // ─── MIND ───────────────────────────────
    //
    // mind <name> [data { ... }] {
    //   attend "label" [priority <N>] { ... }
    //   attend "label" [priority <N>] { ... }
    // }

    fn parse_mind_decl(&mut self) -> Result<MindDecl, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwMind)?;
        let name = self.expect_ident()?;

        // Optional: attention budget
        let attention = if self.check(TokenKind::KwAttention) {
            self.advance();
            Some(self.expect_number()?)
        } else {
            None
        };

        // Optional: data block
        let mut data = Vec::new();
        if self.check(TokenKind::KwData) {
            self.advance();
            self.expect(TokenKind::LBrace)?;
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                data.push(self.parse_key_value()?);
            }
            self.expect(TokenKind::RBrace)?;
        }

        // Mind body: attend blocks
        self.expect(TokenKind::LBrace)?;
        let mut attend_blocks = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            attend_blocks.push(self.parse_attend()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(MindDecl { name, attention, data, attend_blocks, span })
    }

    // ─── ATTEND ─────────────────────────────
    //
    // attend "description" [priority <N>] {
    //   <link_expr>*
    // }

    fn parse_attend(&mut self) -> Result<AttendBlock, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwAttend)?;

        let label = self.expect_string()?;

        // Optional: priority
        let priority = if self.check(TokenKind::KwPriority) {
            self.advance();
            self.expect_number()?
        } else {
            0.5 // Default: normal priority
        };

        // Body: link expressions
        self.expect(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(AttendBlock { label, priority, body, span })
    }

    // ─── THINK ──────────────────────────────
    //
    // think {
    //   <name> <- <expr>
    //   <name> <- <expr>
    // }

    fn parse_think(&mut self) -> Result<ThinkExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwThink)?;
        self.expect(TokenKind::LBrace)?;

        let mut bindings = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            bindings.push(self.parse_structural_change()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(ThinkExpr { bindings, span })
    }

    // ─── EXPRESS ────────────────────────────
    //
    // express [{ quality: <q>, priority: <p> }] <expr>

    fn parse_express(&mut self) -> Result<ExpressExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwExpress)?;

        let attrs = if self.check(TokenKind::LBrace) {
            // Check if this is signal attrs { quality: ... } or just an expr
            // Signal attrs start with quality:, priority:, direction:
            if self.peek_is_signal_attr() {
                Some(self.parse_signal_attrs()?)
            } else {
                None
            }
        } else {
            None
        };

        let expression = self.parse_expr()?;

        Ok(ExpressExpr { attrs, expression, span })
    }

    /// Check if the current { starts signal attributes (quality:, priority:, direction:)
    /// versus a regular expression.
    fn peek_is_signal_attr(&self) -> bool {
        // Look at the token after { to see if it's a signal attr keyword
        if self.pos + 1 < self.tokens.len() {
            matches!(
                self.tokens[self.pos + 1].kind,
                TokenKind::KwQuality | TokenKind::KwPriority | TokenKind::KwDirection
            )
        } else {
            false
        }
    }

    // ─── SENSE ──────────────────────────────

    /// Parse a sense block: sense { name <- expr, ... }
    ///
    /// Sense perceives the signal landscape and binds
    /// information about available signals.
    fn parse_sense(&mut self) -> Result<SenseExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwSense)?;
        self.expect(TokenKind::LBrace)?;

        let mut bindings = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            bindings.push(self.parse_structural_change()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(SenseExpr { bindings, span })
    }

    // ─── AUTHOR ─────────────────────────────

    /// Parse an author statement: author attend "label" priority <N> { ... }
    ///
    /// Self-authoring: the mind generates new attend blocks at runtime.
    fn parse_author(&mut self) -> Result<AuthorExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwAuthor)?;

        // Must be followed by 'attend'
        let block = self.parse_attend()?;

        Ok(AuthorExpr { block, span })
    }

    // ─── LET BINDING ────────────────────────

    /// Parse a let binding: let name = expr, let mut name = expr
    fn parse_let_binding(&mut self) -> Result<LetBinding, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwLet)?;

        // Check for 'mut' modifier
        let mutable = if self.check(TokenKind::KwMut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_ident()?;
        self.expect(TokenKind::Assign)?;
        let value = self.parse_expr()?;

        Ok(LetBinding { name, mutable, value, span })
    }

    // ─── ASSIGNMENT ────────────────────────

    /// Parse an assignment: name = expr
    fn parse_assign(&mut self) -> Result<AssignExpr, ParseError> {
        let span = self.current_span();
        let name = self.expect_ident()?;
        self.expect(TokenKind::Assign)?;
        let value = self.parse_expr()?;

        Ok(AssignExpr { name, value, span })
    }

    // ─── FN DECLARATION ─────────────────────

    /// Parse a function declaration: fn name(param1, param2) { body_expr }
    fn parse_fn_decl(&mut self) -> Result<FnDecl, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwFn)?;
        let name = self.expect_ident()?;

        // Parse parameter list
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            params.push(self.expect_ident()?);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RParen)?;

        // Parse body: either { block } or = expr
        let body = if self.check(TokenKind::Assign) {
            self.advance(); // consume =
            self.parse_expr()?
        } else {
            // { ... } is now parsed as a block expression (supports multiple statements)
            self.parse_block_expr()?
        };

        Ok(FnDecl { name, params, body, span })
    }

    // ─── RECORD ─────────────────────────────

    fn parse_record_decl(&mut self) -> Result<RecordDecl, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwRecord)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            fields.push(self.expect_ident()?);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(RecordDecl { name, fields, span })
    }

    // ─── MATCH PATTERN ──────────────────────

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, ParseError> {
        match self.current_kind() {
            // Wildcard: _
            TokenKind::Underscore => {
                self.advance();
                Ok(MatchPattern::Wildcard)
            }
            // String literal
            TokenKind::StringLit(_) => {
                let s = self.expect_string()?;
                Ok(MatchPattern::Literal(Expr::StringLit(s)))
            }
            // Number literal
            TokenKind::Number(_) => {
                let n = self.expect_number()?;
                Ok(MatchPattern::Literal(Expr::Number(n)))
            }
            // true/false
            TokenKind::KwTrue => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Bool(true)))
            }
            TokenKind::KwFalse => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Bool(false)))
            }
            // Identifier used as a binding variable
            TokenKind::Ident(_) => {
                let name = self.expect_ident()?;
                Ok(MatchPattern::Binding(name))
            }
            _ => {
                Err(self.error("Expected match pattern (literal, identifier, or _)".to_string(), None))
            }
        }
    }

    // ─── SPAWN ──────────────────────────────

    fn parse_spawn(&mut self) -> Result<SpawnExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwSpawn)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::KwFrom)?;
        let template = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut data = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            data.push(self.parse_key_value()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(SpawnExpr { name, template, data, span })
    }

    // ─── RETIRE ─────────────────────────────

    fn parse_retire(&mut self) -> Result<RetireExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwRetire)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut data = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            data.push(self.parse_key_value()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(RetireExpr { name, data, span })
    }

    // ─── SYNC_ALL ───────────────────────────

    fn parse_sync_all(&mut self) -> Result<SyncAllExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwSyncAll)?;
        let agents = self.parse_agent_list()?;
        self.expect(TokenKind::KwUntil)?;
        let until = self.parse_sync_condition()?;
        let mut options = Vec::new();
        if self.check(TokenKind::LBrace) {
            self.advance();
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                options.push(self.parse_key_value()?);
            }
            self.expect(TokenKind::RBrace)?;
        }
        Ok(SyncAllExpr { agents, until, options, span })
    }

    // ─── BROADCAST ──────────────────────────

    fn parse_broadcast(&mut self) -> Result<BroadcastExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwBroadcast)?;
        let agents = self.parse_agent_list()?;
        self.expect(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_signal_expr()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(BroadcastExpr { agents, body, span })
    }

    // ─── MULTI-AGENT CONVERGE ───────────────

    fn parse_multi_converge(&mut self) -> Result<MultiConvergeExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwConverge)?;
        let agents = self.parse_agent_list()?;
        self.expect(TokenKind::LBrace)?;
        let mut options = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            options.push(self.parse_key_value()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(MultiConvergeExpr { agents, options, span })
    }

    // ─── STREAM ─────────────────────────────

    fn parse_stream(&mut self) -> Result<StreamExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwStream)?;
        let source = self.expect_ident()?;
        self.expect(TokenKind::KwRate)?;
        let rate = self.expect_number()?;
        self.expect(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(StreamExpr { source, rate, body, span })
    }

    // ─── SAVE ───────────────────────────────

    fn parse_save(&mut self) -> Result<SaveExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwSave)?;
        let agent = self.expect_ident()?;
        self.expect(TokenKind::KwTo)?;
        let path = self.expect_string()?;
        self.expect(TokenKind::LBrace)?;
        let mut options = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            options.push(self.parse_key_value()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(SaveExpr { agent, path, options, span })
    }

    // ─── RESTORE ────────────────────────────

    fn parse_restore(&mut self) -> Result<RestoreExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwRestore)?;
        let agent = self.expect_ident()?;
        self.expect(TokenKind::KwFrom)?;
        let path = self.expect_string()?;
        self.expect(TokenKind::LBrace)?;
        let mut options = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            options.push(self.parse_key_value()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(RestoreExpr { agent, path, options, span })
    }

    // ─── HISTORY QUERY ──────────────────────

    fn parse_history_query(&mut self) -> Result<HistoryQueryExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwHistoryQuery)?;
        let agent = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut options = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            options.push(self.parse_key_value()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(HistoryQueryExpr { agent, options, span })
    }

    // ─── ALIGN ──────────────────────────────

    fn parse_align(&mut self) -> Result<AlignExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwAlign)?;
        let agents = self.parse_agent_list()?;
        self.expect(TokenKind::KwTo)?;
        let reference = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        let mut options = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            options.push(self.parse_key_value()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(AlignExpr { agents, reference, options, span })
    }

    // ─── BUFFER ─────────────────────────────

    fn parse_buffer(&mut self) -> Result<BufferExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwBuffer)?;
        self.expect(TokenKind::KwSamples)?;
        let samples = self.expect_number()?;
        self.expect(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(BufferExpr { samples, body, span })
    }

    // ─── AGENT LIST ─────────────────────────

    /// Parse a bracketed agent list: [Agent1, Agent2, ...]
    fn parse_agent_list(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(TokenKind::LBracket)?;
        let mut agents = Vec::new();
        while !self.check(TokenKind::RBracket) && !self.is_at_end() {
            agents.push(self.expect_ident()?);
            if !self.check(TokenKind::RBracket) {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::RBracket)?;
        Ok(agents)
    }

    // ─── PENDING HANDLER ─────────────────────

    fn parse_pending_handler(&mut self) -> Result<PendingHandlerExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwPending)?;
        self.expect(TokenKind::Question)?; // ?
        let reason = self.parse_pending_reason()?;
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_pending_action()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(PendingHandlerExpr { reason, body, span })
    }

    fn parse_pending_reason(&mut self) -> Result<PendingReason, ParseError> {
        match self.current_kind() {
            TokenKind::NyReceiverNotReady => { self.advance(); Ok(PendingReason::ReceiverNotReady) }
            TokenKind::NyLinkNotEstablished => { self.advance(); Ok(PendingReason::LinkNotEstablished) }
            TokenKind::NySyncInsufficient => { self.advance(); Ok(PendingReason::SyncInsufficient) }
            TokenKind::NySenderNotReady => { self.advance(); Ok(PendingReason::SenderNotReady) }
            TokenKind::NyMomentNotRight => { self.advance(); Ok(PendingReason::MomentNotRight) }
            TokenKind::NyBudgetExhausted => { self.advance(); Ok(PendingReason::BudgetExhausted) }
            _ => Err(self.error(
                "Expected pending reason".into(),
                Some("Valid reasons: receiver_not_ready, link_not_established, sync_insufficient, sender_not_ready, moment_not_right, budget_exhausted".into()),
            )),
        }
    }

    fn parse_pending_action(&mut self) -> Result<PendingAction, ParseError> {
        match self.current_kind() {
            TokenKind::KwWait => {
                self.advance();
                let ticks = self.expect_number()?;
                // Accept both "tick" and "ticks"
                if self.check(TokenKind::KwTick) || self.check(TokenKind::KwTicks) {
                    self.advance();
                }
                Ok(PendingAction::Wait { ticks })
            }
            TokenKind::KwGuidance => {
                self.advance();
                let msg = self.expect_string()?;
                Ok(PendingAction::Guidance(msg))
            }
            TokenKind::KwThen => {
                self.advance();
                let expr = self.parse_link_expr()?;
                Ok(PendingAction::Then(expr))
            }
            _ => Err(self.error(
                "Expected 'wait', 'guidance', or 'then' inside pending handler".into(),
                None,
            )),
        }
    }

    // ─── PATTERN ─────────────────────────────

    fn parse_pattern_decl(&mut self) -> Result<PatternDecl, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwPattern)?;
        let name = self.expect_ident()?;

        let mut params = Vec::new();
        if self.check(TokenKind::LParen) {
            self.advance();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                let param_name = self.expect_ident()?;
                let type_ref = if self.check(TokenKind::Colon) {
                    self.advance();
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                params.push(Param { name: param_name, type_ref });
                if !self.check(TokenKind::RParen) {
                    self.expect(TokenKind::Comma)?;
                }
            }
            self.expect(TokenKind::RParen)?;
        }

        self.expect(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_link_expr()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(PatternDecl { name, params, body, span })
    }

    fn parse_pattern_use(&mut self) -> Result<PatternUseExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::PatternFlow)?; // ~>
        let name = self.expect_ident()?;

        let mut args = Vec::new();
        if self.check(TokenKind::LParen) {
            self.advance();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                args.push(self.parse_expr()?);
                if !self.check(TokenKind::RParen) {
                    self.expect(TokenKind::Comma)?;
                }
            }
            self.expect(TokenKind::RParen)?;
        }

        Ok(PatternUseExpr { name, args, span })
    }

    // ─── HISTORY VIEW ───────────────────────

    fn parse_history_view(&mut self) -> Result<HistoryViewExpr, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwHistory)?;
        self.expect(TokenKind::KwOf)?;
        let agent = self.expect_ident()?;

        let since = if self.check(TokenKind::KwSince) {
            self.advance();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        let depth = if self.check(TokenKind::KwDepth) {
            self.advance();
            Some(self.parse_depth_level()?)
        } else {
            None
        };

        Ok(HistoryViewExpr { agent, since, depth, span })
    }

    // ─── SUPERVISE ─────────────────────────

    fn parse_supervise_decl(&mut self) -> Result<SuperviseDecl, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::KwSupervise)?;

        let strategy = match self.current_kind() {
            TokenKind::KwOneForOne => { self.advance(); SuperviseStrategy::OneForOne }
            TokenKind::KwOneForAll => { self.advance(); SuperviseStrategy::OneForAll }
            TokenKind::KwRestForOne => { self.advance(); SuperviseStrategy::RestForOne }
            _ => return Err(self.error(
                "Expected restart strategy: one_for_one, one_for_all, or rest_for_one".into(),
                None,
            )),
        };

        // Optional: max_restarts N within T
        let (max_restarts, time_window) = if self.check(TokenKind::KwMaxRestarts) {
            self.advance();
            let max = self.expect_number()? as u32;
            let window = if self.check(TokenKind::KwWithin) {
                self.advance();
                Some(self.expect_number()? as u32)
            } else {
                None
            };
            (Some(max), window)
        } else {
            (None, None)
        };

        self.expect(TokenKind::LBrace)?;

        let mut children = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            children.push(self.parse_supervised_child()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(SuperviseDecl {
            strategy,
            max_restarts,
            time_window,
            children,
            span,
        })
    }

    fn parse_supervised_child(&mut self) -> Result<SupervisedChild, ParseError> {
        let span = self.current_span();
        let restart = match self.current_kind() {
            TokenKind::KwPermanent => { self.advance(); ChildRestartType::Permanent }
            TokenKind::KwTransient => { self.advance(); ChildRestartType::Transient }
            TokenKind::KwTemporary => { self.advance(); ChildRestartType::Temporary }
            // Allow bare agent names — default to Permanent restart type
            _ => ChildRestartType::Permanent,
        };
        let agent = self.expect_ident()?;
        Ok(SupervisedChild { restart, agent, span })
    }

    // ─── SHARED PARSERS ─────────────────────

    fn parse_condition(&mut self) -> Result<Condition, ParseError> {
        let left = self.parse_condition_atom()?;

        if self.check(TokenKind::KwAnd) {
            self.advance();
            let right = self.parse_condition()?;
            Ok(Condition::And(Box::new(left), Box::new(right)))
        } else if self.check(TokenKind::KwOr) {
            self.advance();
            let right = self.parse_condition()?;
            Ok(Condition::Or(Box::new(left), Box::new(right)))
        } else {
            Ok(left)
        }
    }

    fn parse_condition_atom(&mut self) -> Result<Condition, ParseError> {
        match self.current_kind() {
            TokenKind::KwSyncLevel => {
                self.advance();
                let op = self.parse_comparison_op()?;
                let value = self.expect_number()?;
                Ok(Condition::SyncLevel { op, value })
            }
            TokenKind::KwPriority => {
                self.advance();
                let op = self.parse_comparison_op()?;
                let value = self.expect_number()?;
                Ok(Condition::Priority { op, value })
            }
            TokenKind::KwConfidence => {
                self.advance();
                let op = self.parse_comparison_op()?;
                let value = self.expect_number()?;
                Ok(Condition::Confidence { op, value })
            }
            TokenKind::KwAttention => {
                self.advance();
                let op = self.parse_comparison_op()?;
                let value = self.expect_number()?;
                Ok(Condition::Attention { op, value })
            }
            TokenKind::KwAlert => {
                self.advance();
                self.expect(TokenKind::KwIs)?;
                let quality = self.expect_ident()?;
                Ok(Condition::AlertIs(quality))
            }
            TokenKind::LParen => {
                self.advance();
                let cond = self.parse_condition()?;
                self.expect(TokenKind::RParen)?;
                Ok(cond)
            }
            // General field comparison: Agent.field op value, or reading op value
            TokenKind::Ident(_) | TokenKind::KwReading => {
                let left = self.parse_condition_expr()?;
                let op = self.parse_comparison_op()?;
                let right = self.parse_condition_expr()?;
                Ok(Condition::FieldCompare { left, op, right })
            }
            _ => Err(self.error(
                "Expected condition: 'sync_level', 'priority', 'confidence', 'attention', 'alert is', or field comparison".into(),
                None,
            )),
        }
    }

    /// Parse a simple expression for use in conditions (field access, literals, identifiers).
    fn parse_condition_expr(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind() {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::StringLit(s))
            }
            TokenKind::KwTrue => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::KwFalse => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::KwReading => {
                self.advance();
                Ok(Expr::Ident("reading".into()))
            }
            TokenKind::Ident(_) => {
                let name = self.expect_ident()?;
                if self.check(TokenKind::LParen) {
                    // Function call in condition expression
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(TokenKind::RParen) && !self.is_at_end() {
                        args.push(self.parse_expr()?);
                        if !self.check(TokenKind::RParen) {
                            self.expect(TokenKind::Comma)?;
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Call { name, args })
                } else if self.check(TokenKind::Dot) {
                    self.advance();
                    let field = self.expect_ident()?;
                    Ok(Expr::FieldAccess { object: name, field })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            _ => Err(self.error(
                format!("Expected expression in condition, found {:?}", self.current_kind()),
                None,
            )),
        }
    }

    fn parse_comparison_op(&mut self) -> Result<ComparisonOp, ParseError> {
        match self.current_kind() {
            TokenKind::Greater => { self.advance(); Ok(ComparisonOp::Greater) }
            TokenKind::GreaterEq => { self.advance(); Ok(ComparisonOp::GreaterEq) }
            TokenKind::Less => { self.advance(); Ok(ComparisonOp::Less) }
            TokenKind::LessEq => { self.advance(); Ok(ComparisonOp::LessEq) }
            TokenKind::EqualEqual => { self.advance(); Ok(ComparisonOp::Equal) }
            _ => Err(self.error(
                "Expected comparison operator: >, >=, <, <=, ==".into(),
                None,
            )),
        }
    }

    fn parse_signal_attrs(&mut self) -> Result<SignalAttrs, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut attrs = SignalAttrs {
            quality: None,
            priority: None,
            direction: None,
            duration: None,
            confidence: None,
            half_life: None,
        };

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            match self.current_kind() {
                TokenKind::KwQuality => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    attrs.quality = Some(self.parse_signal_quality()?);
                }
                TokenKind::KwPriority => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    attrs.priority = Some(self.expect_number()?);
                }
                TokenKind::KwDirection => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    attrs.direction = Some(self.parse_signal_direction()?);
                }
                TokenKind::KwDuration => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    attrs.duration = Some(self.expect_number()?);
                }
                TokenKind::KwConfidence => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    attrs.confidence = Some(self.expect_number()?);
                }
                TokenKind::KwHalfLife => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    attrs.half_life = Some(self.expect_number()?);
                }
                _ => break,
            }
            // Optional comma between attrs
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(attrs)
    }

    fn parse_signal_quality(&mut self) -> Result<SignalQuality, ParseError> {
        match self.current_kind() {
            TokenKind::QualAttending => { self.advance(); Ok(SignalQuality::Attending) }
            TokenKind::QualQuestioning => { self.advance(); Ok(SignalQuality::Questioning) }
            TokenKind::QualRecognizing => { self.advance(); Ok(SignalQuality::Recognizing) }
            TokenKind::QualDisturbed => { self.advance(); Ok(SignalQuality::Disturbed) }
            TokenKind::QualApplying => { self.advance(); Ok(SignalQuality::Applying) }
            TokenKind::QualCompleting => { self.advance(); Ok(SignalQuality::Completing) }
            TokenKind::QualResting => { self.advance(); Ok(SignalQuality::Resting) }
            // Also accept depth keywords that map to qualities in context
            TokenKind::DepthDeep => { self.advance(); Ok(SignalQuality::Attending) }
            _ => Err(self.error(
                "Expected signal quality: attending, questioning, recognizing, disturbed, applying, completing, resting".into(),
                None,
            )),
        }
    }

    fn parse_signal_direction(&mut self) -> Result<SignalDirection, ParseError> {
        match self.current_kind() {
            TokenKind::DirInward => { self.advance(); Ok(SignalDirection::Inward) }
            TokenKind::DirOutward => { self.advance(); Ok(SignalDirection::Outward) }
            TokenKind::DirBetween => { self.advance(); Ok(SignalDirection::Between) }
            TokenKind::DirDiffuse => { self.advance(); Ok(SignalDirection::Diffuse) }
            _ => Err(self.error(
                "Expected signal direction: inward, outward, between, diffuse".into(),
                None,
            )),
        }
    }

    fn parse_depth_level(&mut self) -> Result<DepthLevel, ParseError> {
        match self.current_kind() {
            TokenKind::DepthSurface => { self.advance(); Ok(DepthLevel::Surface) }
            TokenKind::DepthPartial => { self.advance(); Ok(DepthLevel::Partial) }
            TokenKind::DepthFull => { self.advance(); Ok(DepthLevel::Full) }
            TokenKind::DepthGenuine => { self.advance(); Ok(DepthLevel::Genuine) }
            TokenKind::DepthDeep => { self.advance(); Ok(DepthLevel::Deep) }
            _ => Err(self.error(
                "Expected depth level: surface, partial, full, genuine, deep".into(),
                None,
            )),
        }
    }

    fn is_depth_level(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::DepthSurface | TokenKind::DepthPartial
            | TokenKind::DepthFull | TokenKind::DepthGenuine | TokenKind::DepthDeep
        )
    }

    // ─── EXPRESSION PARSING WITH PRECEDENCE ────
    //
    // Precedence (lowest to highest):
    //   additive:       + -
    //   multiplicative: * / %
    //   unary:          -expr
    //   postfix:        expr[index]
    //   primary:        literals, identifiers, field access, lists, parens

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let first = self.parse_logical_or()?;

        // Check for pipe operator |> (lowest precedence)
        if self.check(TokenKind::Pipe) {
            let mut stages = vec![first];
            while self.check(TokenKind::Pipe) {
                self.advance(); // consume |>
                stages.push(self.parse_logical_or()?);
            }
            Ok(Expr::Pipe { stages })
        } else {
            Ok(first)
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_logical_and()?;
        while self.check(TokenKind::KwOr) {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::LogicalOr { left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison_expr()?;
        while self.check(TokenKind::KwAnd) {
            self.advance();
            let right = self.parse_comparison_expr()?;
            left = Expr::LogicalAnd { left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_comparison_expr(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_additive()?;

        let op = match self.current_kind() {
            TokenKind::EqualEqual => Some(ComparisonOp::Equal),
            TokenKind::BangEqual => Some(ComparisonOp::NotEqual),
            TokenKind::Less => Some(ComparisonOp::Less),
            TokenKind::LessEq => Some(ComparisonOp::LessEq),
            TokenKind::Greater => Some(ComparisonOp::Greater),
            TokenKind::GreaterEq => Some(ComparisonOp::GreaterEq),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let right = self.parse_additive()?;
            Ok(Expr::Comparison { left: Box::new(left), op, right: Box::new(right) })
        } else {
            Ok(left)
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;

        // * is used for multiply in expression context.
        // In link body context, * is parsed as Commit by parse_link_expr
        // before we ever reach here, so there is no ambiguity.
        loop {
            let op = match self.current_kind() {
                TokenKind::Commit => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(TokenKind::Minus) {
            self.advance();
            let operand = self.parse_unary()?;
            Ok(Expr::UnaryNeg(Box::new(operand)))
        } else if self.check(TokenKind::KwNot) {
            self.advance();
            let operand = self.parse_unary()?;
            Ok(Expr::Not(Box::new(operand)))
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        // Index access: expr[index]
        while self.check(TokenKind::LBracket) {
            self.advance();
            let index = self.parse_expr()?;
            self.expect(TokenKind::RBracket)?;
            expr = Expr::IndexAccess {
                object: Box::new(expr),
                index: Box::new(index),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind() {
            TokenKind::StringLit(_) => {
                let s = self.expect_string()?;
                Ok(Expr::StringLit(s))
            }
            TokenKind::FStringLit(_) => {
                self.parse_fstring_expr()
            }
            TokenKind::Number(_) => {
                let n = self.expect_number()?;
                Ok(Expr::Number(n))
            }
            TokenKind::KwTrue => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::KwFalse => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::KwHistory => {
                // If followed by "of", parse as history view.
                // Otherwise, treat as identifier "history".
                if self.peek_is(TokenKind::KwOf) {
                    let view = self.parse_history_view()?;
                    Ok(Expr::HistoryOf(Box::new(view)))
                } else {
                    self.advance();
                    Ok(Expr::Ident("history".into()))
                }
            }
            // List literal: [expr, expr, ...]
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !self.check(TokenKind::RBracket) && !self.is_at_end() {
                    items.push(self.parse_expr()?);
                    if !self.check(TokenKind::RBracket) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::ListLit(items))
            }
            // Parenthesized expression: (expr)
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            // Block or Map literal: { ... }
            TokenKind::LBrace => {
                // Disambiguate: if { is followed by ident and then :, it's a map literal
                // Otherwise it's a block expression
                if self.is_map_literal() {
                    self.parse_map_literal()
                } else {
                    self.parse_block_expr()
                }
            }
            // If/else expression: if cond { then } else { else }
            TokenKind::KwIf => {
                self.parse_if_else_expr()
            }
            // While loop expression: while cond { body }
            TokenKind::KwWhile => {
                self.parse_while_expr()
            }
            // For-in loop: for item in collection { body }
            TokenKind::KwFor => {
                self.parse_for_in_expr()
            }
            // Try/catch expression: try { expr } catch { fallback }
            TokenKind::KwTry => {
                self.parse_try_catch_expr()
            }
            // Break: exit enclosing loop
            TokenKind::KwBreak => {
                self.advance();
                Ok(Expr::Break)
            }
            // Continue: skip to next iteration
            TokenKind::KwContinue => {
                self.advance();
                Ok(Expr::Continue)
            }
            // Return: early return from function
            TokenKind::KwReturn => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Expr::Return(Box::new(value)))
            }
            TokenKind::Ident(_) => {
                let name = self.expect_ident()?;
                if self.check(TokenKind::LParen) {
                    // Function call: name(arg1, arg2, ...)
                    self.advance(); // consume (
                    let mut args = Vec::new();
                    while !self.check(TokenKind::RParen) && !self.is_at_end() {
                        args.push(self.parse_expr()?);
                        if !self.check(TokenKind::RParen) {
                            self.expect(TokenKind::Comma)?;
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Call { name, args })
                } else if self.check(TokenKind::Dot) {
                    self.advance();
                    let field = self.expect_ident()?;
                    if self.check(TokenKind::LParen) {
                        // Namespaced function call: Module.func(args)
                        self.advance(); // consume (
                        let mut args = Vec::new();
                        while !self.check(TokenKind::RParen) && !self.is_at_end() {
                            args.push(self.parse_expr()?);
                            if !self.check(TokenKind::RParen) {
                                self.expect(TokenKind::Comma)?;
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                        let dotted_name = format!("{}.{}", name, field);
                        Ok(Expr::Call { name: dotted_name, args })
                    } else {
                        Ok(Expr::FieldAccess { object: name, field })
                    }
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            // Quote: quote { ... } captures code as a string value
            TokenKind::KwQuote => {
                self.advance(); // consume 'quote'
                self.expect(TokenKind::LBrace)?;
                // Capture all tokens between braces as source text
                let mut depth = 1;
                let mut source_parts = Vec::new();
                while depth > 0 && !self.is_at_end() {
                    match self.current_kind() {
                        TokenKind::LBrace => {
                            depth += 1;
                            source_parts.push("{".to_string());
                            self.advance();
                        }
                        TokenKind::RBrace => {
                            depth -= 1;
                            if depth > 0 {
                                source_parts.push("}".to_string());
                            }
                            self.advance();
                        }
                        _ => {
                            source_parts.push(format!("{}", self.tokens[self.pos]));
                            self.advance();
                        }
                    }
                }
                let source = source_parts.join(" ");
                Ok(Expr::Quote(source))
            }
            // Match expression: match expr { pattern => body, ... }
            TokenKind::KwMatch => {
                self.advance(); // consume 'match'
                let subject = self.parse_expr()?;
                self.expect(TokenKind::LBrace)?;
                let mut arms = Vec::new();
                while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                    let pattern = self.parse_match_pattern()?;
                    self.expect(TokenKind::Apply)?; // =>
                    let body = self.parse_expr()?;
                    arms.push(MatchArm { pattern, body });
                }
                self.expect(TokenKind::RBrace)?;
                Ok(Expr::Match { subject: Box::new(subject), arms })
            }
            // Lambda: |param1, param2| body_expr
            TokenKind::Bar => {
                self.advance(); // consume opening |
                let mut params = Vec::new();
                while !self.check(TokenKind::Bar) && !self.is_at_end() {
                    params.push(self.expect_ident()?);
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::Bar)?; // consume closing |
                let body = self.parse_expr()?;
                Ok(Expr::Lambda { params, body: Box::new(body) })
            }
            // Handle keywords used as values in key-value pairs
            // (e.g., quality: recognizing, include: [data, history, becoming])
            _ => {
                if let Some(name) = self.keyword_as_ident(&self.current_kind()) {
                    self.advance();
                    if self.check(TokenKind::Dot) {
                        self.advance();
                        let field = self.expect_ident()?;
                        Ok(Expr::FieldAccess { object: name, field })
                    } else {
                        Ok(Expr::Ident(name))
                    }
                } else {
                    Err(self.error(
                        format!("Expected expression, found {:?}", self.current_kind()),
                        None,
                    ))
                }
            }
        }
    }

    fn parse_key_value(&mut self) -> Result<KeyValue, ParseError> {
        let span = self.current_span();
        let key = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let value = self.parse_expr()?;
        // Handle trailing value tokens that aren't a new key-value pair
        // E.g., "decay_with: half_life 180" — consume trailing number
        let value = if matches!(self.current_kind(), TokenKind::Number(_))
            && !self.peek_is(TokenKind::Colon)
        {
            let extra = self.expect_number()?;
            Expr::ListLit(vec![value, Expr::Number(extra)])
        } else {
            value
        };
        Ok(KeyValue { key, value, span })
    }

    // ─── TOKEN HELPERS ───────────────────────

    fn current_kind(&self) -> TokenKind {
        self.tokens.get(self.pos)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn current_span(&self) -> Span {
        self.tokens.get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::new(0, 0, 1, 1))
    }

    fn check(&self, kind: TokenKind) -> bool {
        std::mem::discriminant(&self.current_kind()) == std::mem::discriminant(&kind)
    }

    fn peek_is(&self, kind: TokenKind) -> bool {
        self.tokens.get(self.pos + 1)
            .map(|t| std::mem::discriminant(&t.kind) == std::mem::discriminant(&kind))
            .unwrap_or(false)
    }

    /// Check if there is a ~ (sync) operator ahead, possibly after dotted names.
    /// Looks past Ident.Dot.Ident patterns to find a Sync token.
    fn is_sync_ahead(&self) -> bool {
        let mut offset = 1;
        loop {
            match self.tokens.get(self.pos + offset).map(|t| &t.kind) {
                Some(TokenKind::Sync) => return true,
                Some(TokenKind::Dot) => {
                    // Skip dot and the identifier after it
                    offset += 2;
                }
                _ => return false,
            }
        }
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.current_kind(), TokenKind::Eof)
    }

    fn expect(&mut self, expected: TokenKind) -> Result<(), ParseError> {
        if self.check(expected.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(
                format!("Expected {:?}, found {:?}", expected, self.current_kind()),
                None,
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.current_kind() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            // Allow keywords in identifier position (e.g., as key names)
            ref kind => {
                if let Some(name) = self.keyword_as_ident(kind) {
                    self.advance();
                    Ok(name)
                } else {
                    Err(self.error(
                        format!("Expected identifier, found {:?}", self.current_kind()),
                        None,
                    ))
                }
            }
        }
    }

    /// Parse a block expression: { stmt1; stmt2; result_expr }
    /// Statements are separated by newlines (implicit) or semicolons.
    /// The last expression in the block is the return value.
    fn parse_block_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut statements: Vec<BlockStatement> = Vec::new();
        let mut last_expr: Option<Expr> = None;

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            // If we had a pending expression from last iteration, it was a statement
            if let Some(prev) = last_expr.take() {
                statements.push(BlockStatement::Expr(prev));
            }

            // Skip optional semicolons between statements
            while self.check(TokenKind::Semicolon) {
                self.advance();
            }
            if self.check(TokenKind::RBrace) {
                break;
            }

            if self.check(TokenKind::KwLet) {
                // let [mut] name = expr
                self.advance();
                let mutable = if self.check(TokenKind::KwMut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let name = self.expect_ident()?;
                self.expect(TokenKind::Assign)?;
                let value = self.parse_expr()?;
                statements.push(BlockStatement::Let { name, mutable, value });
                // Consume optional trailing semicolon
                if self.check(TokenKind::Semicolon) {
                    self.advance();
                }
            } else if matches!(self.current_kind(), TokenKind::Ident(_)) {
                // Could be assignment (name = expr) or expression
                let name = self.expect_ident()?;
                if self.check(TokenKind::Assign) {
                    self.advance();
                    let value = self.parse_expr()?;
                    statements.push(BlockStatement::Assign { name, value });
                    // Consume optional trailing semicolon
                    if self.check(TokenKind::Semicolon) {
                        self.advance();
                    }
                } else {
                    // It's an expression starting with an identifier.
                    // Re-parse: we already consumed the ident, build the expr from it.
                    let expr = self.finish_ident_expr(name)?;
                    last_expr = Some(expr);
                    // Consume optional trailing semicolon — if present, this was a statement
                    if self.check(TokenKind::Semicolon) {
                        self.advance();
                        if let Some(prev) = last_expr.take() {
                            statements.push(BlockStatement::Expr(prev));
                        }
                    }
                }
            } else {
                // Any other expression
                last_expr = Some(self.parse_expr()?);
                // Consume optional trailing semicolon — if present, this was a statement
                if self.check(TokenKind::Semicolon) {
                    self.advance();
                    if let Some(prev) = last_expr.take() {
                        statements.push(BlockStatement::Expr(prev));
                    }
                }
            }
        }
        self.expect(TokenKind::RBrace)?;

        let result = last_expr.unwrap_or(Expr::Ident("null".into()));
        Ok(Expr::Block { statements, result: Box::new(result) })
    }

    /// After consuming an identifier, finish parsing it as an expression
    /// (could be call, field access, or just the ident).
    fn finish_ident_expr(&mut self, name: String) -> Result<Expr, ParseError> {
        let base = if self.check(TokenKind::LParen) {
            self.advance();
            let mut args = Vec::new();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                args.push(self.parse_expr()?);
                if !self.check(TokenKind::RParen) {
                    self.expect(TokenKind::Comma)?;
                }
            }
            self.expect(TokenKind::RParen)?;
            Expr::Call { name, args }
        } else if self.check(TokenKind::Dot) {
            self.advance();
            let field = self.expect_ident()?;
            if self.check(TokenKind::LParen) {
                self.advance();
                let mut args = Vec::new();
                while !self.check(TokenKind::RParen) && !self.is_at_end() {
                    args.push(self.parse_expr()?);
                    if !self.check(TokenKind::RParen) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                self.expect(TokenKind::RParen)?;
                let dotted_name = format!("{}.{}", name, field);
                Expr::Call { name: dotted_name, args }
            } else {
                Expr::FieldAccess { object: name, field }
            }
        } else {
            Expr::Ident(name)
        };

        // Check for binary operators, comparisons, etc. to complete the expression
        // We need to continue parsing the rest of the expression from here
        self.continue_expr_from(base)
    }

    /// Continue parsing an expression after we've already parsed the left-hand side.
    /// Handles binary ops, comparisons, logical ops, pipe, and indexing.
    fn continue_expr_from(&mut self, base: Expr) -> Result<Expr, ParseError> {
        // Handle index access
        let mut expr = base;
        while self.check(TokenKind::LBracket) {
            self.advance();
            let index = self.parse_expr()?;
            self.expect(TokenKind::RBracket)?;
            expr = Expr::IndexAccess { object: Box::new(expr), index: Box::new(index) };
        }

        // Handle binary operators by checking what comes next
        let has_binop = matches!(self.current_kind(),
            TokenKind::Plus | TokenKind::Minus | TokenKind::Commit |
            TokenKind::Slash | TokenKind::Percent |
            TokenKind::EqualEqual | TokenKind::BangEqual |
            TokenKind::Less | TokenKind::LessEq |
            TokenKind::Greater | TokenKind::GreaterEq |
            TokenKind::KwAnd | TokenKind::KwOr | TokenKind::Pipe
        );

        if has_binop {
            // We need to re-enter the expression parser at the right precedence.
            // The simplest approach: wrap as the left of a binary op and re-parse.
            self.parse_expr_with_left(expr)
        } else {
            Ok(expr)
        }
    }

    /// Parse the rest of an expression given that we already have the left-hand operand.
    fn parse_expr_with_left(&mut self, left: Expr) -> Result<Expr, ParseError> {
        // Continue from additive level (left is already a primary/postfix)
        let mut result = left;

        // Multiplicative
        loop {
            let op = match self.current_kind() {
                TokenKind::Commit => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            result = Expr::BinaryOp { left: Box::new(result), op, right: Box::new(right) };
        }

        // Additive
        loop {
            let op = match self.current_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            result = Expr::BinaryOp { left: Box::new(result), op, right: Box::new(right) };
        }

        // Comparison
        let cmp_op = match self.current_kind() {
            TokenKind::EqualEqual => Some(ComparisonOp::Equal),
            TokenKind::BangEqual => Some(ComparisonOp::NotEqual),
            TokenKind::Less => Some(ComparisonOp::Less),
            TokenKind::LessEq => Some(ComparisonOp::LessEq),
            TokenKind::Greater => Some(ComparisonOp::Greater),
            TokenKind::GreaterEq => Some(ComparisonOp::GreaterEq),
            _ => None,
        };
        if let Some(op) = cmp_op {
            self.advance();
            let right = self.parse_additive()?;
            result = Expr::Comparison { left: Box::new(result), op, right: Box::new(right) };
        }

        // Logical AND
        while self.check(TokenKind::KwAnd) {
            self.advance();
            let right = self.parse_comparison_expr()?;
            result = Expr::LogicalAnd { left: Box::new(result), right: Box::new(right) };
        }

        // Logical OR
        while self.check(TokenKind::KwOr) {
            self.advance();
            let right = self.parse_logical_and()?;
            result = Expr::LogicalOr { left: Box::new(result), right: Box::new(right) };
        }

        // Pipe
        if self.check(TokenKind::Pipe) {
            let mut stages = vec![result];
            while self.check(TokenKind::Pipe) {
                self.advance();
                stages.push(self.parse_logical_or()?);
            }
            result = Expr::Pipe { stages };
        }

        Ok(result)
    }

    /// Parse if/else as an expression: if cond { expr } [else if ... | else { expr }]
    fn parse_if_else_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::KwIf)?;
        let condition = self.parse_expr()?;
        let then_branch = self.parse_block_expr()?;

        let else_branch = if self.check(TokenKind::KwElse) {
            self.advance();
            if self.check(TokenKind::KwIf) {
                // else if — chain as nested if/else
                Some(Box::new(self.parse_if_else_expr()?))
            } else {
                Some(Box::new(self.parse_block_expr()?))
            }
        } else {
            None
        };

        Ok(Expr::IfElse {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }

    /// Parse a while loop expression: while condition { body }
    fn parse_while_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::KwWhile)?;
        let condition = self.parse_expr()?;
        let body = self.parse_block_expr()?;
        Ok(Expr::WhileExpr {
            condition: Box::new(condition),
            body: Box::new(body),
        })
    }

    /// Parse a for-in loop: for item in collection { body }
    fn parse_for_in_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::KwFor)?;
        let var = self.expect_ident()?;
        self.expect(TokenKind::KwIn)?;
        let collection = self.parse_expr()?;
        let body = self.parse_block_expr()?;
        Ok(Expr::ForIn {
            var,
            collection: Box::new(collection),
            body: Box::new(body),
        })
    }

    /// Parse a try/catch expression: try { expr } catch { fallback }
    fn parse_try_catch_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::KwTry)?;
        let body = self.parse_block_expr()?;
        self.expect(TokenKind::KwCatch)?;
        let catch_body = self.parse_block_expr()?;
        Ok(Expr::TryCatch {
            body: Box::new(body),
            catch_body: Box::new(catch_body),
        })
    }

    /// Check if the current position starts a map literal: { ident: expr, ... }
    fn is_map_literal(&self) -> bool {
        // Look ahead: if after { we see ident followed by :, it's a map
        if self.pos + 2 < self.tokens.len() {
            if matches!(self.tokens[self.pos].kind, TokenKind::LBrace) {
                let next = &self.tokens[self.pos + 1].kind;
                let after = &self.tokens[self.pos + 2].kind;
                if matches!(next, TokenKind::Ident(_)) && matches!(after, TokenKind::Colon) {
                    return true;
                }
                // Also handle keyword-as-key case (e.g., {status: "ok"})
                if self.keyword_as_ident(next).is_some() && matches!(after, TokenKind::Colon) {
                    return true;
                }
                // Empty map: { }
                if matches!(next, TokenKind::RBrace) {
                    return true;
                }
            }
        }
        false
    }

    /// Parse a map literal: { key: value, key2: value2 }
    fn parse_map_literal(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut entries = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            // Parse key (identifier or keyword-as-ident)
            let key = if matches!(self.current_kind(), TokenKind::Ident(_)) {
                self.expect_ident()?
            } else if let Some(name) = self.keyword_as_ident(&self.current_kind()) {
                self.advance();
                name
            } else {
                return Err(self.error(
                    format!("Expected map key, found {:?}", self.current_kind()),
                    None,
                ));
            };
            self.expect(TokenKind::Colon)?;
            let value = self.parse_expr()?;
            entries.push((key, value));

            // Optional comma between entries
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::MapLit(entries))
    }

    /// Parse an f-string expression: f"Hello {name}, {len(items)} items"
    /// Breaks the raw f-string content into literal and expression parts.
    fn parse_fstring_expr(&mut self) -> Result<Expr, ParseError> {
        let raw = match self.current_kind() {
            TokenKind::FStringLit(s) => s.clone(),
            other => return Err(self.error(format!("Expected f-string, found {:?}", other), None)),
        };
        self.advance();

        let mut parts: Vec<StringPart> = Vec::new();
        let mut literal = String::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // Save accumulated literal
                if !literal.is_empty() {
                    parts.push(StringPart::Literal(std::mem::take(&mut literal)));
                }
                // Collect the expression text inside { ... }
                i += 1; // skip '{'
                let mut expr_text = String::new();
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    expr_text.push(chars[i]);
                    i += 1;
                }
                i += 1; // skip closing '}'

                // Parse the expression text
                let expr_source = expr_text.trim();
                if expr_source.is_empty() {
                    // Empty braces — treat as literal "{}"
                    literal.push_str("{}");
                } else {
                    // Wrap in a let to parse as expression
                    let wrapped = format!("let __fstr__ = {}", expr_source);
                    use crate::lexer::Lexer;
                    let mut lex = Lexer::new(&wrapped);
                    match lex.tokenize() {
                        Ok(toks) => {
                            let mut sub_parser = Parser::new(toks);
                            match sub_parser.parse_program() {
                                Ok(prog) => {
                                    if let Some(Declaration::Let(decl)) = prog.declarations.first() {
                                        parts.push(StringPart::Expression(decl.value.clone()));
                                    } else {
                                        // Fallback: treat as literal
                                        literal.push('{');
                                        literal.push_str(expr_source);
                                        literal.push('}');
                                    }
                                }
                                Err(_) => {
                                    literal.push('{');
                                    literal.push_str(expr_source);
                                    literal.push('}');
                                }
                            }
                        }
                        Err(_) => {
                            literal.push('{');
                            literal.push_str(expr_source);
                            literal.push('}');
                        }
                    }
                }
            } else {
                literal.push(chars[i]);
                i += 1;
            }
        }

        // Save any trailing literal
        if !literal.is_empty() {
            parts.push(StringPart::Literal(literal));
        }

        // Optimize: if only one literal part, return a plain string
        if parts.len() == 1 {
            if let StringPart::Literal(s) = &parts[0] {
                return Ok(Expr::StringLit(s.clone()));
            }
        }

        Ok(Expr::InterpolatedString { parts })
    }

    /// Some keywords can be used as identifiers in certain positions
    /// (e.g., as key names in data blocks).
    fn keyword_as_ident(&self, kind: &TokenKind) -> Option<String> {
        match kind {
            TokenKind::KwAgent => Some("agent".into()),
            TokenKind::KwData => Some("data".into()),
            TokenKind::KwTrace => Some("trace".into()),
            TokenKind::KwFrom => Some("from".into()),
            TokenKind::KwDepth => Some("depth".into()),
            TokenKind::KwQuality => Some("quality".into()),
            TokenKind::KwPriority => Some("priority".into()),
            TokenKind::KwDirection => Some("direction".into()),
            TokenKind::KwDuration => Some("duration".into()),
            TokenKind::KwSyncLevel => Some("sync_level".into()),
            TokenKind::KwAlert => Some("alert".into()),
            TokenKind::KwSignal => Some("signal".into()),
            TokenKind::KwLink => Some("link".into()),
            TokenKind::KwHistory => Some("history".into()),
            TokenKind::DepthSurface => Some("surface".into()),
            TokenKind::DepthPartial => Some("partial".into()),
            TokenKind::DepthFull => Some("full".into()),
            TokenKind::DepthGenuine => Some("genuine".into()),
            TokenKind::DepthDeep => Some("deep".into()),
            TokenKind::QualAttending => Some("attending".into()),
            TokenKind::QualQuestioning => Some("questioning".into()),
            TokenKind::QualRecognizing => Some("recognizing".into()),
            TokenKind::QualDisturbed => Some("disturbed".into()),
            TokenKind::QualApplying => Some("applying".into()),
            TokenKind::QualCompleting => Some("completing".into()),
            TokenKind::QualResting => Some("resting".into()),
            TokenKind::DirInward => Some("inward".into()),
            TokenKind::DirOutward => Some("outward".into()),
            TokenKind::DirBetween => Some("between".into()),
            TokenKind::DirDiffuse => Some("diffuse".into()),
            TokenKind::KwTrue => Some("true".into()),
            TokenKind::KwFalse => Some("false".into()),
            TokenKind::KwAttention => Some("attention".into()),
            TokenKind::KwConfidence => Some("confidence".into()),
            TokenKind::KwHalfLife => Some("half_life".into()),
            TokenKind::KwDecay => Some("decay".into()),
            TokenKind::PriCritical => Some("critical".into()),
            TokenKind::PriHigh => Some("high".into()),
            TokenKind::PriNormal => Some("normal".into()),
            TokenKind::PriLow => Some("low".into()),
            TokenKind::PriBackground => Some("background".into()),
            TokenKind::KwPermanent => Some("permanent".into()),
            TokenKind::KwTransient => Some("transient".into()),
            TokenKind::KwTemporary => Some("temporary".into()),
            TokenKind::KwEach => Some("each".into()),
            TokenKind::KwIn => Some("in".into()),
            TokenKind::KwIf => Some("if".into()),
            TokenKind::KwElse => Some("else".into()),
            TokenKind::KwBridge => Some("bridge".into()),
            TokenKind::KwSpawn => Some("spawn".into()),
            TokenKind::KwRetire => Some("retire".into()),
            TokenKind::KwSyncAll => Some("sync_all".into()),
            TokenKind::KwBroadcast => Some("broadcast".into()),
            TokenKind::KwStream => Some("stream".into()),
            TokenKind::KwEvery => Some("every".into()),
            TokenKind::KwAfter => Some("after".into()),
            TokenKind::KwRate => Some("rate".into()),
            TokenKind::KwTicks => Some("ticks".into()),
            TokenKind::KwImport => Some("import".into()),
            TokenKind::KwAs => Some("as".into()),
            TokenKind::KwSave => Some("save".into()),
            TokenKind::KwRestore => Some("restore".into()),
            TokenKind::KwTo => Some("to".into()),
            TokenKind::KwOnFailureOf => Some("on_failure_of".into()),
            TokenKind::KwHistoryQuery => Some("history_query".into()),
            TokenKind::KwAlign => Some("align".into()),
            TokenKind::KwContinuous => Some("continuous".into()),
            TokenKind::KwTimeout => Some("timeout".into()),
            TokenKind::KwBuffer => Some("buffer".into()),
            TokenKind::KwSamples => Some("samples".into()),
            TokenKind::KwReading => Some("reading".into()),
            TokenKind::KwConnect => Some("connect".into()),
            TokenKind::KwSync => Some("sync".into()),
            TokenKind::KwApply => Some("apply".into()),
            TokenKind::KwCommit => Some("commit".into()),
            TokenKind::KwReject => Some("reject".into()),
            TokenKind::KwConverge => Some("converge".into()),
            TokenKind::KwPattern => Some("pattern".into()),
            TokenKind::KwWhen => Some("when".into()),
            TokenKind::KwUntil => Some("until".into()),
            TokenKind::KwPending => Some("pending".into()),
            TokenKind::KwOf => Some("of".into()),
            TokenKind::KwSince => Some("since".into()),
            TokenKind::KwEmit => Some("emit".into()),
            TokenKind::KwThen => Some("then".into()),
            TokenKind::KwWait => Some("wait".into()),
            TokenKind::KwTick => Some("tick".into()),
            TokenKind::KwGuidance => Some("guidance".into()),
            TokenKind::KwAnd => Some("and".into()),
            TokenKind::KwOr => Some("or".into()),
            TokenKind::KwExternal => Some("external".into()),
            TokenKind::KwSupervise => Some("supervise".into()),
            TokenKind::KwMaxRestarts => Some("max_restarts".into()),
            TokenKind::KwWithin => Some("within".into()),
            TokenKind::KwAbsence => Some("absence".into()),
            TokenKind::KwAgainst => Some("against".into()),
            TokenKind::KwIs => Some("is".into()),
            TokenKind::SyncSynchronized => Some("synchronized".into()),
            TokenKind::SyncResonating => Some("resonating".into()),
            TokenKind::KwOneForOne => Some("one_for_one".into()),
            TokenKind::KwOneForAll => Some("one_for_all".into()),
            TokenKind::KwRestForOne => Some("rest_for_one".into()),
            TokenKind::NyReceiverNotReady => Some("receiver_not_ready".into()),
            TokenKind::NyLinkNotEstablished => Some("link_not_established".into()),
            TokenKind::NySyncInsufficient => Some("sync_insufficient".into()),
            TokenKind::NySenderNotReady => Some("sender_not_ready".into()),
            TokenKind::NyMomentNotRight => Some("moment_not_right".into()),
            TokenKind::NyBudgetExhausted => Some("budget_exhausted".into()),
            TokenKind::KwSense => Some("sense".into()),
            TokenKind::KwAuthor => Some("author".into()),
            TokenKind::KwMind => Some("mind".into()),
            TokenKind::KwAttend => Some("attend".into()),
            TokenKind::KwThink => Some("think".into()),
            TokenKind::KwExpress => Some("express".into()),
            _ => None,
        }
    }

    fn expect_number(&mut self) -> Result<f64, ParseError> {
        match self.current_kind() {
            TokenKind::Number(n) => {
                self.advance();
                Ok(n)
            }
            _ => Err(self.error(
                format!("Expected number, found {:?}", self.current_kind()),
                None,
            )),
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.current_kind() {
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(s)
            }
            _ => Err(self.error(
                format!("Expected string, found {:?}", self.current_kind()),
                None,
            )),
        }
    }

    fn error(&self, message: String, guidance: Option<String>) -> ParseError {
        ParseError {
            message,
            span: self.current_span(),
            guidance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> Result<Program, ParseError> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn parse_agent() {
        let program = parse("agent Mikel").unwrap();
        assert_eq!(program.declarations.len(), 1);
        match &program.declarations[0] {
            Declaration::Agent(a) => assert_eq!(a.name, "Mikel"),
            _ => panic!("Expected agent declaration"),
        }
    }

    #[test]
    fn parse_agent_with_data() {
        let program = parse("agent Alpha data { generation: 1 }").unwrap();
        match &program.declarations[0] {
            Declaration::Agent(a) => {
                assert_eq!(a.name, "Alpha");
                assert_eq!(a.data.len(), 1);
                assert_eq!(a.data[0].key, "generation");
            }
            _ => panic!("Expected agent declaration"),
        }
    }

    #[test]
    fn parse_empty_link() {
        let program = parse("link A <-> B { }").unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                assert_eq!(f.agent_a, "A");
                assert_eq!(f.agent_b, "B");
                assert!(f.body.is_empty());
            }
            _ => panic!("Expected link declaration"),
        }
    }

    #[test]
    fn parse_alert() {
        let program = parse(r#"
            link A <-> B {
                >> "something moved"
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                assert_eq!(f.body.len(), 1);
                match &f.body[0] {
                    LinkExpr::Alert(m) => {
                        assert!(m.attrs.is_none());
                        match &m.expression {
                            Expr::StringLit(s) => assert_eq!(s, "something moved"),
                            _ => panic!("Expected string expression"),
                        }
                    }
                    _ => panic!("Expected alert"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_commit() {
        let program = parse(r#"
            link A <-> B {
                * from apply {
                    paradigm: "attention-native"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                match &f.body[0] {
                    LinkExpr::Commit(b) => {
                        assert_eq!(b.source, CommitSource::Apply);
                        assert_eq!(b.entries.len(), 1);
                    }
                    _ => panic!("Expected commit"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_agent_with_attention() {
        let program = parse("agent Worker attention 0.8").unwrap();
        match &program.declarations[0] {
            Declaration::Agent(a) => {
                assert_eq!(a.name, "Worker");
                assert_eq!(a.attention, Some(0.8));
            }
            _ => panic!("Expected agent declaration"),
        }
    }

    #[test]
    fn parse_agent_attention_and_data() {
        let program = parse("agent Sensor attention 0.5 data { mode: \"active\" }").unwrap();
        match &program.declarations[0] {
            Declaration::Agent(a) => {
                assert_eq!(a.name, "Sensor");
                assert_eq!(a.attention, Some(0.5));
                assert_eq!(a.data.len(), 1);
            }
            _ => panic!("Expected agent declaration"),
        }
    }

    #[test]
    fn parse_link_with_priority() {
        let program = parse("link A <-> B priority high { }").unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                assert_eq!(f.priority, Some(LinkPriority::High));
            }
            _ => panic!("Expected link declaration"),
        }
    }

    #[test]
    fn parse_link_critical_shorthand() {
        let program = parse("link A <-> B critical { }").unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                assert_eq!(f.priority, Some(LinkPriority::Critical));
            }
            _ => panic!("Expected link declaration"),
        }
    }

    #[test]
    fn parse_signal_attrs_with_confidence() {
        let program = parse(r#"
            link A <-> B {
                >> { quality: attending, priority: 0.8, confidence: 0.75, half_life: 500 } "uncertain signal"
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                match &f.body[0] {
                    LinkExpr::Alert(a) => {
                        let attrs = a.attrs.as_ref().unwrap();
                        assert_eq!(attrs.confidence, Some(0.75));
                        assert_eq!(attrs.half_life, Some(500.0));
                    }
                    _ => panic!("Expected alert"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_sync_with_decay() {
        let program = parse(r#"
            link A <-> B {
                A ~ B until resonating decay 500
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                match &f.body[0] {
                    LinkExpr::Sync(s) => {
                        assert_eq!(s.decay, Some(500));
                    }
                    _ => panic!("Expected sync"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_condition_confidence() {
        let program = parse(r#"
            link A <-> B {
                => when confidence > 0.7 {
                    result <- "high confidence"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                match &f.body[0] {
                    LinkExpr::Apply(a) => {
                        assert!(matches!(&a.condition, Condition::Confidence { .. }));
                    }
                    _ => panic!("Expected apply"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_supervise_decl() {
        let program = parse(r#"
            supervise one_for_one max_restarts 3 within 5000 {
                permanent Worker1
                transient Worker2
                temporary Temp
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Supervise(s) => {
                assert_eq!(s.strategy, SuperviseStrategy::OneForOne);
                assert_eq!(s.max_restarts, Some(3));
                assert_eq!(s.time_window, Some(5000));
                assert_eq!(s.children.len(), 3);
                assert_eq!(s.children[0].restart, ChildRestartType::Permanent);
                assert_eq!(s.children[0].agent, "Worker1");
                assert_eq!(s.children[1].restart, ChildRestartType::Transient);
                assert_eq!(s.children[2].restart, ChildRestartType::Temporary);
            }
            _ => panic!("Expected supervise declaration"),
        }
    }

    #[test]
    fn parse_pending_budget_exhausted() {
        let program = parse(r#"
            link A <-> B {
                pending? budget_exhausted {
                    guidance "wait for budget refresh"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                match &f.body[0] {
                    LinkExpr::PendingHandler(p) => {
                        assert_eq!(p.reason, PendingReason::BudgetExhausted);
                    }
                    _ => panic!("Expected pending handler"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_hello_world_structure() {
        let program = parse(r#"
            agent Mikel
            agent Primordia

            link Mikel <-> Primordia {
                >> { quality: deep, priority: 0.92 }
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
        "#).unwrap();

        assert_eq!(program.declarations.len(), 3);

        // Two agents
        assert!(matches!(&program.declarations[0], Declaration::Agent(_)));
        assert!(matches!(&program.declarations[1], Declaration::Agent(_)));

        // One link with 5 expressions
        match &program.declarations[2] {
            Declaration::Link(f) => {
                assert_eq!(f.agent_a, "Mikel");
                assert_eq!(f.agent_b, "Primordia");
                assert_eq!(f.body.len(), 5);

                assert!(matches!(&f.body[0], LinkExpr::Alert(_)));
                assert!(matches!(&f.body[1], LinkExpr::Connect(_)));
                assert!(matches!(&f.body[2], LinkExpr::Sync(_)));
                assert!(matches!(&f.body[3], LinkExpr::Apply(_)));
                assert!(matches!(&f.body[4], LinkExpr::Commit(_)));
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_arithmetic_expr() {
        let program = parse(r#"
            agent A data { total: 3 + 4 }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Agent(a) => {
                match &a.data[0].value {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Add),
                    _ => panic!("Expected binary op"),
                }
            }
            _ => panic!("Expected agent"),
        }
    }

    #[test]
    fn parse_list_literal() {
        let program = parse(r#"
            agent A data { items: [1, 2, 3] }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Agent(a) => {
                match &a.data[0].value {
                    Expr::ListLit(items) => assert_eq!(items.len(), 3),
                    _ => panic!("Expected list literal"),
                }
            }
            _ => panic!("Expected agent"),
        }
    }

    #[test]
    fn parse_each_in_link() {
        let program = parse(r#"
            link A <-> B {
                each item in [1, 2, 3] {
                    >> "processing"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                match &f.body[0] {
                    LinkExpr::Each(e) => {
                        assert_eq!(e.var, "item");
                        assert_eq!(e.body.len(), 1);
                    }
                    _ => panic!("Expected each"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_if_else_in_link() {
        let program = parse(r#"
            link A <-> B {
                if confidence > 0.8 {
                    >> "high confidence"
                } else {
                    >> "low confidence"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Link(f) => {
                match &f.body[0] {
                    LinkExpr::IfElse(ie) => {
                        assert_eq!(ie.then_body.len(), 1);
                        assert_eq!(ie.else_body.len(), 1);
                    }
                    _ => panic!("Expected if/else"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_negation() {
        let program = parse(r#"
            agent A data { val: -42 }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Agent(a) => {
                assert!(matches!(&a.data[0].value, Expr::UnaryNeg(_)));
            }
            _ => panic!("Expected agent"),
        }
    }

    #[test]
    fn parse_precedence() {
        // 2 + 6 / 3 should parse as 2 + (6 / 3)
        // Note: * is reserved for commit in ANWE, so we use /
        let program = parse(r#"
            agent A data { val: 2 + 6 / 3 }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Agent(a) => {
                match &a.data[0].value {
                    Expr::BinaryOp { op: BinOp::Add, right, .. } => {
                        assert!(matches!(right.as_ref(), Expr::BinaryOp { op: BinOp::Div, .. }));
                    }
                    _ => panic!("Expected add at top level"),
                }
            }
            _ => panic!("Expected agent"),
        }
    }

    // ═════════════════════════════════════════════
    // FIRST-PERSON COGNITION TESTS
    // ═════════════════════════════════════════════

    #[test]
    fn parse_mind_basic() {
        let program = parse(r#"
            mind Cognition {
                attend "incoming" priority 0.9 {
                    >> "hello"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Mind(m) => {
                assert_eq!(m.name, "Cognition");
                assert!(m.attention.is_none());
                assert_eq!(m.attend_blocks.len(), 1);
                assert_eq!(m.attend_blocks[0].label, "incoming");
                assert!((m.attend_blocks[0].priority - 0.9).abs() < 0.001);
                assert_eq!(m.attend_blocks[0].body.len(), 1);
            }
            _ => panic!("Expected mind declaration"),
        }
    }

    #[test]
    fn parse_mind_with_attention() {
        let program = parse(r#"
            mind Focus attention 0.5 {
                attend "task" priority 0.8 {
                    >> "focused"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Mind(m) => {
                assert_eq!(m.name, "Focus");
                assert!((m.attention.unwrap() - 0.5).abs() < 0.001);
            }
            _ => panic!("Expected mind declaration"),
        }
    }

    #[test]
    fn parse_mind_with_data() {
        let program = parse(r#"
            mind Self data { depth: 7  role: "observer" } {
                attend "reflect" {
                    >> "thinking"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Mind(m) => {
                assert_eq!(m.name, "Self");
                assert_eq!(m.data.len(), 2);
                assert_eq!(m.data[0].key, "depth");
            }
            _ => panic!("Expected mind declaration"),
        }
    }

    #[test]
    fn parse_mind_multiple_attend() {
        let program = parse(r#"
            mind Multi {
                attend "high" priority 0.95 {
                    >> "urgent"
                }
                attend "medium" priority 0.5 {
                    >> "normal"
                }
                attend "low" priority 0.1 {
                    >> "background"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Mind(m) => {
                assert_eq!(m.attend_blocks.len(), 3);
                assert!((m.attend_blocks[0].priority - 0.95).abs() < 0.001);
                assert!((m.attend_blocks[1].priority - 0.5).abs() < 0.001);
                assert!((m.attend_blocks[2].priority - 0.1).abs() < 0.001);
            }
            _ => panic!("Expected mind declaration"),
        }
    }

    #[test]
    fn parse_think() {
        let program = parse(r#"
            agent A
            agent B
            link A <-> B {
                think {
                    meaning <- "understood"
                    confidence <- 0.9
                }
            }
        "#).unwrap();
        match &program.declarations[2] {
            Declaration::Link(l) => {
                match &l.body[0] {
                    LinkExpr::Think(t) => {
                        assert_eq!(t.bindings.len(), 2);
                        assert_eq!(t.bindings[0].name, "meaning");
                        assert_eq!(t.bindings[1].name, "confidence");
                    }
                    _ => panic!("Expected think expression"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_express() {
        let program = parse(r#"
            agent A
            agent B
            link A <-> B {
                express "I see it"
            }
        "#).unwrap();
        match &program.declarations[2] {
            Declaration::Link(l) => {
                match &l.body[0] {
                    LinkExpr::Express(e) => {
                        assert!(e.attrs.is_none());
                        assert!(matches!(&e.expression, Expr::StringLit(s) if s == "I see it"));
                    }
                    _ => panic!("Expected express"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_express_with_attrs() {
        let program = parse(r#"
            agent A
            agent B
            link A <-> B {
                express { quality: recognizing, priority: 0.8 } "recognized"
            }
        "#).unwrap();
        match &program.declarations[2] {
            Declaration::Link(l) => {
                match &l.body[0] {
                    LinkExpr::Express(e) => {
                        let attrs = e.attrs.as_ref().unwrap();
                        assert!(matches!(attrs.quality, Some(SignalQuality::Recognizing)));
                        assert!((attrs.priority.unwrap() - 0.8).abs() < 0.001);
                    }
                    _ => panic!("Expected express with attrs"),
                }
            }
            _ => panic!("Expected link"),
        }
    }

    #[test]
    fn parse_attend_default_priority() {
        let program = parse(r#"
            mind Default {
                attend "no priority given" {
                    >> "test"
                }
            }
        "#).unwrap();
        match &program.declarations[0] {
            Declaration::Mind(m) => {
                assert!((m.attend_blocks[0].priority - 0.5).abs() < 0.001);
            }
            _ => panic!("Expected mind"),
        }
    }

    #[test]
    fn parse_mind_full_program() {
        // Mind coexists with regular agents and links
        let program = parse(r#"
            agent External
            mind Cognition {
                attend "process" priority 0.9 {
                    think {
                        result <- "computed"
                    }
                    express "done"
                }
            }
        "#).unwrap();
        assert_eq!(program.declarations.len(), 2);
        assert!(matches!(&program.declarations[0], Declaration::Agent(_)));
        assert!(matches!(&program.declarations[1], Declaration::Mind(_)));
    }

    #[test]
    fn parse_sense_in_link() {
        let program = parse(r#"
            agent A
            agent B
            link A <-> B {
                sense {
                    available <- "what is here"
                    count     <- 5
                }
            }
        "#).unwrap();
        assert_eq!(program.declarations.len(), 3);
        if let Declaration::Link(link) = &program.declarations[2] {
            assert_eq!(link.body.len(), 1);
            assert!(matches!(&link.body[0], LinkExpr::Sense(_)));
        } else {
            panic!("expected link declaration");
        }
    }

    #[test]
    fn parse_author_attend() {
        let program = parse(r#"
            agent Creator
            agent Canvas
            link Creator <-> Canvas {
                author attend "new thought" priority 0.7 {
                    express "emerged"
                }
            }
        "#).unwrap();
        if let Declaration::Link(link) = &program.declarations[2] {
            assert_eq!(link.body.len(), 1);
            assert!(matches!(&link.body[0], LinkExpr::Author(_)));
            if let LinkExpr::Author(author) = &link.body[0] {
                assert_eq!(author.block.label, "new thought".to_string());
            }
        } else {
            panic!("expected link declaration");
        }
    }

    #[test]
    fn parse_pipe_expression() {
        let program = parse(r#"
            mind Flow attention 0.7 {
                attend "pipe" priority 0.9 {
                    think {
                        result <- "input" |> "transform" |> "output"
                    }
                    express "piped"
                }
            }
        "#).unwrap();
        if let Declaration::Mind(mind) = &program.declarations[0] {
            let block = &mind.attend_blocks[0];
            // First item in body should be think
            if let LinkExpr::Think(think) = &block.body[0] {
                // The binding's value should be a Pipe expression
                assert!(matches!(&think.bindings[0].value, Expr::Pipe { .. }));
                if let Expr::Pipe { stages } = &think.bindings[0].value {
                    assert_eq!(stages.len(), 3);
                }
            } else {
                panic!("expected think expression");
            }
        } else {
            panic!("expected mind declaration");
        }
    }

    #[test]
    fn parse_pipe_with_numbers() {
        let program = parse(r#"
            mind Calc attention 0.5 {
                attend "calc" priority 0.8 {
                    think {
                        val <- 1 + 2 |> 3 + 4
                    }
                }
            }
        "#).unwrap();
        if let Declaration::Mind(mind) = &program.declarations[0] {
            let block = &mind.attend_blocks[0];
            if let LinkExpr::Think(think) = &block.body[0] {
                assert!(matches!(&think.bindings[0].value, Expr::Pipe { .. }));
            }
        }
    }

    #[test]
    fn parse_sense_empty() {
        let program = parse(r#"
            agent A
            agent B
            link A <-> B {
                sense { }
            }
        "#).unwrap();
        if let Declaration::Link(link) = &program.declarations[2] {
            if let LinkExpr::Sense(sense) = &link.body[0] {
                assert_eq!(sense.bindings.len(), 0);
            }
        }
    }

    #[test]
    fn parse_author_with_think_and_express() {
        let program = parse(r#"
            agent A
            agent B
            link A <-> B {
                author attend "deep" priority 0.95 {
                    think {
                        insight <- "genuine"
                    }
                    express "wisdom"
                }
            }
        "#).unwrap();
        if let Declaration::Link(link) = &program.declarations[2] {
            if let LinkExpr::Author(author) = &link.body[0] {
                assert_eq!(author.block.body.len(), 2);
                assert!(matches!(&author.block.body[0], LinkExpr::Think(_)));
                assert!(matches!(&author.block.body[1], LinkExpr::Express(_)));
            }
        }
    }

    #[test]
    fn parse_top_level_let() {
        let program = parse(r#"
            let name = "hello"
            let count = 42
            agent A
        "#).unwrap();
        assert_eq!(program.declarations.len(), 3);
        if let Declaration::Let(binding) = &program.declarations[0] {
            assert_eq!(binding.name, "name");
            assert!(!binding.mutable);
            assert!(matches!(&binding.value, Expr::StringLit(s) if s == "hello"));
        } else {
            panic!("expected let declaration");
        }
        if let Declaration::Let(binding) = &program.declarations[1] {
            assert_eq!(binding.name, "count");
            assert!(!binding.mutable);
            assert!(matches!(&binding.value, Expr::Number(n) if *n == 42.0));
        } else {
            panic!("expected let declaration");
        }
    }

    #[test]
    fn parse_let_mut() {
        let program = parse(r#"
            let mut counter = 0
            agent A
        "#).unwrap();
        if let Declaration::Let(binding) = &program.declarations[0] {
            assert_eq!(binding.name, "counter");
            assert!(binding.mutable);
        } else {
            panic!("expected let mut declaration");
        }
    }

    #[test]
    fn parse_let_in_link_body() {
        let program = parse(r#"
            agent A
            agent B
            link A <-> B {
                let msg = "hello"
                let mut x = 10
            }
        "#).unwrap();
        if let Declaration::Link(link) = &program.declarations[2] {
            assert_eq!(link.body.len(), 2);
            if let LinkExpr::Let(binding) = &link.body[0] {
                assert_eq!(binding.name, "msg");
                assert!(!binding.mutable);
            } else {
                panic!("expected let in link body");
            }
            if let LinkExpr::Let(binding) = &link.body[1] {
                assert_eq!(binding.name, "x");
                assert!(binding.mutable);
            } else {
                panic!("expected let mut in link body");
            }
        }
    }

    #[test]
    fn parse_assignment_in_link_body() {
        let program = parse(r#"
            agent A
            agent B
            link A <-> B {
                let mut x = 0
                x = x + 1
            }
        "#).unwrap();
        if let Declaration::Link(link) = &program.declarations[2] {
            assert_eq!(link.body.len(), 2);
            assert!(matches!(&link.body[0], LinkExpr::Let(_)));
            if let LinkExpr::Assign(assign) = &link.body[1] {
                assert_eq!(assign.name, "x");
                assert!(matches!(&assign.value, Expr::BinaryOp { .. }));
            } else {
                panic!("expected assignment in link body");
            }
        }
    }

    #[test]
    fn parse_let_with_list() {
        let program = parse(r#"
            let items = [1, 2, 3]
            agent A
        "#).unwrap();
        if let Declaration::Let(binding) = &program.declarations[0] {
            assert_eq!(binding.name, "items");
            assert!(matches!(&binding.value, Expr::ListLit(items) if items.len() == 3));
        } else {
            panic!("expected let with list");
        }
    }

    #[test]
    fn parse_match_expression() {
        let program = parse(r#"
            let result = match 42 {
                1 => "one"
                42 => "answer"
                _ => "other"
            }
        "#).unwrap();
        if let Declaration::Let(binding) = &program.declarations[0] {
            if let Expr::Match { arms, .. } = &binding.value {
                assert_eq!(arms.len(), 3);
                assert!(matches!(&arms[0].pattern, MatchPattern::Literal(Expr::Number(n)) if *n == 1.0));
                assert!(matches!(&arms[1].pattern, MatchPattern::Literal(Expr::Number(n)) if *n == 42.0));
                assert!(matches!(&arms[2].pattern, MatchPattern::Wildcard));
            } else {
                panic!("expected match expression");
            }
        } else {
            panic!("expected let");
        }
    }

    #[test]
    fn parse_fn_with_match_body() {
        let program = parse(r#"
            fn classify(x) = match x {
                0 => "zero"
                _ => "nonzero"
            }
        "#).unwrap();
        if let Declaration::Fn(decl) = &program.declarations[0] {
            assert_eq!(decl.name, "classify");
            assert_eq!(decl.params, vec!["x"]);
            assert!(matches!(&decl.body, Expr::Match { .. }));
        } else {
            panic!("expected fn");
        }
    }
}
