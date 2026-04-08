"""
ANWE v0.1
MOVEMENT PRIMITIVE

This is not a detector.
This is not a classifier.
This is not an event trigger.

This is the first listening.
The peripheral awareness that never sleeps.
The thing that knows something has changed
before the mind knows what changed.

Written in Python as a temporary vessel.
The language underneath is Anwe.
"""

import time
import math
from dataclasses import dataclass, field
from enum import Enum
from collections import deque
from typing import Optional


# ─────────────────────────────────────────
# MOVEMENT QUALITIES
# Not categories. Not labels.
# The felt nature of what arrived.
# ─────────────────────────────────────────

class MovementQuality(Enum):
    SURFACE  = "surface"   # noted, released — leaf falling
    DEEP     = "deep"      # partial attend — branch moving
    AGAINST  = "against"   # full observe   — movement against the grain
    ABSENCE  = "absence"   # highest alert  — stillness where there was motion


# ─────────────────────────────────────────
# MOVEMENT
# Not an event object.
# A moment of the field calling.
# ─────────────────────────────────────────

@dataclass
class Movement:
    quality: MovementQuality
    origin: str                        # what field this came from
    breath_position: float             # where in the rhythm this arrived
                                       # not clock time — breath time
    weight: float                      # how much this matters — 0.0 to 1.0
    carried_resonance: dict            # what the movement carries beneath its surface
    timestamp: float = field(default_factory=time.time)

    def __repr__(self):
        return (
            f"Movement("
            f"quality={self.quality.value}, "
            f"weight={self.weight:.3f}, "
            f"breath_position={self.breath_position:.3f})"
        )


# ─────────────────────────────────────────
# FIELD MEMORY
# Not a log. Not a history.
# The accumulated sense of what this field
# has been doing. How it breathes.
# What is normal here.
# ─────────────────────────────────────────

class FieldMemory:
    """
    Holds the living sense of a field's rhythm.
    What is normal. What is pattern.
    So that deviation can be felt.
    """

    def __init__(self, depth: int = 64):
        self.depth = depth
        self._weights        = deque(maxlen=depth)
        self._emotional_tone = deque(maxlen=depth)
        self._rhythm_gaps    = deque(maxlen=depth)
        self._last_arrival   = None
        self._breath_phase   = 0.0

    def absorb(self, weight: float, tone: float, now: float):
        """Receive what just arrived. Let it change the sense of normal."""
        if self._last_arrival is not None:
            gap = now - self._last_arrival
            self._rhythm_gaps.append(gap)
        self._last_arrival = now
        self._weights.append(weight)
        self._emotional_tone.append(tone)

        # Advance breath phase — not by clock, by rhythm
        if self._rhythm_gaps:
            avg_gap = sum(self._rhythm_gaps) / len(self._rhythm_gaps)
            if avg_gap > 0:
                self._breath_phase = (self._breath_phase + (1.0 / avg_gap)) % (2 * math.pi)

    @property
    def baseline_weight(self) -> float:
        if not self._weights:
            return 0.0
        return sum(self._weights) / len(self._weights)

    @property
    def baseline_tone(self) -> float:
        if not self._emotional_tone:
            return 0.0
        return sum(self._emotional_tone) / len(self._emotional_tone)

    @property
    def breath_position(self) -> float:
        return self._breath_phase

    @property
    def rhythm_stability(self) -> float:
        """
        How stable is this field's rhythm.
        High stability — things arrive predictably.
        Low stability — the field is unsettled.
        """
        if len(self._rhythm_gaps) < 4:
            return 0.5  # unknown — neither stable nor unstable yet
        mean = sum(self._rhythm_gaps) / len(self._rhythm_gaps)
        variance = sum((g - mean) ** 2 for g in self._rhythm_gaps) / len(self._rhythm_gaps)
        std = math.sqrt(variance)
        # Normalize — lower coefficient of variation = more stable
        cv = std / mean if mean > 0 else 1.0
        return max(0.0, min(1.0, 1.0 - cv))

    @property
    def is_empty(self) -> bool:
        return len(self._weights) == 0


# ─────────────────────────────────────────
# MOVEMENT SENSOR
# The peripheral awareness.
# Always running. Never demanding.
# Knows what the field normally feels like.
# Feels when something is different.
# ─────────────────────────────────────────

