// -----------------------------------------------------------------
// ANWE v0.1 -- TEMPORAL TYPES
//
// Time as a first-class citizen.
// Not just timestamps. Decay functions. Half-lives. Expiration.
//
// A memory fact carries its own half-life.
// A belief decays at a rate proportional to how loosely it was held.
// A sync level fades if agents don't maintain contact.
//
// This is how a cognitive system forgets —
// not by deleting, but by letting salience decay
// until the signal falls below the noise threshold.
// -----------------------------------------------------------------

use crate::signal::{Tick, Priority};
use core::fmt;

/// A value that decays over time.
///
/// Like a memory — vivid when fresh, fading with time.
/// The decay follows exponential half-life:
///   value(t) = floor + (initial - floor) * 2^(-elapsed/half_life)
///
/// When half_life is 0, the value never decays (permanent).
#[derive(Clone, Copy, PartialEq)]
pub struct DecayingValue {
    /// The initial value when created.
    pub initial: f32,
    /// When this value was created (tick-time).
    pub created_at: Tick,
    /// How many ticks until the value halves.
    /// 0 = no decay (permanent, like a committed truth).
    pub half_life: u32,
    /// The minimum value after full decay.
    /// Even decayed memories leave a trace.
    pub floor: f32,
}

impl DecayingValue {
    /// Create a new decaying value.
    pub fn new(initial: f32, created_at: Tick, half_life: u32) -> Self {
        DecayingValue {
            initial,
            created_at,
            half_life,
            floor: 0.0,
        }
    }

    /// Create a permanent value (no decay).
    pub fn permanent(value: f32, created_at: Tick) -> Self {
        DecayingValue {
            initial: value,
            created_at,
            half_life: 0,
            floor: value,
        }
    }

    /// Create a decaying value with a floor.
    pub fn with_floor(mut self, floor: f32) -> Self {
        self.floor = floor;
        self
    }

    /// Evaluate this value at a given point in time.
    /// Returns the decayed value.
    pub fn value_at(&self, now: Tick) -> f32 {
        if self.half_life == 0 {
            return self.initial;
        }

        let elapsed = now.raw().saturating_sub(self.created_at.raw());
        if elapsed == 0 {
            return self.initial;
        }

        // Exponential decay: value = floor + (initial - floor) * 2^(-t/half_life)
        let ratio = elapsed as f64 / self.half_life as f64;
        let decay_factor = (-0.693147 * ratio).exp() as f32; // ln(2) ≈ 0.693147
        self.floor + (self.initial - self.floor) * decay_factor
    }

    /// Has this value decayed below a threshold?
    pub fn is_expired(&self, now: Tick, threshold: f32) -> bool {
        self.value_at(now) < threshold
    }

    /// Has this value decayed below the noise threshold?
    pub fn is_noise(&self, now: Tick) -> bool {
        self.is_expired(now, Priority::NOISE_THRESHOLD.as_f32())
    }

    /// How much of the original value remains? (0.0 to 1.0)
    pub fn retention(&self, now: Tick) -> f32 {
        if self.initial == 0.0 {
            return 0.0;
        }
        self.value_at(now) / self.initial
    }
}

impl fmt::Debug for DecayingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DecayingValue({:.3}, half_life={}, floor={:.3})",
            self.initial, self.half_life, self.floor)
    }
}

/// Compute a recency weight for a past event.
///
/// Returns a value between 0.0 (ancient) and 1.0 (just happened).
/// Uses exponential decay with the given rate.
///
/// This is how attention naturally favors recent events
/// without explicit priority assignment.
#[inline]
pub fn recency_weight(created_at: Tick, now: Tick, decay_rate: f32) -> f32 {
    let elapsed = now.raw().saturating_sub(created_at.raw()) as f32;
    (-decay_rate * elapsed).exp()
}

/// A temporal wrapper that gives any value a time dimension.
///
/// The value itself doesn't change, but its *relevance* decays.
/// This is the difference between "I know X" and "I recently learned X."
#[derive(Debug, Clone, Copy)]
pub struct Temporal<T: Copy> {
    /// The wrapped value.
    pub value: T,
    /// When this value was created.
    pub created_at: Tick,
    /// Half-life in tick units. 0 = permanent.
    pub half_life: u16,
    /// Relevance at creation time (usually 1.0).
    pub initial_relevance: f32,
}

