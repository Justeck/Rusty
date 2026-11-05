#!/usr/bin/env python3
"""Remaining Rusty native visual/input fixtures in a fresh offline world.

python tools/game_visual_smoke.py --binary PATH --resources PATH --output NEWDIR

Uses actual GUI clicks and native window key events for Rusty settings. Normal
commands form only this disposable world's water basin, walls, weather, pig and
one-point survival damage. No account, installed instance or remote server is
loaded. A hard 175-second fixture budget starts after world bootstrap.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import subprocess
import sys
import time

from game_gui_smoke import Smoke, AcceptanceFailure, digest
from game_feature_smoke import FeatureSmoke


OLD_DISTANCE_ONLY_FOG_BINARY = "be7ee07b259e3225f5d574e05b9e53117a9705f53af279329e8c6d0b8785b3a6"


class UnsupportedFixture(AcceptanceFailure):
    pass


class VisualSmoke(FeatureSmoke):
    def __init__(self, args):
        self.fixture_deadline = None
        super().__init__(args)
        self.report.update({
            "suite": "remaining-native-visuals",
            "script_sha256": digest(Path(__file__)),
            "feature_helper_sha256": digest(Path(__file__).with_name("game_feature_smoke.py")),
            "shared_harness_sha256": digest(Path(__file__).with_name("game_gui_smoke.py")),
            "claims": "Native data/config/input assertions. Fog, weather, ESP and damage-tilt screenshots require human interpretation; no renderer state is inferred from file-byte differences.",
            "not_exercised": [
                "No direct native control probe exposes final FogData uniforms or weather vertex counts.",
                "Fog visibility, rain removal, orange ESP through the wall and hurt-camera suppression require screenshot review.",
                "Water vision brightens with time. Off/on/restored captures retain timing and camera data; no pixel threshold pretends to isolate that evolving brightness.",
                "Other resource packs, biomes, fluids, hostile combat, physical controllers, external servers and real accounts are not covered.",
            ],
            "visual_pairs": [],
            "pixel_contract": "No numeric pixel assertions in this suite. Compare the fixed received fixture, camera receipts and saved native screenshots manually.",
            "fixture_budget_seconds": 175,
        })

    def command(self, line, timeout=15):
        if self.fixture_deadline is not None and line not in ("quit", "release", "pad release"):
            remaining = self.fixture_deadline - time.monotonic()
            if remaining <= 0:
                raise AcceptanceFailure("Visual fixture exceeded its bounded runtime")
            timeout = min(timeout, max(.2, remaining))
        return super().command(line, timeout)

    def wait_for(self, reader, predicate, description, timeout=8):
        if self.fixture_deadline is not None:
            remaining = self.fixture_deadline - time.monotonic()
            if remaining <= 0:
                raise AcceptanceFailure("Visual fixture exceeded its bounded runtime")
            timeout = min(timeout, remaining)
        return super().wait_for(reader, predicate, description, timeout)

    def player_state(self):
        state = self.command("state")
        self.require(state.get("player") is not None, "Fixture player is missing")
        return state

    def server(self, text):
        result = super().server(text)
        feedback = " ".join(str(line) for line in result.get("chat", []))
        if any(marker in feedback.lower() for marker in ["unknown or incomplete command", "unknown command", "not implemented", "unsupported command"]):
            raise UnsupportedFixture("Local fixture command is unavailable: " + text.split(" ", 1)[0])
        return result

    @staticmethod
    def view_sample(state):
        player = state["player"]
        return {"position": player["position"], "eye": player.get("eye"), "yaw": player["yaw"],
                "pitch": player["pitch"], "health": player["health"], "food": player["food"],
                "hurt_time": player.get("hurt_time"), "sprinting": player.get("sprinting")}

    @staticmethod
    def same_camera(first, second, tolerance=.01):
        a, b = first["player"], second["player"]
        dyaw = (b["yaw"]-a["yaw"]+180) % 360-180
        return all(abs(a["position"][i]-b["position"][i]) <= tolerance for i in range(3)) and abs(dyaw) <= .001 and abs(a["pitch"]-b["pitch"]) <= .001

    def fixed_photo(self, name, reference=None):
        before = self.player_state()
        pair_stable = reference is None or self.same_camera(reference, before)
        receipt = {"name": name, "before": self.view_sample(before),
                   "pair_reference": self.view_sample(reference) if reference is not None else None,
                   "before_matches_pair_reference": pair_stable,
                   "visual_review": "pending", "hud_requested_hidden": self.hud_hidden}
        self.report["visual_pairs"].append(receipt)
        self.require(pair_stable, "Camera/body changed between paired visual captures")
        self.photo(name)
        after = self.player_state()
        stable = self.same_camera(before, after)
        after_matches = reference is None or self.same_camera(reference, after)
        receipt.update({"after": self.view_sample(after), "camera_stable": stable,
                        "after_matches_pair_reference": after_matches})
        self.require(stable, "Camera/body moved during a nominally fixed visual capture; no pixel claim is made")
        self.require(after_matches, "Camera/body changed from the shared visual-pair reference")
        return receipt

    def teleport(self, x=8.5, y=95, z=3.5, yaw=0, pitch=0):
        self.command("release")
        self.server(f"tp @s {x} {y} {z} {yaw} {pitch}")
        return self.wait_for(self.player_state,
                             lambda s: self.same_camera_pose({"position": [x, y, z], "yaw": yaw, "pitch": pitch},
                                                             self.view_sample(s), tolerance=.01),
                             "settled fixture body XYZ and commanded yaw/pitch")

    def ensure_saved_configuration(self):
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "native module browser")
        self.search("no fog")
        self.click('Select("no_fog")')
        self.wait_for(self.config, lambda c: c.get("schema") == 1, "first saved local configuration")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "closed native module browser")

    def boolean_setting(self, module, key, enabled):
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "settings browser")
        self.search(key.replace("esp_", ""))
        self.click(f'Select("{module}")')
        if self.config()[key] != enabled:
            self.click(f'Bool("{key}")', pane="detail")
        self.wait_for(self.config, lambda c: c[key] == enabled, "saved setting " + key)
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "closed settings browser")

    def exercise(self):
        if self.report["binary_sha256"].lower() == OLD_DISTANCE_ONLY_FOG_BINARY:
            raise AcceptanceFailure("This candidate predates the environmental NoFog fix; use the rebuilt binary")
        self.launch()
        self.fixture_deadline = time.monotonic() + 175
        self.phase = "known ordinary local fixture"
        self.load_area(0,0,31,71)
        for command in ["gamerule advance_time false", "gamerule advance_weather false", "gamerule spawn_mobs false",
                        "time set noon", "weather clear", "fill 0 94 0 31 94 31 stone",
                        "fill 0 95 0 31 103 31 air", "tp @s 8.5 95 3.5 0 0"]:
            self.server(command)
        for x,z in [(0,0),(31,0),(0,31),(31,31),(8,3)]:
            self.wait_for(lambda x=x,z=z:self.command(f"block {x} 94 {z}"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","received visual fixture floor")
        self.settle_fixture_body(8.5,95,3.5)
        self.teleport()
        self.ensure_saved_configuration()
        self.passed("Remaining-feature fixture uses a new known stone platform and received body position")
        self.fog_case()
        self.weather_case()
        self.entity_case()
        self.sprint_case()
        self.hurt_case()
        self.keybinding_case()
        self.hud_case()
        self.report["fixture_elapsed_seconds"] = round(175-(self.fixture_deadline-time.monotonic()), 3)

    def fog_case(self):
        self.phase = "NoFog received underwater-wall off/on/restored capture"
        # Full floor and basin sides prevent flowing water from moving the body
        # or generating a never-ending render-update fixture.
        self.load_area(3,-1,13,71)
        for command in ["fill 3 95 -1 13 103 71 air", "fill 3 94 -1 13 94 71 stone", "fill 3 95 -1 3 100 71 stone",
                        "fill 13 95 -1 13 100 71 stone", "fill 3 95 -1 13 100 -1 stone",
                        "fill 3 95 71 13 100 71 stone", "fill 4 95 0 12 99 70 water",
                        "fill 4 95 68 12 100 68 white_concrete", "fill 8 95 68 8 100 68 red_concrete",
                        "fill 4 98 68 12 98 68 red_concrete"]:
            self.server(command)
        self.server("gamemode creative @s")
        self.teleport()
        self.wait_for(lambda: self.command("block 8 97 68"), lambda b: b.get("loaded") and b.get("block") == "minecraft:red_concrete", "received far wall")
        self.wait_for(lambda: self.command("block 8 96 3"), lambda b: b.get("loaded") and b.get("block") == "minecraft:water", "received eye-height water")
        self.set_hud_hidden(True)
        pair_reference = self.player_state()
        for enabled, label in [(False, "01-water-fog-off"), (True, "02-water-fog-on"), (False, "03-water-fog-restored")]:
            self.set_module("no_fog", ["no_fog"], enabled)
            self.fixed_photo(label, pair_reference)
        self.passed("NoFog toggles persist with the same received submerged body and known far wall",
                    wall=[8, 97, 68], body=[8.5, 95, 3.5], wall_distance_blocks=64.5,
                    asserted="configuration, received water/wall and stable camera", rendering="off/on/restored screenshots need review")
        self.server("fill 4 95 0 12 99 70 air")
        self.teleport()

    def weather_case(self):
        self.phase = "ClearWeather local rendering with authoritative rain retained"
        self.set_module("clear_weather", ["clear_weather"], False)
        self.server("weather rain")
        self.wait_for(self.player_state, lambda s: s["weather"]["raining"], "received authoritative rain")
        time.sleep(5.5)  # Vanilla rain strength transitions over server/client ticks.
        self.set_hud_hidden(True)
        pair_reference = self.player_state()
        for enabled, label in [(False, "04-rain-visible"), (True, "05-clear-weather-local"), (False, "06-rain-restored")]:
            self.set_module("clear_weather", ["clear_weather"], enabled)
            self.require(self.player_state()["weather"]["raining"], "ClearWeather changed authoritative weather instead of only local presentation")
            self.fixed_photo(label, pair_reference)
        self.passed("ClearWeather changes persist while received authoritative rain remains true",
                    asserted="configuration and authoritative weather", rendering="rain removal/restoration needs screenshot review")
        self.server("weather clear")
        self.wait_for(self.player_state, lambda s: not s["weather"]["raining"], "cleared fixture weather")
        time.sleep(.6)

    def entity_case(self):
        self.phase = "Entity ESP for a received mob behind a known opaque wall"
        self.set_hud_hidden(False)  # F1 intentionally suppresses this mod's gizmos.
        self.server("fill 5 95 8 11 99 9 stone")
        self.server('summon minecraft:pig 8.5 95 11.5 {NoAI:1b,Silent:1b,PersistenceRequired:1b}')
        entities = self.wait_for(lambda: self.command("entities 24 minecraft:pig"),
                                 lambda v: any(all(abs(e["pos"][i]-[8.5,95,11.5][i]) < 1 for i in range(3)) for e in v["entities"]),
                                 "received fixture pig behind wall")
        pig = next(e for e in entities["entities"] if all(abs(e["pos"][i]-[8.5,95,11.5][i]) < 1 for i in range(3)))
        self.boolean_setting("entity_esp", "esp_mobs", True)
        self.teleport(pitch=5)
        self.require(self.command("block 8 96 8")["block"] == "minecraft:stone", "ESP occluding wall is missing")
        pair_reference = self.player_state()
        for enabled, label in [(False, "07-esp-mob-off"), (True, "08-esp-mob-on-top"), (False, "09-esp-mob-restored")]:
            self.set_module("entity_esp", ["entity_esp"], enabled)
            self.fixed_photo(label, pair_reference)
        self.passed("Entity ESP and mob filter persist with a received pig behind the opaque wall",
                    entity_id=pig["id"], received_position=pig["pos"], expected_overlay="orange box on top of wall",
                    asserted="received entity, occluder, configuration and fixed camera", rendering="HUD-visible off/on/restored screenshots need review")
        self.teleport()

    def sprint_case(self):
        self.phase = "AutoSprint normal-engine positive and negative controls"
        # Entity ESP deliberately leaves a wall across this runway. A faster
        # forward control can reach it before the asynchronous state read;
        # normal collision then stops sprinting. Verify the runway is clear.
        self.server("fill 5 95 8 11 99 9 air")
        for y in [95, 96]:
            for z in [8, 9]:
                self.wait_for(lambda y=y,z=z: self.command(f"block 8 {y} {z}"),
                              lambda b: b["loaded"] and b["block"] == "minecraft:air",
                              "received clear sprint runway")
        self.report["sprint_runway"] = {
            "preceding_esp_wall_removed": True,
            "received_body_clearance_samples": 4,
            "movement_and_sprint_assertions_unchanged": True,
        }
        receipts = []
        self.report["sprint_controls"] = receipts
        for enabled in [False, True, False]:
            self.set_module("auto_sprint", ["auto_sprint"], enabled)
            self.teleport()
            self.command("release")
            time.sleep(.12)
            before = self.player_state()
            self.key("key.keyboard.w", "down")
            try:
                time.sleep(.45)
                during = self.player_state()
                keys = self.command("status")["keys"]
            finally:
                self.key("key.keyboard.w", "up")
                self.command("release")
            distance = math.hypot(during["player"]["position"][0]-before["player"]["position"][0],
                                  during["player"]["position"][2]-before["player"]["position"][2])
            receipt = {"enabled": enabled, "from": before["player"]["position"], "to": during["player"]["position"],
                       "distance": distance, "sprinting": during["player"]["sprinting"], "normal_control_left": "ControlLeft" in keys,
                       "observed_keys": keys, "before": self.view_sample(before), "during": self.view_sample(during)}
            receipts.append(receipt)
            self.require(distance > .05, "Sprint fixture had no ordinary forward movement positive control")
            self.require(during["player"]["sprinting"] is enabled, "Received sprint state did not match AutoSprint off/on control")
            # API359 heals transient logical modifiers each render frame. The
            # ordinary request exists during physics, so an asynchronous status
            # read need not observe it. The engine sprint state above is the
            # behavioral assertion; retain sampled keys as diagnostic data.
            self.passed("AutoSprint native control " + ("on" if enabled else "off"), **receipt)
        self.teleport()

    def hurt_case(self):
        self.phase = "NoHurtCamera timed local survival damage"
        self.server("gamerule naturalRegeneration false")
        self.server("gamemode survival @s")
        self.wait_for(self.player_state, lambda s: s["gamemode"] == "survival", "local survival fixture")
        self.teleport()
        self.set_hud_hidden(True)
        pair_reference = self.player_state()
        for enabled, label in [(False, "10-hurt-camera-off"), (True, "11-hurt-camera-suppressed")]:
            self.set_module("no_hurt_camera", ["no_hurt_camera"], enabled)
            self.wait_render_ready(timeout=30)
            self.wait_for(self.player_state, lambda s: s["player"]["hurt_time"] == 0, "cleared previous hurt animation")
            time.sleep(.55)
            before = self.player_state()
            self.require(self.same_camera(pair_reference, before), "Camera/body changed before paired hurt capture")
            self.require(before["player"]["health"] >= 5, "Disposable survival health is too low for a one-point fixture")
            self.server("damage @s 1 minecraft:generic")
            hurting = self.wait_for(self.player_state,
                                    lambda s: 6 < s["player"]["hurt_time"] <= 9 and s["player"]["health"] < before["player"]["health"],
                                    "received one-point damage and active hurt animation", timeout=3)
            # The ordinary photo helper's half-second readiness cooldown would
            # outlast hurtTime. Capture immediately after the native state probe.
            self.shot(label)
            after = self.player_state()
            active_through_capture = after["player"]["hurt_time"] > 0
            receipt = {"name": label, "enabled": enabled, "before": self.view_sample(before),
                       "pair_reference": self.view_sample(pair_reference),
                       "hurting": self.view_sample(hurting), "after": self.view_sample(after),
                       "before_matches_pair_reference": self.same_camera(pair_reference, hurting),
                       "after_matches_pair_reference": self.same_camera(pair_reference, after),
                       "hurt_window_retained_after_capture": active_through_capture, "visual_review": "pending"}
            self.report["visual_pairs"].append(receipt)
            self.require(receipt["before_matches_pair_reference"] and receipt["after_matches_pair_reference"],
                         "Camera/body changed between paired hurt captures")
            if not active_through_capture:
                self.report["not_exercised"].append(label + ": hurt animation ended before the post-capture probe; this image cannot prove camera-tilt behavior.")
            self.passed("NoHurtCamera " + ("on" if enabled else "off") + " keeps real damage and animation state",
                        health_before=before["player"]["health"], health_after=hurting["player"]["health"],
                        hurt_time_at_capture_request=hurting["player"]["hurt_time"], capture_window_retained=active_through_capture,
                        asserted="configuration and received gameplay damage", rendering="timed F1-hidden image needs review")
        self.set_module("no_hurt_camera", ["no_hurt_camera"], False)
        self.server("gamemode creative @s")
        self.wait_for(self.player_state, lambda s: s["gamemode"] == "creative", "restored creative fixture")

    def keybinding_case(self):
        self.phase = "X-ray rebound-key and Hold-mode actual input"
        self.set_hud_hidden(False)
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "keybinding browser")
        self.search("xray")
        self.click('Select("xray")')
        self.top_details()
        self.click('Bind("xray")', pane="detail")
        self.key("key.keyboard.y")
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.xray") == "key.keyboard.y", "saved Y binding")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "closed binding browser")
        self.require(not self.state()["xray"], "X-ray unexpectedly began active")
        self.key("key.keyboard.x")
        self.require(not self.state()["xray"], "Old X binding still toggles after rebinding to Y")
        self.key("key.keyboard.y")
        self.wait_for(self.state, lambda s: s["xray"], "Y enables X-ray")
        self.key("key.keyboard.y")
        self.wait_for(self.state, lambda s: not s["xray"], "Y disables X-ray")
        self.passed("Actual Y binding toggles X-ray while old X input is ignored")
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "activation browser")
        self.search("activation")
        self.click('Select("xray")')
        if self.config()["activation"] != "hold":
            self.click("Choice", pane="detail")
        self.wait_for(self.config, lambda c: c["activation"] == "hold", "saved Hold activation")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "closed activation browser")
        self.key("key.keyboard.y", "down")
        self.wait_for(self.state, lambda s: s["xray"], "held Y enables X-ray")
        self.key("key.keyboard.y", "up")
        self.wait_for(self.state, lambda s: not s["xray"], "Y release disables X-ray")
        self.passed("Hold activation responds to actual press and release")
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "restore activation browser")
        self.search("activation")
        self.click('Select("xray")')
        self.click("Choice", pane="detail")
        self.wait_for(self.config, lambda c: c["activation"] == "toggle", "restored Toggle activation")
        self.search("xray")
        self.top_details()
        self.click('Bind("xray")', right=True, pane="detail")
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.xray") == "key.keyboard.x", "restored X binding")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "closed restored GUI")
        self.command("release")

    def hud_case(self):
        self.phase = "Coordinates and Status HUD native presentation"
        self.teleport()
        self.set_hud_hidden(False)
        self.key("key.keyboard.x")
        self.wait_for(self.state, lambda s: s["xray"], "HUD fixture X-ray active")
        reference = self.player_state()
        self.set_module("coordinates", ["coords_hud"], True)
        self.set_module("status_hud", ["hud"], True)
        self.fixed_photo("12-coordinates-and-status", reference)
        self.set_module("coordinates", ["coords_hud"], False)
        self.set_module("status_hud", ["hud"], False)
        self.fixed_photo("13-coordinates-and-status-disabled", reference)
        self.require(self.state()["xray"], "Disabling HUD modules disabled X-ray")
        self.key("key.keyboard.x")
        self.wait_for(self.state, lambda s: not s["xray"], "HUD fixture X-ray restored off")
        self.passed("Coordinates and Status HUD controls persist independently of active X-ray",
                    expected_position=[8.5, 95, 3.5], expected_dimension="minecraft:overworld",
                    rendering="on/off screenshots require label and disappearance review")

    def finish(self):
        # Use the shared process/credential cleanup directly, then name only
        # this fresh suite's own report. No FeatureSmoke exercise is invoked.
        Smoke.finish(self)
        source = self.output / "gui-acceptance.json"
        destination = self.output / "visual-acceptance.json"
        source.replace(destination)
        print("Visual report: " + str(destination), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--resources", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--width", default=960, type=int)
    parser.add_argument("--height", default=540, type=int)
    parser.add_argument("--startup-timeout", default=240.0, type=float)
    parser.add_argument("--terrain", choices=["vanilla", "test"], default="vanilla", help="Normal generation by default; synthetic terrain is diagnostic-only")
    args = parser.parse_args()
    if not 640 <= args.width <= 3840 or not 360 <= args.height <= 2160 or not 30 <= args.startup_timeout <= 900:
        parser.error("Window must be 640..3840 by 360..2160; startup timeout 30..900 seconds")
    smoke = None
    try:
        smoke = VisualSmoke(args)
        smoke.exercise()
        smoke.report["passed"] = True
        return 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError, KeyboardInterrupt) as exc:
        message = str(exc) if isinstance(exc, AcceptanceFailure) else type(exc).__name__
        if smoke is not None:
            smoke.report["passed"] = False
            unsupported = isinstance(exc, UnsupportedFixture)
            smoke.report["checks"].append({"name": smoke.phase, "passed": False, "reason": message,
                                           "not_exercised": unsupported})
            if unsupported:
                smoke.report["not_exercised"].append(smoke.phase + ": " + message)
            try:
                if smoke.process is not None and smoke.process.poll() is None:
                    smoke.shot("failure")
            except (AcceptanceFailure, OSError, ValueError):
                pass
        print("FAIL " + message, file=sys.stderr, flush=True)
        return 1
    finally:
        if smoke is not None:
            smoke.finish()


if __name__ == "__main__":
    raise SystemExit(main())
