// -----------------------------------------------------------------
// ANWE v0.1 -- SUPERVISION TREES
//
// Agents fail. This is not a bug. It is a fact.
// What matters is what happens after failure.
//
// Erlang taught us: let it crash, then restart cleanly.
// A supervisor watches its children and applies a strategy:
//
//   OneForOne   — restart only the failed child
//   OneForAll   — restart all children if one fails
//   RestForOne  — restart failed child and everything after it
//
// The supervisor itself is an agent — it participates in links,
// receives signals, and makes decisions. But its special role
// is maintaining the health of its subtree.
//
// Max restarts within a time window. If exceeded, the supervisor
// itself fails — escalating to its own supervisor, all the way
// up to the root. This is how the system stays alive.
// -----------------------------------------------------------------

use crate::signal::{AgentId, Tick};
use core::fmt;

/// Restart strategy for a supervisor.
///
/// Determines what happens when a child agent fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartStrategy {
    /// Restart only the failed child.
    /// Other children are unaffected.
    OneForOne,
    /// Restart all children if any one fails.
    /// For groups where all must be consistent.
    OneForAll,
    /// Restart the failed child and all children
    /// started after it. For ordered dependency chains.
    RestForOne,
}

/// How a child should be restarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildRestart {
    /// Always restart on failure.
    Permanent,
    /// Restart only on abnormal termination.
    Transient,
    /// Never restart. If it fails, it stays down.
    Temporary,
}

/// Why did a child fail?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReason {
    /// Agent crashed (panic, unrecoverable error).
    Crash,
    /// Agent exceeded its attention budget.
    BudgetExhausted,
    /// Agent failed to respond within timeout.
    Timeout,
    /// Agent explicitly requested termination.
    Shutdown,
    /// Agent was killed by supervisor.
    Killed,
}

/// Specification for a supervised child agent.
#[derive(Debug, Clone)]
pub struct ChildSpec {
    /// The agent being supervised.
    pub agent_id: AgentId,
    /// How to restart this child.
    pub restart: ChildRestart,
    /// Maximum time (in ticks) to wait for graceful shutdown.
    pub shutdown_timeout: u32,
    /// How many times this child has been restarted.
    restarts: u32,
}

impl ChildSpec {
    /// Create a new child specification.
    pub fn new(agent_id: AgentId, restart: ChildRestart) -> Self {
        ChildSpec {
            agent_id,
            restart,
            shutdown_timeout: 5000,
            restarts: 0,
        }
    }

    /// Create a permanent child (always restarted).
    pub fn permanent(agent_id: AgentId) -> Self {
        Self::new(agent_id, ChildRestart::Permanent)
    }

    /// Create a transient child (restarted on abnormal exit).
    pub fn transient(agent_id: AgentId) -> Self {
        Self::new(agent_id, ChildRestart::Transient)
    }

    /// Create a temporary child (never restarted).
    pub fn temporary(agent_id: AgentId) -> Self {
        Self::new(agent_id, ChildRestart::Temporary)
    }

    /// With a custom shutdown timeout.
    pub fn with_shutdown_timeout(mut self, ticks: u32) -> Self {
        self.shutdown_timeout = ticks;
        self
    }

    /// Should this child be restarted given the failure reason?
    pub fn should_restart(&self, reason: FailureReason) -> bool {
        match self.restart {
            ChildRestart::Permanent => reason != FailureReason::Shutdown,
            ChildRestart::Transient => matches!(
                reason,
                FailureReason::Crash | FailureReason::BudgetExhausted | FailureReason::Timeout
            ),
            ChildRestart::Temporary => false,
        }
    }

    /// Record a restart.
    pub fn record_restart(&mut self) {
        self.restarts += 1;
    }

    /// How many times has this child been restarted?
    pub fn restart_count(&self) -> u32 {
        self.restarts
    }
}

/// A record of when a restart happened.
#[derive(Debug, Clone, Copy)]
struct RestartRecord {
    tick: Tick,
}

/// A supervisor — watches children and restarts them on failure.
///
/// The supervisor is identified by its own AgentId.
/// It manages a list of child specs and applies a restart strategy.
pub struct Supervisor {
    /// The supervisor's own agent identity.
    pub id: AgentId,
    /// Restart strategy.
    pub strategy: RestartStrategy,
    /// Children being supervised, in order.
    children: Vec<ChildSpec>,
    /// Maximum restarts allowed within the time window.
    max_restarts: u32,
    /// Time window (in ticks) for counting restarts.
    time_window: u32,
    /// Recent restart timestamps for rate limiting.
    restart_history: Vec<RestartRecord>,
}

