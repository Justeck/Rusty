#!/usr/bin/env python3
"""Capture expanded default HUD columns and a real custom placement in an owned world.

Data assertions establish fixture setup; screenshot review accepts spacing.
Never connects to a server or reads an installed instance/account.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

from game_gui_smoke import AcceptanceFailure, Smoke, digest
from game_ui12_smoke import Ui12Smoke


class ColumnsSmoke(Ui12Smoke):
    native_terrain = True

    def __init__(self, args):
        super().__init__(args)
        self.report.update(suite="native-hud-columns", script_sha256=digest(Path(__file__)),
                           claims="Expanded default columns and one native custom-position edit; appearance requires image review.",
                           not_exercised=["Other viewport sizes, packs and graphics backends; this is a bounded composition fixture."])
        self.config_path.parent.mkdir(parents=True, exist_ok=True)
        self.config_path.write_text(json.dumps({
            "schema": 1, "hud": True, "finder_enabled": True, "hud_counts": True,
            "navigation": {"compass_enabled": True, "waypoint_hud": True},
            "utilities": {"equipment_hud": True, "environment_hud": True,
                          "container_preview": True, "preview_rows": 9},
            "awareness": {k: True for k in ("minimap", "fps_hud", "clicks_hud", "portal_coordinates",
                                             "clock_hud", "speed_hud", "target_info", "trajectories")},
            "client": {"hud_layout": {}},
        }) + "\n", encoding="utf-8")

    def exercise(self):
        self.launch()
        self.deadline = time.monotonic() + self.args.budget
        self.command("options-set guiScale 2")
        self.open_browser(); self.select("environment_hud"); self.close_browser()
        self.phase = "expanded received HUD data in a fixed normal-world scene"
        self.load_area(-10, -10, 26, 26, y=120)
        for text in ["gamerule advance_time false", "gamerule advance_weather false", "time set 6000", "weather clear",
                     "fill -10 119 -10 26 119 26 minecraft:stone",
                     "fill -10 120 -10 26 130 26 minecraft:air",
                     "fill 4 120 7 12 124 10 minecraft:stone",
                     "setblock 8 120 7 minecraft:oak_fence",
                     "setblock 5 121 9 minecraft:diamond_ore",
                     "setblock 7 121 9 minecraft:iron_ore",
                     "setblock 10 121 9 minecraft:gold_ore",
                     "tp @s 8.5 120 0.5 0 0", "gamemode survival",
                     "effect give @s minecraft:resistance 600 5 true"]:
            self.server(text)
        self.wait_for(lambda: self.command("state"), lambda s: s["player"]["on_ground"], "grounded on owned platform", timeout=30)
        self.typed_command("rusty waypoint add Column_Marker")
        self.wait_waypoint("Column_Marker")
        self.server("tp @s 8.5 120 3.5 0 18")
        entries = ','.join('{slot:%d,item:{id:"minecraft:%s",count:%d}}' %
                           (i, "diamond" if i % 2 == 0 else "stone", i+1) for i in range(10))
        self.server('item replace entity @s weapon.mainhand with minecraft:shulker_box[minecraft:container=['+entries+']] 1')
        held = self.wait_for(lambda: self.command("inventory")["held"],
                             lambda h: h and h.get("item") == "minecraft:shulker_box" and
                             "minecraft:container" in h.get("components", ""), "received ten-entry container")
        self.wait_for(lambda: self.command("block 8 120 7"),
                      lambda b: b["loaded"] and b["block"] == "minecraft:oak_fence", "received stateful fence")
        self.wait_for(self.state, lambda s: s.get("minimap") is not None and s.get("targets", 0) > 0,
                      "native map and received Finder results", timeout=60)
        self.wait_render_ready()
        self.wait_for(self.state, lambda s: s.get("pending") == 0 and s.get("targets", 0) > 0,
                      "stationary Finder queue drained", timeout=90)
        self.command("options-set chatVisibility 2")
        self.require(self.command("options-get chatVisibility")["options"]["chatVisibility"]["value"] == 2,
                     "Fixture chat must be hidden for expanded Target Info review")
        self.command("look 0 18")
        self.require(self.config()["client"]["hud_layout"] == {}, "Default fixture has unexpected custom placement")
        self.capture("01-default-expanded-columns", "No collisions between expanded TargetInfo/Speed, Finder/Compass/Waypoint, and Equipment/Environment/nine-row Container Preview above the minimap")
        self.passed("Owned scene supplies stateful target, Finder ores, waypoint and ten-entry container at default HUD positions",
                    supplied_entries=10, preview_rows=9, container_components=held["components"], image_acceptance="pending")
        self.phase = "independent Environment HUD native placement"
        self.open_browser(); self.select("environment_hud")
        for axis, fraction, want in [("x", 0, 0), ("y", .18, .18), ("scale", 1/6, .75)]:
            self.slider("hud:environment_hud:"+axis, fraction,
                        lambda c, a=axis, w=want: abs(c["client"]["hud_layout"].get("environment_hud", {}).get(a, -99)-w)<.001,
                        "saved Environment " + axis, pane="detail")
        self.require(set(self.config()["client"]["hud_layout"]) == {"environment_hud"}, "Editing Environment changed another placement")
        self.close_browser(); self.command("look 0 18")
        self.capture("02-custom-environment-placement", "Environment moves to a custom left position at 75 percent while other default panels continue fitting their columns")
        self.passed("Native X/Y/size sliders change only the Environment panel", image_acceptance="pending")

    def finish(self):
        self.deadline = None
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"hud-columns-acceptance.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("binary", "resources", "output"):
        parser.add_argument("--"+name, type=Path, required=True)
    parser.add_argument("--width", type=int, default=960)
    parser.add_argument("--height", type=int, default=540)
    parser.add_argument("--startup-timeout", type=float, default=300)
    parser.add_argument("--budget", type=float, default=360)
    args = parser.parse_args()
    if not 960 <= args.width <= 3840 or not 540 <= args.height <= 2160 or not 30 <= args.startup_timeout <= 900 or not 120 <= args.budget <= 900:
        parser.error("Use width960..3840, height540..2160, startup30..900 and budget120..900")
    smoke = None
    try:
        smoke = ColumnsSmoke(args); smoke.exercise(); smoke.report["passed"] = True
        return 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError, KeyboardInterrupt) as error:
        if smoke:
            smoke.report["passed"] = False
            smoke.report["checks"].append({"name": smoke.phase, "passed": False, "reason": str(error)})
        print("FAIL " + str(error), file=sys.stderr, flush=True)
        return 1
    finally:
        if smoke: smoke.finish()


if __name__ == "__main__":
    raise SystemExit(main())
