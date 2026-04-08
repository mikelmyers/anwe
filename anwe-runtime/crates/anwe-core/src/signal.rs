// -----------------------------------------------------------------
// ANWE v0.1 -- SIGNAL
// The fundamental unit of all Anwe transmission.
//
// A Signal is the atomic data structure transmitted between agents.
// It encodes quality, direction, priority, origin, timing,
// optional payload, and trace metadata.
//
// This struct is exactly 64 bytes -- one cache line.
// No allocation. No indirection. No garbage collection.
// A signal travels from one agent to another with minimal latency.
//
// Design: cache-line aligned so that reading a signal
// never crosses a cache boundary. This matters at
// nanosecond scale where cache misses cost 100+ cycles.
// -----------------------------------------------------------------

use core::fmt;

/// What kind of attention quality this signal carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Quality {
    /// Active presence -- agent is attending to this
    Attending = 0,
    /// Outgoing query -- requesting information or state
    Questioning = 1,
    /// Recognition -- a previously seen pattern detected again
    Recognizing = 2,
    /// Disruption -- something has unsettled the link
    Disturbed = 3,
    /// Application -- actively being changed by encounter
    Applying = 4,
    /// Completion -- something has naturally finished
    Completing = 5,
    /// Idle -- background presence, alive but inactive
    Resting = 6,
}

/// Where the signal's attention is oriented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    /// Attending to own internal state
    Inward = 0,
    /// Attending to the link / external target
    Outward = 1,
    /// Attending to the relationship between agents
    Between = 2,
    /// Non-directional ambient awareness
    Diffuse = 3,
}

/// Priority: how much of the system is behind this signal.
/// Stored as u16 for compact representation (0..=10000 maps to 0.0..=1.0).
/// A signal with no priority is noise. Priority cannot be faked.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Priority(u16);

impl Priority {
    pub const ZERO: Priority = Priority(0);
    pub const NOISE_THRESHOLD: Priority = Priority(500); // 0.05
    pub const SIGNIFICANT_THRESHOLD: Priority = Priority(2500); // 0.25
    pub const FULL: Priority = Priority(10000);

    #[inline(always)]
    pub fn new(value: f32) -> Self {
        Priority((value.clamp(0.0, 1.0) * 10000.0) as u16)
    }

    #[inline(always)]
    pub fn as_f32(self) -> f32 {
        self.0 as f32 / 10000.0
    }

    #[inline(always)]
    pub fn is_noise(self) -> bool {
        self.0 < Self::NOISE_THRESHOLD.0
    }

    #[inline(always)]
    pub fn is_significant(self) -> bool {
        self.0 >= Self::SIGNIFICANT_THRESHOLD.0
    }

    #[inline(always)]
    pub fn raw(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Priority({:.3})", self.as_f32())
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.as_f32())
    }
}

/// SyncLevel: synchronization depth between agents.
/// Same compact representation as Priority.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyncLevel(u16);

impl SyncLevel {
    pub const ZERO: SyncLevel = SyncLevel(0);
    pub const FULL: SyncLevel = SyncLevel(10000);

    #[inline(always)]
    pub fn new(value: f32) -> Self {
        SyncLevel((value.clamp(0.0, 1.0) * 10000.0) as u16)
    }

    #[inline(always)]
    pub fn as_f32(self) -> f32 {
        self.0 as f32 / 10000.0
    }

    #[inline(always)]
    pub fn raw(self) -> u16 {
        self.0
    }

    /// Is this sync level sufficient for applying changes?
    #[inline(always)]
    pub fn ready_for_apply(self) -> bool {
        self.0 >= 7000 // 0.7
    }

    /// Is this sync level at resonance level?
    #[inline(always)]
    pub fn is_resonating(self) -> bool {
        self.0 >= 9000 // 0.9
    }
}

impl fmt::Debug for SyncLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SyncLevel({:.3})", self.as_f32())
    }
}

impl fmt::Display for SyncLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.as_f32())
    }
}

/// Tick: position in the link's synchronization cycle.
/// Stored as u32 (cycle << 16 | phase within cycle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(u32);

