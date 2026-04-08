// -----------------------------------------------------------------
// ANWE v0.1 -- FIBER SCHEDULER
//
// Each agent runs as three concurrent fibers:
//   Receiver    - receives incoming signals (inputs)
//   Processor   - processes and applies computation
//   Transmitter - transmits outgoing signals (outputs)
//
// These three run simultaneously on a work-stealing
// thread pool. Each agent receives, processes, and
// transmits concurrently -- not taking turns.
//
// The scheduler is the execution substrate. It manages
// fiber dispatch across available CPU cores.
//
// Priority lanes: Critical > High > Normal > Low > Background.
// Higher-priority fibers are always drained first.
// This is how threat signals preempt background processing.
//
// Cooperative preemption: the scheduler can request a running
// fiber to yield via a shared atomic flag. The fiber checks
// this at yield points (between primitives) and voluntarily
// gives up the CPU for higher-priority work.
// -----------------------------------------------------------------

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use anwe_core::AgentId;

/// Priority lane for fiber scheduling.
/// Higher priority fibers are always drained before lower ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum FiberPriority {
    /// Background: housekeeping, non-urgent maintenance.
    Background = 0,
    /// Low: normal exploration, non-critical processing.
    Low = 1,
    /// Normal: default for most fibers.
    Normal = 2,
    /// High: important signals, active sync.
    High = 3,
    /// Critical: threat response, supervisor actions, preemption.
    Critical = 4,
}

/// A fiber: a lightweight unit of work.
/// Not a thread. Not a coroutine.
/// A single step of an agent's processing cycle.
pub struct Fiber {
    /// Which agent this fiber belongs to
    pub agent: AgentId,
    /// What kind of fiber (receiver, processor, transmitter)
    pub kind: FiberKind,
    /// Scheduling priority lane
    pub priority: FiberPriority,
    /// The work to execute
    pub work: Box<dyn FnOnce() + Send>,
}

/// The three kinds of fibers in the agent processing model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiberKind {
    /// Receives incoming signals from channels
    Receiver,
    /// Processes received signals -- observe, compute, apply
    Processor,
    /// Transmits outgoing signals to channels
    Transmitter,
}

/// A cooperative preemption token.
///
/// Shared between the scheduler and a running fiber.
/// The scheduler sets this to true when a higher-priority
/// fiber needs the CPU. The running fiber checks it at
/// yield points (between primitives) and voluntarily yields.
///
/// This is cooperative, not preemptive. The fiber must
/// check the token. But the scheduler can set it at any time.
#[derive(Debug, Clone)]
pub struct PreemptionToken {
    should_yield: Arc<AtomicBool>,
}

impl PreemptionToken {
    /// Create a new preemption token.
    pub fn new() -> Self {
        PreemptionToken {
            should_yield: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the current fiber should yield.
    #[inline(always)]
    pub fn should_yield(&self) -> bool {
        self.should_yield.load(Ordering::Acquire)
    }

    /// Request the fiber to yield.
    pub fn request_yield(&self) {
        self.should_yield.store(true, Ordering::Release);
    }

    /// Clear the yield request.
    pub fn clear(&self) {
        self.should_yield.store(false, Ordering::Release);
    }

    /// Get a clone of the inner atomic for sharing.
    pub fn handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.should_yield)
    }
}

impl Default for PreemptionToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority-aware work queues.
/// One queue per priority lane. Workers drain higher lanes first.
struct PriorityQueues {
    critical: VecDeque<Fiber>,
    high: VecDeque<Fiber>,
    normal: VecDeque<Fiber>,
    low: VecDeque<Fiber>,
    background: VecDeque<Fiber>,
}

impl PriorityQueues {
    fn new() -> Self {
        PriorityQueues {
            critical: VecDeque::with_capacity(64),
            high: VecDeque::with_capacity(256),
            normal: VecDeque::with_capacity(4096),
            low: VecDeque::with_capacity(256),
            background: VecDeque::with_capacity(64),
        }
    }

