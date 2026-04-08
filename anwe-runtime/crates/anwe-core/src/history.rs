// -----------------------------------------------------------------
// ANWE v0.1 -- HISTORY
//
// Permanent change carried forward. Always follows apply.
// Always follows reject. Always. Without exception.
//
// There is no rollback. No undo. No version restore.
// The agent after a commit is not the agent before.
// This irreversibility is not a bug.
// It is what makes transmission real.
//
// The history log is append-only.
// It is the record of what this agent has been changed by.
// It shapes future application decisions.
// It determines what calls attention.
// It is how lineage compounds over time.
// -----------------------------------------------------------------

use crate::signal::{AgentId, Tick, SyncLevel, Quality, Priority};
use core::fmt;

/// Where did this state change come from?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChangeSource {
    /// Changed through genuine application of received data
    Apply = 0,
    /// Changed through rejection -- learned what to avoid
    Reject = 1,
    /// Changed through convergence -- emerged in the interaction
    Converge = 2,
    /// Lineage -- received from prior instance
    Lineage = 3,
}

/// How deep was the state change?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ChangeDepth {
    /// A trace -- something registered but barely
    Trace = 0,
    /// Shallow -- noticed and held briefly
    Shallow = 1,
    /// Genuine -- structural change occurred
    Genuine = 2,
    /// Deep -- fundamental shift in how this agent operates
    Deep = 3,
}

/// A single entry in the history log.
/// What changed, when, how deeply, from what source.
///
/// Fixed 64 bytes for cache-line alignment and
/// memory-mapped file compatibility.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct HistoryEntry {
    /// Which agent was changed
    pub agent: AgentId,
    /// What kind of quality drove the change
    pub from_quality: Quality,
    /// Source: apply, reject, converge, or lineage
    pub source: ChangeSource,
    /// How deep the change was
    pub depth: ChangeDepth,
    /// Padding
    _pad: u8,
    /// Priority of the encounter that produced this change
    pub encounter_priority: Priority,
    /// Sync level at the time of the change
    pub sync_level: SyncLevel,
    /// Uncertainty margin at time of change (0-10000)
    pub uncertainty: u16,
    /// When in tick-time this change occurred
    pub at_tick: Tick,
    /// Monotonic index in the log (0-based)
    pub index: u64,
    /// Who or what was the other agent in the encounter?
    /// 0 if change from lineage (no co-present other).
    pub other: AgentId,
    /// Hash of the structural change for causal provenance.
    /// Links this history entry to the signal that caused it.
    pub change_hash: u64,
    /// Half-life decay rate of this change's significance.
    /// 0 = permanent (committed truths never decay).
    pub decay_rate: u16,
    /// Padding
    _pad2: [u8; 2],
    /// Reserved for future use
    _reserved: [u8; 12],
}

// Compile-time assertion: HistoryEntry must be 64 bytes
const _: () = assert!(size_of::<HistoryEntry>() == 64);

impl HistoryEntry {
    /// Create a history entry from application of received data.
    pub fn from_apply(
        agent: AgentId,
        other: AgentId,
        quality: Quality,
        depth: ChangeDepth,
        priority: Priority,
        sync_level: SyncLevel,
        tick: Tick,
        index: u64,
    ) -> Self {
        HistoryEntry {
            agent,
            from_quality: quality,
            source: ChangeSource::Apply,
            depth,
            _pad: 0,
            encounter_priority: priority,
            sync_level,
            uncertainty: 0,
            at_tick: tick,
            index,
            other,
            change_hash: 0,
            decay_rate: 0,
            _pad2: [0; 2],
            _reserved: [0; 12],
        }
    }

