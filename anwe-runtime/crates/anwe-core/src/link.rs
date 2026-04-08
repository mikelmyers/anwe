// -----------------------------------------------------------------
// ANWE v0.1 -- LINK
//
// The shared connection between agents.
// A link is the communication channel through which
// signals are transmitted and synchronization occurs.
//
// A link is opened between two or more agents.
// Everything within a link runs concurrently.
// Sequence only occurs through signal dependency.
//
// The link tracks sync level -- how deeply synchronized
// the agents are. Sync level builds through genuine
// synchronization cycles, not through assignment.
// -----------------------------------------------------------------

use core::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering};
use crate::signal::{AgentId, Tick, SyncLevel};

/// Maximum agents in a single link.
/// Convergence requires exactly 2, but links can hold more
/// for observation and transmission.
pub const MAX_LINK_AGENTS: usize = 8;

/// Link state -- where in its lifecycle this link is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LinkState {
    /// Link declared but agents not yet present
    Opening = 0,
    /// Agents present, not yet synchronized
    Present = 1,
    /// Synchronization has begun, sync level building
    Syncing = 2,
    /// Sufficient sync level for applying changes
    Synchronized = 3,
    /// Deep sync level -- resonance between agents
    Resonating = 4,
    /// Link naturally completing
    Completing = 5,
    /// Link closed -- connection finished
    Closed = 6,
}

/// Link ID: unique identifier for this connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinkId(u32);

impl LinkId {
    pub fn new(id: u32) -> Self {
        LinkId(id)
    }

    pub fn raw(self) -> u32 {
        self.0
    }
}

/// The Link -- shared connection between agents.
///
/// Uses atomics for sync level and tick tracking so that
/// multiple agent fibers can read/write simultaneously
/// without locks. This is how receiving, processing, and
/// transmitting happen at the same time.
pub struct Link {
    /// Unique identifier for this link
    pub id: LinkId,

    /// Agents present in this link.
    /// Fixed array -- no heap allocation during operation.
    agents: [AgentId; MAX_LINK_AGENTS],

    /// How many agents are currently present
    agent_count: AtomicU32,

    /// Current sync level (atomic u16, maps to 0.0-1.0).
    /// Updated by the sync primitive.
    /// Read by apply to determine readiness.
    sync_level: AtomicU16,

    /// Current tick cycle (atomic, monotonically increasing).
    tick_cycle: AtomicU32,

    /// Current tick phase within cycle (atomic u16).
    tick_phase: AtomicU16,

    /// Total signals transmitted through this link.
    signal_count: AtomicU64,

    /// Link state (atomic u32 for lock-free state transitions).
    state: AtomicU32,

    /// Peak sync level achieved in this link.
    /// Append-only: only increases, never decreases.
    peak_sync_level: AtomicU16,
}

impl Link {
    /// Open a new link. Creates the connection space between agents.
    pub fn open(id: LinkId) -> Self {
        Link {
            id,
            agents: [AgentId::new(0); MAX_LINK_AGENTS],
            agent_count: AtomicU32::new(0),
            sync_level: AtomicU16::new(0),
            tick_cycle: AtomicU32::new(0),
            tick_phase: AtomicU16::new(0),
            signal_count: AtomicU64::new(0),
            state: AtomicU32::new(LinkState::Opening as u32),
            peak_sync_level: AtomicU16::new(0),
        }
    }

    /// An agent enters the link.
    /// Returns true if the agent was admitted.
    pub fn enter(&mut self, agent: AgentId) -> bool {
        let count = self.agent_count.load(Ordering::Acquire) as usize;
        if count >= MAX_LINK_AGENTS {
            return false;
        }
        self.agents[count] = agent;
        self.agent_count.store((count + 1) as u32, Ordering::Release);

        // If we now have 2+ agents, link becomes present
        if count + 1 >= 2 {
            self.transition_to(LinkState::Present);
        }
        true
    }

    /// Get the current sync level.
    /// Lock-free read -- can be called from any fiber.
    #[inline(always)]
    pub fn sync_level(&self) -> SyncLevel {
        SyncLevel::new(self.sync_level.load(Ordering::Relaxed) as f32 / 10000.0)
    }

