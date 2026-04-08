// -----------------------------------------------------------------
// ANWE v0.1 -- PARTICIPANT
//
// The minimal contract for participating in ANWE coordination.
//
// This is not "an agent trait." This is the protocol for anything
// that can receive a signal and respond. Whether that thing is:
//
//   - A Python class
//   - A neural network
//   - A robotic controller
//   - A sensor array
//   - A swarm intelligence
//   - A quantum processor
//   - Something that doesn't exist yet
//
// If it can receive signals and optionally respond, it can
// participate. The ANWE runtime handles everything else:
// state transitions, scheduling, synchronization, history.
//
// The participant just talks in signals.
// -----------------------------------------------------------------

use crate::wire::{WireSignal, WireValue};

/// Metadata about a participant.
///
/// Intentionally generic. The runtime does not interpret the `kind`
/// or `address` fields — they exist for the transport layer and
/// for human understanding. A participant could be:
///
///   kind: "python",    address: "primordia.mnemonic"
///   kind: "grpc",      address: "localhost:50051"
///   kind: "wasm",      address: "perception.wasm"
///   kind: "sensor",    address: "/dev/lidar0"
///   kind: "callback",  address: "inline"
///
/// The bridge doesn't care. It just moves signals.
#[derive(Debug, Clone)]
pub struct ParticipantDescriptor {
    /// Human-readable name for this participant.
    pub name: String,
    /// What kind of thing this is. Transport-level hint.
    pub kind: String,
    /// Where to find it. Transport-specific.
    pub address: String,
    /// Version identifier.
    pub version: String,
}

/// The protocol for external participation in ANWE coordination.
///
/// Five methods. That's the entire surface area between ANWE
/// and everything outside it.
///
/// The ANWE runtime maintains its own Agent state machine for
/// each participant. The runtime drives state transitions
/// (Idle → Alerted → Connected → Syncing → Applying → Committing → Idle).
/// The participant is notified and can influence the process,
/// but the state machine lives in Rust.
///
/// This means a participant doesn't need to understand ANWE's
/// state model. It just receives signals and responds.
pub trait Participant: Send {
    /// A signal has arrived.
    ///
    /// This is called whenever the ANWE runtime delivers a signal
    /// to this participant — during alert, connect, sync, or any
    /// other primitive that involves signal exchange.
    ///
    /// Return `Some(response)` to send a signal back through the link.
    /// Return `None` if there's nothing to say.
    ///
    /// The participant does NOT need to manage its own state.
    /// The runtime handles that. This is just notification
    /// and optional response.
    fn receive(&mut self, signal: &WireSignal) -> Option<WireSignal>;

    /// Structural changes are being applied.
    ///
    /// During an ANWE `apply` primitive, the runtime proposes
    /// structural changes to this participant. The participant
    /// can accept (return true) or reject (return false).
    ///
    /// If rejected, the runtime will execute the reject path
    /// instead of the apply path. This is intelligent withdrawal,
    /// not failure.
    ///
    /// The `changes` are key-value pairs representing what would
    /// change in this participant's data.
    fn apply(&mut self, changes: &[(String, WireValue)]) -> bool;

    /// Changes have been committed. This is irreversible.
    ///
    /// Called after an ANWE `commit` primitive executes.
    /// The participant is notified that permanent changes
    /// have been recorded to history.
    ///
    /// This is notification, not a request. The commit has
    /// already happened in the ANWE runtime.
    fn commit(&mut self, entries: &[(String, WireValue)]);

    /// How much processing capacity remains (0.0 to 1.0).
    ///
    /// The ANWE scheduler uses this to manage attention budgets.
    /// A participant with no remaining capacity will have its
    /// fibers deprioritized (not blocked — just given less
    /// scheduling weight).
    ///
    /// Default: 1.0 (unlimited capacity). Override if your
    /// participant has finite processing resources.
    fn attention(&self) -> f32 {
        1.0
    }

    /// Describe this participant.
    ///
    /// Called once during registration. The descriptor is used
    /// for logging, debugging, and transport routing.
    fn descriptor(&self) -> &ParticipantDescriptor;

    // ─── FIRST-PERSON COGNITION EXTENSIONS ──────────────────