impl Supervisor {
    /// Create a new supervisor.
    pub fn new(id: AgentId, strategy: RestartStrategy) -> Self {
        Supervisor {
            id,
            strategy,
            children: Vec::new(),
            max_restarts: 3,
            time_window: 5000,
            restart_history: Vec::new(),
        }
    }

    /// Set the max restarts within a time window.
    pub fn with_limits(mut self, max_restarts: u32, time_window: u32) -> Self {
        self.max_restarts = max_restarts;
        self.time_window = time_window;
        self
    }

    /// Add a child to supervise.
    pub fn add_child(&mut self, spec: ChildSpec) {
        self.children.push(spec);
    }

    /// Number of supervised children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Get a child spec by agent ID.
    pub fn get_child(&self, agent_id: AgentId) -> Option<&ChildSpec> {
        self.children.iter().find(|c| c.agent_id == agent_id)
    }

    /// Handle a child failure. Returns the list of agents to restart,
    /// or None if the supervisor itself should fail (too many restarts).
    pub fn handle_failure(
        &mut self,
        failed_agent: AgentId,
        reason: FailureReason,
        now: Tick,
    ) -> Option<Vec<AgentId>> {
        // Find the failed child
        let child_idx = self.children.iter()
            .position(|c| c.agent_id == failed_agent)?;

        // Check if the child should be restarted
        if !self.children[child_idx].should_restart(reason) {
            return Some(Vec::new());
        }

        // Check restart rate limit
        self.prune_old_restarts(now);
        if self.restart_history.len() as u32 >= self.max_restarts {
            // Too many restarts — supervisor itself should fail
            return None;
        }

        // Record this restart
        self.restart_history.push(RestartRecord { tick: now });
        self.children[child_idx].record_restart();

        // Apply strategy
        let to_restart = match self.strategy {
            RestartStrategy::OneForOne => {
                vec![failed_agent]
            }
            RestartStrategy::OneForAll => {
                self.children.iter().map(|c| c.agent_id).collect()
            }
            RestartStrategy::RestForOne => {
                self.children[child_idx..]
                    .iter()
                    .map(|c| c.agent_id)
                    .collect()
            }
        };

        Some(to_restart)
    }

    /// Prune restart records older than the time window.
    fn prune_old_restarts(&mut self, now: Tick) {
        let cutoff = now.raw().saturating_sub(self.time_window);
        self.restart_history.retain(|r| r.tick.raw() >= cutoff);
    }

    /// Has this supervisor exceeded its restart limit?
    pub fn is_overwhelmed(&self, now: Tick) -> bool {
        let cutoff = now.raw().saturating_sub(self.time_window);
        let recent = self.restart_history.iter()
            .filter(|r| r.tick.raw() >= cutoff)
            .count();
        recent as u32 >= self.max_restarts
    }

    /// Total restarts across all children.
    pub fn total_restarts(&self) -> u32 {
        self.children.iter().map(|c| c.restarts).sum()
    }

    /// Get children as a slice.
    pub fn children(&self) -> &[ChildSpec] {
        &self.children
    }

    /// Get max restarts setting.
    pub fn max_restarts(&self) -> u32 {
        self.max_restarts
    }

    /// Get time window (in ticks).
    pub fn time_window(&self) -> u32 {
        self.time_window
    }
}

impl fmt::Debug for Supervisor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Supervisor")
            .field("id", &self.id)
            .field("strategy", &self.strategy)
            .field("children", &self.children.len())
            .field("max_restarts", &self.max_restarts)
            .field("time_window", &self.time_window)
            .field("total_restarts", &self.total_restarts())
            .finish()
    }
}

