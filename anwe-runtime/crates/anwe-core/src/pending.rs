// -----------------------------------------------------------------
// ANWE v0.1 -- PENDING
//
// Not failure. Not error. Not a bug.
//
// PENDING is the delivery result indicating that
// conditions are not yet right for transmission.
// The agent should wait and retry according to the
// provided guidance rather than forcing delivery.
//
// Forcing delivery through a PENDING state produces
// false state changes -- the most dangerous condition
// in Anwe. An agent that believes it received
// something it did not.
//
// PENDING is a first-class state, not an error.
// -----------------------------------------------------------------

use crate::signal::{Tick, SyncLevel, Signal};
use core::fmt;

/// Why delivery is not yet ready.
/// Each reason implies a different kind of waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PendingReason {
    /// The receiving agent is not in a state
    /// where genuine application is possible.
    ReceiverNotReady = 0,

    /// The shared link between agents
    /// has not yet achieved enough synchronization
    /// for delivery to succeed.
    LinkNotEstablished = 1,

    /// Synchronization achieved some level
    /// but not enough for this depth of delivery.
    SyncLevelInsufficient = 2,

    /// The sending agent has not yet applied
    /// what it is trying to transmit.
    SenderNotReady = 3,

    /// Everything else is ready
    /// but the timing itself is wrong.
    /// Retry after natural scheduling resumes.
    MomentNotRight = 4,
}

/// Guidance on what the agent should do while waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WaitGuidance {
    /// Return to idle state. Let the receiver complete its work.
    ReturnToIdle = 0,
    /// Synchronize longer together. Let sync level deepen.
    SyncLonger = 1,
    /// Begin with a lower-priority signal. Build sync level before depth.
    StartLighter = 2,
    /// Complete your own pending state change first.
    FinishCommit = 3,
    /// Release entirely. Return to normal scheduling. It will come.
    ReleaseAndWait = 4,
}

/// PENDING -- the valid state of unready delivery.
///
/// Compact: 32 bytes. Carries everything the agent needs
/// to understand the wait condition and respond correctly.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pending {
    /// Why delivery is not yet ready
    pub reason: PendingReason,
    /// What to do while waiting
    pub guidance: WaitGuidance,
    /// Padding
    _pad: [u8; 2],
    /// How many ticks to wait before the agent may retry.
    /// 0 = retry when next naturally scheduled
    /// Higher = longer wait required
    pub wait_ticks: u16,
    /// Padding
    _pad2: [u8; 2],
    /// The sync level at the time of the pending result.
    /// Even failed deliveries build synchronization.
    pub current_sync_level: SyncLevel,
    /// Padding
    _pad3: [u8; 2],
    /// The tick position when the pending result occurred.
    pub at_tick: Tick,
    /// Sequence of the attempted signal (for correlation).
    pub attempted_sequence: u64,
    /// Residual sync: how much synchronization the attempt achieved
    /// even though delivery didn't complete.
    /// Not wasted. Deepens the link for next time.
    pub resonance: SyncLevel,
    /// Padding
    _pad4: [u8; 6],
}

impl Pending {
    /// Receiver is not ready for application.
    /// Wait. Do not push.
    pub fn receiver_not_ready(
        signal: &Signal,
        sync_level: SyncLevel,
    ) -> Self {
        Pending {
            reason: PendingReason::ReceiverNotReady,
            guidance: WaitGuidance::ReturnToIdle,
            _pad: [0; 2],
            wait_ticks: 1,
            _pad2: [0; 2],
            current_sync_level: sync_level,
            _pad3: [0; 2],
            at_tick: signal.tick,
            attempted_sequence: signal.sequence,
            resonance: SyncLevel::new(sync_level.as_f32() * 0.1),
            _pad4: [0; 6],
        }
    }

    /// The shared link has not been established.
    /// Synchronize longer together.
    pub fn link_not_established(
        signal: &Signal,
        sync_level: SyncLevel,
    ) -> Self {
        Pending {
            reason: PendingReason::LinkNotEstablished,
            guidance: WaitGuidance::SyncLonger,
            _pad: [0; 2],
            wait_ticks: 2,
            _pad2: [0; 2],
            current_sync_level: sync_level,
            _pad3: [0; 2],
            at_tick: signal.tick,
            attempted_sequence: signal.sequence,
            resonance: SyncLevel::new(sync_level.as_f32() * 0.05),
            _pad4: [0; 6],
        }
    }

