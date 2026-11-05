#!/usr/bin/env python3
"""Exercise Rusty 1.2 through real native input in one fresh offline QA home.

python tools/game_ui12_smoke.py --binary PATH --resources PATH --output NEW_DIR

Captures actual frames for a separate visual review. Tests all 18 screen HUD
layout controls; conditional HUD contents still need their feature fixtures.
The only process restarted is this suite's owned child, in its new QA instance.
No installed instance, personal settings/world, account or clipboard is used.
"""
from __future__ import annotations

import argparse
import copy
import json
import math
import struct
from pathlib import Path
import subprocess
import sys
import time
from unittest.mock import patch

from game_feature_smoke import FeatureSmoke
from game_gui_smoke import AcceptanceFailure, Smoke, canonical_digest, digest


PANELS = {
    "status_hud": "Status HUD", "coordinates": "Coordinates", "finder": "Finder",
    "compass": "Compass", "waypoints": "Waypoints", "equipment_hud": "Equipment HUD",
    "environment_hud": "Environment HUD", "container_preview": "Container Preview",
    "build_planner": "Build Planner", "fps_hud": "Frame Rate", "clicks_hud": "Click Counter",
    "portal_coordinates": "Portal Coordinates", "clock_hud": "World Clock",
    "speed_hud": "Speed HUD", "target_info": "Target Info", "light_overlay": "Light Risk",
    "trajectories": "Trajectories", "minimap": "Minimap",
}


def same_native_layout(actual, saved):
    """Compare exact f32 values across Value snapshots and shortest-decimal files.

    serde_json::Value widens native f32 fields to f64; direct file serialization
    keeps their shortest f32 decimal. This accepts only that representation
    difference, including rejecting a one-ULP native position change.
    """
    if not isinstance(actual, dict) or not isinstance(saved, dict) or actual.keys() != saved.keys():
        return False
    try:
        for panel, expected in saved.items():
            observed = actual[panel]
            if panel not in PANELS or not isinstance(expected, dict) or not isinstance(observed, dict):
                return False
            if set(expected) != {"x", "y", "scale"} or observed.keys() != expected.keys():
                return False
            for axis, value in expected.items():
                got = observed[axis]
                lo, hi = (.5, 2.0) if axis == "scale" else (0.0, 1.0)
                if type(value) not in (int, float) or type(got) not in (int, float):
                    return False
                if not math.isfinite(value) or not math.isfinite(got) or not lo <= value <= hi:
                    return False
                if got != struct.unpack("<f", struct.pack("<f", value))[0]:
                    return False
        return True
    except (OverflowError, ValueError, TypeError, struct.error):
        return False