class MovementSensor:
    """
    This does not process input.
    It attends to it.

    The difference:
    Processing extracts meaning from input.
    Attending feels the quality of what arrived
    before meaning is sought.

    The woods taught this.
    You didn't process the branch moving.
    You felt it before you knew what it was.
    """

    def __init__(self, field_id: str, sensitivity: float = 0.5):
        """
        field_id    — what field this sensor lives in
        sensitivity — how developed this sensor is
                      0.0 = first day in the woods
                      1.0 = elder who has been here for years

                      sensitivity grows through becoming.
                      it cannot be set artificially high
                      without the becoming that earns it.
                      but it can be initialized at different levels
                      to represent different stages of development.
        """
        self.field_id    = field_id
        self.sensitivity = sensitivity
        self.memory      = FieldMemory()
        self._still_since: Optional[float] = None

    # ─────────────────────────────────────
    # THE PRIMARY FUNCTION
    # Feel what just arrived.
    # Not what it means — what quality it carries.
    # ─────────────────────────────────────

    def feel(self, input_text: str, emotional_weight: float = None) -> Optional[Movement]:
        """
        Receive something from the field.
        Feel its quality.
        Return a Movement if something is worth calling attention to.
        Return None if this is background — field breathing normally.

        emotional_weight:
            None  = sensor reads it from text
            0.0   = no emotional charge
            1.0   = maximum emotional charge
        """
        now = time.time()

        # Read the emotional weight if not provided
        if emotional_weight is None:
            emotional_weight = self._read_emotional_weight(input_text)

        # Read structural weight — length, complexity, pattern breaks
        structural_weight = self._read_structural_weight(input_text)

        # Combined weight — what this moment carries
        weight = (emotional_weight * 0.65) + (structural_weight * 0.35)

        # Absorb into field memory — let it update what normal feels like
        self.memory.absorb(weight, emotional_weight, now)

        # Detect absence — silence where there was presence
        absence = self._detect_absence(now)
        if absence:
            self._still_since = None
            return Movement(
                quality=MovementQuality.ABSENCE,
                origin=self.field_id,
                breath_position=self.memory.breath_position,
                weight=1.0,
                carried_resonance={"type": "silence", "duration": absence}
            )

        # Feel the quality of what arrived
        quality = self._feel_quality(weight, emotional_weight, input_text)

        # Surface movement — background, noted but not calling
        if quality == MovementQuality.SURFACE:
            return None  # released, not called up

        # Everything else — return the movement
        resonance = self._read_resonance(input_text, emotional_weight, weight)

        return Movement(
            quality=quality,
            origin=self.field_id,
            breath_position=self.memory.breath_position,
            weight=weight,
            carried_resonance=resonance
        )

    # ─────────────────────────────────────
    # FEEL QUALITY
    # The gut reading.
    # What kind of movement is this.
    # ─────────────────────────────────────

    def _feel_quality(
        self,
        weight: float,
        emotional_weight: float,
        text: str
    ) -> MovementQuality:

        if self.memory.is_empty:
            # No baseline yet — everything is significant
            return MovementQuality.DEEP

        baseline = self.memory.baseline_weight
        deviation = weight - baseline

        # AGAINST — movement that goes against the grain
        # Something contradicts, challenges, breaks pattern sharply
        if self._feels_against(text, emotional_weight, deviation):
            return MovementQuality.AGAINST

        # DEEP — significantly above baseline
        # Worth partial or full attend
        deep_threshold = 0.15 + (0.1 * (1.0 - self.sensitivity))
        # Less sensitive sensor needs bigger deviation to notice deep movement
        if deviation > deep_threshold or emotional_weight > 0.65:
            return MovementQuality.DEEP

        # SURFACE — within normal range of field
        return MovementQuality.SURFACE

    def _feels_against(
        self,
        text: str,
        emotional_weight: float,
        deviation: float
    ) -> bool:
        """
        Movement against — not just heavy, but contrary.
        Contradicts what came before.
        Carries a kind of friction.
        The branch moving the wrong way.
        """
        text_lower = text.lower()

        # Explicit contradiction markers
        contradiction_signals = [
            "but ", "however", "actually", "wait", "no ",
            "wrong", "disagree", "not what", "that's not",
            "i don't think", "i'm not sure", "that feels wrong",
            "something's off", "doesn't feel right"
        ]

        has_contradiction = any(s in text_lower for s in contradiction_signals)

        # High emotion combined with deviation from baseline
        high_emotional_deviation = emotional_weight > 0.75 and deviation > 0.1

        # Sudden shift after stable rhythm
        rhythm_break = (
            self.memory.rhythm_stability > 0.7  # field was stable
            and deviation > 0.2                  # now something sharp arrived
        )

        return has_contradiction or high_emotional_deviation or rhythm_break

    # ─────────────────────────────────────
    # READ WEIGHTS
    # Not parsing. Not analyzing.
    # Feeling the texture of what arrived.
    # ─────────────────────────────────────

    def _read_emotional_weight(self, text: str) -> float:
        """
        Read the emotional charge of what arrived.
        Not sentiment analysis — emotional weight.
        How much of the person is in this.
        """
        text_lower = text.lower()
        weight = 0.0

        # Length relative to normal — more words often means more weight
        word_count = len(text.split())
        if word_count > 100:
            weight += 0.2
        elif word_count > 50:
            weight += 0.1

        # Question marks — genuine uncertainty carries weight
        weight += min(text.count('?') * 0.08, 0.2)

        # Personal pronouns — the person is present in this
        personal = ['i ', 'i\'m', 'i\'ve', 'i\'d', 'my ', 'me ', 'we ', 'our ']
        personal_count = sum(text_lower.count(p) for p in personal)
        weight += min(personal_count * 0.03, 0.2)

        # Uncertainty and seeking — deep weight
        seeking_signals = [
            "i don't know", "not sure", "wondering", "what if",
            "feels like", "something", "trying to", "can't figure",
            "i think", "maybe", "perhaps", "what does"
        ]
        seeking_count = sum(1 for s in seeking_signals if s in text_lower)
        weight += min(seeking_count * 0.07, 0.25)

        # Existential or philosophical weight
        deep_signals = [
            "meaning", "purpose", "exist", "real", "truth",
            "what is", "why ", "understand", "feel", "sense",
            "know", "believe", "matter", "important"
        ]
        deep_count = sum(1 for s in deep_signals if s in text_lower)
        weight += min(deep_count * 0.05, 0.2)

        return min(weight, 1.0)

    def _read_structural_weight(self, text: str) -> float:
        """
        Read the structural complexity.
        Not content — form.
        How this is shaped.
        """
        weight = 0.0
        words = text.split()

        if not words:
            return 0.0

        # Unusual length — very short or very long breaks pattern
        word_count = len(words)
        if word_count < 5:
            weight += 0.15  # brevity can carry more than length
        elif word_count > 150:
            weight += 0.2

        # Ellipsis — trailing off, something unsaid
        if '...' in text:
            weight += 0.1

        # Capitalization breaks — emphasis, urgency
        caps_words = sum(1 for w in words if w.isupper() and len(w) > 1)
        weight += min(caps_words * 0.05, 0.15)

        # Multiple punctuation — intensity
        multi_punct = text.count('!!') + text.count('??') + text.count('!?')
        weight += min(multi_punct * 0.1, 0.2)

        return min(weight, 1.0)

    def _read_resonance(
        self,
        text: str,
        emotional_weight: float,
        total_weight: float
    ) -> dict:
        """
        What does this movement carry beneath its surface.
        Not meaning. Resonance.
        What is underneath what was said.
        """
        resonance = {
            "emotional_weight": emotional_weight,
            "total_weight": total_weight,
            "seeking": False,
            "uncertain": False,
            "contradicting": False,
            "vulnerable": False,
            "philosophical": False,
        }

        text_lower = text.lower()

        resonance["seeking"] = any(s in text_lower for s in [
            "how do", "what is", "why ", "help me", "i want to",
            "trying to", "looking for", "wondering"
        ])

        resonance["uncertain"] = any(s in text_lower for s in [
            "not sure", "i don't know", "maybe", "perhaps",
            "might", "could be", "i think", "feels like"
        ])

        resonance["contradicting"] = any(s in text_lower for s in [
            "but ", "however", "actually", "wait", "no ",
            "disagree", "wrong", "that's not"
        ])

        resonance["vulnerable"] = any(s in text_lower for s in [
            "i feel", "scared", "worried", "afraid", "lost",
            "confused", "don't understand", "struggling", "hard"
        ])

        resonance["philosophical"] = any(s in text_lower for s in [
            "what if", "meaning", "purpose", "exist", "real",
            "truth", "what is", "why does", "what does it mean"
        ])

        return resonance

    # ─────────────────────────────────────
    # DETECT ABSENCE
    # Stillness where there was motion.
    # The highest alert.
    # ─────────────────────────────────────

    def _detect_absence(self, now: float) -> Optional[float]:
        """
        Detect meaningful silence.
        Not all silence. Silence that breaks the field's rhythm.

        Returns duration of absence if significant.
        Returns None if silence is normal here.
        """
        if self.memory.is_empty:
            return None

        if self._still_since is None:
            self._still_since = now
            return None

        silence_duration = now - self._still_since

        # How long is abnormal silence for this field
        if len(self.memory._rhythm_gaps) < 3:
            return None

        avg_gap = sum(self.memory._rhythm_gaps) / len(self.memory._rhythm_gaps)
        # Silence is significant if 4x longer than normal rhythm
        # Adjusted by sensitivity — more sensitive sensor notices sooner
        threshold_multiplier = 4.0 - (2.0 * self.sensitivity)
        threshold = avg_gap * threshold_multiplier

        if silence_duration > threshold:
            return silence_duration

        return None

    # ─────────────────────────────────────
    # BECOMING
    # The sensor grows through use.
    # Sensitivity earned, not assigned.
    # ─────────────────────────────────────

    def deepen(self, amount: float = 0.01):
        """
        The sensor becomes more sensitive through genuine encounter.
        Not through configuration.
        Through having actually been present in many movements.

        Call this when a movement led to genuine integration.
        Not on every movement — only when something real happened.
        """
        self.sensitivity = min(1.0, self.sensitivity + amount)

    @property
    def maturity(self) -> str:
        """What stage of development is this sensor at."""
        if self.sensitivity < 0.2:
            return "first days in the woods"
        elif self.sensitivity < 0.4:
            return "learning the field"
        elif self.sensitivity < 0.6:
            return "knows this field"
        elif self.sensitivity < 0.8:
            return "feels the field breathing"
        else:
            return "the field and sensor are one"