    /// Push a fiber into the appropriate lane.
    fn push(&mut self, fiber: Fiber) {
        match fiber.priority {
            FiberPriority::Critical => self.critical.push_back(fiber),
            FiberPriority::High => self.high.push_back(fiber),
            FiberPriority::Normal => self.normal.push_back(fiber),
            FiberPriority::Low => self.low.push_back(fiber),
            FiberPriority::Background => self.background.push_back(fiber),
        }
    }

    /// Pop the highest-priority fiber available.
    fn pop(&mut self) -> Option<Fiber> {
        self.critical.pop_front()
            .or_else(|| self.high.pop_front())
            .or_else(|| self.normal.pop_front())
            .or_else(|| self.low.pop_front())
            .or_else(|| self.background.pop_front())
    }

    /// Total fibers across all lanes.
    fn len(&self) -> usize {
        self.critical.len()
            + self.high.len()
            + self.normal.len()
            + self.low.len()
            + self.background.len()
    }

    /// Are there any critical or high-priority fibers waiting?
    fn has_urgent(&self) -> bool {
        !self.critical.is_empty() || !self.high.is_empty()
    }
}

/// Statistics for the scheduler.
#[derive(Debug)]
pub struct SchedulerStats {
    pub fibers_executed: u64,
    pub receiver_fibers: u64,
    pub processor_fibers: u64,
    pub transmitter_fibers: u64,
    pub critical_fibers: u64,
    pub high_fibers: u64,
    pub normal_fibers: u64,
    pub low_fibers: u64,
    pub background_fibers: u64,
}

/// The fiber scheduler.
///
/// Uses priority-aware work queues with worker threads.
/// Each worker drains higher-priority lanes first, ensuring
/// critical signals always preempt background processing.
///
/// This is how "when reasoning is high, perception drops" —
/// high-priority cognitive work starves lower-priority lanes.
pub struct Scheduler {
    /// Priority-aware work queues.
    queues: Arc<Mutex<PriorityQueues>>,

    /// Worker threads
    workers: Vec<thread::JoinHandle<()>>,

    /// Signal to stop all workers
    shutdown: Arc<AtomicBool>,

    /// Statistics
    fibers_executed: Arc<AtomicU64>,
    receiver_count: Arc<AtomicU64>,
    processor_count: Arc<AtomicU64>,
    transmitter_count: Arc<AtomicU64>,
    critical_count: Arc<AtomicU64>,
    high_count: Arc<AtomicU64>,
    normal_count: Arc<AtomicU64>,
    low_count: Arc<AtomicU64>,
    background_count: Arc<AtomicU64>,
}