    /// Sync level not deep enough for this priority of signal.
    /// Start lighter or wait for deeper synchronization.
    pub fn sync_level_insufficient(
        signal: &Signal,
        sync_level: SyncLevel,
    ) -> Self {
        Pending {
            reason: PendingReason::SyncLevelInsufficient,
            guidance: WaitGuidance::StartLighter,
            _pad: [0; 2],
            wait_ticks: 1,
            _pad2: [0; 2],
            current_sync_level: sync_level,
            _pad3: [0; 2],
            at_tick: signal.tick,
            attempted_sequence: signal.sequence,
            resonance: SyncLevel::new(sync_level.as_f32() * 0.2),
            _pad4: [0; 6],
        }
    }

    /// Sender hasn't finished committing what it's transmitting.
    /// Complete the pending state change first.
    pub fn sender_not_ready(
        signal: &Signal,
        sync_level: SyncLevel,
    ) -> Self {
        Pending {
            reason: PendingReason::SenderNotReady,
            guidance: WaitGuidance::FinishCommit,
            _pad: [0; 2],
            wait_ticks: 4,
            _pad2: [0; 2],
            current_sync_level: sync_level,
            _pad3: [0; 2],
            at_tick: signal.tick,
            attempted_sequence: signal.sequence,
            resonance: SyncLevel::ZERO,
            _pad4: [0; 6],
        }
    }

    /// Everything is ready but the timing is wrong.
    /// Release entirely. It will come when scheduling allows.
    pub fn moment_not_right(
        signal: &Signal,
        sync_level: SyncLevel,
    ) -> Self {
        Pending {
            reason: PendingReason::MomentNotRight,
            guidance: WaitGuidance::ReleaseAndWait,
            _pad: [0; 2],
            wait_ticks: 0, // No prescribed wait. Normal scheduling will call.
            _pad2: [0; 2],
            current_sync_level: sync_level,
            _pad3: [0; 2],
            at_tick: signal.tick,
            attempted_sequence: signal.sequence,
            resonance: SyncLevel::new(sync_level.as_f32() * 0.3),
            _pad4: [0; 6],
        }
    }
}

impl fmt::Debug for Pending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pending")
            .field("reason", &self.reason)
            .field("guidance", &self.guidance)
            .field("wait_ticks", &self.wait_ticks)
            .field("sync_level", &self.current_sync_level)
            .field("resonance", &self.resonance)
            .finish()
    }
}

impl fmt::Display for Pending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let guidance = match self.guidance {
            WaitGuidance::ReturnToIdle => "return to idle",
            WaitGuidance::SyncLonger => "synchronize longer together",
            WaitGuidance::StartLighter => "start lighter",
            WaitGuidance::FinishCommit => "finish pending commit first",
            WaitGuidance::ReleaseAndWait => "release and wait",
        };
        write!(f, "Pending({:?} -- {})", self.reason, guidance)
    }
}

/// Delivery result: either the signal was received,
/// or it returns Pending. Nothing else.
#[derive(Debug, Clone, Copy)]
pub enum DeliveryResult {
    /// The signal was genuinely received.
    /// Application can begin. State change will follow.
    Received {
        /// How much synchronization the delivery achieved
        resonance: SyncLevel,
    },
    /// PENDING. The conditions are not right.
    /// The Pending value tells you why and what to do while waiting.
    Waiting(Pending),
}

impl DeliveryResult {
    #[inline(always)]
    pub fn is_received(&self) -> bool {
        matches!(self, DeliveryResult::Received { .. })
    }

    #[inline(always)]
    pub fn is_waiting(&self) -> bool {
        matches!(self, DeliveryResult::Waiting(_))
    }

    /// Resonance achieved, whether received or not.
    /// Even PENDING results leave traces of synchronization.
    pub fn resonance(&self) -> SyncLevel {
        match self {
            DeliveryResult::Received { resonance } => *resonance,
            DeliveryResult::Waiting(pending) => pending.resonance,
        }
    }
}