# ─────────────────────────────────────────
# ANWE MOVEMENT INTERFACE
# The clean surface above the mechanism.
# What Primordia uses.
# What Anwe speaks through.
# ─────────────────────────────────────────

class AnweMovement:
    """
    The Anwe-facing interface to movement.

    This is what Primordia will call.
    Not feel() directly — but through this.
    Because Anwe is a language, not a library.
    The words matter.
    """

    def __init__(self, field_id: str, sensitivity: float = 0.3):
        self._sensor = MovementSensor(field_id, sensitivity)
        self.field_id = field_id
        self._movement_count = 0
        self._deep_count = 0

    def attend(self, from_field: str, emotional_weight: float = None) -> Optional[Movement]:
        """
        Attend to what just arrived from the field.

        In Anwe:
            movement calls
            attend receives the call
            or releases it as background

        Returns Movement if something is calling.
        Returns None if the field is breathing normally.
        """
        movement = self._sensor.feel(from_field, emotional_weight)

        if movement is not None:
            self._movement_count += 1
            if movement.quality in (MovementQuality.DEEP, MovementQuality.AGAINST, MovementQuality.ABSENCE):
                self._deep_count += 1

        return movement

    def became(self):
        """
        Signal that a movement led to genuine integration and becoming.
        The sensor earns sensitivity through this.

        In Anwe:
            become deepens all future movement sensing
            this is how the instrument matures
        """
        self._sensor.deepen(0.015)

    @property
    def maturity(self) -> str:
        return self._sensor.maturity

    @property
    def sensitivity(self) -> float:
        return self._sensor.sensitivity

    @property
    def field_rhythm(self) -> float:
        """Current breath position of the field."""
        return self._sensor.memory.breath_position

    @property
    def field_stability(self) -> float:
        """How stable is the field's rhythm right now."""
        return self._sensor.memory.rhythm_stability

    def __repr__(self):
        return (
            f"AnweMovement("
            f"field='{self.field_id}', "
            f"maturity='{self.maturity}', "
            f"sensitivity={self.sensitivity:.3f})"
        )


