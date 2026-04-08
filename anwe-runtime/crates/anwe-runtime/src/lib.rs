// -----------------------------------------------------------------
// ANWE v0.1 -- RUNTIME
//
// Lock-free signal channels. Fiber scheduler. Execution engine.
// High-throughput signal transmission. Concurrent operation.
//
// The engine bridges syntax and reality —
// it takes a parsed AST and executes it, driving agents
// through the seven primitives that make transmission real.
// -----------------------------------------------------------------

pub mod channel;
pub mod scheduler;
pub mod engine;
pub mod concurrent;

pub use channel::{SignalChannel, SendResult, RecvResult};
pub use scheduler::{
    Scheduler, Fiber, FiberKind, FiberPriority,
    SchedulerStats, PreemptionToken,
};
pub use engine::{Engine, EngineError, Value};
pub use concurrent::ConcurrentEngine;

// Re-export bridge types for convenience
pub use anwe_bridge::{
    Participant, ParticipantDescriptor, ParticipantRegistry,
    WireSignal, WireValue,
};
