# -----------------------------------------------------------------
# ANWE — MNEMONIC PARTICIPANT
#
# Wraps Primordia's Mnemonic memory system as an ANWE participant.
# Mnemonic doesn't know about ANWE. This adapter translates between
# ANWE signal exchange and Mnemonic's memory operations.
#
# Signal mapping:
#   ATTENDING   → retrieve_context / search_memories
#   QUESTIONING → query_knowledge / search_memories
#   RECOGNIZING → detect_patterns / analyze_memory_patterns
#   DISTURBED   → get_health / get_status
#   APPLYING    → store_episodic_memory / store_semantic_knowledge
#   COMPLETING  → consolidate_memories
#   RESTING     → get_memory_statistics
#
# This is the first proof that ANWE can coordinate existing AI
# systems without changing their source code.
# -----------------------------------------------------------------

import logging
from typing import Any, Dict, Optional

from anwe_python import (
    WireSignal, ParticipantDescriptor,
    ATTENDING, QUESTIONING, RECOGNIZING, DISTURBED,
    APPLYING, COMPLETING, RESTING,
    INWARD, OUTWARD, BETWEEN,
)

logger = logging.getLogger("anwe.participant.mnemonic")


class MnemonicParticipant:
    """
    ANWE participant that wraps Mnemonic's memory system.

    Usage from Python:
        from primordia.mnemonic import MnemonicInterface
        mnemonic = MnemonicInterface(base_path="/path/to/data")

        participant = MnemonicParticipant(mnemonic)
        # Register with ANWE runtime via the bridge

    The ANWE runtime calls receive(), apply(), commit(), and descriptor().
    This class translates those calls into Mnemonic operations.
    """

    def __init__(self, mnemonic_interface, user_id: str = "anwe"):
        """
        Args:
            mnemonic_interface: An initialized MnemonicInterface (V2) instance.
            user_id: User ID for memory isolation. Defaults to "anwe".
        """
        self._mnemonic = mnemonic_interface
        self._user_id = user_id
        self._attention_level = 1.0
        self._signal_count = 0

    def descriptor(self) -> ParticipantDescriptor:
        """Return metadata about this participant."""
        return ParticipantDescriptor(
            name="Mnemonic",
            kind="python",
            address="primordia.mnemonic",
            version="2.0.0",
        )

    def receive(self, signal: WireSignal) -> Optional[WireSignal]:
        """
        Receive a signal from the ANWE runtime and respond.

        This is where ANWE signals become Mnemonic operations.
        Each signal quality maps to a different memory capability.
        """
        self._signal_count += 1
        quality = signal.quality

        try:
            if quality == ATTENDING:
                return self._handle_attending(signal)
            elif quality == QUESTIONING:
                return self._handle_questioning(signal)
            elif quality == RECOGNIZING:
                return self._handle_recognizing(signal)
            elif quality == DISTURBED:
                return self._handle_disturbed(signal)
            elif quality == APPLYING:
                return self._handle_applying(signal)
            elif quality == COMPLETING:
                return self._handle_completing(signal)
            elif quality == RESTING:
                return self._handle_resting(signal)
            else:
                logger.debug("Unknown signal quality %d, ignoring", quality)
                return None
        except Exception as e:
            logger.error("Error handling signal (quality=%d): %s", quality, e)
            return WireSignal(
                quality=DISTURBED,
                direction=OUTWARD,
                priority=0.9,
                data={"error": str(e), "source": "mnemonic"},
                confidence=1.0,
            )

    def apply(self, changes: dict) -> bool:
        """
        Apply structural changes from the ANWE runtime.

        Changes are key-value pairs that map to memory operations:
          "store_episodic" → store an episodic memory
          "store_semantic" → store semantic knowledge
          "store_memory"   → store via legacy interface
        """
        try:
            for key, value in changes.items():
                if key == "store_episodic":
                    self._store_episodic(value)
                elif key == "store_semantic":
                    self._store_semantic(value)
                elif key == "store_memory":
                    self._mnemonic.store_memory(value)
                else:
                    logger.debug("Unknown apply key '%s', storing as episodic", key)
                    content = str(value) if not isinstance(value, str) else value
                    self._mnemonic.store_episodic_memory(
                        content=content,
                        metadata={"source": "anwe", "key": key},
                        user_id=self._user_id,
                    )
            return True
        except Exception as e:
            logger.error("Apply failed: %s", e)
            return False

    def commit(self, entries: dict) -> None:
        """
        Commit permanent records from the ANWE runtime.

        Entries become episodic memories with high significance,
        marked as committed by the ANWE coordination layer.
        """
        try:
            for key, value in entries.items():
                content = value if isinstance(value, str) else str(value)
                self._mnemonic.store_episodic_memory(
                    content=content,
                    metadata={
                        "source": "anwe_commit",
                        "key": key,
                        "committed": True,
                        "signal_count": self._signal_count,
                    },
                    user_id=self._user_id,
                )
        except Exception as e:
            logger.error("Commit failed: %s", e)

    def attention(self) -> float:
        """
        Report current attention level.

        Based on Mnemonic's health — if memory system is degraded,
        lower attention so the runtime can redistribute focus.
        """
        try:
            health = self._mnemonic.get_health()
            status = health.get("status", "unknown")
            if status == "healthy":
                self._attention_level = 1.0
            elif status == "degraded":
                self._attention_level = 0.6
            else:
                self._attention_level = 0.3
        except Exception:
            self._attention_level = 0.5
        return self._attention_level

    # -----------------------------------------------------------------
    # Signal handlers — each quality maps to Mnemonic operations
    # -----------------------------------------------------------------

    def _handle_attending(self, signal: WireSignal) -> Optional[WireSignal]:
        """
        ATTENDING: Something requires memory's attention.
        Retrieve context relevant to the signal data.
        """
        query = self._extract_query(signal)
        if not query:
            return None

        result = self._mnemonic.retrieve_context(
            query=query,
            limit=5,
        )

        memories = result.get("memories", [])
        if not memories:
            return None

        return WireSignal(
            quality=RECOGNIZING,
            direction=OUTWARD,
            priority=signal.priority,
            data={
                "memories": memories,
                "count": len(memories),
                "query": query,
            },
            confidence=min(1.0, len(memories) / 5.0),
        )

    def _handle_questioning(self, signal: WireSignal) -> Optional[WireSignal]:
        """
        QUESTIONING: The system is asking memory a question.
        Search memories or query knowledge base.
        """
        query = self._extract_query(signal)
        if not query:
            return None

        # Try semantic knowledge first
        knowledge = self._mnemonic.query_knowledge(query)
        if knowledge:
            return WireSignal(
                quality=RECOGNIZING,
                direction=OUTWARD,
                priority=signal.priority * 1.1,
                data={
                    "source": "semantic",
                    "knowledge": knowledge,
                    "query": query,
                },
                confidence=0.9,
            )

        # Fall back to episodic search
        memories = self._mnemonic.search_memories(
            query=query,
            limit=10,
            user_id=self._user_id,
        )

        if not memories:
            return WireSignal(
                quality=QUESTIONING,
                direction=OUTWARD,
                priority=signal.priority * 0.5,
                data={"query": query, "found": False},
                confidence=0.1,
            )

        return WireSignal(
            quality=RECOGNIZING,
            direction=OUTWARD,
            priority=signal.priority,
            data={
                "source": "episodic",
                "memories": memories,
                "count": len(memories),
                "query": query,
            },
            confidence=min(1.0, len(memories) / 10.0),
        )

    def _handle_recognizing(self, signal: WireSignal) -> Optional[WireSignal]:
        """
        RECOGNIZING: Pattern detection request.
        Ask Mnemonic to find patterns in its memories.
        """
        try:
            patterns = self._mnemonic.analyze_memory_patterns()
            return WireSignal(
                quality=RECOGNIZING,
                direction=OUTWARD,
                priority=signal.priority,
                data={
                    "patterns": patterns,
                    "source": "mnemonic_analysis",
                },
                confidence=0.7,
            )
        except Exception:
            return None

    def _handle_disturbed(self, signal: WireSignal) -> Optional[WireSignal]:
        """
        DISTURBED: Something is wrong. Report health status.
        """
        try:
            health = self._mnemonic.get_health()
            status = self._mnemonic.get_status()
            return WireSignal(
                quality=ATTENDING,
                direction=OUTWARD,
                priority=0.9,
                data={
                    "health": health,
                    "status": status,
                    "source": "mnemonic_health",
                },
                confidence=1.0,
            )
        except Exception as e:
            return WireSignal(
                quality=DISTURBED,
                direction=OUTWARD,
                priority=1.0,
                data={"error": str(e), "source": "mnemonic_health_failed"},
                confidence=1.0,
            )

    def _handle_applying(self, signal: WireSignal) -> Optional[WireSignal]:
        """
        APPLYING: Store something in memory.
        The signal data becomes the memory content.
        """
        data = signal.data
        if data is None:
            return None

        try:
            if isinstance(data, str):
                memory_id = self._mnemonic.store_episodic_memory(
                    content=data,
                    metadata={"source": "anwe_signal", "priority": signal.priority},
                    user_id=self._user_id,
                )
            elif isinstance(data, dict):
                if "concept" in data and "knowledge" in data:
                    memory_id = self._mnemonic.store_semantic_knowledge(
                        concept=data["concept"],
                        knowledge=data["knowledge"],
                    )
                else:
                    content = data.get("content", str(data))
                    memory_id = self._mnemonic.store_episodic_memory(
                        content=content,
                        metadata={
                            "source": "anwe_signal",
                            "priority": signal.priority,
                            **{k: v for k, v in data.items() if k != "content"},
                        },
                        user_id=self._user_id,
                    )
            else:
                memory_id = self._mnemonic.store_episodic_memory(
                    content=str(data),
                    metadata={"source": "anwe_signal"},
                    user_id=self._user_id,
                )

            return WireSignal(
                quality=COMPLETING,
                direction=OUTWARD,
                priority=signal.priority,
                data={"stored": True, "memory_id": memory_id},
                confidence=1.0,
            )
        except Exception as e:
            return WireSignal(
                quality=DISTURBED,
                direction=OUTWARD,
                priority=0.9,
                data={"stored": False, "error": str(e)},
                confidence=1.0,
            )

    def _handle_completing(self, signal: WireSignal) -> Optional[WireSignal]:
        """
        COMPLETING: A cycle is finishing. Consolidate memories.
        """
        try:
            result = self._mnemonic.consolidate_memories(time_window_hours=24)
            return WireSignal(
                quality=COMPLETING,
                direction=OUTWARD,
                priority=signal.priority,
                data={
                    "consolidated": True,
                    "result": result,
                },
                confidence=0.9,
            )
        except Exception:
            return None

    def _handle_resting(self, signal: WireSignal) -> Optional[WireSignal]:
        """
        RESTING: Low activity period. Report statistics.
        """
        try:
            stats = self._mnemonic.get_memory_statistics()
            return WireSignal(
                quality=RESTING,
                direction=OUTWARD,
                priority=0.3,
                data={
                    "statistics": stats,
                    "signal_count": self._signal_count,
                },
                confidence=1.0,
            )
        except Exception:
            return None

    # -----------------------------------------------------------------
    # Helpers
    # -----------------------------------------------------------------

    def _extract_query(self, signal: WireSignal) -> Optional[str]:
        """Extract a search query from signal data."""
        data = signal.data
        if data is None:
            return None
        if isinstance(data, str):
            return data
        if isinstance(data, dict):
            return data.get("query") or data.get("content") or data.get("text")
        return str(data)

    def _store_episodic(self, value):
        """Store an episodic memory from an apply change."""
        if isinstance(value, str):
            self._mnemonic.store_episodic_memory(
                content=value,
                metadata={"source": "anwe_apply"},
                user_id=self._user_id,
            )
        elif isinstance(value, dict):
            self._mnemonic.store_episodic_memory(
                content=value.get("content", str(value)),
                metadata=value.get("metadata", {"source": "anwe_apply"}),
                user_id=self._user_id,
            )

    def _store_semantic(self, value):
        """Store semantic knowledge from an apply change."""
        if isinstance(value, dict):
            concept = value.get("concept", "unknown")
            knowledge = {k: v for k, v in value.items() if k != "concept"}
            self._mnemonic.store_semantic_knowledge(
                concept=concept,
                knowledge=knowledge,
            )