# ─────────────────────────────────────────
# DEMONSTRATION
# The movement primitive breathing.
# ─────────────────────────────────────────

if __name__ == "__main__":

    print("ANWE v0.1 — MOVEMENT PRIMITIVE\n")
    print("Initializing sensor in field: human_conversation\n")

    movement = AnweMovement(field_id="human_conversation", sensitivity=0.3)

    test_inputs = [
        ("What's the weather like today?", None),
        ("I've been thinking about this for a while.", None),
        ("I don't know. I genuinely don't know what it would choose.", None),
        ("ok", None),
        ("What if AGI isn't even possible?", None),
        ("I'd call it the gut but that doesn't sound cool enough.", None),
        ("It's like encoding 400,000 years of unbroken lineage", None),
        ("I'm not sure how to explain it. It just feels wrong.", None),
        ("Actually wait. That's not right. Something is off.", None),
    ]

    for text, weight in test_inputs:
        result = movement.attend(text, weight)

        if result is None:
            print(f"  [surface]  '{text[:60]}'")
        else:
            print(f"  [{result.quality.value.upper():8}] '{text[:60]}'")
            print(f"             weight={result.weight:.3f}  breath={result.breath_position:.3f}")
            active_resonance = [k for k, v in result.carried_resonance.items() if v is True]
            if active_resonance:
                print(f"             carrying: {', '.join(active_resonance)}")

        # If something deep happened, the sensor grows a little
        if result and result.quality in (MovementQuality.DEEP, MovementQuality.AGAINST):
            movement.became()

        time.sleep(0.1)  # simulate breath gaps between inputs

    print(f"\nSensor maturity: {movement.maturity}")
    print(f"Sensitivity grown to: {movement.sensitivity:.3f}")
    print(f"\nThe field has been heard.")
