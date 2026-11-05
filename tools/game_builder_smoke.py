#!/usr/bin/env python3
"""Bounded native Build Planner checks in a fresh disposable integrated world.

This checks real native UI actions, received blocks/inventory and an independent
server-side block predicate. It is not external Java multiplayer acceptance or
proof of visual quality. No reference client or existing user world is opened.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time
from game_feature_smoke import FeatureSmoke
from game_gui_smoke import Smoke, AcceptanceFailure, canonical_digest, digest


class BuilderSmoke(FeatureSmoke):
    native_terrain = True

    def __init__(self, args):
        super().__init__(args)
        self.report.update(
            suite="native-build-planner", script_sha256=digest(Path(__file__)),
            claims="Actual bounded preview, native GUI single placement, server acknowledgment and authoritative fixture predicate.",
            not_exercised=["External Java multiplayer, high latency, controller remappings, focus loss, multi-block items and custom component packs.",
                           "Screenshots require separate visual review; source geometry counts are asserted.",
                           "The negative cancellation case uses the native control facade while normal placement uses real GUI actions."],
        )
        for key in ["pixel_contract", "fixture_correction"]:
            self.report.pop(key, None)

    def build(self):
        return self.state()["build"]

    def photo(self, name, expected_pose=None):
        # Preserve the HUD and world gizmos while hiding fixture command chat,
        # which otherwise covers the planner's own status text.
        previous = self.command("options-get chatVisibility")["options"]["chatVisibility"]["value"]
        self.command("options-set chatVisibility 2")
        try:
            result = super().photo(name, expected_pose)
            self.report.setdefault("capture_chat_visibility", []).append(
                {"name": name, "during_capture": 2, "restored_to": previous})
            return result
        finally:
            self.command(f"options-set chatVisibility {previous}")

    def builder_menu(self):
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "builder menu")
        self.search("build planner")
        self.click('Select("build_planner")')
        self.top_details()

    def action(self, name):
        self.builder_menu()
        self.click(name, pane="detail")
        self.wait_for(self.ui, lambda s: not s["open"], "explicit builder action closes menu")

    def maximum_dimension(self, name):
        rect = self.hit(f'Numeric("{name}",', pane="detail")["rect"]
        y = rect[1] + rect[3] / 2
        self.command(f"drag-at {rect[0]+rect[2]/2:.3f} {y:.3f} {rect[0]+rect[2]-.001:.3f} {y:.3f}")

    def exercise(self):
        self.launch()
        self.builder_menu()
        self.wait_for(self.config, lambda c: "build" in c, "builder preferences saved")
        self.key("key.keyboard.escape")
        self.phase = "supported survival building fixture"
        self.server("gamemode spectator")
        self.load_area(-8, -8, 12, 12, y=200)
        for command in ["gamerule advance_time false", "gamerule advance_weather false", "gamerule spawn_mobs false",
                        "time set noon", "weather clear", "fill -8 199 -8 12 199 12 minecraft:stone",
                        "fill -8 200 -8 12 216 12 minecraft:air", "tp @s 0.5 200 -2.5 0 22.2",
                        "gamemode survival", "effect give @s minecraft:resistance 600 5 true",
                        "item replace entity @s hotbar.0 with minecraft:cobblestone 64"]:
            self.server(command)
        self.command("hotbar 1")
        self.wait_for(lambda: self.command("state")["player"], lambda p: p["on_ground"] and abs(p["position"][1]-200)<.01, "grounded fixture body")
        self.wait_for(lambda: self.command("inventory"), lambda inv: inv["held"].get("item")=="minecraft:cobblestone" and inv["held"].get("count")==64, "received building material")
        self.command("look 0 22.2")
        before = self.command("inventory")
        self.set_module("build_planner", ["build", "enabled"])
        self.action("BuildAnchor")
        anchored = self.wait_for(self.build, lambda s: s["origin"] is not None and s["cells"]>0, "received-face anchored preview")
        self.require(anchored["origin"]==[0,200,1], "Fixture look did not select the intended supported origin")
        self.require(anchored["cells"]<=4096 and anchored["shown"]<=128 and anchored["depth_lines"]<=1536, "Preview exceeded geometry budget")
        self.require(canonical_digest(before)==canonical_digest(self.command("inventory")), "Read-only preview changed inventory")
        self.require(self.command("block 0 200 1")["block"]=="minecraft:air", "Read-only preview placed a block")
        self.photo("01-bounded-plan-preview")
        self.passed("Anchored preview reads received terrain with bounded geometry and unchanged inventory", preview=anchored)

        self.phase = "one explicit ordinary placement and authoritative confirmation"
        self.action("BuildPlace")
        settled = self.wait_for(self.build, lambda s: s["accepted"]==1 and not s["busy"], "native prediction settled to matching server state", timeout=8)
        block = self.wait_for(lambda: self.command("block 0 200 1"), lambda b: b["loaded"] and b["block"]=="minecraft:cobblestone", "received final block")
        inventory = self.wait_for(lambda: self.command("inventory"), lambda inv: inv["held"].get("count")==63, "ordinary survival item consumption")
        marker = "RUSTY_BUILDER_AUTHORITATIVE_ONE"
        self.server(f"execute if block 0 200 1 minecraft:cobblestone run say {marker}")
        message = self.wait_for(lambda: self.command("messages 20"), lambda m: marker in json.dumps(m), "independent server-side final block predicate")
        time.sleep(1)
        self.require(self.build()["accepted"]==1 and not self.build()["busy"], "Builder continued without another explicit action")
        self.require(self.command("inventory")["held"].get("count")==63, "Builder placed another block without an explicit request")
        self.photo("02-one-confirmed-placement")
        self.passed("Place Next dispatches one ordinary action, consumes one item and waits for server confirmation", settled=settled, block=block, inventory=inventory, server_predicate=message)

        self.phase = "manual input cancels queued placement without resuming"
        self.key("key.keyboard.w", "down")
        try:
            self.command("rusty-build-place")
            cancelled = self.wait_for(self.build, lambda s: not s["busy"] and "manual input" in (s.get("result") or ""), "held manual movement cancels queued request", timeout=3)
        finally:
            self.key("key.keyboard.w", "up")
        time.sleep(.6)
        self.require(self.build()["accepted"]==1 and not self.build()["busy"], "Cancelled request resumed after manual release")
        self.require(self.command("inventory")["held"].get("count")==63, "Cancelled request consumed a block")
        self.passed("Held movement cancels the queued action permanently before dispatch", cancelled=cancelled)

        self.phase = "4096-cell plan and 128-cell display budget"
        self.builder_menu()
        for name in ["build_width", "build_depth", "build_height"]:
            self.maximum_dimension(name)
        self.click('Bool("build_hollow")', pane="detail")
        self.wait_for(self.config, lambda c: all(c["build"][name]==16 for name in ["width","depth","height"]) and not c["build"]["hollow"], "maximum solid plan preferences")
        self.key("key.keyboard.escape")
        bounded = self.wait_for(self.build, lambda s: s["cells"]==4096 and s["shown"]==128, "bounded large plan preview")
        self.require(bounded["depth_lines"]==1536 and bounded["accepted"]==1 and not bounded["busy"], "Large preview did not remain bounded and passive")
        self.passed("Maximum plan computes at most4096cells and draws128nearest without placement", preview=bounded)
        self.set_module("build_planner", ["build","enabled"], False)
        disabled = self.wait_for(self.build, lambda s: s["cells"]==0 and s["depth_lines"]==0 and not s["busy"], "disabled plan and action clear")
        self.passed("Disabling clears transient anchor, geometry and pending action", disabled=disabled)

    def finish(self):
        Smoke.finish(self)
        source = self.output / "gui-acceptance.json"
        if source.exists():
            source.replace(self.output / "builder-acceptance.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--resources", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--width", default=960, type=int)
    parser.add_argument("--height", default=540, type=int)
    parser.add_argument("--startup-timeout", default=300, type=float)
    args = parser.parse_args()
    smoke = None
    try:
        smoke = BuilderSmoke(args)
        smoke.exercise()
        smoke.report["passed"] = True
        return 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError, KeyboardInterrupt) as exc:
        if smoke:
            smoke.report["passed"] = False
            smoke.report["checks"].append({"name":smoke.phase, "passed":False, "reason":str(exc)})
        print("FAIL " + str(exc), file=sys.stderr, flush=True)
        return 1
    finally:
        if smoke:
            smoke.finish()


if __name__ == "__main__":
    raise SystemExit(main())