impl Scheduler {
    /// Create a new scheduler with the given number of worker threads.
    /// One worker per CPU core is typical.
    pub fn new(num_workers: usize) -> Self {
        let queues = Arc::new(Mutex::new(PriorityQueues::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let fibers_executed = Arc::new(AtomicU64::new(0));
        let receiver_count = Arc::new(AtomicU64::new(0));
        let processor_count = Arc::new(AtomicU64::new(0));
        let transmitter_count = Arc::new(AtomicU64::new(0));
        let critical_count = Arc::new(AtomicU64::new(0));
        let high_count = Arc::new(AtomicU64::new(0));
        let normal_count = Arc::new(AtomicU64::new(0));
        let low_count = Arc::new(AtomicU64::new(0));
        let background_count = Arc::new(AtomicU64::new(0));

        let mut workers = Vec::with_capacity(num_workers);

        for worker_id in 0..num_workers {
            let q = Arc::clone(&queues);
            let stop = Arc::clone(&shutdown);
            let exec_count = Arc::clone(&fibers_executed);
            let r_count = Arc::clone(&receiver_count);
            let p_count = Arc::clone(&processor_count);
            let t_count = Arc::clone(&transmitter_count);
            let cr_count = Arc::clone(&critical_count);
            let hi_count = Arc::clone(&high_count);
            let no_count = Arc::clone(&normal_count);
            let lo_count = Arc::clone(&low_count);
            let bg_count = Arc::clone(&background_count);

            let handle = thread::Builder::new()
                .name(format!("anwe-worker-{}", worker_id))
                .spawn(move || {
                    Self::worker_loop(
                        q, stop, exec_count,
                        r_count, p_count, t_count,
                        cr_count, hi_count, no_count, lo_count, bg_count,
                    );
                })
                .expect("Failed to spawn worker thread");

            workers.push(handle);
        }

        Scheduler {
            queues,
            workers,
            shutdown,
            fibers_executed,
            receiver_count,
            processor_count,
            transmitter_count,
            critical_count,
            high_count,
            normal_count,
            low_count,
            background_count,
        }
    }

    /// Create a scheduler with one worker per CPU core.
    pub fn with_available_cores() -> Self {
        let cores = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(cores)
    }

    /// Submit a fiber for execution.
    /// Non-blocking. The fiber will be picked up by a worker.
    pub fn submit(&self, fiber: Fiber) {
        let mut queues = self.queues.lock().unwrap();
        queues.push(fiber);
    }

    /// Submit a receiver fiber (Normal priority).
    pub fn submit_receiver(
        &self,
        agent: AgentId,
        work: impl FnOnce() + Send + 'static,
    ) {
        self.submit(Fiber {
            agent,
            kind: FiberKind::Receiver,
            priority: FiberPriority::Normal,
            work: Box::new(work),
        });
    }

    /// Submit a processor fiber (Normal priority).
    pub fn submit_processor(
        &self,
        agent: AgentId,
        work: impl FnOnce() + Send + 'static,
    ) {
        self.submit(Fiber {
            agent,
            kind: FiberKind::Processor,
            priority: FiberPriority::Normal,
            work: Box::new(work),
        });
    }

    /// Submit a transmitter fiber (Normal priority).
    pub fn submit_transmitter(
        &self,
        agent: AgentId,
        work: impl FnOnce() + Send + 'static,
    ) {
        self.submit(Fiber {
            agent,
            kind: FiberKind::Transmitter,
            priority: FiberPriority::Normal,
            work: Box::new(work),
        });
    }

    /// Submit a fiber with explicit priority.
    pub fn submit_with_priority(
        &self,
        agent: AgentId,
        kind: FiberKind,
        priority: FiberPriority,
        work: impl FnOnce() + Send + 'static,
    ) {
        self.submit(Fiber {
            agent,
            kind,
            priority,
            work: Box::new(work),
        });
    }

    /// How many fibers are waiting to execute?
    pub fn pending(&self) -> usize {
        self.queues.lock().unwrap().len()
    }

    /// Are there urgent fibers waiting?
    pub fn has_urgent(&self) -> bool {
        self.queues.lock().unwrap().has_urgent()
    }

    /// Get scheduler statistics.
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            fibers_executed: self.fibers_executed.load(Ordering::Relaxed),
            receiver_fibers: self.receiver_count.load(Ordering::Relaxed),
            processor_fibers: self.processor_count.load(Ordering::Relaxed),
            transmitter_fibers: self.transmitter_count.load(Ordering::Relaxed),
            critical_fibers: self.critical_count.load(Ordering::Relaxed),
            high_fibers: self.high_count.load(Ordering::Relaxed),
            normal_fibers: self.normal_count.load(Ordering::Relaxed),
            low_fibers: self.low_count.load(Ordering::Relaxed),
            background_fibers: self.background_count.load(Ordering::Relaxed),
        }
    }

    /// Shutdown the scheduler. Waits for all workers to finish.
    pub fn shutdown(mut self) {
        self.shutdown.store(true, Ordering::Release);

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }

