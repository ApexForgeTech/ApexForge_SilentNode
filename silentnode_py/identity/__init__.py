"""
Phase 10 — Identity & Narrative Systems

Living Signature computation and SVG rendering (pure Python + stdlib).
Personal Lore Chronicle and Hero's Journey Mapping.
"""

from silentnode_py.identity.signature import (
    LivingSignatureGenerator,
    compute_signature,
    render_signature_svg,
    render_signature_ascii,
)
from silentnode_py.identity.chronicle import (
    PersonalChronicle,
    HeroJourneyMapper,
    generate_chronicle,
    heroes_journey_narrative,
    heroes_journey_analysis,
)

__all__ = [
    "LivingSignatureGenerator",
    "compute_signature",
    "render_signature_svg",
    "render_signature_ascii",
    "PersonalChronicle",
    "HeroJourneyMapper",
    "generate_chronicle",
    "heroes_journey_narrative",
    "heroes_journey_analysis",
]
