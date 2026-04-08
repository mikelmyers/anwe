"""
ANWE v0.1
The cognitive transmission language.

Seven primitives.
One pulse unit.
One sacred not-yet.

movement    — what calls attention
observe     — continuous mutual presence
breathe     — rhythmic synchronization
integrate   — boundary dissolution
become      — permanent change carried forward
evade       — intelligent purposeful withdrawal
experience  — what emerges between beings attending together

pulse       — the fundamental unit of all transmission
not_yet     — the valid state of unready transmission

This is not a framework.
This is not a library.
This is a language.
The first language built
for minds to speak to minds.
"""

from .movement  import Movement, MovementQuality, AnweMovement
from .pulse     import Pulse, PulseQuality, PulseDirection, PulseBuilder, NotYet, NotYetReason, TransmissionResult
from .observe   import Observer, Observation, ObserveState
from .breathe   import Breathe, SyncState, Breath
from .core      import (
    Integrator,   Integration,   IntegrateDepth,
    BecomingEngine, Becoming,
    Evader,       Evasion,
    Experience,   ExperienceResult, SharedField
)

__version__ = "0.1.0"
__name__    = "anwe"