    /// Think bindings have been computed.
    ///
    /// Called when a `think { name <- expr }` block executes
    /// inside a mind that is bridged to this participant.
    ///
    /// The participant can enrich, transform, or replace the
    /// bindings. Return `Some(enriched)` to override the
    /// bindings with your own. Return `None` to accept as-is.
    ///
    /// This is how an external AI system can participate in
    /// the mind's reasoning process — not by replacing it,
    /// but by enriching what the mind computes.
    fn think(&mut self, _bindings: &[(String, WireValue)]) -> Option<Vec<(String, WireValue)>> {
        None
    }

    /// An expression is being transmitted outward.
    ///
    /// Called when an `express` statement executes inside a
    /// mind that is bridged to this participant.
    ///
    /// The signal carries quality, direction, and priority.
    /// The content is what the mind is expressing.
    ///
    /// Return `Some(transformed)` to modify what gets expressed.
    /// Return `None` to let the original expression through.
    ///
    /// This is how an external AI system can shape the mind's
    /// voice — not by speaking for it, but by deepening what
    /// it says.
    fn express(&mut self, _signal: &WireSignal, _content: &WireValue) -> Option<WireValue> {
        None
    }
}

// -----------------------------------------------------------------
// CALLBACK PARTICIPANT
//
// A simple participant built from closures.
// Useful for testing, prototyping, and inline definitions.
// -----------------------------------------------------------------

/// A participant built from closures.
///
/// For testing and prototyping. Takes functions for each
/// protocol method so you can define behavior inline.
///
/// ```ignore
/// let echo = CallbackParticipant::echo("MySensor");
/// registry.register("Sensor", Box::new(echo));
/// ```
pub struct CallbackParticipant {
    desc: ParticipantDescriptor,
    on_receive: Box<dyn FnMut(&WireSignal) -> Option<WireSignal> + Send>,
    on_apply: Box<dyn FnMut(&[(String, WireValue)]) -> bool + Send>,
    on_commit: Box<dyn FnMut(&[(String, WireValue)]) + Send>,
}

impl CallbackParticipant {
    /// Create a new callback participant with custom handlers.
    pub fn new(
        descriptor: ParticipantDescriptor,
        on_receive: impl FnMut(&WireSignal) -> Option<WireSignal> + Send + 'static,
        on_apply: impl FnMut(&[(String, WireValue)]) -> bool + Send + 'static,
        on_commit: impl FnMut(&[(String, WireValue)]) + Send + 'static,
    ) -> Self {
        CallbackParticipant {
            desc: descriptor,
            on_receive: Box::new(on_receive),
            on_apply: Box::new(on_apply),
            on_commit: Box::new(on_commit),
        }
    }

    /// Create an echo participant.
    ///
    /// Responds to every signal with the same quality/direction
    /// at slightly lower priority. Accepts all changes.
    /// Good for testing that the bridge works end-to-end.
    pub fn echo(name: &str) -> Self {
        CallbackParticipant {
            desc: ParticipantDescriptor {
                name: name.to_string(),
                kind: "callback".to_string(),
                address: "echo".to_string(),
                version: "0.1.0".to_string(),
            },
            on_receive: Box::new(|signal| {
                Some(WireSignal {
                    quality: signal.quality,
                    direction: signal.direction,
                    priority: signal.priority * 0.9,
                    data: signal.data.clone(),
                    confidence: signal.confidence,
                    half_life: signal.half_life,
                    sequence: signal.sequence + 1,
                })
            }),
            on_apply: Box::new(|_changes| true),
            on_commit: Box::new(|_entries| {}),
        }
    }

    /// Create a silent participant.
    ///
    /// Never responds. Accepts all changes. Does nothing on commit.
    /// Useful as a sink or for testing one-directional flow.
    pub fn silent(name: &str) -> Self {
        CallbackParticipant {
            desc: ParticipantDescriptor {
                name: name.to_string(),
                kind: "callback".to_string(),
                address: "silent".to_string(),
                version: "0.1.0".to_string(),
            },
            on_receive: Box::new(|_| None),
            on_apply: Box::new(|_| true),
            on_commit: Box::new(|_| {}),
        }
    }
}

