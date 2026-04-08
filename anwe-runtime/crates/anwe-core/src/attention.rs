// -----------------------------------------------------------------
// ANWE v0.1 -- ATTENTION AS A RESOURCE
//
// There is a finite amount of processing the system can do.
// Attention is the mechanism for allocating it.
//
// When reasoning load is high, perception resolution drops.
// When a threat signal arrives, background processing pauses.
// When everything is calm, exploration gets more budget.
//
// This is not implemented in application logic.
// This is the scheduler's native resource model.
// The runtime itself knows about attention scarcity.
// -----------------------------------------------------------------

use core::fmt;
use core::sync::atomic::{AtomicU32, Ordering};

/// How much attention an agent currently has available.
///
/// Each agent gets a budget from the global pool.
/// Consuming attention (processing signals, applying changes)
/// draws down the budget. The budget refreshes over time.
///
/// When budget is exhausted, the agent's fibers are deprioritized —
/// not blocked, just given less scheduling weight.
#[derive(Debug)]
pub struct AttentionBudget {
    /// Total budget allocated to this agent (0.0 to 1.0).
    total: f32,
    /// How much has been consumed this cycle.
    consumed: f32,
    /// Priority boost from high-salience signals.
    /// Decays each cycle. Allows bursting above budget.
    boost: f32,
}

impl AttentionBudget {
    /// Create a new budget with a given total.
    pub fn new(total: f32) -> Self {
        AttentionBudget {
            total: total.clamp(0.0, 1.0),
            consumed: 0.0,
            boost: 0.0,
        }
    }

    /// Default budget for a newly created agent.
    pub fn default_budget() -> Self {
        Self::new(0.5)
    }

    /// How much budget remains?
    #[inline]
    pub fn remaining(&self) -> f32 {
        (self.total + self.boost - self.consumed).max(0.0)
    }

    /// Is the budget exhausted?
    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.remaining() <= 0.0
    }

    /// Consume some attention.
    /// Returns how much was actually consumed (may be less if near empty).
    pub fn consume(&mut self, amount: f32) -> f32 {
        let available = self.remaining();
        let actual = amount.min(available);
        self.consumed += actual;
        actual
    }

    /// Add a temporary boost (from high-priority signals).
    /// Allows the agent to exceed its normal budget briefly.
    pub fn boost(&mut self, amount: f32) {
        self.boost = (self.boost + amount).min(1.0);
    }

    /// Refresh the budget for a new cycle.
    /// Consumed resets. Boost decays.
    pub fn refresh(&mut self) {
        self.consumed = 0.0;
        self.boost *= 0.5; // Boost halves each cycle
    }

    /// Set the total budget (e.g., when the pool reallocates).
    pub fn set_total(&mut self, total: f32) {
        self.total = total.clamp(0.0, 1.0);
    }

    /// Get the total budget allocation.
    pub fn total(&self) -> f32 { self.total }

    /// Get the consumed amount this cycle.
    pub fn consumed(&self) -> f32 { self.consumed }

    /// Get the current boost amount.
    pub fn boost_amount(&self) -> f32 { self.boost }

    /// Restore from saved state (total, consumed, boost).
    pub fn restore(total: f32, consumed: f32, boost: f32) -> Self {
        AttentionBudget {
            total: total.clamp(0.0, 1.0),
            consumed: consumed.max(0.0),
            boost: boost.clamp(0.0, 1.0),
        }
    }

    /// What fraction of the budget has been used?
    pub fn utilization(&self) -> f32 {
        if self.total <= 0.0 {
            return 1.0;
        }
        (self.consumed / self.total).min(1.0)
    }
}

/// The global attention pool.
///
/// A fixed amount of processing capacity shared by all agents.
/// When a new agent joins, everyone's budget shrinks.
/// When load increases, budgets tighten.
/// When a critical signal arrives, budget shifts to the receiver.
///
/// This is the mechanism for "when reasoning is high,
/// perception drops." The pool is finite. Distribution
/// is the scheduler's job.
pub struct AttentionPool {
    /// Total system capacity.
    total_capacity: f32,
    /// Number of agents currently drawing from the pool.
    agent_count: AtomicU32,
    /// How much of the pool is currently allocated.
    allocated: AtomicU32, // stored as value * 10000
    /// Reserved capacity for critical signals.
    /// Always available, never allocated to normal processing.
    critical_reserve: f32,
}

impl AttentionPool {
    /// Create a new attention pool with the given total capacity.
    pub fn new(total_capacity: f32) -> Self {
        AttentionPool {
            total_capacity,
            agent_count: AtomicU32::new(0),
            allocated: AtomicU32::new(0),
            critical_reserve: total_capacity * 0.1, // 10% always reserved
        }
    }