    /// Update sync level. Called by the sync primitive.
    /// Uses Relaxed ordering because sync level is monotonically
    /// observed -- small races are acceptable (like neurons).
    #[inline(always)]
    pub fn update_sync_level(&self, new_sync_level: SyncLevel) {
        let raw = new_sync_level.raw();
        self.sync_level.store(raw, Ordering::Relaxed);

        // Track peak (relaxed CAS loop -- races are fine)
        let mut peak = self.peak_sync_level.load(Ordering::Relaxed);
        while raw > peak {
            match self.peak_sync_level.compare_exchange_weak(
                peak,
                raw,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }

        // State transitions based on sync level
        if new_sync_level.is_resonating() {
            self.transition_to(LinkState::Resonating);
        } else if new_sync_level.ready_for_apply() {
            self.transition_to(LinkState::Synchronized);
        }
    }

    /// Advance the tick. Called by the sync primitive.
    #[inline(always)]
    pub fn advance_tick(&self, phase: u16) {
        self.tick_phase.store(phase, Ordering::Relaxed);
        if phase == 0 {
            self.tick_cycle.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get the current tick.
    #[inline(always)]
    pub fn tick(&self) -> Tick {
        Tick::new(
            self.tick_cycle.load(Ordering::Relaxed) as u16,
            self.tick_phase.load(Ordering::Relaxed),
        )
    }

    /// Record that a signal was transmitted through this link.
    #[inline(always)]
    pub fn record_signal(&self) -> u64 {
        self.signal_count.fetch_add(1, Ordering::Relaxed)
    }

    /// How many signals have been transmitted?
    pub fn signal_count(&self) -> u64 {
        self.signal_count.load(Ordering::Relaxed)
    }

    /// Get current state.
    pub fn state(&self) -> LinkState {
        match self.state.load(Ordering::Acquire) {
            0 => LinkState::Opening,
            1 => LinkState::Present,
            2 => LinkState::Syncing,
            3 => LinkState::Synchronized,
            4 => LinkState::Resonating,
            5 => LinkState::Completing,
            _ => LinkState::Closed,
        }
    }

    /// Is this link ready for applying changes?
    #[inline(always)]
    pub fn ready_for_apply(&self) -> bool {
        self.sync_level().ready_for_apply()
    }

    /// How many agents are present?
    #[inline(always)]
    pub fn agent_count(&self) -> usize {
        self.agent_count.load(Ordering::Relaxed) as usize
    }

    /// Does this link have enough agents for convergence?
    /// Convergence requires minimum two.
    #[inline(always)]
    pub fn can_converge(&self) -> bool {
        self.agent_count() >= 2
    }

    /// Get agent at index.
    pub fn agent(&self, index: usize) -> Option<AgentId> {
        let count = self.agent_count();
        if index < count {
            Some(self.agents[index])
        } else {
            None
        }
    }

    /// Peak sync level ever achieved.
    pub fn peak_sync_level(&self) -> SyncLevel {
        SyncLevel::new(self.peak_sync_level.load(Ordering::Relaxed) as f32 / 10000.0)
    }

    /// Transition to a new state. Only advances forward.
    fn transition_to(&self, new_state: LinkState) {
        let new = new_state as u32;
        let mut current = self.state.load(Ordering::Acquire);
        // Only advance forward (no going back except to Closed)
        while new > current || new_state == LinkState::Closed {
            match self.state.compare_exchange_weak(
                current,
                new,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(actual) => {
                    current = actual;
                    if current >= new && new_state != LinkState::Closed {
                        return; // Already past this state
                    }
                }
            }
        }
    }

    /// Begin synchronization between agents.
    pub fn begin_sync(&self) {
        self.transition_to(LinkState::Syncing);
    }

    /// Complete this link naturally.
    pub fn complete(&self) {
        self.transition_to(LinkState::Completing);
    }

    /// Close this link.
    pub fn close(&self) {
        self.transition_to(LinkState::Closed);
    }
}

impl core::fmt::Debug for Link {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Link")
            .field("id", &self.id)
            .field("agents", &self.agent_count())
            .field("sync_level", &self.sync_level())
            .field("state", &self.state())
            .field("tick", &self.tick())
            .field("signals", &self.signal_count.load(Ordering::Relaxed))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_lifecycle() {
        let mut link = Link::open(LinkId::new(1));
        assert_eq!(link.state(), LinkState::Opening);
        assert_eq!(link.agent_count(), 0);

        // First agent enters -- still opening (need 2)
        assert!(link.enter(AgentId::new(1)));
        assert_eq!(link.state(), LinkState::Opening);

        // Second agent enters -- now present
        assert!(link.enter(AgentId::new(2)));
        assert_eq!(link.state(), LinkState::Present);
        assert!(link.can_converge());
    }

    #[test]
    fn link_sync_level_builds() {
        let link = Link::open(LinkId::new(1));

        link.update_sync_level(SyncLevel::new(0.3));
        assert!(!link.ready_for_apply());

        link.update_sync_level(SyncLevel::new(0.75));
        assert!(link.ready_for_apply());
        assert_eq!(link.state(), LinkState::Synchronized);

        link.update_sync_level(SyncLevel::new(0.95));
        assert_eq!(link.state(), LinkState::Resonating);
    }

    #[test]
    fn link_tick_advances() {
        let link = Link::open(LinkId::new(1));
        link.advance_tick(100);
        let t = link.tick();
        assert_eq!(t.cycle(), 0);
        assert_eq!(t.phase(), 100);

        // New cycle
        link.advance_tick(0);
        let t = link.tick();
        assert_eq!(t.cycle(), 1);
        assert_eq!(t.phase(), 0);
    }
}