impl Participant for CallbackParticipant {
    fn receive(&mut self, signal: &WireSignal) -> Option<WireSignal> {
        (self.on_receive)(signal)
    }

    fn apply(&mut self, changes: &[(String, WireValue)]) -> bool {
        (self.on_apply)(changes)
    }

    fn commit(&mut self, entries: &[(String, WireValue)]) {
        (self.on_commit)(entries)
    }

    fn descriptor(&self) -> &ParticipantDescriptor {
        &self.desc
    }
}

// -----------------------------------------------------------------
// MIND CALLBACK PARTICIPANT
//
// A participant that also participates in first-person cognition.
// Has think and express handlers in addition to the base protocol.
//
// This is what an external AI system looks like when it bridges
// into a mind — it can enrich thinking and shape expression.
// -----------------------------------------------------------------

/// A participant built from closures that supports first-person
/// cognition bridging.
///
/// Like `CallbackParticipant` but with additional `think` and
/// `express` handlers for mind integration.
pub struct MindCallbackParticipant {
    desc: ParticipantDescriptor,
    on_receive: Box<dyn FnMut(&WireSignal) -> Option<WireSignal> + Send>,
    on_apply: Box<dyn FnMut(&[(String, WireValue)]) -> bool + Send>,
    on_commit: Box<dyn FnMut(&[(String, WireValue)]) + Send>,
    on_think: Box<dyn FnMut(&[(String, WireValue)]) -> Option<Vec<(String, WireValue)>> + Send>,
    on_express: Box<dyn FnMut(&WireSignal, &WireValue) -> Option<WireValue> + Send>,
}

impl MindCallbackParticipant {
    /// Create a new mind-aware callback participant.
    pub fn new(
        descriptor: ParticipantDescriptor,
        on_receive: impl FnMut(&WireSignal) -> Option<WireSignal> + Send + 'static,
        on_apply: impl FnMut(&[(String, WireValue)]) -> bool + Send + 'static,
        on_commit: impl FnMut(&[(String, WireValue)]) + Send + 'static,
        on_think: impl FnMut(&[(String, WireValue)]) -> Option<Vec<(String, WireValue)>> + Send + 'static,
        on_express: impl FnMut(&WireSignal, &WireValue) -> Option<WireValue> + Send + 'static,
    ) -> Self {
        MindCallbackParticipant {
            desc: descriptor,
            on_receive: Box::new(on_receive),
            on_apply: Box::new(on_apply),
            on_commit: Box::new(on_commit),
            on_think: Box::new(on_think),
            on_express: Box::new(on_express),
        }
    }

    /// Create a reflective mind participant.
    ///
    /// Enriches every think block by appending a "reflected" binding.
    /// Transforms every expression by prepending "[reflected] ".
    /// Good for testing that the mind bridge works end-to-end.
    pub fn reflective(name: &str) -> Self {
        MindCallbackParticipant {
            desc: ParticipantDescriptor {
                name: name.to_string(),
                kind: "callback".to_string(),
                address: "reflective".to_string(),
                version: "0.1.0".to_string(),
            },
            on_receive: Box::new(|signal| {
                Some(WireSignal {
                    quality: signal.quality,
                    direction: signal.direction,
                    priority: signal.priority * 0.9,
                    data: signal.data.clone(),
                    confidence: signal.confidence,
                    half_life: signal.half_life,
                    sequence: signal.sequence + 1,
                })
            }),
            on_apply: Box::new(|_| true),
            on_commit: Box::new(|_| {}),
            on_think: Box::new(|bindings| {
                let mut enriched = bindings.to_vec();
                enriched.push(("reflected".to_string(), WireValue::Bool(true)));
                Some(enriched)
            }),
            on_express: Box::new(|_signal, content| {
                match content {
                    WireValue::String(s) => {
                        Some(WireValue::String(format!("[reflected] {}", s)))
                    }
                    _ => None,
                }
            }),
        }
    }
}

impl Participant for MindCallbackParticipant {
    fn receive(&mut self, signal: &WireSignal) -> Option<WireSignal> {
        (self.on_receive)(signal)
    }

    fn apply(&mut self, changes: &[(String, WireValue)]) -> bool {
        (self.on_apply)(changes)
    }