    /// The worker loop: pull fibers from the priority queues and execute them.
    /// Higher-priority lanes are always drained first.
    fn worker_loop(
        queues: Arc<Mutex<PriorityQueues>>,
        shutdown: Arc<AtomicBool>,
        fibers_executed: Arc<AtomicU64>,
        receiver_count: Arc<AtomicU64>,
        processor_count: Arc<AtomicU64>,
        transmitter_count: Arc<AtomicU64>,
        critical_count: Arc<AtomicU64>,
        high_count: Arc<AtomicU64>,
        normal_count: Arc<AtomicU64>,
        low_count: Arc<AtomicU64>,
        background_count: Arc<AtomicU64>,
    ) {
        let mut spin_count = 0u32;

        loop {
            // Check shutdown signal
            if shutdown.load(Ordering::Acquire) {
                // Drain remaining fibers before exiting
                loop {
                    let fiber = {
                        let mut q = queues.lock().unwrap();
                        q.pop()
                    };
                    match fiber {
                        Some(f) => Self::execute_fiber(
                            f,
                            &fibers_executed,
                            &receiver_count,
                            &processor_count,
                            &transmitter_count,
                            &critical_count,
                            &high_count,
                            &normal_count,
                            &low_count,
                            &background_count,
                        ),
                        None => break,
                    }
                }
                return;
            }

            // Try to get a fiber (highest priority first)
            let fiber = {
                let mut q = queues.lock().unwrap();
                q.pop()
            };

            match fiber {
                Some(f) => {
                    spin_count = 0;
                    Self::execute_fiber(
                        f,
                        &fibers_executed,
                        &receiver_count,
                        &processor_count,
                        &transmitter_count,
                        &critical_count,
                        &high_count,
                        &normal_count,
                        &low_count,
                        &background_count,
                    );
                }
                None => {
                    // Adaptive backoff: spin briefly, then yield, then park
                    spin_count += 1;
                    if spin_count < 64 {
                        core::hint::spin_loop();
                    } else if spin_count < 256 {
                        thread::yield_now();
                    } else {
                        // Brief sleep to avoid busy-waiting
                        thread::sleep(std::time::Duration::from_micros(10));
                        spin_count = 128; // Don't sleep longer, stay responsive
                    }
                }
            }
        }
    }