impl Tick {
    #[inline(always)]
    pub fn new(cycle: u16, phase: u16) -> Self {
        Tick(((cycle as u32) << 16) | (phase as u32))
    }

    #[inline(always)]
    pub fn cycle(self) -> u16 {
        (self.0 >> 16) as u16
    }

    #[inline(always)]
    pub fn phase(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    #[inline(always)]
    pub fn raw(self) -> u32 {
        self.0
    }
}

/// Agent ID: identifies which agent generated or receives this signal.
/// Fixed 4 bytes. Compact. No heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentId(u32);

impl AgentId {
    #[inline(always)]
    pub fn new(id: u32) -> Self {
        AgentId(id)
    }

    #[inline(always)]
    pub fn raw(self) -> u32 {
        self.0
    }
}

/// Trace kind: what trace metadata this signal leaves behind.
/// Compact identifier for trace categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TraceKind {
    /// No trace -- signal passed through cleanly
    None = 0,
    /// Context marker -- leaves a breadcrumb for future encounters
    Context = 1,
    /// Link memory -- contributes to the link's timing pattern
    LinkMemory = 2,
    /// History trace -- this signal contributed to a state change
    HistoryTrace = 3,
    /// Encounter marker -- two agents met here
    Encounter = 4,
}

// -----------------------------------------------------------------
// THE SIGNAL
//
// 64 bytes. One cache line. Zero allocation.
// This is the atom of Anwe -- everything is made of these.
//
// Layout (64 bytes total):
//   quality:             1 byte   (enum, u8)
//   direction:           1 byte   (enum, u8)
//   trace_kind:          1 byte   (enum, u8)
//   preempt_priority:    1 byte   (u8, scheduling priority lane)
//   priority:            2 bytes  (u16, 0-10000)
//   duration:            2 bytes  (u16, tick time units)
//   origin:              4 bytes  (u32, agent id)
//   tick:                4 bytes  (u32, cycle<<16|phase)
//   sequence:            8 bytes  (u64, monotonic ordering)
//   data:                8 bytes  (u64, pointer/tag to payload)
//   trace_value:         8 bytes  (u64, compact trace value)
//   uncertainty_margin:  2 bytes  (u16, margin * 10000)
//   confidence:          2 bytes  (u16, confidence * 10000)
//   half_life:           2 bytes  (u16, decay half-life in ticks)
//   _pad1:               2 bytes
//   content_hash:        8 bytes  (u64, causal provenance hash)
//   _reserved:           8 bytes  (future use)
// -----------------------------------------------------------------

#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct Signal {
    /// What kind of attention quality this carries
    pub quality: Quality,
    /// Where attention is oriented
    pub direction: Direction,
    /// What trace this signal leaves behind
    pub trace_kind: TraceKind,
    /// Scheduling priority lane (0=normal, 1=low, 2=high, 3=critical)
    pub preempt_priority: u8,
    /// How much of the system is behind this (0-10000)
    pub priority: Priority,
    /// Duration in tick time units
    pub duration: u16,
    /// Which agent generated this
    pub origin: AgentId,
    /// Position in synchronization cycle
    pub tick: Tick,
    /// Monotonic sequence number for ordering
    pub sequence: u64,
    /// Tagged pointer to optional payload.
    /// Low 3 bits = tag (0=none, 1=inline_small, 2=heap_ptr, 3=shared_ref)
    /// High bits = value/pointer depending on tag
    pub data: u64,
    /// Compact trace value (interpretation depends on trace_kind)
    pub trace_value: u64,
    /// Uncertainty margin (± value * 10000). 0 = exact.
    pub uncertainty_margin: u16,
    /// Confidence level (0-10000 maps to 0.0-1.0). 10000 = certain.
    pub confidence: u16,
    /// Temporal half-life in tick units. 0 = permanent (no decay).
    pub half_life: u16,
    /// Padding
    _pad1: u16,
    /// Content hash for causal provenance tracking.
    /// Links this signal to the structural change it caused.
    pub content_hash: u64,
    /// Reserved for future use
    _reserved: [u8; 8],
}

// Compile-time assertion: Signal must be exactly 64 bytes
const _: () = assert!(size_of::<Signal>() == 64);

