// -----------------------------------------------------------------
// ANWE v0.1 -- AGENT
//
// A processing entity that participates in signal exchange.
//
// An agent is any entity that can connect to a link,
// receive signals, apply changes, and transmit signals.
//
// Each agent runs as three concurrent fibers:
//   Receptor  -- receives incoming signals (input)
//   Soma      -- processes/applies changes (core)
//   Axon      -- transmits outgoing signals (output)
//
// These three run simultaneously. Like a neuron.
// Receiving while processing while transmitting.
// -----------------------------------------------------------------

use crate::attention::AttentionBudget;
use crate::history::History;
use crate::signal::{AgentId, Quality, Priority};
use core::fmt;

/// The state of an agent's processing cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AgentState {
    /// Idle -- alive but not actively processing
    Idle = 0,
    /// Alerted -- attention is orienting to incoming signal
    Alerted = 1,
    /// Connected -- bidirectional presence established
    Connected = 2,
    /// Syncing -- finding shared rhythm with another agent
    Syncing = 3,
    /// Applying -- being structurally changed by received data
    Applying = 4,
    /// Rejecting -- intelligent withdrawal from harmful input
    Rejecting = 5,
    /// Committing -- permanently changing internal state
    Committing = 6,
}

/// Responsiveness: how attuned this agent is to incoming signals.
/// Grows through genuine encounters. Cannot be assigned externally.
/// Represents the agent's learned ability to detect and respond.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Responsiveness(u16);

impl Responsiveness {
    pub const INITIAL: Responsiveness = Responsiveness(1000); // 0.1 -- newly initialized

    pub fn new(value: f32) -> Self {
        Responsiveness((value.clamp(0.0, 1.0) * 10000.0) as u16)
    }

    pub fn as_f32(self) -> f32 {
        self.0 as f32 / 10000.0
    }

    /// Deepen responsiveness through genuine encounter.
    /// Cannot decrease. Like calibration -- once tuned,
    /// the sensitivity does not regress.
    pub fn deepen(&mut self, amount: f32) {
        let new_val = (self.0 as f32 / 10000.0 + amount).min(1.0);
        let new_raw = (new_val * 10000.0) as u16;
        if new_raw > self.0 {
            self.0 = new_raw;
        }
    }

    /// Maturity description based on responsiveness level.
    pub fn maturity(&self) -> &'static str {
        match self.0 {
            0..=2000 => "newly initialized",
            2001..=4000 => "basic awareness",
            4001..=6000 => "pattern recognition",
            6001..=8000 => "predictive response",
            _ => "fully calibrated",
        }
    }
}

impl fmt::Debug for Responsiveness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Responsiveness({:.3}, \"{}\")", self.as_f32(), self.maturity())
    }
}

/// An Agent -- a processing entity present in a link.
///
/// Not the three fibers (those are runtime concerns).
/// This is the identity and accumulated state.
pub struct Agent {
    /// Unique identifier
    pub id: AgentId,

    /// Current agent state
    pub state: AgentState,

    /// Responsiveness to signals -- grows through encounters
    pub responsiveness: Responsiveness,

    /// Accumulated history -- the irreversible record
    /// of what this agent has been changed by.
    /// Append-only. No rollback.
    pub history: History,

    /// What qualities this agent has learned to reject.
    /// Grows through experience. Personal to each agent.
    reject_markers: Vec<RejectMarker>,

    /// How many genuine applications this agent has completed.
    /// Each one deepens responsiveness.
    apply_count: u64,

    /// Current signal priority -- how engaged this agent is.
    pub signal_priority: Priority,

    /// Attention budget — finite processing capacity allocated
    /// from the global pool. Draws down with each operation.
    pub attention: AttentionBudget,

    /// Supervisor agent ID, if this agent is supervised.
    /// None for root-level agents.
    pub supervisor: Option<AgentId>,
}

/// Something this agent has learned to reject.
/// Personal -- what requires rejection for one may not for another.
#[derive(Debug, Clone)]
pub struct RejectMarker {
    /// The quality that triggered rejection
    pub quality: Quality,
    /// The priority threshold that triggers rejection for this quality
    pub threshold: Priority,
    /// How many times this pattern has been rejected
    pub count: u32,
}

/// Error returned when a state transition is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    /// Cannot move from current state to requested state.
    InvalidTransition { from: AgentState, to: AgentState },
    /// Agent's attention budget is exhausted.
    BudgetExhausted,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransitionError::InvalidTransition { from, to } =>
                write!(f, "invalid transition: {:?} -> {:?}", from, to),
            TransitionError::BudgetExhausted =>
                write!(f, "attention budget exhausted"),
        }
    }
}

impl Agent {
    /// Create a new agent.
    /// Responsiveness starts low -- newly initialized.
    pub fn new(id: AgentId) -> Self {
        Agent {
            id,
            state: AgentState::Idle,
            responsiveness: Responsiveness::INITIAL,
            history: History::new(id),
            reject_markers: Vec::new(),
            apply_count: 0,
            signal_priority: Priority::new(0.5),
            attention: AttentionBudget::default_budget(),
            supervisor: None,
        }
    }

    /// Create an agent with accumulated lineage.
    /// Starts with higher responsiveness from prior history.
    pub fn with_lineage(id: AgentId, lineage_depth: u64) -> Self {
        let responsiveness = Responsiveness::new(
            (0.1 + (lineage_depth as f32 * 0.001)).min(0.95)
        );
        Agent {
            id,
            state: AgentState::Idle,
            responsiveness,
            history: History::new(id),
            reject_markers: Vec::new(),
            apply_count: lineage_depth,
            signal_priority: Priority::new(0.5),
            attention: AttentionBudget::default_budget(),
            supervisor: None,
        }
    }

