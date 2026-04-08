// -----------------------------------------------------------------
// ANWE v0.1 -- PARTICIPANT REGISTRY
//
// Maps agent names to their external participants.
//
// The registry is the connection point between the ANWE runtime
// and everything outside it. When an .anwe program declares:
//
//   agent Sensor external("python", "perception.sensor")
//
// The engine looks up "Sensor" in the registry to find the
// participant implementation that handles signal exchange.
//
// The registry is designed to be shared between sequential
// and concurrent engines. Participants are wrapped in
// Arc<Mutex<>> so they can be accessed from multiple fibers.
// -----------------------------------------------------------------

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::participant::Participant;

/// Registry of external participants.
///
/// Create a registry, register participants by agent name,
/// then pass it to the engine before execution.
///
/// ```ignore
/// let mut registry = ParticipantRegistry::new();
/// registry.register("Sensor", Box::new(my_sensor));
///
/// let mut engine = Engine::with_participants(registry);
/// engine.execute(&program)?;
/// ```
pub struct ParticipantRegistry {
    participants: HashMap<String, Arc<Mutex<Box<dyn Participant>>>>,
}

impl ParticipantRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        ParticipantRegistry {
            participants: HashMap::new(),
        }
    }

    /// Register a participant for the given agent name.
    ///
    /// The name must match the agent name in the .anwe program.
    /// If the .anwe program declares `agent Sensor external(...)`,
    /// then `register("Sensor", ...)` connects them.
    pub fn register(&mut self, agent_name: &str, participant: Box<dyn Participant>) {
        self.participants.insert(
            agent_name.to_string(),
            Arc::new(Mutex::new(participant)),
        );
    }

    /// Check if an agent name has an external participant.
    pub fn is_external(&self, agent_name: &str) -> bool {
        self.participants.contains_key(agent_name)
    }

    /// Get the participant for an agent name.
    ///
    /// Returns an Arc<Mutex<>> so the participant can be
    /// shared between concurrent fibers safely.
    pub fn get(&self, agent_name: &str) -> Option<Arc<Mutex<Box<dyn Participant>>>> {
        self.participants.get(agent_name).cloned()
    }

    /// How many external participants are registered?
    pub fn count(&self) -> usize {
        self.participants.len()
    }

    /// List all registered agent names.
    pub fn names(&self) -> Vec<&str> {
        self.participants.keys().map(|s| s.as_str()).collect()
    }
}

impl Clone for ParticipantRegistry {
    /// Clone the registry.
    ///
    /// This is cheap — it just clones Arc pointers.
    /// The actual participants are shared, not duplicated.
    fn clone(&self) -> Self {
        ParticipantRegistry {
            participants: self.participants.clone(),
        }
    }
}

impl Default for ParticipantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// -----------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::participant::CallbackParticipant;

    #[test]
    fn register_and_lookup() {
        let mut registry = ParticipantRegistry::new();
        registry.register("Echo", Box::new(CallbackParticipant::echo("Echo")));

        assert!(registry.is_external("Echo"));
        assert!(!registry.is_external("NotRegistered"));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn clone_shares_participants() {
        let mut registry = ParticipantRegistry::new();
        registry.register("Sensor", Box::new(CallbackParticipant::echo("Sensor")));

        let cloned = registry.clone();
        assert!(cloned.is_external("Sensor"));
        assert_eq!(cloned.count(), 1);

        // Both registries point to the same participant
        let p1 = registry.get("Sensor").unwrap();
        let p2 = cloned.get("Sensor").unwrap();
        assert!(Arc::ptr_eq(&p1, &p2));
    }

    #[test]
    fn names_list() {
        let mut registry = ParticipantRegistry::new();
        registry.register("A", Box::new(CallbackParticipant::silent("A")));
        registry.register("B", Box::new(CallbackParticipant::silent("B")));

        let mut names = registry.names();
        names.sort();
        assert_eq!(names, vec!["A", "B"]);
    }
}