class Ui12Smoke(FeatureSmoke):
    def __init__(self, args):
        super().__init__(args)
        self.deadline = None
        self.restart_spec = None
        self.report.update(
            suite="native-ui-1.2", script_sha256=digest(Path(__file__)),
            feature_harness_sha256=digest(Path(__file__).with_name("game_feature_smoke.py")),
            claims="Real native pointer/key routing, saved settings, live Enabled membership, named-preset import and owned process restart; screenshots require visual review.",
            manual_review="pending", expected_image_content={}, hud_control_receipts=[],
            not_exercised=[
                "Image composition, labels, clipping and map texture alignment need review of the saved native frames; hit/config assertions alone do not accept their appearance.",
                "All18 HUD layout controls are changed/reset; conditional finder/target/trajectory/container/build/waypoint content is not individually manufactured or accepted here.",
                "Physical device input, live window resize, other graphics backends/resource packs and external servers are outside this bounded fixture.",
            ],
        )

    def command(self, *args, **kwargs):
        if self.deadline is not None and time.monotonic() > self.deadline:
            raise AcceptanceFailure("UI1.2 fixture exceeded its post-launch time budget")
        return super().command(*args, **kwargs)

    def launch(self):
        # Instrument the inherited launch without changing argv/environment.
        # Keep the environment private; do not serialize ephemeral credentials.
        original = subprocess.Popen

        def record(*args, **kwargs):
            self.restart_spec = (copy.copy(args), copy.copy(kwargs))
            return original(*args, **kwargs)

        with patch("game_gui_smoke.subprocess.Popen", side_effect=record):
            Smoke.launch(self)

    def open_browser(self):
        if not self.ui()["open"]:
            self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "actionable native browser")

    def close_browser(self):
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "closed native browser")
        self.wait_for(lambda: self.command("status"), lambda s: s["screen"] == "playing", "Playing after browser")

    def select(self, module):
        self.click("Collection(All)")
        self.search(PANELS.get(module, module))
        self.click(f'Select("{module}")')
        self.wait_for(self.ui, lambda s: s["selected"] == module, "selected " + module)
        self.top_details()

    def capture(self, name, expected):
        time.sleep(.25)  # Bounded opening fade; not a rendering-success assertion.
        self.shot(name)
        self.report["expected_image_content"][name] = {"expected": expected, "review": "pending"}

    def slider(self, name, fraction, predicate, description, pane=None):
        rect = self.hit(f'Numeric("{name}",', pane=pane)["rect"]
        self.require(len(rect) == 4 and all(math.isfinite(x) for x in rect) and rect[2] > 1 and rect[3] > 0,
                     "Invalid rendered slider target " + name)
        x = rect[0] + max(.001, min(rect[2] - .001, rect[2] * fraction))
        y = rect[1] + rect[3] / 2
        self.command(f"drag-at {rect[0]+rect[2]/2:.4f} {y:.4f} {x:.4f} {y:.4f}")
        self.wait_for(self.config, predicate, description)
        self.wait_for(self.ui, lambda s: not s["dirty"] and not s["save_blocked"], "flushed " + name)
        return rect

    def enabled_and_binding(self):
        self.phase = "live Enabled collection and explicit module unbinding"
        self.select("fullbright")
        if self.config()["fullbright"]:
            self.click('Toggle("fullbright")')
            self.wait_for(self.config, lambda c: not c["fullbright"], "Fullbright initially off")
        self.click("Collection(Enabled)")
        self.wait_for(self.ui, lambda s: s["collection"] == "Enabled" and not self.matching_hits(s, 'Select("fullbright")'), "inactive Fullbright excluded")
        self.capture("01-enabled-off", "Enabled collection with Fullbright search has no inactive Fullbright row")
        self.close_browser()
        self.key("key.keyboard.b")
        self.wait_for(self.state, lambda s: s["fullbright"], "actual B activates Fullbright")
        self.open_browser(); self.search("fullbright"); self.click("Collection(Enabled)")
        self.wait_for(self.ui, lambda s: bool(self.matching_hits(s, 'Select("fullbright")')), "active Fullbright included")
        self.capture("02-enabled-live", "Enabled collection includes live Fullbright and its enabled detail control")
        self.click('Select("fullbright")'); self.click('Toggle("fullbright")')
        self.wait_for(self.ui, lambda s: not self.matching_hits(s, 'Select("fullbright")'), "disabled row removed live")
        self.wait_for(self.state, lambda s: not s["fullbright"], "Fullbright restored off")
        self.passed("Enabled membership follows actual hotkey activation and native toggle deactivation")
        self.select("fullbright")
        self.click('ClearBind("fullbright")', pane="detail")
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.fullbright") == "key.keyboard.unknown", "saved explicit module unbind")
        self.close_browser(); self.key("key.keyboard.b")
        first = self.command("status")["frames"]
        self.wait_for(lambda: self.command("status"), lambda s: s["frames"] >= first + 3, "fresh frames after unbound key")
        self.require(not self.state()["fullbright"], "Cleared B binding still activated Fullbright")
        self.open_browser(); self.select("fullbright"); self.click('Bind("fullbright")', pane="detail")
        self.wait_for(self.ui, lambda s: s["capture"] == "fullbright", "module key capture")
        self.key("key.keyboard.f8")
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.fullbright") == "key.keyboard.f8", "new module binding")
        self.require(not self.state()["fullbright"], "Capture key leaked into module activation")
        self.close_browser(); self.key("key.keyboard.f8")
        self.wait_for(self.state, lambda s: s["fullbright"], "rebound F8 activates Fullbright")
        self.key("key.keyboard.f8"); self.wait_for(self.state, lambda s: not s["fullbright"], "rebound F8 restores Fullbright")
        self.open_browser(); self.select("fullbright"); self.click('Bind("fullbright")', right=True, pane="detail")
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.fullbright") == "key.keyboard.b", "default module key restored")
        self.passed("ClearBind saves unbound state, old key stops, captured new key routes, default restores")

    def menu_recovery(self):
        self.phase = "menu unbind and real chat recovery"
        self.click('ClearBind("menu")')
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.menu") == "key.keyboard.unknown", "saved explicit menu unbind")
        self.capture("03-menu-cleared", "Menu key shows unbound and recovery guidance")
        self.close_browser(); self.key("key.keyboard.right.shift")
        first = self.command("status")["frames"]
        self.wait_for(lambda: self.command("status"), lambda s: s["frames"] >= first + 3, "fresh frames after old menu key")
        self.require(not self.ui()["open"], "Cleared menu key still opens the browser")
        self.key("key.keyboard.t")
        self.wait_for(lambda: self.command("status"), lambda s: s["screen"] == "chat", "actual chat entry")
        self.command("type /rusty menu"); self.key("key.keyboard.enter")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "unbound menu command recovery")
        self.click('Bind("menu")'); self.wait_for(self.ui, lambda s: s["capture"] == "menu", "menu capture")
        self.key("key.keyboard.f9")
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.menu") == "key.keyboard.f9", "saved recovered menu key")
        self.close_browser(); self.key("key.keyboard.f9")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "new menu key reopens")
        self.click('Bind("menu")', right=True)
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.menu") == "key.keyboard.right.shift", "menu default restored")
        self.passed("Cleared menu recovers through actual /rusty menu chat and accepts a working new key")

    def scales_and_palettes(self):
        self.phase = "menu endpoints, rendered hit alignment and three palettes"
        for value, percent, fraction in [(.35, 50, 0), (2.1, 300, 1)]:
            self.slider("menu_scale", fraction, lambda c, v=value: abs(c["client"]["menu_scale"] - v) < .001, "saved menu scale endpoint")
            state = self.wait_for(self.ui, lambda s: abs(s["menu_scale"] - value) < .001, "native scale snapshot")
            self.require(0 < state["effective_menu_scale"] <= value + .001, "Invalid fitted menu scale")
            self.require(abs(state['menu_percent']-percent)<.001, 'Menu percentage does not use compact baseline')
            viewport = self.command("ui")["gui_size"]
            for hit in state["hits"]:
                x, y, w, h = hit["rect"]
                self.require(all(math.isfinite(v) for v in [x,y,w,h]) and x >= -.1 and y >= -.1 and w > 0 and h > 0 and x+w <= viewport[0]+1 and y+h <= viewport[1]+1, "Scaled target lies outside rendered GUI viewport")
            self.select("fullbright")
            before = self.config()["fullbright"]
            self.click('Toggle("fullbright")', pane="detail")
            self.wait_for(self.config, lambda c: c["fullbright"] != before, "scaled pointer hits intended toggle")
            self.click('Toggle("fullbright")', pane="detail")
            self.wait_for(self.config, lambda c: c["fullbright"] == before, "scaled toggle restored")
            self.capture("04-menu-scale-" + str(percent), "Requested menu size endpoint; widgets, text and pointer targets align and remain reachable")
            self.passed("Menu scale endpoint saves and real scaled target routes", requested=value, percent=percent, effective=state["effective_menu_scale"])
        self.click("ResetMenuScale")
        self.wait_for(self.config, lambda c: abs(c["client"]["menu_scale"]-.7)<.001, "compact 100 percent reset")
        self.wait_for(self.ui, lambda s: abs(s["menu_percent"]-100)<.001 and abs(s['effective_menu_scale']-.7)<.001, 'rendered compact reset')
        for theme in ["Classic", "Graphite", "Copper"]:
            for _ in range(3):
                if self.ui()["theme"] == theme: break
                self.click("Theme")
            self.wait_for(self.ui, lambda s, t=theme: s["theme"] == t, "native palette " + theme)
            self.wait_for(self.config, lambda c, t=theme.lower(): c["client"]["theme"] == t, "persisted palette " + theme)
            self.capture("05-palette-" + theme.lower(), "Minecraft widgets and readable labels with " + theme + " palette")
        self.passed("All three palette controls persist and produce captured native frames")

    def hud_controls(self):
        self.phase = "independent layout controls and reset for all18 HUD panels"
        for panel in PANELS:
            self.select(panel)
            before = copy.deepcopy(self.config()["client"]["hud_layout"])
            initial_controls = copy.deepcopy(self.ui()["local_values"])
            self.require(all(f"hud:{panel}:{axis}" in initial_controls for axis in ("x","y","scale")), "Selected HUD lacks its real local controls")
            targets = [("x", 1, 1), ("y", 0, 0), ("scale", 1, 2)]
            for axis, fraction, want in targets:
                self.slider(f"hud:{panel}:{axis}", fraction,
                            lambda c, p=panel, a=axis, w=want: abs(c["client"]["hud_layout"].get(p,{}).get(a,-99)-w)<.001,
                            "saved " + panel + " " + axis, pane="detail")
            changed = self.config()["client"]["hud_layout"]
            self.require({k:v for k,v in changed.items() if k!=panel} == {k:v for k,v in before.items() if k!=panel}, "Editing one HUD moved a different HUD")
            if panel in ("coordinates", "fps_hud", "minimap"):
                self.capture("06-controls-" + panel, "Independent X/Y/size controls at right/top/200% for " + panel)
            self.click(f'ResetHud("{panel}")', pane="detail")
            self.wait_for(self.config, lambda c,p=panel: p not in c["client"]["hud_layout"], "per-panel reset removes override")
            self.wait_for(self.ui, lambda s,expected=initial_controls: s["local_values"]==expected, "rendered HUD controls reflect reset")
            self.require(self.config()["client"]["hud_layout"] == before, "Per-panel reset changed another panel")
            self.report["hud_control_receipts"].append({"panel":panel,"changed":{"x":1,"y":0,"scale":2},"unrelated_panels_unchanged":True,"reset_restored":True})
        self.passed("All18 HUD panels independently save X/Y/size and reset through actual native controls", panels=len(PANELS))
        # Keep a few visible, spatially distinct panels for real-frame review and
        # import/restart persistence; other conditional panels remain disabled.
        for panel, fractions, placement in [
            ("coordinates", (.2,.2,.5), {"x":.2,"y":.2,"scale":1.25}),
            ("fps_hud", (.8,.3,1/3), {"x":.8,"y":.3,"scale":1.0}),
            ("clock_hud", (.2,.8,.5), {"x":.2,"y":.8,"scale":1.25}),
            ("minimap", (1,1,.5), {"x":1,"y":1,"scale":1.25}),
        ]:
            self.select(panel)
            for axis, fraction in zip(("x","y","scale"), fractions):
                want = placement[axis]
                self.slider(f"hud:{panel}:{axis}", fraction,
                            lambda c,p=panel,a=axis,w=want: abs(c["client"]["hud_layout"].get(p,{}).get(a,-99)-w)<.001,
                            "persistent sample layout " + panel + " " + axis, pane="detail")
            if panel == "coordinates": enabled = self.config()["coords_hud"]
            else: enabled = self.config()["awareness"][panel]
            if not enabled:
                self.top_details(); self.click(f'Toggle("{panel}")', pane="detail")
                self.wait_for(self.config, lambda c,p=panel: c["coords_hud"] if p=="coordinates" else c["awareness"][p], "visible sample HUD enabled")
        self.close_browser()
        self.capture("07-hud-layout-live", "Coordinates and FPS at separate custom positions, clock lower left, scaled textured minimap bottom right")
        self.open_browser()

    def widget(self, label):
        state = self.wait_for(lambda:self.command("ui"), lambda s:any(str(w.get("label","")).lstrip("> ")==label and w.get("enabled") for w in s.get("widgets",[])), "visible native widget " + label)
        matches = [w for w in state["widgets"] if str(w.get("label","")).lstrip("> ")==label and w.get("enabled")]
        self.require(len(matches)==1, "Ambiguous native widget " + label)
        self.command("click " + matches[0]["id"])

    def import_preserves_layout(self):
        self.phase = "actual named preset import preserves local UI layout"
        before = copy.deepcopy(self.config())
        preset = copy.deepcopy(before)
        preset["fullbright"] = not before["fullbright"]
        preset["client"]["menu_scale"] = .5
        preset["client"]["theme"] = "classic"
        preset["client"]["hud_layout"] = {"coordinates":{"x":1,"y":1,"scale":2}}
        presets = self.config_path.parent / "presets"
        presets.mkdir(exist_ok=True)
        path = presets / "my-preset.json"
        self.require(not path.exists(), "Fresh named-preset fixture path is occupied")
        path.write_text(json.dumps(preset)+"\n", encoding="utf-8")
        self.click("Advanced")
        self.wait_for(lambda:self.command("status"), lambda s:s["screen"]=="options", "native Advanced settings")
        self.widget("Profiles")
        self.widget("Load named preset")
        loaded = self.wait_for(self.config, lambda c:c["fullbright"]==preset["fullbright"], "imported feature value proves native preset load")
        for field in ("hud_layout", "menu_scale", "theme"):
            self.require(loaded["client"][field]==before["client"][field], "Feature import replaced local " + field)
        self.capture("08-native-preset-import", "Native Profiles screen after named preset import; local UI preferences preserved")
        self.key("key.keyboard.escape")
        self.wait_for(lambda:self.command("status"),lambda s:s["screen"]=="playing","closed advanced profile screen")
        self.open_browser()
        self.require(same_native_layout(self.ui()["hud_layout"],before["client"]["hud_layout"]),"Native browser did not reload preserved HUD layout")
        self.passed("Native named-preset import applies feature settings and preserves menu size, palette and HUD layout",preserved_fields=["hud_layout","menu_scale","theme"])

    def restart_and_verify(self):
        self.phase = "owned child process restart and persisted UI reload"
        expected = copy.deepcopy(self.config()["client"])
        bindings = copy.deepcopy(self.bindings())
        self.close_browser()
        self.command("quit",timeout=10); self.process.wait(timeout=30)
        self.require(self.process.returncode==0,"Owned process did not exit cleanly before restart")
        self.log.close(); self.endpoint.unlink(missing_ok=True)
        args, kwargs = self.restart_spec
        kwargs = dict(kwargs)
        self.log = (self.private/"process-restart.log").open("wb")
        kwargs["stdout"] = self.log
        self.process = subprocess.Popen(*args,**kwargs)
        self.wait_for(lambda:self.command("status",timeout=2),lambda s:s.get("player") is not None and s.get("screen")=="playing" and s.get("chunks",0)>=9,"same owned QA world after process restart",timeout=self.args.startup_timeout)
        self.open_browser()
        state = self.ui()
        self.require(same_native_layout(state["hud_layout"],expected["hud_layout"]) and abs(state["menu_scale"]-expected["menu_scale"])<.001 and state["theme"].lower()==expected["theme"],"Restart lost native layout/size/palette preferences")
        self.require(self.bindings()==bindings,"Restart lost saved restored bindings")
        self.require(self.config()["client"]==expected,"Restart changed saved client preferences")
        self.capture("09-after-process-restart", "Native browser reloads saved palette, size and HUD layout from the same isolated QA home")
        self.close_browser(); self.capture("10-hud-after-restart", "Custom HUD positions/size and textured minimap survive a real process restart")
        self.passed("New owned process reloads saved layout, menu size, palette and bindings", client_preferences_sha256=canonical_digest(expected),instance_scope="suite-created QA home only")

    def exercise(self):
        self.launch(); self.deadline=time.monotonic()+self.args.budget
        self.command("options-set guiScale 2")
        self.open_browser(); self.select("fullbright")
        initial=self.ui()
        self.require(abs(initial['menu_percent']-100)<.001 and abs(initial['effective_menu_scale']-.7)<.001, 'New menu default must render at former 70 percent size and display 100 percent')
        self.passed('Fresh menu default is compact 100 percent with former 70 percent physical scale')
        self.wait_for(self.config,lambda c:"hud_layout" in c["client"],"persisted1.2 configuration")
        inventory=canonical_digest(self.command("inventory"))
        self.capture("00-classic-initial", "Minecraft-style native module browser with size control, clear binding and Enabled collection")
        self.enabled_and_binding(); self.menu_recovery(); self.scales_and_palettes(); self.hud_controls()
        self.require(canonical_digest(self.command("inventory"))==inventory,"UI layout actions changed fixture inventory")
        self.import_preserves_layout(); self.restart_and_verify()
        self.report["final_fixture_preferences"]=self.config()["client"]

    def finish(self):
        self.deadline=None
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"ui12-acceptance.json")


def main():
    parser=argparse.ArgumentParser(description=__doc__,formatter_class=argparse.RawDescriptionHelpFormatter)
    for name in ("binary","resources","output"):parser.add_argument("--"+name,required=True,type=Path)
    parser.add_argument("--width",type=int,default=1280); parser.add_argument("--height",type=int,default=720)
    parser.add_argument("--startup-timeout",type=float,default=300)
    parser.add_argument("--budget",type=float,default=600,help="Post-initial-launch seconds, including owned process restart")
    args=parser.parse_args()
    if not 960<=args.width<=3840 or not 540<=args.height<=2160 or not 30<=args.startup_timeout<=900 or not 120<=args.budget<=1800:
        parser.error("Window960..3840 by540..2160; startup30..900s; post-launch budget120..1800s")
    smoke=None
    try:
        smoke=Ui12Smoke(args); smoke.exercise(); smoke.report["passed"]=True; return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as error:
        if smoke:
            smoke.report["passed"]=False
            smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(error)})
        print("FAIL "+str(error),file=sys.stderr,flush=True); return 1
    finally:
        if smoke:smoke.finish()


if __name__=="__main__":raise SystemExit(main())
