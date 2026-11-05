#!/usr/bin/env python3
"""Native waypoint walking acceptance in one fresh offline vanilla world.

The fixture constructs its own platform and markers. No account, installed
world, external server or existing settings are used. Focus/controller/manual
hardware behavior remains outside this authenticated native-input fixture.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import sys
import time

from game_feature_smoke import FeatureSmoke
from game_gui_smoke import AcceptanceFailure, Smoke, digest


class WaypointSmoke(FeatureSmoke):
    native_terrain = True

    def __init__(self, args):
        super().__init__(args)
        self.report.update(suite="native-waypoint-walk", script_sha256=digest(Path(__file__)),claims="Actual native route arrival/ownership/input assertions; screenshots require visual review.",
            not_exercised=["Real remote servers, latency, hardware controllers and physical focus loss are not covered.",
                           "The host planner uses coarse solid/fluid/lava cells, loaded terrain and a node budget; no arbitrary-shape safety or shortest-path claim.",
                           "World/dimension selection ownership has pure regression coverage; runtime scope-transfer coverage is not claimed."])

    def player(self):
        return self.command("state")["player"]

    def walk_state(self):
        return self.command("rusty-walk-status")

    def origin(self):
        self.command("release")
        self.server("tp @s 0.5 200 0.5 0 0")
        self.wait_for(self.player, lambda p: abs(p["position"][0]-.5)<.05 and abs(p["position"][1]-200)<.05 and abs(p["position"][2]-.5)<.05 and p["on_ground"] and not p["flying"], "grounded platform origin",timeout=30)

    def begin(self, method="control"):
        if method == "typed":
            self.typed_command("rusty waypoint walk")
        elif method == "ui":
            self.key("key.keyboard.right.shift")
            self.wait_for(self.ui, lambda state: state["open"] and bool(state["hits"]), "native route browser")
            self.search("waypoint walk")
            self.click('Select("waypoint_walk")')
            self.click('Toggle("waypoint_walk")')
            self.wait_for(self.ui, lambda state: not state["open"], "route action closes menu")
        else:
            self.command("rusty-walk-toggle")
        self.wait_for(self.walk_state, lambda s: s["active"], "explicit native route started")
        self.require(self.command("state")["busy"] == "rusty_waypoint_walk", "Native route did not own its named host task")

    def exercise(self):
        self.launch()
        for cmd in ["gamerule advance_time false", "gamerule spawn_mobs false", "time set noon", "weather clear",
                    "gamemode spectator @s", "tp @s 0.5 210 0.5 0 0"]:
            self.server(cmd)
        self.wait_for(self.player, lambda p:all(abs(p["position"][i]-[.5,210,.5][i])<.05 for i in range(3)), "loaded spectator origin", timeout=30)
        for x,z in [(-3,-3),(-3,29),(3,-3),(3,29)]:
            self.wait_for(lambda x=x,z=z:self.command(f"block {x} 199 {z}"),lambda b:b["loaded"],"platform corner chunk",timeout=60)
        self.server("fill -3 200 -3 3 205 29 air")
        self.server("fill -3 198 -3 3 199 29 stone")
        for x,z in [(-3,-3),(-3,29),(3,-3),(3,29),(0,0),(0,24)]:
            self.wait_for(lambda x=x,z=z:self.command(f"block {x} 199 {z}"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","verified platform support")
        self.server("tp @s 0.5 200 24.5 0 0")
        self.server("gamemode survival @s")
        self.wait_for(self.player, lambda p: p["on_ground"] and not p["flying"] and all(abs(p["position"][i]-[.5,200,24.5][i])<.05 for i in range(3)), "grounded destination",timeout=30)
        time.sleep(.25)
        self.typed_command("rusty waypoint add QA_Walk_Target")
        self.wait_waypoint("QA_Walk_Target")
        self.origin()

        self.phase = "ordinary host path arrival"
        self.begin("typed")
        end = self.wait_for(self.walk_state, lambda s: not s["active"] and s["arrived"], "route arrival through loaded platform", timeout=20)
        player = self.player()
        self.require(abs(player["position"][2]-24.5)<.75 and abs(player["position"][0]-.5)<.75, "Arrival flag did not match body destination")
        self.require(self.command("state")["busy"] is None, "Completed route retained a host task")
        self.shot("01-waypoint-arrived")
        self.passed("Explicit waypoint walk reaches selected marker using native host Goto", result=end, player=player)

        self.phase = "menu cancels without resuming"
        self.origin(); self.begin("ui")
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"], "native menu opened")
        cancelled = self.wait_for(self.walk_state, lambda s: not s["active"], "menu cancels route")
        self.require(self.command("state")["busy"] is None, "Menu cancellation retained owned task")
        self.key("key.keyboard.escape")
        before = self.player()["position"]; time.sleep(.5); after = self.player()["position"]
        self.require(not self.walk_state()["active"] and abs(after[2]-before[2])<.3, "Closing menu resumed walking")
        self.passed("Opening a menu cancels walking and closing it never restarts", result=cancelled)

        self.phase = "manual forward input takes ownership"
        self.origin(); self.begin()
        self.key("key.keyboard.w", "down")
        try:
            cancelled = self.wait_for(self.walk_state, lambda s: not s["active"], "physical-mapped W cancels route")
            before = self.player()["position"]; time.sleep(.5); after = self.player()["position"]
            self.require(after[2]-before[2]>.5, "Route cancellation swallowed the held manual forward key")
            self.require(self.command("state")["busy"] is None, "Manual takeover left owned host task")
        finally:
            self.key("key.keyboard.w", "up"); self.command("release")
        self.passed("Manual forward cancels only route input and remains held by the user", result=cancelled, distance=after[2]-before[2])

        self.phase = "virtual controller takeover"
        self.origin(); self.begin()
        self.command("pad stick left 0 -1")
        try:
            self.wait_for(self.walk_state, lambda s: not s["active"], "native controller movement cancels route")
            self.require(self.command("state")["busy"] is None, "Controller takeover retained route task")
            pad = self.command("pad status")
            self.require(pad["left"] == [0.0, -1.0], "Route cancellation overwrote native controller input")
        finally:
            self.command("pad release"); self.command("release")
        time.sleep(.3)
        self.require(not self.walk_state()["active"], "Controller release restarted route")
        self.passed("Native virtual controller movement cancels route while retaining controller state")

        self.phase = "existing task cannot be stolen"
        self.origin()
        with ThreadPoolExecutor(max_workers=1) as pool:
            waiting = pool.submit(self.command, "wait 200", 15)
            self.wait_for(lambda: self.command("state"), lambda s: s["busy"] == "wait", "independent owned fixture wait task")
            rejected = False
            try:
                self.command("rusty-walk-toggle")
            except AcceptanceFailure:
                rejected = True
            self.require(rejected and not self.walk_state()["active"], "Walking stole an existing host task")
            self.require(self.command("state")["busy"] == "wait", "Rejected route cancelled another task")
            self.command("stop")  # This fixture owns the wait task as well.
            try:
                waiting.result(timeout=5)
            except AcceptanceFailure:
                pass
        self.passed("Existing agent task is refused and survives rejected route start")

        self.phase = "Freecam cancels body walking"
        self.origin(); self.begin()
        self.key("key.keyboard.u")
        self.wait_for(self.state, lambda s: s["freecam"], "Freecam entered")
        self.wait_for(self.walk_state, lambda s: not s["active"], "Freecam cancels route")
        self.require(self.command("state")["busy"] is None, "Freecam left a body walking task")
        self.key("key.keyboard.u")
        self.wait_for(self.state, lambda s: not s["freecam"], "Freecam exited")
        self.require(not self.walk_state()["active"], "Freecam exit resumed walking")
        self.passed("Freecam cancels ordinary body route and does not resume it")

        self.phase = "death releases walking"
        self.origin(); self.begin(); self.server("kill @s")
        stopped = self.wait_for(self.walk_state, lambda s: not s["active"], "death cancels waypoint route")
        self.require(self.command("state")["busy"] is None, "Death retained a native route task")
        self.passed("Player death ends route ownership", result=stopped)

    def finish(self):
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"waypoint-walk-acceptance.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["binary", "resources", "output"]:
        parser.add_argument("--"+name, required=True, type=Path)
    parser.add_argument("--width", default=960, type=int)
    parser.add_argument("--height", default=540, type=int)
    parser.add_argument("--startup-timeout", default=240, type=float)
    args = parser.parse_args()
    smoke = None
    try:
        smoke = WaypointSmoke(args); smoke.exercise(); smoke.report["passed"] = True; return 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError, KeyboardInterrupt) as exc:
        if smoke:
            smoke.report["passed"] = False
            smoke.report["checks"].append({"name":smoke.phase, "passed":False, "reason":str(exc)})
        print("FAIL "+str(exc), file=sys.stderr, flush=True); return 1
    finally:
        if smoke: smoke.finish()


if __name__ == "__main__":
    raise SystemExit(main())