// ─── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_for_one_restarts_only_failed() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::OneForOne);
        sup.add_child(ChildSpec::permanent(AgentId::new(1)));
        sup.add_child(ChildSpec::permanent(AgentId::new(2)));
        sup.add_child(ChildSpec::permanent(AgentId::new(3)));

        let result = sup.handle_failure(
            AgentId::new(2),
            FailureReason::Crash,
            Tick::new(0, 100),
        );
        assert_eq!(result, Some(vec![AgentId::new(2)]));
    }

    #[test]
    fn one_for_all_restarts_everyone() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::OneForAll);
        sup.add_child(ChildSpec::permanent(AgentId::new(1)));
        sup.add_child(ChildSpec::permanent(AgentId::new(2)));
        sup.add_child(ChildSpec::permanent(AgentId::new(3)));

        let result = sup.handle_failure(
            AgentId::new(2),
            FailureReason::Crash,
            Tick::new(0, 100),
        );
        let agents = result.unwrap();
        assert_eq!(agents.len(), 3);
    }

    #[test]
    fn rest_for_one_restarts_from_failed_onward() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::RestForOne);
        sup.add_child(ChildSpec::permanent(AgentId::new(1)));
        sup.add_child(ChildSpec::permanent(AgentId::new(2)));
        sup.add_child(ChildSpec::permanent(AgentId::new(3)));

        let result = sup.handle_failure(
            AgentId::new(2),
            FailureReason::Crash,
            Tick::new(0, 100),
        );
        let agents = result.unwrap();
        assert_eq!(agents, vec![AgentId::new(2), AgentId::new(3)]);
    }

    #[test]
    fn temporary_child_never_restarted() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::OneForOne);
        sup.add_child(ChildSpec::temporary(AgentId::new(1)));

        let result = sup.handle_failure(
            AgentId::new(1),
            FailureReason::Crash,
            Tick::new(0, 100),
        );
        assert_eq!(result, Some(vec![]));
    }

    #[test]
    fn transient_child_restarted_on_crash() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::OneForOne);
        sup.add_child(ChildSpec::transient(AgentId::new(1)));

        let crash = sup.handle_failure(
            AgentId::new(1),
            FailureReason::Crash,
            Tick::new(0, 100),
        );
        assert_eq!(crash, Some(vec![AgentId::new(1)]));

        let shutdown = sup.handle_failure(
            AgentId::new(1),
            FailureReason::Shutdown,
            Tick::new(0, 200),
        );
        assert_eq!(shutdown, Some(vec![]));
    }

    #[test]
    fn max_restarts_causes_supervisor_failure() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::OneForOne)
            .with_limits(2, 10000);
        sup.add_child(ChildSpec::permanent(AgentId::new(1)));

        // First restart: OK
        let r1 = sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 100));
        assert!(r1.is_some());

        // Second restart: OK (at limit)
        let r2 = sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 200));
        assert!(r2.is_some());

        // Third restart within window: supervisor fails
        let r3 = sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 300));
        assert!(r3.is_none());
    }

    #[test]
    fn old_restarts_expire_from_window() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::OneForOne)
            .with_limits(2, 1000);
        sup.add_child(ChildSpec::permanent(AgentId::new(1)));

        // Two restarts early
        sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 100));
        sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 200));

        // Later, outside the window — old restarts expired
        let r = sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 5000));
        assert!(r.is_some());
    }

    #[test]
    fn restart_count_tracked_per_child() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::OneForOne)
            .with_limits(10, 50000);
        sup.add_child(ChildSpec::permanent(AgentId::new(1)));
        sup.add_child(ChildSpec::permanent(AgentId::new(2)));

        sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 100));
        sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 200));
        sup.handle_failure(AgentId::new(2), FailureReason::Crash, Tick::new(0, 300));

        assert_eq!(sup.get_child(AgentId::new(1)).unwrap().restart_count(), 2);
        assert_eq!(sup.get_child(AgentId::new(2)).unwrap().restart_count(), 1);
        assert_eq!(sup.total_restarts(), 3);
    }

    #[test]
    fn is_overwhelmed_reflects_window() {
        let mut sup = Supervisor::new(AgentId::new(0), RestartStrategy::OneForOne)
            .with_limits(2, 1000);
        sup.add_child(ChildSpec::permanent(AgentId::new(1)));

        assert!(!sup.is_overwhelmed(Tick::new(0, 0)));

        sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 100));
        sup.handle_failure(AgentId::new(1), FailureReason::Crash, Tick::new(0, 200));

        assert!(sup.is_overwhelmed(Tick::new(0, 500)));
        // After the window passes, no longer overwhelmed
        assert!(!sup.is_overwhelmed(Tick::new(0, 5000)));
    }
}
