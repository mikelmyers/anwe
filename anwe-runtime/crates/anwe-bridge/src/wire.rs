// -----------------------------------------------------------------
// ANWE v0.1 -- WIRE TYPES
//
// Language-agnostic data types for crossing the bridge boundary.
//
// These types are how data moves between the ANWE runtime
// and external participants. They are intentionally simple:
// no Rust-specific types, no Python-specific types.
// Just data.
//
// WireSignal is the bridge representation of an ANWE Signal.
// WireValue is the bridge representation of arbitrary data.
//
// Any language that can represent these types can participate.
// -----------------------------------------------------------------

use anwe_core::{
    Signal, Quality, Direction, Priority, AgentId, Tick, TraceKind,
};

/// A signal as seen across the bridge boundary.
///
/// This is NOT the internal 64-byte cache-aligned Signal.
/// This is the human-readable, language-agnostic representation
/// that external participants work with.
///
/// Qualities and directions are u8 codes, not Rust enums.
/// Priorities and confidence are f32, not quantized u16.
/// Data is WireValue, not tagged u64 pointers.
///
/// The runtime converts Signal ↔ WireSignal at the bridge boundary.
/// External participants never see the internal representation.
#[derive(Debug, Clone)]
pub struct WireSignal {
    /// Quality code (0=Attending, 1=Questioning, 2=Recognizing,
    /// 3=Disturbed, 4=Applying, 5=Completing, 6=Resting)
    pub quality: u8,
    /// Direction code (0=Inward, 1=Outward, 2=Between, 3=Diffuse)
    pub direction: u8,
    /// Priority (0.0 to 1.0)
    pub priority: f32,
    /// Optional data payload
    pub data: Option<WireValue>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Temporal half-life in ticks (0 = permanent)
    pub half_life: u16,
    /// Sequence number for ordering
    pub sequence: u64,
}

/// A value that can cross the bridge boundary.
///
/// Language-agnostic. Any language binding (Python, JavaScript,
/// Go, C, whatever comes next) can represent these types.
///
/// This is the common vocabulary between ANWE and the outside world.
#[derive(Debug, Clone, PartialEq)]
pub enum WireValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<WireValue>),
    Map(Vec<(String, WireValue)>),
}

impl WireValue {
    /// Convert to a display string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            WireValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            WireValue::Float(f) => Some(*f),
            WireValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            WireValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            WireValue::Integer(i) => Some(*i),
            WireValue::Float(f) => Some(*f as i64),
            _ => None,
        }
    }
}

impl core::fmt::Display for WireValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WireValue::Null => write!(f, "null"),
            WireValue::Bool(b) => write!(f, "{}", b),
            WireValue::Integer(i) => write!(f, "{}", i),
            WireValue::Float(v) => write!(f, "{}", v),
            WireValue::String(s) => write!(f, "\"{}\"", s),
            WireValue::Bytes(b) => write!(f, "<{} bytes>", b.len()),
            WireValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            WireValue::Map(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

// -----------------------------------------------------------------
// CONVERSION: Signal ↔ WireSignal
//
// The bridge boundary. Internal representation meets
// external representation. No information is lost.
// -----------------------------------------------------------------

impl WireSignal {
    /// Convert an internal Signal to a WireSignal.
    ///
    /// This is called when the runtime delivers a signal
    /// to an external participant. The internal 64-byte
    /// cache-aligned struct becomes a language-agnostic
    /// representation.
    pub fn from_signal(signal: &Signal) -> Self {
        let data = if signal.has_payload() {
            signal.inline_payload().map(|v| WireValue::Integer(v as i64))
        } else {
            None
        };

        WireSignal {
            quality: signal.quality as u8,
            direction: signal.direction as u8,
            priority: signal.priority.as_f32(),
            data,
            confidence: signal.confidence_f32(),
            half_life: signal.half_life,
            sequence: signal.sequence,
        }
    }

    /// Convert a WireSignal back to an internal Signal.
    ///
    /// This is called when an external participant sends
    /// a response signal back through the bridge.
    /// The language-agnostic representation becomes a
    /// 64-byte cache-aligned struct ready for the channel.
    pub fn to_signal(&self, origin: AgentId, tick: Tick) -> Signal {
        let quality = match self.quality {
            0 => Quality::Attending,
            1 => Quality::Questioning,
            2 => Quality::Recognizing,
            3 => Quality::Disturbed,
            4 => Quality::Applying,
            5 => Quality::Completing,
            _ => Quality::Resting,
        };

        let direction = match self.direction {
            0 => Direction::Inward,
            1 => Direction::Outward,
            2 => Direction::Between,
            _ => Direction::Diffuse,
        };

        let mut signal = Signal::new(
            quality,
            direction,
            Priority::new(self.priority),
            origin,
            tick,
        )
        .with_sequence(self.sequence)
        .with_uncertainty(0.0, self.confidence)
        .with_half_life(self.half_life);

        // Attach inline data if present
        if let Some(ref data) = self.data {
            match data {
                WireValue::Integer(v) => {
                    signal = signal.with_data_inline(*v as u64);
                }
                _ => {
                    // For non-integer data, hash it and store as trace
                    signal = signal.with_trace(
                        TraceKind::Context,
                        simple_hash(&format!("{}", data)),
                    );
                }
            }
        }

        signal
    }
}

/// Simple non-cryptographic hash for data provenance.
fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

// -----------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_round_trip() {
        let original = Signal::new(
            Quality::Questioning,
            Direction::Between,
            Priority::new(0.85),
            AgentId::new(7),
            Tick::new(3, 100),
        )
        .with_data_inline(42)
        .with_sequence(99)
        .with_uncertainty(0.0, 0.9)
        .with_half_life(500);

        let wire = WireSignal::from_signal(&original);
        assert_eq!(wire.quality, 1); // Questioning
        assert_eq!(wire.direction, 2); // Between
        assert!((wire.priority - 0.85).abs() < 0.01);
        assert_eq!(wire.data, Some(WireValue::Integer(42)));
        assert_eq!(wire.half_life, 500);
        assert_eq!(wire.sequence, 99);

        // Convert back
        let restored = wire.to_signal(AgentId::new(7), Tick::new(3, 100));
        assert_eq!(restored.quality, Quality::Questioning);
        assert_eq!(restored.direction, Direction::Between);
        assert!((restored.priority.as_f32() - 0.85).abs() < 0.01);
        assert!(restored.has_payload());
        assert_eq!(restored.inline_payload(), Some(42));
    }

    #[test]
    fn wire_value_display() {
        assert_eq!(format!("{}", WireValue::Null), "null");
        assert_eq!(format!("{}", WireValue::Bool(true)), "true");
        assert_eq!(format!("{}", WireValue::Integer(42)), "42");
        assert_eq!(format!("{}", WireValue::String("hello".into())), "\"hello\"");
    }

    #[test]
    fn wire_value_accessors() {
        assert_eq!(WireValue::String("test".into()).as_str(), Some("test"));
        assert_eq!(WireValue::Float(3.14).as_f64(), Some(3.14));
        assert_eq!(WireValue::Bool(true).as_bool(), Some(true));
        assert_eq!(WireValue::Integer(10).as_i64(), Some(10));
    }
}
