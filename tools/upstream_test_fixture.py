"""Pinned upstream test fixture correction. No production code changes.

9aa4be30 Resources.rgb is a u16 bitmask; one cfg(test) initializer omitted it.
This script is exclusively for generated test stages, not the pristine SDK.
"""
from pathlib import Path
import argparse,hashlib,json
PIN="9aa4be30f8ebde1b681843000d046306b9b6b957"
FIXED_PIN="6475f62f3c834b7489e1816bf47f47ea5eb2b879"
FIXED_HASH="177f0596e8c49f067bd400c646c1792c940f3c5cb35aa50b83221cc5ac637f01"
REL=Path("crates/horizon-client-render/src/gfx/pack_post.rs")
BEFORE="f9bb1346deebb6c0080d8581c4c2e680cab8d7a79682fc97c2298b336a1fa5ba"
AFTER="674a0c61e4ce95f4ceeb45d3a2bf2882330244e98a709f7fd178a0a85752e274"
OLD='            storable: 0,\n            released: std::cell::Cell::new(("", 0)),\n            stand_ins: &stand_ins,\n'
NEW='            storable: 0,\n            rgb: 0,\n            released: std::cell::Cell::new(("", 0)),\n            stand_ins: &stand_ins,\n'
def patch_text(text):
    digest=hashlib.sha256(text.encode()).hexdigest()
    if digest==AFTER:return text
    if digest!=BEFORE:raise ValueError("Unrecognized upstream test source; inspect before adapting the fixture")
    if text.count(OLD)!=1:raise ValueError("Expected one test fixture initializer")
    test_start=text.index("#[cfg(test)]\nmod tests {")
    test_fn=text.index("fn a_composite_program_blends_onto_its_buffer()",test_start)
    site=text.index(OLD)
    if site<test_fn:raise ValueError("Fixture initializer is outside its expected cfg(test) function")
    result=text.replace(OLD,NEW,1)
    if result[:test_start]!=text[:test_start]:raise ValueError("Production source changed")
    if hashlib.sha256(result.encode()).hexdigest()!=AFTER:raise ValueError("Patched fixture hash differs")
    return result

def apply_stage(stage, revision):
    """Correct only the known cfg(test) fixture in a generated test stage."""
    stage = Path(stage).resolve()
    if revision not in (PIN, FIXED_PIN) or (stage / ".horizon-revision").read_text().strip() != revision:
        raise ValueError("Unexpected SDK pin for test-only fixture correction")
    if stage.name != "stage" and not stage.name.startswith("stage-"):
        raise ValueError("Test fixture correction requires a generated stage directory")
    path = stage / REL
    before = path.read_bytes().decode("utf-8")
    if revision == FIXED_PIN:
        digest = hashlib.sha256(before.encode()).hexdigest()
        if digest != FIXED_HASH:
            raise ValueError("Unrecognized fixed upstream test source")
        return {"pin": revision, "file": REL.as_posix(), "before_sha256": digest,
                "after_sha256": digest, "scope": "unmodified upstream cfg(test) fixture",
                "change": "none; upstream fixture already fixed"}
    after = patch_text(before)
    if before != after:
        path.write_bytes(after.encode("utf-8"))
    return {"pin": PIN, "file": REL.as_posix(), "before_sha256": BEFORE,
            "after_sha256": AFTER, "scope": "cfg(test) initializer only", "change": "rgb: 0"}
