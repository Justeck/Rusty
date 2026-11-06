#!/usr/bin/env python3
"""Owned native schematic acceptance; never opens a personal save or server.

Use the final candidate binary/resources and a NEW output directory. The normal
isolated Smoke launcher owns its process/world. This script never substitutes
server /item updates for a swap acknowledgment. Separate-server authority and
real Explorer Open/Save/Cancel/clipboard UI are additional parent-run checks.
"""
from __future__ import annotations
import argparse
import gzip
import json
from pathlib import Path
import struct
import subprocess
import sys
import time
from game_feature_smoke import FeatureSmoke
from game_gui_smoke import AcceptanceFailure, Smoke, digest


def schematic(path: Path, size, cells):
    """Small independent Sponge3 writer, unrelated to Rusty's codec."""
    def string(value):
        data = value.encode("utf-8")
        return struct.pack(">H", len(data)) + data
    def tag(kind, name, data): return bytes([kind]) + string(name) + data
    def integer(name, value): return tag(3, name, struct.pack(">i", value))
    palette = {"minecraft:air": 0}
    data = bytearray(size[0] * size[1] * size[2])
    for at, state in cells:
        palette.setdefault(state, len(palette))
        assert len(palette) < 128
        data[at[0] + size[0] * (at[2] + size[2] * at[1])] = palette[state]
    blocks = tag(10, "Palette", b"".join(integer(name, value) for name, value in palette.items()) + b"\0")
    blocks += tag(7, "Data", struct.pack(">i", len(data)) + data) + b"\0"
    body = integer("Version", 3) + integer("DataVersion", 0)
    body += b"".join(tag(2, axis, struct.pack(">h", value)) for axis, value in zip(("Width", "Height", "Length"), size))
    body += tag(10, "Blocks", blocks) + b"\0"
    path.write_bytes(gzip.compress(tag(10, "", tag(10, "Schematic", body) + b"\0"), mtime=0))


CELLS = [
    ([0, 0, 0], "minecraft:cobblestone"), ([1, 0, 0], "minecraft:oak_planks"),
    ([0, 0, 1], "minecraft:glass"), ([1, 0, 1], "minecraft:oak_log[axis=x]"),
    ([0, 0, 3], "minecraft:oak_slab[type=bottom,waterlogged=false]"),
    ([1, 0, 3], "minecraft:oak_stairs[facing=north,half=bottom,shape=straight,waterlogged=false]"),
    ([0, 0, 6], "minecraft:cobblestone"), ([0, 1, 6], "minecraft:oak_planks"),
    ([1, 0, 6], "minecraft:oak_door[facing=east,half=lower,hinge=left,open=false,powered=false]"),
    ([1, 1, 6], "minecraft:oak_door[facing=east,half=upper,hinge=left,open=false,powered=false]"),
    ([2, 0, 6], "minecraft:red_bed[facing=south,occupied=false,part=foot]"),
    ([2, 0, 7], "minecraft:red_bed[facing=south,occupied=false,part=head]"),
]
MATERIALS = {"minecraft:cobblestone": 2, "minecraft:oak_planks": 2, "minecraft:glass": 1,
             "minecraft:oak_log": 1, "minecraft:oak_slab": 1, "minecraft:oak_stairs": 1,
             "minecraft:oak_door": 1, "minecraft:red_bed": 1}


def descriptor(text):
    name, _, props = text.partition("[")
    return name, dict(pair.split("=", 1) for pair in props.rstrip("]").split(",") if pair)