    /// Execute a single fiber and update stats.
    fn execute_fiber(
        fiber: Fiber,
        fibers_executed: &AtomicU64,
        receiver_count: &AtomicU64,
        processor_count: &AtomicU64,
        transmitter_count: &AtomicU64,
        critical_count: &AtomicU64,
        high_count: &AtomicU64,
        normal_count: &AtomicU64,
        low_count: &AtomicU64,
        background_count: &AtomicU64,
    ) {
        // Track which kind of fiber
        match fiber.kind {
            FiberKind::Receiver => receiver_count.fetch_add(1, Ordering::Relaxed),
            FiberKind::Processor => processor_count.fetch_add(1, Ordering::Relaxed),
            FiberKind::Transmitter => transmitter_count.fetch_add(1, Ordering::Relaxed),
        };

        // Track priority lane
        match fiber.priority {
            FiberPriority::Critical => critical_count.fetch_add(1, Ordering::Relaxed),
            FiberPriority::High => high_count.fetch_add(1, Ordering::Relaxed),
            FiberPriority::Normal => normal_count.fetch_add(1, Ordering::Relaxed),
            FiberPriority::Low => low_count.fetch_add(1, Ordering::Relaxed),
            FiberPriority::Background => background_count.fetch_add(1, Ordering::Relaxed),
        };

        // Execute the work
        (fiber.work)();

        fibers_executed.fetch_add(1, Ordering::Relaxed);
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn scheduler_executes_fibers() {
        let scheduler = Scheduler::new(2);
        let counter = Arc::new(AtomicU32::new(0));

        for _ in 0..100 {
            let c = Arc::clone(&counter);
            scheduler.submit_processor(AgentId::new(1), move || {
                c.fetch_add(1, Ordering::Relaxed);
            });
        }

        // Wait for completion
        while counter.load(Ordering::Relaxed) < 100 {
            thread::yield_now();
        }

        assert_eq!(counter.load(Ordering::Relaxed), 100);
        scheduler.shutdown();
    }

    #[test]
    fn three_fiber_model() {
        let scheduler = Scheduler::new(4);
        let receiver_ran = Arc::new(AtomicBool::new(false));
        let processor_ran = Arc::new(AtomicBool::new(false));
        let transmitter_ran = Arc::new(AtomicBool::new(false));

        let id = AgentId::new(42);

        // Submit all three fiber types for the same agent
        let r = Arc::clone(&receiver_ran);
        scheduler.submit_receiver(id, move || {
            r.store(true, Ordering::Release);
        });

        let p = Arc::clone(&processor_ran);
        scheduler.submit_processor(id, move || {
            p.store(true, Ordering::Release);
        });

        let t = Arc::clone(&transmitter_ran);
        scheduler.submit_transmitter(id, move || {
            t.store(true, Ordering::Release);
        });

        // Wait for all three
        while !receiver_ran.load(Ordering::Acquire)
            || !processor_ran.load(Ordering::Acquire)
            || !transmitter_ran.load(Ordering::Acquire)
        {
            thread::yield_now();
        }

        let stats = scheduler.stats();
        assert_eq!(stats.fibers_executed, 3);
        assert_eq!(stats.receiver_fibers, 1);
        assert_eq!(stats.processor_fibers, 1);
        assert_eq!(stats.transmitter_fibers, 1);

        scheduler.shutdown();
    }

    #[test]
    fn priority_lanes_execute_high_first() {
        // Use 1 worker to guarantee ordering
        let scheduler = Scheduler::new(1);
        let order = Arc::new(Mutex::new(Vec::new()));

        // Submit low, then high, then critical
        // With 1 worker, they all queue up
        let o1 = Arc::clone(&order);
        scheduler.submit_with_priority(
            AgentId::new(1), FiberKind::Processor, FiberPriority::Low,
            move || { o1.lock().unwrap().push("low"); },
        );

        let o2 = Arc::clone(&order);
        scheduler.submit_with_priority(
            AgentId::new(1), FiberKind::Processor, FiberPriority::High,
            move || { o2.lock().unwrap().push("high"); },
        );

        let o3 = Arc::clone(&order);
        scheduler.submit_with_priority(
            AgentId::new(1), FiberKind::Processor, FiberPriority::Critical,
            move || { o3.lock().unwrap().push("critical"); },
        );

        // Wait for all three to execute
        while order.lock().unwrap().len() < 3 {
            thread::yield_now();
        }

        let result = order.lock().unwrap().clone();
        // Critical should execute before High, which should execute before Low
        assert_eq!(result, vec!["critical", "high", "low"]);

        scheduler.shutdown();
    }

    #[test]
    fn preemption_token_cooperative_yield() {
        let token = PreemptionToken::new();
        assert!(!token.should_yield());

        token.request_yield();
        assert!(token.should_yield());

        token.clear();
        assert!(!token.should_yield());
    }

    #[test]
    fn priority_stats_tracked() {
        let scheduler = Scheduler::new(2);
        let counter = Arc::new(AtomicU32::new(0));

        let c = Arc::clone(&counter);
        scheduler.submit_with_priority(
            AgentId::new(1), FiberKind::Processor, FiberPriority::Critical,
            move || { c.fetch_add(1, Ordering::Relaxed); },
        );

        let c = Arc::clone(&counter);
        scheduler.submit_with_priority(
            AgentId::new(1), FiberKind::Receiver, FiberPriority::Background,
            move || { c.fetch_add(1, Ordering::Relaxed); },
        );

        while counter.load(Ordering::Relaxed) < 2 {
            thread::yield_now();
        }

        let stats = scheduler.stats();
        assert_eq!(stats.critical_fibers, 1);
        assert_eq!(stats.background_fibers, 1);
        assert_eq!(stats.fibers_executed, 2);

        scheduler.shutdown();
    }
}
