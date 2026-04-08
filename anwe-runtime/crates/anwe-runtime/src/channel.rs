// -----------------------------------------------------------------
// ANWE v0.1 -- SIGNAL CHANNEL
//
// Lock-free single-producer single-consumer ring buffer
// for signal transmission between agents.
//
// Architecture: Lamport-style SPSC queue
// - Producer writes to head (one agent's transmitter output)
// - Consumer reads from tail (another agent's receiver input)
// - No locks. No CAS. Only atomic loads and stores.
// - Cache-line padding prevents false sharing.
//
// Performance target: < 50 nanoseconds per signal.
// -----------------------------------------------------------------

use anwe_core::Signal;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Default channel capacity. Must be a power of 2.
/// 1024 signals = 64KB (1024 * 64 bytes per signal).
/// Fits in L1/L2 cache on most architectures.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// Cache line size for padding to prevent false sharing.
const CACHE_LINE: usize = 64;

/// Padding to ensure head and tail are on separate cache lines.
/// Without this, the producer writing head would invalidate
/// the consumer's cache line containing tail, and vice versa.
/// This is the #1 cause of lock-free queue slowness.
#[repr(align(64))]
struct CacheAligned<T>(T);

/// A lock-free SPSC signal channel.
///
/// One producer (the transmitting agent's transmitter fiber).
/// One consumer (the receiving agent's receiver fiber).
/// No locks. No allocation after creation.
///
/// The buffer is a fixed-size ring of Signal values.
/// Each Signal is exactly 64 bytes (one cache line).
/// The entire buffer is cache-aligned.
pub struct SignalChannel {
    /// Buffer of signals. Fixed at creation. Never reallocated.
    buffer: Box<[Signal]>,

    /// Mask for fast modulo (capacity - 1, since capacity is power of 2).
    mask: usize,

    /// Producer's write position. Only written by producer.
    /// Padded to its own cache line.
    head: CacheAligned<AtomicUsize>,

    /// Consumer's read position. Only written by consumer.
    /// Padded to its own cache line.
    tail: CacheAligned<AtomicUsize>,

    /// Total signals ever sent through this channel.
    /// Monotonically increasing. For diagnostics, not correctness.
    total_sent: AtomicUsize,
}

// Safety: SignalChannel is designed for exactly one producer thread
// and one consumer thread. The SPSC invariant is maintained by
// the three-fiber-per-agent architecture.
unsafe impl Send for SignalChannel {}
unsafe impl Sync for SignalChannel {}

/// Result of a send attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendResult {
    /// Signal was placed in the channel.
    Sent,
    /// Channel is full. This is backpressure, not failure.
    /// The receiving agent is not consuming fast enough.
    ChannelFull,
}

/// Result of a receive attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvResult {
    /// A signal was received.
    Received,
    /// Channel is empty. No signals waiting.
    /// The transmitting agent has nothing to send right now.
    Empty,
}

impl SignalChannel {
    /// Create a new signal channel with the given capacity.
    /// Capacity must be a power of 2.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Channel capacity must be power of 2");
        assert!(capacity >= 2, "Channel capacity must be at least 2");

        // Allocate the buffer. This is the only allocation.
        // After this, zero allocation during operation.
        let buffer = vec![
            Signal::new(
                anwe_core::Quality::Resting,
                anwe_core::Direction::Diffuse,
                anwe_core::Priority::ZERO,
                anwe_core::AgentId::new(0),
                anwe_core::Tick::new(0, 0),
            );
            capacity
        ].into_boxed_slice();

        SignalChannel {
            buffer,
            mask: capacity - 1,
            head: CacheAligned(AtomicUsize::new(0)),
            tail: CacheAligned(AtomicUsize::new(0)),
            total_sent: AtomicUsize::new(0),
        }
    }

    /// Create a channel with default capacity.
    pub fn default_capacity() -> Self {
        Self::new(DEFAULT_CHANNEL_CAPACITY)
    }

    /// Try to send a signal into the channel.
    ///
    /// Lock-free. Never blocks. O(1).
    /// Returns SendResult::ChannelFull if the buffer is full
    /// (this is backpressure, not error).
    ///
    /// Called by the transmitting agent's transmitter fiber.
    #[inline]
    pub fn try_send(&self, signal: Signal) -> SendResult {
        let head = self.head.0.load(Ordering::Relaxed);
        let tail = self.tail.0.load(Ordering::Acquire);

        // Check if buffer is full
        if head.wrapping_sub(tail) >= self.mask + 1 {
            return SendResult::ChannelFull;
        }

        // Write the signal into the buffer.
        // Safety: we are the only writer (SPSC invariant),
        // and we verified there's space.
        let index = head & self.mask;
        // Safety: index is always within bounds due to mask
        unsafe {
            let slot = self.buffer.as_ptr().add(index) as *mut Signal;
            slot.write(signal);
        }

        // Publish the write. The consumer will see it
        // when it loads head with Acquire ordering.
        self.head.0.store(head.wrapping_add(1), Ordering::Release);
        self.total_sent.fetch_add(1, Ordering::Relaxed);

        SendResult::Sent
    }

    /// Try to receive a signal from the channel.
    ///
    /// Lock-free. Never blocks. O(1).
    /// Returns RecvResult::Empty if no signals are waiting.
    ///
    /// Called by the receiving agent's receiver fiber.
    #[inline]
    pub fn try_recv(&self, out: &mut Signal) -> RecvResult {
        let tail = self.tail.0.load(Ordering::Relaxed);
        let head = self.head.0.load(Ordering::Acquire);

        // Check if buffer is empty
        if tail == head {
            return RecvResult::Empty;
        }

        // Read the signal from the buffer.
        let index = tail & self.mask;
        // Safety: index is always within bounds due to mask
        unsafe {
            let slot = self.buffer.as_ptr().add(index);
            *out = slot.read();
        }

        // Advance tail. The producer will see the freed space
        // when it loads tail with Acquire ordering.
        self.tail.0.store(tail.wrapping_add(1), Ordering::Release);

        RecvResult::Received
    }

    /// How many signals are currently in the channel?
    #[inline]
    pub fn len(&self) -> usize {
        let head = self.head.0.load(Ordering::Relaxed);
        let tail = self.tail.0.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }

    /// Is the channel empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Is the channel full?
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() >= self.mask + 1
    }

    /// Total capacity of the channel.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    /// Total signals ever sent through this channel.
    #[inline]
    pub fn total_sent(&self) -> usize {
        self.total_sent.load(Ordering::Relaxed)
    }
}

