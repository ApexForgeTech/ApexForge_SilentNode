"""
Phase 9 — Generative Audio System (Python layer)

Maps live workspace cognitive state to AudioParameters that the Rust
AudioEngine uses for procedural sound synthesis.

No audio output here — this is pure analysis.
"""

from silentnode_py.audio.generator import (
    AudioStateMapper,
    map_workspace_to_audio,
    map_workspace_to_audio_parametric,
)

__all__ = ["AudioStateMapper", "map_workspace_to_audio", "map_workspace_to_audio_parametric"]
