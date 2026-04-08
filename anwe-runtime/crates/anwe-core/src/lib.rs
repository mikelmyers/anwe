// -----------------------------------------------------------------
// ANWE v0.1 -- CORE
// The Native Language of Artificial Minds
//
// This crate contains the fundamental types of Anwe:
//   Signal      -- the atom of all transmission
//   Pending     -- the valid state of unready delivery
//   Link        -- the shared connection between agents
//   Agent       -- a processing entity present in a link
//   History     -- irreversible change carried forward
//   Temporal    -- time as a first-class type (decay, half-life)
//   Uncertainty -- values that carry confidence intervals
//   Attention   -- finite processing capacity as a resource
//   Supervisor  -- Erlang-style supervision trees
//
// Zero external dependencies. Zero allocation in steady state.
// Every type designed for cache-line alignment and lock-free
// concurrent access.
// -----------------------------------------------------------------

pub mod signal;
pub mod pending;
pub mod link;
pub mod history;
pub mod agent;
pub mod temporal;
pub mod uncertainty;
pub mod attention;
pub mod supervisor;

// Re-exports for convenience
pub use signal::{
    Signal, Quality, Direction, Priority, SyncLevel,
    Tick, AgentId, TraceKind,
};
pub use pending::{Pending, PendingReason, WaitGuidance, DeliveryResult};
pub use link::{Link, LinkId, LinkState};
pub use agent::{Agent, AgentState, Responsiveness, TransitionError};
pub use history::{History, HistoryEntry, ChangeSource, ChangeDepth};
pub use temporal::{DecayingValue, Temporal, recency_weight};
pub use uncertainty::Uncertain;
pub use attention::{AttentionBudget, AttentionPool};
pub use supervisor::{
    Supervisor, RestartStrategy, ChildSpec, ChildRestart, FailureReason,
};