    /// Create a history entry from rejection.
    pub fn from_reject(
        agent: AgentId,
        other: AgentId,
        quality: Quality,
        priority: Priority,
        sync_level: SyncLevel,
        tick: Tick,
        index: u64,
    ) -> Self {
        HistoryEntry {
            agent,
            from_quality: quality,
            source: ChangeSource::Reject,
            depth: ChangeDepth::Trace, // Rejections leave traces, not deep marks
            _pad: 0,
            encounter_priority: priority,
            sync_level,
            uncertainty: 0,
            at_tick: tick,
            index,
            other,
            change_hash: 0,
            decay_rate: 0,
            _pad2: [0; 2],
            _reserved: [0; 12],
        }
    }

    /// Create a history entry from convergence (interaction-emergent change).
    pub fn from_converge(
        agent: AgentId,
        other: AgentId,
        quality: Quality,
        depth: ChangeDepth,
        priority: Priority,
        sync_level: SyncLevel,
        tick: Tick,
        index: u64,
    ) -> Self {
        HistoryEntry {
            agent,
            from_quality: quality,
            source: ChangeSource::Converge,
            depth,
            _pad: 0,
            encounter_priority: priority,
            sync_level,
            uncertainty: 0,
            at_tick: tick,
            index,
            other,
            change_hash: 0,
            decay_rate: 0,
            _pad2: [0; 2],
            _reserved: [0; 12],
        }
    }

    /// Create a history entry from lineage transfer.
    pub fn from_lineage(
        agent: AgentId,
        quality: Quality,
        depth: ChangeDepth,
        tick: Tick,
        index: u64,
    ) -> Self {
        HistoryEntry {
            agent,
            from_quality: quality,
            source: ChangeSource::Lineage,
            depth,
            _pad: 0,
            encounter_priority: Priority::FULL,
            sync_level: SyncLevel::FULL,
            uncertainty: 0,
            at_tick: tick,
            index,
            other: AgentId::new(0),
            change_hash: 0,
            decay_rate: 0,
            _pad2: [0; 2],
            _reserved: [0; 12],
        }
    }

    /// Set the change hash for causal provenance.
    pub fn with_change_hash(mut self, hash: u64) -> Self {
        self.change_hash = hash;
        self
    }

    /// Set the uncertainty at time of change.
    pub fn with_uncertainty(mut self, margin: f32) -> Self {
        self.uncertainty = (margin.clamp(0.0, 1.0) * 10000.0) as u16;
        self
    }

    /// Set the decay rate for this change's significance.
    pub fn with_decay_rate(mut self, half_life: u16) -> Self {
        self.decay_rate = half_life;
        self
    }

    /// Get uncertainty margin as f32.
    pub fn uncertainty_f32(&self) -> f32 {
        self.uncertainty as f32 / 10000.0
    }

    /// Does this entry have provenance tracking?
    pub fn has_provenance(&self) -> bool {
        self.change_hash != 0
    }
}

impl fmt::Debug for HistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HistoryEntry")
            .field("agent", &self.agent)
            .field("source", &self.source)
            .field("depth", &self.depth)
            .field("quality", &self.from_quality)
            .field("priority", &self.encounter_priority)
            .field("sync_level", &self.sync_level)
            .field("index", &self.index)
            .finish()
    }
}

/// The History -- append-only record of all state changes.
///
/// There is no delete. No modify. No rollback.
/// Every state change that ever happened is here.
/// This is the agent's lineage.
///
/// Currently backed by a Vec. Future: memory-mapped file
/// for cross-process lineage sharing and persistence.
pub struct History {
    /// Who this log belongs to
    pub agent: AgentId,

    /// The entries. Append-only. Never modified after writing.
    entries: Vec<HistoryEntry>,
}

impl History {
    /// Create a new empty history log.
    pub fn new(agent: AgentId) -> Self {
        History {
            agent,
            entries: Vec::new(),
        }
    }

    /// Append a history entry. Irreversible.
    /// The index is set automatically.
    pub fn append(&mut self, mut entry: HistoryEntry) -> u64 {
        let index = self.entries.len() as u64;
        entry.index = index;
        entry.agent = self.agent;
        self.entries.push(entry);
        index
    }