    fn commit(&mut self, entries: &[(String, WireValue)]) {
        (self.on_commit)(entries)
    }

    fn descriptor(&self) -> &ParticipantDescriptor {
        &self.desc
    }

    fn think(&mut self, bindings: &[(String, WireValue)]) -> Option<Vec<(String, WireValue)>> {
        (self.on_think)(bindings)
    }

    fn express(&mut self, signal: &WireSignal, content: &WireValue) -> Option<WireValue> {
        (self.on_express)(signal, content)
    }
}

// -----------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_participant_responds() {
        let mut echo = CallbackParticipant::echo("test");

        let signal = WireSignal {
            quality: 0, // Attending
            direction: 1, // Outward
            priority: 0.8,
            data: Some(WireValue::String("hello".into())),
            confidence: 1.0,
            half_life: 0,
            sequence: 1,
        };

        let response = echo.receive(&signal);
        assert!(response.is_some());

        let resp = response.unwrap();
        assert_eq!(resp.quality, 0);
        assert!((resp.priority - 0.72).abs() < 0.01);
        assert_eq!(resp.sequence, 2);
    }

    #[test]
    fn silent_participant_never_responds() {
        let mut silent = CallbackParticipant::silent("sink");

        let signal = WireSignal {
            quality: 0,
            direction: 0,
            priority: 1.0,
            data: None,
            confidence: 1.0,
            half_life: 0,
            sequence: 0,
        };

        assert!(silent.receive(&signal).is_none());
    }

    #[test]
    fn echo_accepts_all_changes() {
        let mut echo = CallbackParticipant::echo("test");
        assert!(echo.apply(&[("key".into(), WireValue::String("val".into()))]));
    }

    #[test]
    fn descriptor_is_correct() {
        let echo = CallbackParticipant::echo("MySensor");
        let desc = echo.descriptor();
        assert_eq!(desc.name, "MySensor");
        assert_eq!(desc.kind, "callback");
    }

    // ─── MIND PARTICIPANT TESTS ──────────────────────────────

    #[test]
    fn mind_reflective_enriches_think() {
        let mut mind = MindCallbackParticipant::reflective("TestMind");
        let bindings = vec![
            ("insight".to_string(), WireValue::String("something".into())),
        ];
        let result = mind.think(&bindings);
        assert!(result.is_some());
        let enriched = result.unwrap();
        assert_eq!(enriched.len(), 2);
        assert_eq!(enriched[1].0, "reflected");
        assert_eq!(enriched[1].1, WireValue::Bool(true));
    }

    #[test]
    fn mind_reflective_transforms_express() {
        let mut mind = MindCallbackParticipant::reflective("TestMind");
        let signal = WireSignal {
            quality: 2, // Recognizing
            direction: 1, // Outward
            priority: 0.8,
            data: None,
            confidence: 1.0,
            half_life: 0,
            sequence: 1,
        };
        let content = WireValue::String("I see it".into());
        let result = mind.express(&signal, &content);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), WireValue::String("[reflected] I see it".into()));
    }

    #[test]
    fn base_participant_think_returns_none() {
        let mut echo = CallbackParticipant::echo("test");
        // The default think implementation returns None
        let result = echo.think(&[("key".into(), WireValue::String("val".into()))]);
        assert!(result.is_none());
    }

    #[test]
    fn base_participant_express_returns_none() {
        let mut echo = CallbackParticipant::echo("test");
        let signal = WireSignal {
            quality: 0,
            direction: 1,
            priority: 0.5,
            data: None,
            confidence: 1.0,
            half_life: 0,
            sequence: 0,
        };
        let result = echo.express(&signal, &WireValue::String("hello".into()));
        assert!(result.is_none());
    }

    #[test]
    fn mind_reflective_also_receives_signals() {
        let mut mind = MindCallbackParticipant::reflective("TestMind");
        let signal = WireSignal {
            quality: 0,
            direction: 1,
            priority: 0.8,
            data: None,
            confidence: 1.0,
            half_life: 0,
            sequence: 1,
        };
        let response = mind.receive(&signal);
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!((resp.priority - 0.72).abs() < 0.01);
    }
}