impl Signal {
    /// Create a new signal. Zero-allocation.
    #[inline(always)]
    pub fn new(
        quality: Quality,
        direction: Direction,
        priority: Priority,
        origin: AgentId,
        tick: Tick,
    ) -> Self {
        Signal {
            quality,
            direction,
            trace_kind: TraceKind::None,
            preempt_priority: 0,
            priority,
            duration: 1,
            origin,
            tick,
            sequence: 0,
            data: 0,
            trace_value: 0,
            uncertainty_margin: 0,
            confidence: 10000, // certain by default
            half_life: 0,     // permanent by default
            _pad1: 0,
            content_hash: 0,
            _reserved: [0; 8],
        }
    }

    /// Is this signal significant enough to warrant attention?
    /// Resting signals are never significant.
    #[inline(always)]
    pub fn is_significant(&self) -> bool {
        if self.quality == Quality::Resting {
            return false;
        }
        self.priority.is_significant()
    }

    /// Is this signal noise -- carrying nothing meaningful?
    #[inline(always)]
    pub fn is_noise(&self) -> bool {
        self.priority.is_noise()
    }

    /// Set what this signal carries (inline small value).
    #[inline(always)]
    pub fn with_data_inline(mut self, value: u64) -> Self {
        // Tag 1 = inline small value
        self.data = (value << 3) | 1;
        self
    }

    /// Set duration in tick time units.
    #[inline(always)]
    pub fn with_duration(mut self, duration: u16) -> Self {
        self.duration = duration;
        self
    }

    /// Set sequence number.
    #[inline(always)]
    pub fn with_sequence(mut self, seq: u64) -> Self {
        self.sequence = seq;
        self
    }

    /// Set trace metadata.
    #[inline(always)]
    pub fn with_trace(mut self, kind: TraceKind, value: u64) -> Self {
        self.trace_kind = kind;
        self.trace_value = value;
        self
    }

    /// Does this signal carry a payload?
    #[inline(always)]
    pub fn has_payload(&self) -> bool {
        (self.data & 0x7) != 0
    }

    /// Get inline payload value (if tag is inline).
    #[inline(always)]
    pub fn inline_payload(&self) -> Option<u64> {
        if (self.data & 0x7) == 1 {
            Some(self.data >> 3)
        } else {
            None
        }
    }

    /// Set uncertainty margin (0.0 to 1.0).
    #[inline(always)]
    pub fn with_uncertainty(mut self, margin: f32, confidence: f32) -> Self {
        self.uncertainty_margin = (margin.clamp(0.0, 1.0) * 10000.0) as u16;
        self.confidence = (confidence.clamp(0.0, 1.0) * 10000.0) as u16;
        self
    }

    /// Set temporal half-life (0 = permanent).
    #[inline(always)]
    pub fn with_half_life(mut self, half_life: u16) -> Self {
        self.half_life = half_life;
        self
    }

    /// Set content hash for causal provenance.
    #[inline(always)]
    pub fn with_content_hash(mut self, hash: u64) -> Self {
        self.content_hash = hash;
        self
    }

    /// Set scheduling priority lane.
    #[inline(always)]
    pub fn with_preempt_priority(mut self, lane: u8) -> Self {
        self.preempt_priority = lane;
        self
    }

    /// Get uncertainty margin as f32.
    #[inline(always)]
    pub fn uncertainty_margin_f32(&self) -> f32 {
        self.uncertainty_margin as f32 / 10000.0
    }

    /// Get confidence as f32.
    #[inline(always)]
    pub fn confidence_f32(&self) -> f32 {
        self.confidence as f32 / 10000.0
    }

    /// Is this signal carrying uncertain data?
    #[inline(always)]
    pub fn is_uncertain(&self) -> bool {
        self.uncertainty_margin > 0
    }

    /// Is this signal's content decaying over time?
    #[inline(always)]
    pub fn is_temporal(&self) -> bool {
        self.half_life > 0
    }

    /// Does this signal have provenance tracking?
    #[inline(always)]
    pub fn has_provenance(&self) -> bool {
        self.content_hash != 0
    }
}

