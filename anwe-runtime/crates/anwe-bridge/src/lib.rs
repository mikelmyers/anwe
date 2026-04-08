// -----------------------------------------------------------------
// ANWE v0.1 -- BRIDGE
//
// The protocol for external participation in ANWE coordination.
//
// This crate defines what anything — agent, model, sensor, swarm,
// neural network, or something that doesn't exist yet — needs to
// implement in order to participate in ANWE signal exchange.
//
// The protocol is intentionally minimal:
//   - Receive a signal, optionally respond
//   - Accept or reject structural changes
//   - Be notified of irreversible commits
//   - Report remaining processing capacity
//
// That's it. Everything else is the participant's internal affair.
// The ANWE runtime manages the state machine, the scheduling,
// the synchronization. The participant just communicates through
// signals.
//
// This is not an "agent API." This is a signal protocol.
// Whatever AI becomes next, if it can exchange signals,
// it can participate.
// -----------------------------------------------------------------

pub mod participant;
pub mod wire;
pub mod registry;

pub mod stdio;

pub use participant::{Participant, ParticipantDescriptor, CallbackParticipant, MindCallbackParticipant};
pub use wire::{WireSignal, WireValue};
pub use registry::ParticipantRegistry;
pub use stdio::StdioParticipant;