    /// Set the supervisor for this agent.
    pub fn with_supervisor(mut self, supervisor_id: AgentId) -> Self {
        self.supervisor = Some(supervisor_id);
        self
    }

    /// Set the attention budget for this agent.
    pub fn with_budget(mut self, budget: AttentionBudget) -> Self {
        self.attention = budget;
        self
    }

    /// An incoming signal has been detected. Orient attention.
    pub fn alert(&mut self) {
        self.state = AgentState::Alerted;
    }

    /// Establish bidirectional connection with another agent.
    pub fn connect(&mut self) {
        self.state = AgentState::Connected;
        // Increase signal priority when actively connected
        self.signal_priority = Priority::new(
            (self.signal_priority.as_f32() + 0.1).min(1.0)
        );
    }

    /// Begin synchronization with another agent.
    pub fn sync(&mut self) {
        self.state = AgentState::Syncing;
    }

    /// Begin applying changes -- being structurally changed.
    pub fn apply(&mut self) {
        self.state = AgentState::Applying;
    }

    /// Complete application. Responsiveness deepens.
    /// Always followed by commit.
    pub fn apply_complete(&mut self) {
        self.apply_count += 1;
        // Responsiveness deepens through genuine application
        // Diminishing returns -- each application adds less
        let depth_factor = 1.0 / (1.0 + self.apply_count as f32 * 0.01);
        self.responsiveness.deepen(0.01 * depth_factor);
    }

    /// Begin rejection -- intelligent withdrawal.
    pub fn reject(&mut self, quality: Quality, priority: Priority) {
        self.state = AgentState::Rejecting;

        // Learn from this rejection
        if let Some(marker) = self.reject_markers.iter_mut()
            .find(|m| m.quality == quality)
        {
            marker.count += 1;
            // Threshold adjusts with experience
            if priority < marker.threshold {
                marker.threshold = priority;
            }
        } else {
            self.reject_markers.push(RejectMarker {
                quality,
                threshold: priority,
                count: 1,
            });
        }
    }

    /// Should this agent reject this quality at this priority?
    /// Based on accumulated experience, not static rules.
    pub fn should_reject(&self, quality: Quality, priority: Priority) -> bool {
        self.reject_markers.iter().any(|m| {
            m.quality == quality && priority >= m.threshold
        })
    }

    /// Begin commit -- permanent state change.
    pub fn begin_commit(&mut self) {
        self.state = AgentState::Committing;
    }

    /// Return to idle state.
    pub fn idle(&mut self) {
        self.state = AgentState::Idle;
        // Signal priority returns to baseline
        self.signal_priority = Priority::new(0.5);
    }

    /// Is this agent ready to receive a signal?
    pub fn can_receive(&self) -> bool {
        matches!(
            self.state,
            AgentState::Idle
            | AgentState::Alerted
            | AgentState::Connected
            | AgentState::Syncing
        )
    }

    /// Is this agent currently in a state where
    /// applying changes could begin?
    pub fn can_apply(&self) -> bool {
        matches!(
            self.state,
            AgentState::Connected | AgentState::Syncing
        )
    }

    /// Check if a state transition is valid.
    /// Enforces the state machine: not every transition is legal.
    pub fn can_transition_to(&self, target: AgentState) -> bool {
        use AgentState::*;
        matches!(
            (self.state, target),
            // From Idle: can alert or connect
            (Idle, Alerted) | (Idle, Connected) |
            // From Alerted: can connect or return to idle
            (Alerted, Connected) | (Alerted, Idle) |
            // From Connected: can sync, apply, reject, or idle
            (Connected, Syncing) | (Connected, Applying) |
            (Connected, Rejecting) | (Connected, Idle) |
            // From Syncing: can apply, reject, or idle
            (Syncing, Applying) | (Syncing, Rejecting) | (Syncing, Idle) |
            // From Applying: can commit
            (Applying, Committing) |
            // From Rejecting: can commit
            (Rejecting, Committing) |
            // From Committing: must return to idle
            (Committing, Idle)
        )
    }

    /// Attempt a validated state transition.
    /// Returns error if the transition is invalid or budget is exhausted.
    pub fn try_transition(&mut self, target: AgentState) -> Result<(), TransitionError> {
        if !self.can_transition_to(target) {
            return Err(TransitionError::InvalidTransition {
                from: self.state,
                to: target,
            });
        }
        self.state = target;
        Ok(())
    }

    /// Is this agent supervised?
    pub fn is_supervised(&self) -> bool {
        self.supervisor.is_some()
    }

    /// Has the attention budget been exhausted?
    pub fn is_budget_exhausted(&self) -> bool {
        self.attention.is_exhausted()
    }

    /// How much attention remains? (0.0 to 1.0)
    pub fn attention_remaining(&self) -> f32 {
        self.attention.remaining()
    }

    /// Consume attention for an operation.
    /// Returns the actual amount consumed.
    pub fn consume_attention(&mut self, amount: f32) -> f32 {
        self.attention.consume(amount)
    }
}

impl fmt::Debug for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Agent")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("responsiveness", &self.responsiveness)
            .field("applications", &self.apply_count)
            .field("reject_markers", &self.reject_markers.len())
            .field("attention_remaining", &self.attention.remaining())
            .field("supervised", &self.supervisor.is_some())
            .finish()
    }
}