class SchematicSmoke(FeatureSmoke):
    native_terrain = True
    def __init__(self, args):
        super().__init__(args)
        self.report.update(suite="native-schematic", script_sha256=digest(Path(__file__)),
            claims="Actual owned survival construction; received exact states, bounded walking, inventory preservation and explicit safety cancellation.",
            not_exercised=["OS Explorer Open/Save/Cancel and clipboard UI need separate actual desktop inspection.",
                "A separate vanilla server and deliberately rejected inventory click require parent authority fixtures.",
                "Focus loss, controller hardware and arbitrary third-party server plugins are not simulated here."])
        self.config_path.parent.mkdir(parents=True, exist_ok=True)
        self.config_path.write_text(json.dumps({"build": {"enabled": True}}))
        self.file = self.private / "materials_Русский.schem"
        schematic(self.file, [4, 3, 9], CELLS)
    def sch(self, operation="status", **args):
        value = self.command("rusty-schematic " + operation + " " + json.dumps(args, ensure_ascii=True))
        self.require("error" not in value, "Schematic operation failed: " + str(value))
        return value
    def player(self): return self.command("state")["player"]
    def start_or_permission_refusal(self):
        try:
            return False, self.sch("start")
        except AcceptanceFailure as exc:
            # Both owned control clients distinguish an authenticated game
            # error from transport/auth/JSON failures. The separate-server
            # client adds exactly this private-receipt notice; no prefix match.
            if str(exc) not in (
                "Game rejected the rusty-schematic command",
                "Game rejected the rusty-schematic command; see private control error receipt",
            ):
                raise
            result = self.sch()
            self.require(not result["active"] and result["result"] ==
                         "Construction requires Survival or Creative build permission",
                         "Start failed for an unrelated reason: " + str(result))
            return True, result
    def assert_damage_stop(self, before, after):
        self.require(not after["active"], "Damage did not stop construction")
        self.require(after["damage_receipts"] > before["damage_receipts"],
                     "No genuine received local-player damage event")
        self.require(after["ordinary_block_uses"] == before["ordinary_block_uses"]
                     and after["accepted_writes"] == before["accepted_writes"] == 0,
                     "Construction used or acknowledged a block after the safety baseline")
    def unchanged_world_inventory(self, world, inventory):
        after_world = self.command("blocks -3 200 0 7 204 18")
        after_inventory = self.command("inventory")
        self.require(after_world["blocks"] == world["blocks"], "Safety stop changed the received world")
        for key in ("counts", "hotbar", "main", "armor", "offhand", "carried"):
            self.require(after_inventory.get(key) == inventory.get(key), "Safety stop changed inventory " + key)
        return after_world, after_inventory
    def fixture(self, main=False, missing=False):
        self.sch("stop"); self.command("release")
        for cmd in ["gamemode spectator @s", "tp @s 0.5 210 0.5 0 0",
                    "fill -10 200 -5 12 207 19 air", "fill -10 199 -5 12 199 19 bedrock",
                    "clear @s", "kill @e[type=minecraft:item]", "effect clear @s", "effect give @s minecraft:instant_health 1 10",
                    "effect give @s minecraft:saturation 1 10"]: self.server(cmd)
        if main:
            for slot in range(9): self.server(f"item replace entity @s hotbar.{slot} with minecraft:stick {slot + 1}")
        if not missing:
            for slot, item in enumerate(MATERIALS):
                self.server(f"item replace entity @s {'inventory' if main else 'hotbar'}.{slot} with {item} {1 if item.endswith('_bed') else 64}")
        self.server("tp @s 0.5 200 0.5 0 0"); self.server("gamemode survival @s")
        self.wait_for(self.player, lambda p: p["on_ground"] and not p["flying"] and abs(p["position"][1] - 200) < .05, "grounded builder", timeout=30)
        self.sch("load", path=str(self.file))
        loaded = self.wait_for(self.sch, lambda s: not s["file_pending"] and not s["resolving"], "worker file import", timeout=20)
        self.require(loaded["cells"] == 12 and loaded["materials"] == MATERIALS, "Resolved material counts differ: " + str(loaded))
        self.sch("origin", position=[0, 200, 3])
        self.wait_for(self.sch, lambda s: not s["resolving"], "origin activation")
        time.sleep(.3)
    def completed(self, timeout=150):
        phases, maximum_z = [], self.player()["position"][2]
        until = time.monotonic() + timeout
        while time.monotonic() < until:
            value = self.sch(); phases.append(value["phase"])
            maximum_z = max(maximum_z, self.player()["position"][2])
            if not value["active"]:
                value["observed_phases"] = sorted({p for p in phases if p})
                value["maximum_z"] = maximum_z
                return value
            time.sleep(.1)
        raise AcceptanceFailure("Construction exceeded fixture deadline: " + str(self.sch()))
    def build_case(self, main):
        self.fixture(main=main)
        before = self.command("inventory")
        self.sch("start"); result = self.completed()
        self.require(result["result"].startswith("Exact block-state verification complete"), str(result))
        terminal = self.wait_for(self.sch, lambda s: not s["terminal_survey_pending"], "terminal exact-state summary")
        outcome = terminal["outcome"]
        self.require(outcome["initially_matching"] == 0 and outcome["finally_matching"] == 12
                     and outcome["completed"] == 12 and outcome["remaining_missing"] == 0
                     and outcome["unsupported_inaccessible"] == 0 and outcome["remaining_manual"] == 0
                     and outcome["survey_complete"], "Terminal distinctions differ: " + str(outcome))
        result["outcome"] = outcome
        self.require(result["maximum_z"] > 4 and "walking" in result["observed_phases"], "Far targets did not prove ordinary walking")
        if main: self.require(result["inventory_confirmed"] >= 8, "Main-inventory sourcing did not confirm its swaps")
        observed = []
        for local, expected in CELLS:
            at = [local[0], 200 + local[1], 3 + local[2]]
            actual = self.command("block " + " ".join(map(str, at)))
            name, props = descriptor(actual["state"]); wanted, properties = descriptor(expected)
            self.require(actual["loaded"] and name == wanted and properties.items() <= props.items(), f"Exact state mismatch at {at}: {actual}")
            observed.append(actual)
        outside = self.command("blocks -3 200 0 7 204 15")
        self.require(len(outside["blocks"]) == len(CELLS), "Construction produced unplanned extra blocks")
        after = self.command("inventory")
        for item, cost in MATERIALS.items():
            self.require(before["counts"].get(item, 0) - after["counts"].get(item, 0) == cost, "Material consumption differs for " + item)
        if main: self.require(after["counts"].get("minecraft:stick") == 45, "A displaced full-hotbar stack was lost")
        self.passed("Full-hotbar main-inventory build" if main else "Directional and coupled hotbar build", result=result, blocks=observed, before=before, after=after)
    def partial(self):
        self.fixture()
        path = self.private / "double-slab.schem"
        schematic(path, [1, 1, 1], [([0, 0, 0], "minecraft:oak_slab[type=double,waterlogged=false]")])
        self.sch("load", path=str(path)); self.wait_for(self.sch, lambda s: not s["file_pending"] and not s["resolving"], "double slab import")
        self.sch("start"); result = self.completed()
        actual = self.command("block 0 200 3")
        self.require(result["result"].startswith("Exact block-state verification complete") and "type=double" in actual["state"], str(result))
        self.require(result["accepted_writes"] == 2, "Double slab was not acknowledged in two ordinary stages")
        self.passed("Double slab final state after two ordinary acknowledgments", result=result, block=actual)
    def safety(self, case):
        if case == "denied":
            return self.denied()
        if case in ("health", "absorbed", "revoked"):
            return self.walking_safety(case)
        self.fixture(missing=case == "missing")
        self.sch("start")
        if case == "manual": self.key("key.keyboard.w", "down")
        elif case == "menu": self.key("key.keyboard.right.shift")
        elif case == "scope": self.server("execute in minecraft:the_nether run tp @s 0.5 200 0.5")
        try: result = self.completed(timeout=40)
        finally:
            if case == "manual": self.key("key.keyboard.w", "up"); self.command("release")
            if case == "menu": self.key("key.keyboard.escape")
        self.require(not result["result"].startswith("Exact block-state verification complete"), "Unsafe run falsely completed")
        time.sleep(.5); self.require(not self.sch()["active"], "Construction automatically resumed")
        if case == "scope": self.require(self.sch()["cells"] == 0 and self.sch()["corners"] == [None, None], "Scope transition retained schematic/selection")
        self.passed("Construction safety stop: " + case, result=result)
    def denied(self):
        self.fixture()
        self.server("gamemode adventure @s")
        denied = self.wait_for(lambda: self.command("state"), lambda s: s["gamemode"] == "adventure",
                               "received Adventure mode")
        before = self.sch()
        world = self.command("blocks -3 200 0 7 204 18")
        inventory = self.command("inventory")
        refused, result = self.start_or_permission_refusal()
        if not refused:
            result = self.completed(timeout=3)
        self.require(not result["active"] and result["accepted_writes"] == (before["accepted_writes"] if refused else 0)
                     and result["ordinary_block_uses"] == before["ordinary_block_uses"]
                     and result["inventory_confirmed"] == (before["inventory_confirmed"] if refused else 0)
                     and "build permission" in result["result"], "Denied permission did not promptly stop: " + str(result))
        self.server("gamemode survival @s")
        restored = self.wait_for(lambda: self.command("state"), lambda s: s["gamemode"] == "survival",
                                 "received restored Survival mode")
        time.sleep(.5)
        final = self.sch()
        self.require(not final["active"] and final["ordinary_block_uses"] == before["ordinary_block_uses"],
                     "Permission restoration automatically resumed construction")
        after_world, after_inventory = self.unchanged_world_inventory(world, inventory)
        self.passed("Construction safety stop: denied", refused_at_start=refused, received_mode=denied["gamemode"],
                    restored_mode=restored["gamemode"], result=final, before=inventory, after=after_inventory,
                    world_before=world, world_after=after_world)
    def walking_safety(self, case):
        self.fixture()
        # A single target beyond normal reach guarantees no nearby placement
        # can race the damage command. The original 12-cell build cases remain
        # unchanged and are still exercised separately in this same suite.
        path = self.private / "distant-safety.schem"
        schematic(path, [1, 1, 1], [([0, 0, 0], "minecraft:cobblestone")])
        self.sch("load", path=str(path))
        self.wait_for(self.sch, lambda s: not s["file_pending"] and not s["resolving"], "distant safety import")
        self.sch("origin", position=[0, 200, 11])
        self.wait_for(self.sch, lambda s: not s["resolving"], "distant safety origin")
        if case == "absorbed":
            self.server("effect give @s minecraft:absorption 60 0")
        condition = self.wait_for(self.player, lambda p: p["health"] == 20 and p["absorption"] == (4 if case == "absorbed" else 0),
                                  "received safety health/absorption baseline")
        world = self.command("blocks -3 200 0 7 204 18")
        inventory = self.command("inventory")
        self.sch("start")
        baseline = self.wait_for(self.sch, lambda s: s["active"] and s["phase"] == "walking", "active distant-target walking", timeout=15)
        walking = self.player()
        self.require(baseline["accepted_writes"] == 0 and abs(walking["position"][2] - 11) > 6,
                     "Safety baseline was not walking safely beyond placement reach")
        if case == "revoked":
            self.server("gamemode adventure @s")
            denied = self.wait_for(lambda: self.command("state"), lambda s: s["gamemode"] == "adventure", "received permission revocation")
        else:
            self.server("damage @s 1 minecraft:generic")
        result = self.completed(timeout=3)
        if case == "revoked":
            self.require(not result["active"] and result["accepted_writes"] == 0
                         and result["ordinary_block_uses"] == baseline["ordinary_block_uses"]
                         and "build permission" in result["result"], "Active permission revocation was not prompt")
            self.server("gamemode survival @s")
            self.wait_for(lambda: self.command("state"), lambda s: s["gamemode"] == "survival", "received restored permission")
        else:
            self.assert_damage_stop(baseline, result)
            condition_after = self.wait_for(self.player,
                lambda p: (p["health"] == 20 and p["absorption"] == 3) if case == "absorbed" else p["health"] < condition["health"],
                "received absorbed/ordinary damage result")
            self.server("effect give @s minecraft:instant_health 1 10")
        time.sleep(.5)
        final = self.sch()
        self.require(not final["active"] and final["ordinary_block_uses"] == baseline["ordinary_block_uses"]
                     and final["accepted_writes"] == 0, "Safety recovery automatically resumed or placed")
        after_world, after_inventory = self.unchanged_world_inventory(world, inventory)
        self.passed("Construction safety stop: " + case, result=final, baseline=baseline, walking=walking,
                    condition_before=condition, condition_after=denied if case == "revoked" else condition_after,
                    before=inventory, after=after_inventory, world_before=world, world_after=after_world)
    def coupled(self):
        self.fixture()
        path = self.private / "missing-door-upper.schem"
        schematic(path, [1, 1, 1], [([0, 0, 0], "minecraft:oak_door[facing=south,half=lower,hinge=left,open=false,powered=false]")])
        self.sch("load", path=str(path)); self.wait_for(self.sch, lambda s: not s["file_pending"] and not s["resolving"], "incomplete door import")
        self.sch("start"); result = self.completed()
        self.require(result["accepted_writes"] == 0 and self.command("block 0 200 3")["block"] == "minecraft:air" and self.command("block 0 201 3")["block"] == "minecraft:air", "An unplanned coupled write escaped the dry validation")
        self.passed("Door outside the blueprint is refused before any write", result=result)
    def roundtrip(self):
        self.fixture()
        for local, state in CELLS: self.server(f"setblock {local[0]} {200 + local[1]} {3 + local[2]} {state}")
        self.sch("first", position=[0, 200, 3]); self.sch("second", position=[3, 202, 11])
        path = self.private / "selection_Русский.schem"
        self.sch("save", path=str(path))
        saved = self.wait_for(self.sch, lambda s: not s["file_pending"] and s["export_scanned"] is None, "received states-only export", timeout=20)
        self.require(path.is_file() and saved["result"].startswith("Saved "), str(saved))
        self.sch("clear"); self.sch("load", path=str(path))
        loaded = self.wait_for(self.sch, lambda s: not s["file_pending"] and not s["resolving"], "export re-import")
        self.require(loaded["cells"] == 12 and loaded["materials"] == MATERIALS, "Export/import did not retain exact states/materials")
        self.sch("copy"); self.passed("Inclusive received states-only export/import and clipboard action", saved=saved, loaded=loaded, export_sha256=digest(path))
    def survey(self):
        # Actual owned-world block/bulk changes between bounded scan batches. This does not
        # claim network chunk replacement/unload coverage or fabricate received packets.
        self.fixture()
        self.load_area(0, 0, 63, 63, y=250)
        for lo, hi in ((220, 227), (228, 235)):
            self.server(f"fill 0 {lo} 0 63 {hi} 63 stone")
        self.settle_fixture_body(0.5, 200, -2.5, creative=False)
        path = self.private / "large-coherent-survey.schem"
        cells = [([x, y, z], "minecraft:stone") for y in range(16) for z in range(64) for x in range(64)]
        schematic(path, [64, 16, 64], cells)
        self.sch("load", path=str(path))
        self.wait_for(self.sch, lambda v: not v["file_pending"] and not v["resolving"], "large plan validation", timeout=30)
        self.sch("origin", position=[0, 220, 0])
        self.wait_for(self.sch, lambda v: not v["resolving"], "large plan transform", timeout=30)
        for kind, command in (("block", "setblock 0 220 0 bedrock"),
                              ("bulk", "fill 0 220 0 15 220 15 bedrock strict")):
            self.server("fill 0 220 0 15 220 15 stone strict")
            self.sch("start")
            before = self.wait_for(self.sch, lambda v: v["active"] and (v.get("survey_scanned") or 0) >= 2048,
                                   "first survey batch", timeout=10)
            self.server(command)
            result = self.wait_for(self.sch, lambda v: not v["active"], "invalidated survey stops incomplete", timeout=30)
            self.require(not result["result"].startswith("Exact block-state verification complete"), "Stale completion after " + kind)
            final = self.wait_for(self.sch, lambda v: not v["terminal_survey_pending"], "coherent terminal counts", timeout=70)
            self.require(final["outcome"]["survey_complete"] and final["outcome"]["occupied"] > 0, str(final))
            self.passed("Already-scanned target invalidated by actual " + kind + " mutation", before=before, result=final)
        self.server("fill 0 220 0 15 220 15 stone strict")
        self.sch("start")
        result = self.wait_for(self.sch, lambda v: not v["active"], "stable large verification", timeout=30)
        self.require(result["result"].startswith("Exact block-state verification complete"), str(result))
        self.passed("Stable multi-batch survey completes after retry", result=result)
        self.sch("start")
        self.wait_for(self.sch, lambda v: v["active"] and (v.get("survey_scanned") or 0) >= 2048,
                      "stop during partial survey", timeout=10)
        stopped = self.sch("stop")
        self.require(not stopped["active"] and not stopped["outcome"]["survey_complete"], str(stopped))
        final = self.wait_for(self.sch, lambda v: not v["terminal_survey_pending"], "read-only post-stop survey", timeout=70)
        self.require(not final["active"] and final["outcome"]["survey_complete"], str(final))
        self.passed("Stopped multi-batch survey refreshes without resuming", result=final)

    def exercise(self):
        self.launch()
        for cmd in ["gamerule advance_time false", "gamerule spawn_mobs false", "gamerule keep_inventory true", "time set noon", "weather clear"]: self.server(cmd)
        self.load_area(-10, -5, 12, 19, y=210)
        for case in self.args.cases.split(","):
            self.phase = case
            if case == "main": self.build_case(True)
            elif case == "directional": self.build_case(False)
            elif case == "partial": self.partial()
            elif case == "coupled": self.coupled()
            elif case == "roundtrip": self.roundtrip()
            elif case == "survey": self.survey()
            else: self.safety(case)
    def finish(self):
        Smoke.finish(self)
        (self.output / "gui-acceptance.json").replace(self.output / "schematic-acceptance.json")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ("binary", "resources", "output"): p.add_argument("--" + name, required=True, type=Path)
    p.add_argument("--cases", default="directional,main,partial,coupled,roundtrip,missing,manual,health,absorbed,menu,denied,revoked,survey,scope")
    p.add_argument("--width", type=int, default=960); p.add_argument("--height", type=int, default=540)
    p.add_argument("--startup-timeout", type=float, default=240)
    args = p.parse_args(); smoke = None
    try:
        smoke = SchematicSmoke(args); smoke.exercise(); smoke.report["passed"] = True; return 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError, KeyboardInterrupt) as exc:
        if smoke: smoke.report["passed"] = False; smoke.report["checks"].append({"name": smoke.phase, "passed": False, "reason": str(exc)})
        print("FAIL " + str(exc), file=sys.stderr, flush=True); return 1
    finally:
        if smoke: smoke.finish()


if __name__ == "__main__": raise SystemExit(main())