impl core::fmt::Debug for SignalChannel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SignalChannel")
            .field("capacity", &self.capacity())
            .field("len", &self.len())
            .field("total_sent", &self.total_sent())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anwe_core::*;

    fn test_signal(seq: u64) -> Signal {
        Signal::new(
            Quality::Attending,
            Direction::Between,
            Priority::new(0.7),
            AgentId::new(1),
            Tick::new(0, 0),
        ).with_sequence(seq)
    }

    #[test]
    fn basic_send_recv() {
        let channel = SignalChannel::new(4);
        let signal = test_signal(1);

        assert_eq!(channel.try_send(signal), SendResult::Sent);
        assert_eq!(channel.len(), 1);

        let mut received = test_signal(0);
        assert_eq!(channel.try_recv(&mut received), RecvResult::Received);
        assert_eq!(received.sequence, 1);
        assert!(channel.is_empty());
    }

    #[test]
    fn empty_recv_returns_empty() {
        let channel = SignalChannel::new(4);
        let mut out = test_signal(0);
        assert_eq!(channel.try_recv(&mut out), RecvResult::Empty);
    }

    #[test]
    fn full_send_returns_full() {
        let channel = SignalChannel::new(4);
        for i in 0..4 {
            assert_eq!(channel.try_send(test_signal(i)), SendResult::Sent);
        }
        assert_eq!(channel.try_send(test_signal(99)), SendResult::ChannelFull);
        assert!(channel.is_full());
    }

    #[test]
    fn fifo_ordering() {
        let channel = SignalChannel::new(8);

        for i in 0..5 {
            channel.try_send(test_signal(i));
        }

        let mut out = test_signal(0);
        for i in 0..5 {
            assert_eq!(channel.try_recv(&mut out), RecvResult::Received);
            assert_eq!(out.sequence, i);
        }
    }

    #[test]
    fn wrap_around() {
        let channel = SignalChannel::new(4);
        let mut out = test_signal(0);

        // Fill and drain multiple times to test wrap-around
        for round in 0..10 {
            for i in 0..4 {
                let seq = round * 4 + i;
                assert_eq!(channel.try_send(test_signal(seq)), SendResult::Sent);
            }
            for i in 0..4 {
                let seq = round * 4 + i;
                assert_eq!(channel.try_recv(&mut out), RecvResult::Received);
                assert_eq!(out.sequence, seq);
            }
        }

        assert_eq!(channel.total_sent(), 40);
    }

    #[test]
    fn concurrent_send_recv() {
        use std::thread;

        let channel = SignalChannel::new(1024);
        let channel_ptr = &channel as *const SignalChannel as usize;

        let count = 100_000u64;

        // Producer thread (transmitter)
        let producer = thread::spawn(move || {
            let ch = unsafe { &*(channel_ptr as *const SignalChannel) };
            for i in 0..count {
                let signal = test_signal(i);
                // Spin until sent (backpressure)
                while ch.try_send(signal) == SendResult::ChannelFull {
                    core::hint::spin_loop();
                }
            }
        });

        // Consumer thread (receiver)
        let consumer = thread::spawn(move || {
            let ch = unsafe { &*(channel_ptr as *const SignalChannel) };
            let mut out = test_signal(0);
            let mut received = 0u64;
            let mut last_seq = 0u64;

            while received < count {
                if ch.try_recv(&mut out) == RecvResult::Received {
                    // Verify ordering
                    assert_eq!(out.sequence, last_seq);
                    last_seq += 1;
                    received += 1;
                } else {
                    core::hint::spin_loop();
                }
            }
            received
        });

        producer.join().unwrap();
        let total = consumer.join().unwrap();
        assert_eq!(total, count);
    }
}