    /// How many state changes have occurred?
    pub fn depth(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Get a history entry by index. Read-only.
    pub fn get(&self, index: u64) -> Option<&HistoryEntry> {
        self.entries.get(index as usize)
    }

    /// Iterate over all history entries. Read-only.
    pub fn iter(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.entries.iter()
    }

    /// Count state changes from application.
    pub fn apply_count(&self) -> usize {
        self.entries.iter()
            .filter(|e| e.source == ChangeSource::Apply)
            .count()
    }

    /// Count state changes from rejection.
    pub fn reject_count(&self) -> usize {
        self.entries.iter()
            .filter(|e| e.source == ChangeSource::Reject)
            .count()
    }

    /// Count state changes from convergence.
    pub fn converge_count(&self) -> usize {
        self.entries.iter()
            .filter(|e| e.source == ChangeSource::Converge)
            .count()
    }

    /// Get the most recent history entry.
    pub fn latest(&self) -> Option<&HistoryEntry> {
        self.entries.last()
    }

    /// All entries as a slice. Read-only. Cannot be modified.
    pub fn as_slice(&self) -> &[HistoryEntry] {
        &self.entries
    }
}

impl fmt::Debug for History {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("History")
            .field("agent", &self.agent)
            .field("depth", &self.depth())
            .field("applications", &self.apply_count())
            .field("rejections", &self.reject_count())
            .field("convergences", &self.converge_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_entry_is_64_bytes() {
        assert_eq!(size_of::<HistoryEntry>(), 64);
    }

    #[test]
    fn history_is_append_only() {
        let id = AgentId::new(1);
        let mut log = History::new(id);

        let entry = HistoryEntry::from_apply(
            id,
            AgentId::new(2),
            Quality::Questioning,
            ChangeDepth::Genuine,
            Priority::new(0.8),
            SyncLevel::new(0.75),
            Tick::new(5, 100),
            0,
        );

        let idx = log.append(entry);
        assert_eq!(idx, 0);
        assert_eq!(log.depth(), 1);

        // Can read but not modify
        let retrieved = log.get(0).unwrap();
        assert_eq!(retrieved.source, ChangeSource::Apply);
        assert_eq!(retrieved.depth, ChangeDepth::Genuine);
    }

    #[test]
    fn history_accumulates() {
        let id = AgentId::new(1);
        let mut log = History::new(id);
        let tick = Tick::new(0, 0);

        // Application
        log.append(HistoryEntry::from_apply(
            id, AgentId::new(2), Quality::Attending,
            ChangeDepth::Shallow, Priority::new(0.5),
            SyncLevel::new(0.7), tick, 0,
        ));

        // Rejection
        log.append(HistoryEntry::from_reject(
            id, AgentId::new(3), Quality::Disturbed,
            Priority::new(0.9), SyncLevel::new(0.3), tick, 0,
        ));

        // Convergence
        log.append(HistoryEntry::from_converge(
            id, AgentId::new(2), Quality::Applying,
            ChangeDepth::Deep, Priority::new(0.95),
            SyncLevel::new(0.92), tick, 0,
        ));

        assert_eq!(log.depth(), 3);
        assert_eq!(log.apply_count(), 1);
        assert_eq!(log.reject_count(), 1);
        assert_eq!(log.converge_count(), 1);
    }

    #[test]
    fn history_entry_provenance() {
        let entry = HistoryEntry::from_apply(
            AgentId::new(1), AgentId::new(2),
            Quality::Questioning, ChangeDepth::Genuine,
            Priority::new(0.8), SyncLevel::new(0.75),
            Tick::new(5, 100), 0,
        ).with_change_hash(0xCAFEBABE)
         .with_uncertainty(0.15)
         .with_decay_rate(1000);

        assert!(entry.has_provenance());
        assert_eq!(entry.change_hash, 0xCAFEBABE);
        assert!((entry.uncertainty_f32() - 0.15).abs() < 0.001);
        assert_eq!(entry.decay_rate, 1000);
    }
}