impl<T: Copy> Temporal<T> {
    /// Create a new temporal value.
    pub fn new(value: T, created_at: Tick, half_life: u16) -> Self {
        Temporal {
            value,
            created_at,
            half_life,
            initial_relevance: 1.0,
        }
    }

    /// Create a permanent temporal value (never decays).
    pub fn permanent(value: T, created_at: Tick) -> Self {
        Temporal {
            value,
            created_at,
            half_life: 0,
            initial_relevance: 1.0,
        }
    }

    /// How relevant is this value right now?
    pub fn relevance_at(&self, now: Tick) -> f32 {
        if self.half_life == 0 {
            return self.initial_relevance;
        }
        let elapsed = now.raw().saturating_sub(self.created_at.raw());
        let ratio = elapsed as f64 / self.half_life as f64;
        self.initial_relevance * (-0.693147 * ratio).exp() as f32
    }

    /// Is this value still relevant (above a threshold)?
    pub fn is_relevant(&self, now: Tick, threshold: f32) -> bool {
        self.relevance_at(now) >= threshold
    }

    /// Has this value expired (fallen below noise threshold)?
    pub fn is_expired(&self, now: Tick) -> bool {
        self.relevance_at(now) < Priority::NOISE_THRESHOLD.as_f32()
    }

    /// Get the value and its current relevance as a tuple.
    pub fn weighted(&self, now: Tick) -> (T, f32) {
        (self.value, self.relevance_at(now))
    }
}

// ─── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permanent_value_never_decays() {
        let v = DecayingValue::permanent(1.0, Tick::new(0, 0));
        assert_eq!(v.value_at(Tick::new(0, 0)), 1.0);
        assert_eq!(v.value_at(Tick::new(100, 0)), 1.0);
        assert_eq!(v.value_at(Tick::new(10000, 0)), 1.0);
    }

    #[test]
    fn value_decays_by_half_at_half_life() {
        let v = DecayingValue::new(1.0, Tick::new(0, 0), 1000);
        let at_half_life = v.value_at(Tick::new(0, 1000));
        // Should be approximately 0.5 (within floating point)
        assert!((at_half_life - 0.5).abs() < 0.01,
            "Expected ~0.5, got {}", at_half_life);
    }

    #[test]
    fn value_decays_to_quarter_at_two_half_lives() {
        let v = DecayingValue::new(1.0, Tick::new(0, 0), 500);
        let at_two = v.value_at(Tick::new(0, 1000));
        assert!((at_two - 0.25).abs() < 0.01,
            "Expected ~0.25, got {}", at_two);
    }

    #[test]
    fn floor_prevents_full_decay() {
        let v = DecayingValue::new(1.0, Tick::new(0, 0), 100)
            .with_floor(0.1);
        let very_late = v.value_at(Tick::new(100, 0));
        assert!(very_late >= 0.1,
            "Value {} should not go below floor 0.1", very_late);
    }

    #[test]
    fn expiration_detection() {
        let v = DecayingValue::new(1.0, Tick::new(0, 0), 100);
        assert!(!v.is_expired(Tick::new(0, 0), 0.5));
        assert!(v.is_expired(Tick::new(0, 1000), 0.5));
    }

    #[test]
    fn recency_weight_decays() {
        let now = Tick::new(0, 1000);
        let recent = recency_weight(Tick::new(0, 999), now, 0.01);
        let old = recency_weight(Tick::new(0, 0), now, 0.01);
        assert!(recent > old,
            "Recent {} should be more salient than old {}", recent, old);
    }

    #[test]
    fn temporal_wrapper_tracks_relevance() {
        let t = Temporal::new(42u32, Tick::new(0, 0), 500);
        assert_eq!(t.value, 42);
        let (val, rel) = t.weighted(Tick::new(0, 500));
        assert_eq!(val, 42);
        assert!((rel - 0.5).abs() < 0.01);
    }

    #[test]
    fn temporal_permanent_stays_relevant() {
        let t = Temporal::permanent("important", Tick::new(0, 0));
        assert!(t.is_relevant(Tick::new(10000, 0), 0.99));
    }
}