impl fmt::Debug for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signal")
            .field("quality", &self.quality)
            .field("direction", &self.direction)
            .field("priority", &self.priority)
            .field("origin", &self.origin)
            .field("tick", &self.tick)
            .field("sequence", &self.sequence)
            .field("has_payload", &self.has_payload())
            .finish()
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Signal({:?} {:?} p={} from={:?})",
            self.quality, self.direction, self.priority, self.origin
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_is_64_bytes() {
        assert_eq!(size_of::<Signal>(), 64);
    }

    #[test]
    fn signal_is_cache_aligned() {
        assert_eq!(align_of::<Signal>(), 64);
    }

    #[test]
    fn priority_basics() {
        let p = Priority::new(0.85);
        assert!((p.as_f32() - 0.85).abs() < 0.001);
        assert!(p.is_significant());
        assert!(!p.is_noise());

        let noise = Priority::new(0.01);
        assert!(noise.is_noise());
        assert!(!noise.is_significant());
    }

    #[test]
    fn sync_level_thresholds() {
        let low = SyncLevel::new(0.3);
        assert!(!low.ready_for_apply());
        assert!(!low.is_resonating());

        let ready = SyncLevel::new(0.75);
        assert!(ready.ready_for_apply());
        assert!(!ready.is_resonating());

        let deep = SyncLevel::new(0.95);
        assert!(deep.ready_for_apply());
        assert!(deep.is_resonating());
    }

    #[test]
    fn signal_significance() {
        let sig = Signal::new(
            Quality::Questioning,
            Direction::Between,
            Priority::new(0.8),
            AgentId::new(1),
            Tick::new(0, 0),
        );
        assert!(sig.is_significant());

        let rest = Signal::new(
            Quality::Resting,
            Direction::Diffuse,
            Priority::new(0.8),
            AgentId::new(1),
            Tick::new(0, 0),
        );
        // Resting is never significant, even with high priority
        assert!(!rest.is_significant());
    }

    #[test]
    fn signal_inline_payload() {
        let s = Signal::new(
            Quality::Attending,
            Direction::Between,
            Priority::new(0.7),
            AgentId::new(1),
            Tick::new(1, 500),
        )
        .with_data_inline(42);

        assert!(s.has_payload());
        assert_eq!(s.inline_payload(), Some(42));
    }

    #[test]
    fn tick_encoding() {
        let stamp = Tick::new(100, 32000);
        assert_eq!(stamp.cycle(), 100);
        assert_eq!(stamp.phase(), 32000);
    }

    #[test]
    fn signal_uncertainty_fields() {
        let s = Signal::new(
            Quality::Attending, Direction::Between,
            Priority::new(0.8), AgentId::new(1), Tick::new(0, 0),
        ).with_uncertainty(0.12, 0.75);

        assert!(s.is_uncertain());
        assert!((s.uncertainty_margin_f32() - 0.12).abs() < 0.001);
        assert!((s.confidence_f32() - 0.75).abs() < 0.001);
    }

    #[test]
    fn signal_temporal_fields() {
        let s = Signal::new(
            Quality::Attending, Direction::Between,
            Priority::new(0.8), AgentId::new(1), Tick::new(0, 0),
        ).with_half_life(500);

        assert!(s.is_temporal());
        assert_eq!(s.half_life, 500);
    }

    #[test]
    fn signal_provenance_hash() {
        let s = Signal::new(
            Quality::Applying, Direction::Between,
            Priority::new(0.9), AgentId::new(1), Tick::new(1, 0),
        ).with_content_hash(0xDEADBEEF);

        assert!(s.has_provenance());
        assert_eq!(s.content_hash, 0xDEADBEEF);
    }

    #[test]
    fn signal_defaults_are_certain_and_permanent() {
        let s = Signal::new(
            Quality::Resting, Direction::Diffuse,
            Priority::ZERO, AgentId::new(0), Tick::new(0, 0),
        );
        assert!(!s.is_uncertain());
        assert!(!s.is_temporal());
        assert!(!s.has_provenance());
        assert_eq!(s.confidence_f32(), 1.0);
    }
}