    /// Default pool for a runtime.
    pub fn default_pool() -> Self {
        Self::new(1.0)
    }

    /// Register a new agent in the pool. Returns its initial budget.
    pub fn register_agent(&self) -> AttentionBudget {
        let count = self.agent_count.fetch_add(1, Ordering::Relaxed) + 1;
        let per_agent = self.per_agent_budget(count);
        AttentionBudget::new(per_agent)
    }

    /// Remove an agent from the pool.
    pub fn unregister_agent(&self) {
        self.agent_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// How much budget does each agent get?
    /// Even distribution minus critical reserve.
    pub fn per_agent_budget(&self, agent_count: u32) -> f32 {
        if agent_count == 0 {
            return 0.0;
        }
        let available = self.total_capacity - self.critical_reserve;
        (available / agent_count as f32).max(0.01)
    }

    /// Current number of agents.
    pub fn agent_count(&self) -> u32 {
        self.agent_count.load(Ordering::Relaxed)
    }

    /// Total system load (0.0 = idle, 1.0 = saturated).
    pub fn system_load(&self) -> f32 {
        let allocated = self.allocated.load(Ordering::Relaxed) as f32 / 10000.0;
        (allocated / self.total_capacity).min(1.0)
    }

    /// Record that attention was consumed.
    pub fn record_consumption(&self, amount: f32) {
        let raw = (amount * 10000.0) as u32;
        self.allocated.fetch_add(raw, Ordering::Relaxed);
    }

    /// Refresh the pool for a new cycle.
    pub fn refresh(&self) {
        self.allocated.store(0, Ordering::Relaxed);
    }

    /// Is the system under high load?
    pub fn is_saturated(&self) -> bool {
        self.system_load() > 0.85
    }

    /// Available capacity for critical signals (always positive).
    pub fn critical_capacity(&self) -> f32 {
        self.critical_reserve
    }
}

impl fmt::Debug for AttentionPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AttentionPool")
            .field("capacity", &self.total_capacity)
            .field("agents", &self.agent_count())
            .field("load", &self.system_load())
            .field("critical_reserve", &self.critical_reserve)
            .finish()
    }
}

// ─── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_starts_full() {
        let b = AttentionBudget::new(0.5);
        assert_eq!(b.remaining(), 0.5);
        assert!(!b.is_exhausted());
    }

    #[test]
    fn consuming_attention_reduces_budget() {
        let mut b = AttentionBudget::new(0.5);
        b.consume(0.3);
        assert!((b.remaining() - 0.2).abs() < 0.001);
    }

    #[test]
    fn budget_exhaustion() {
        let mut b = AttentionBudget::new(0.1);
        b.consume(0.1);
        assert!(b.is_exhausted());
    }

    #[test]
    fn cannot_consume_more_than_available() {
        let mut b = AttentionBudget::new(0.1);
        let consumed = b.consume(1.0);
        assert!((consumed - 0.1).abs() < 0.001);
    }

    #[test]
    fn boost_extends_budget() {
        let mut b = AttentionBudget::new(0.1);
        b.boost(0.5);
        assert!((b.remaining() - 0.6).abs() < 0.001);
    }

    #[test]
    fn refresh_resets_consumed_and_decays_boost() {
        let mut b = AttentionBudget::new(0.5);
        b.consume(0.3);
        b.boost(0.4);
        b.refresh();
        assert!((b.remaining() - 0.7).abs() < 0.01); // 0.5 + 0.2 boost
    }

    #[test]
    fn pool_distributes_evenly() {
        let pool = AttentionPool::new(1.0);
        let b1 = pool.register_agent();
        assert!(b1.remaining() > 0.4); // roughly 0.9/1 = 0.9
        let b2 = pool.register_agent();
        assert!(b2.remaining() > 0.2); // roughly 0.9/2 = 0.45
    }

    #[test]
    fn pool_tracks_load() {
        let pool = AttentionPool::new(1.0);
        assert_eq!(pool.system_load(), 0.0);
        pool.record_consumption(0.5);
        assert!((pool.system_load() - 0.5).abs() < 0.01);
    }

    #[test]
    fn pool_saturation() {
        let pool = AttentionPool::new(1.0);
        pool.record_consumption(0.9);
        assert!(pool.is_saturated());
    }

    #[test]
    fn critical_reserve_exists() {
        let pool = AttentionPool::new(1.0);
        assert!(pool.critical_capacity() > 0.0);
    }
}
