#!/usr/bin/env python3
"""Exercise the native Rusty GUI through actual local game input events.

Example:
  python tools/game_gui_smoke.py --binary PATH --resources PATH --output target/qa/gui-report

The output directory must be new. Every invocation creates its own offline game
and user directories under this checkout's ignored target/qa. No account, session,
installed instance, existing world or existing settings are loaded. Screenshots
are evidence for a separate visual review; this script does not score their art.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import time
import uuid


class AcceptanceFailure(RuntimeError):
    pass


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def canonical_digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


class Smoke:
    def __init__(self, args: argparse.Namespace):
        self.args = args
        self.binary = args.binary.resolve(strict=True)
        self.resources = args.resources.resolve(strict=True)
        if not self.binary.is_file() or not self.resources.is_dir():
            raise AcceptanceFailure("A binary file and prepared resources directory are required")
        self.output = args.output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        (self.output / "screenshots").mkdir()
        qa_root = Path(__file__).resolve().parents[1] / "target" / "qa"
        qa_root.mkdir(parents=True, exist_ok=True)
        run_id = time.strftime("gui-%Y%m%d-%H%M%S-") + uuid.uuid4().hex[:8]
        self.private = qa_root / run_id
        self.private.mkdir(exist_ok=False)
        self.home = self.private / "instance"
        self.home.mkdir()
        self.endpoint = self.private / "control.json"
        self.config_path = self.home / "horizon-mods" / ".horizon-mod-data" / "rusty" / "config"
        self.bindings_path = self.home / "horizon-mods.txt"
        self.process: subprocess.Popen | None = None
        self.log = None
        self.phase = "initialization"
        self.report = {
            "schema": 1,
            "run_id": run_id,
            "binary_name": self.binary.name,
            "binary_sha256": digest(self.binary),
            "script_sha256": digest(Path(__file__)),
            "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "isolated_offline_world": True,
            "requested_window": [args.width, args.height],
            "terrain": getattr(args, "terrain", "vanilla"),
            "checks": [],
            "screenshots": [],
            "expected_failures": [],
            "not_exercised": [
                "Live resize: no documented window resize command in the inspected API359 control surface; rerun at other --width/--height values.",
                "GUI scale changes: no direct screen input route for that option is assumed by this GUI-only test.",
                "Visual composition/readability needs inspection of the actual saved screenshots.",
                "Feature-specific X-ray, freecam, navigation and inventory behavior belong to separate fixtures.",
            ],
        }

    def command(self, line: str, timeout: float = 15) -> dict:
        if "\n" in line or "\r" in line:
            raise AcceptanceFailure("A control command cannot contain line breaks")
        # The endpoint token never enters a log, assertion, exception or report.
        try:
            endpoint = json.loads(self.endpoint.read_text(encoding="utf-8"))
            port = int(endpoint["port"])
            token = endpoint["token"]
            if not 1 <= port <= 65535 or not isinstance(token, str):
                raise ValueError
            with socket.create_connection(("127.0.0.1", port), timeout=timeout) as connection:
                connection.settimeout(timeout)
                with connection.makefile("rwb") as stream:
                    stream.write(("auth " + token + "\n").encode())
                    stream.flush()
                    authentication = json.loads(stream.readline(1024 * 1024))
                    if not authentication.get("ok"):
                        raise AcceptanceFailure("Private control authentication failed")
                    stream.write((line + "\n").encode())
                    stream.flush()
                    raw = stream.readline(16 * 1024 * 1024)
                    result = json.loads(raw)
            if not isinstance(result, dict):
                raise AcceptanceFailure("Control reply was not an object")
            if "error" in result:
                raise AcceptanceFailure("Game rejected the " + line.split(" ", 1)[0] + " command")
            return result
        except AcceptanceFailure:
            raise
        except (OSError, ValueError, KeyError, TypeError) as exc:
            raise AcceptanceFailure("Private control connection or reply failed: " + type(exc).__name__) from None

    def wait_for(self, reader, predicate, description: str, timeout: float = 8):
        until = time.monotonic() + timeout
        while time.monotonic() < until:
            if self.process is not None and self.process.poll() is not None:
                raise AcceptanceFailure("Owned game exited while waiting for " + description)
            try:
                value = reader()
                if predicate(value):
                    return value
            except AcceptanceFailure:
                pass
            time.sleep(0.08)
        raise AcceptanceFailure("Timed out waiting for " + description)

    def ui(self) -> dict:
        value = self.command("rusty-ui-state")
        if "open" not in value or not isinstance(value.get("hits"), list):
            raise AcceptanceFailure("Native GUI snapshot interface is missing")
        if len(value["hits"]) > 512:
            raise AcceptanceFailure("GUI snapshot exceeded the bounded hit limit")
        return value

    def config(self) -> dict:
        try:
            if self.config_path.stat().st_size > 4 * 1024 * 1024:
                raise AcceptanceFailure("Fixture configuration exceeds its supported limit")
            value = json.loads(self.config_path.read_text(encoding="utf-8"))
            if not isinstance(value, dict):
                raise ValueError
            return value
        except (OSError, ValueError):
            raise AcceptanceFailure("Saved fixture configuration is not available") from None

    def bindings(self) -> dict[str, str]:
        if not self.bindings_path.exists():
            return {}
        entries = {}
        for line in self.bindings_path.read_text(encoding="utf-8").splitlines():
            if line.startswith("key.rusty.") and ":" in line:
                key, value = line.split(":", 1)
                entries[key] = value
        return entries

    def passed(self, name: str, **details):
        self.report["checks"].append({"name": name, "passed": True, **details})
        print("PASS " + name + (" " + json.dumps(details, sort_keys=True) if details else ""), flush=True)

    def require(self, condition: bool, message: str):
        if not condition:
            raise AcceptanceFailure(message)

    def key(self, name: str, action: str = "tap"):
        self.command("key " + name + " " + action)

    @staticmethod
    def matching_hits(state: dict, prefix: str) -> list[dict]:
        return [h for h in state["hits"] if str(h.get("action", "")).startswith(prefix)]

    def hit(self, prefix: str, pane: str | None = None) -> dict:
        # Query the current rendered hit map; never invent pixel positions.
        if pane is None:
            state = self.wait_for(self.ui, lambda s: s.get("open") and bool(self.matching_hits(s, prefix)),
                                  "visible native GUI action " + prefix.split("(", 1)[0])
            return self.matching_hits(state, prefix)[0]
        for attempt in range(12 if pane else 1):
            state = self.wait_for(self.ui, lambda s: s.get("open") and bool(s["hits"]), "rendered GUI")
            hits = self.matching_hits(state, prefix)
            if hits:
                return hits[0]
            if pane:
                rect = state[pane + "_rect"]
                self.command(f"scroll-at {rect[0]+rect[2]/2:.3f} {rect[1]+rect[3]/2:.3f} -3")
                time.sleep(0.1)
        raise AcceptanceFailure("No visible native GUI action: " + prefix.split("(", 1)[0])

    def click(self, prefix: str, right: bool = False, pane: str | None = None):
        hit = self.hit(prefix, pane)
        rect = hit["rect"]
        self.require(len(rect) == 4 and all(math.isfinite(n) for n in rect) and rect[2] > 0 and rect[3] > 0, "Invalid visible hit rectangle")
        self.command(f"click-at {rect[0]+rect[2]/2:.3f} {rect[1]+rect[3]/2:.3f}" + (" right" if right else ""))
        time.sleep(0.14)

    def search(self, text: str):
        self.click("Search")
        self.key("key.keyboard.left.control", "down")
        try:
            self.key("key.keyboard.a")
        finally:
            self.key("key.keyboard.left.control", "up")
        self.key("key.keyboard.backspace")
        if text:
            self.command("type " + text)
        self.wait_for(self.ui, lambda s: s.get("query") == text, "search text")
        time.sleep(0.12)

    def top_details(self):
        state = self.ui()
        rect = state["detail_rect"]
        self.command(f"scroll-at {rect[0]+rect[2]/2:.3f} {rect[1]+rect[3]/2:.3f} 50")
        time.sleep(0.12)

    def shot(self, name: str):
        path = self.output / "screenshots" / (name + ".png")
        self.command("screenshot " + str(path), timeout=45)
        with path.open("rb") as image:
            header = image.read(24)
        self.require(header[:8] == b"\x89PNG\r\n\x1a\n" and len(header) == 24, "Screenshot was not a PNG")
        width, height = struct.unpack(">II", header[16:24])
        self.require(width > 0 and height > 0 and path.stat().st_size > 1024, "Screenshot contains no usable frame")
        self.report["screenshots"].append({"path": "screenshots/" + path.name, "sha256": digest(path), "width": width, "height": height, "visual_review": "pending"})

    def launch(self):
        self.phase = "isolated launch and readiness"
        for name in ["home", "launcher", "xdg-data", "xdg-config", "xdg-cache", "runtime", "temp", "appdata", "localappdata"]:
            (self.private / name).mkdir()
        env = dict(os.environ)
        for key in ("HORIZON_CONTROL", "HORIZON_HOME", "HORIZON_LAUNCHER_HOME"):
            env.pop(key, None)
        env.update({
            "HOME": str(self.private / "home"), "USERPROFILE": str(self.private / "home"),
            "APPDATA": str(self.private / "appdata"), "LOCALAPPDATA": str(self.private / "localappdata"),
            "HORIZON_HOME": str(self.home), "HORIZON_LAUNCHER_HOME": str(self.private / "launcher"),
            "XDG_DATA_HOME": str(self.private / "xdg-data"), "XDG_CONFIG_HOME": str(self.private / "xdg-config"),
            "XDG_CACHE_HOME": str(self.private / "xdg-cache"), "XDG_RUNTIME_DIR": str(self.private / "runtime"),
            "TEMP": str(self.private / "temp"), "TMP": str(self.private / "temp"),
            "HORIZON_NO_AUDIO": "1", "HORIZON_LAUNCHER_NO_SELF_UPDATE": "1",
        })
        (self.home / "options.txt").write_text("lang:en_us\npauseOnLostFocus:false\nmaxFps:60\nrenderDistance:4\ntutorialStep:none\n", encoding="utf-8")
        self.log = (self.private / "process.log").open("wb")
        command = [str(self.binary), "--resources", str(self.resources), "--game-dir", str(self.home), "--home", str(self.home),
                   "--player-dir", str(self.home), "--offscreen", "--control", "--control-socket", str(self.endpoint),
                   "--width", str(self.args.width), "--height", str(self.args.height), "--render-distance", "4",
                   "--world", "RustyGuiSmoke", "--seed", "1", "--creative", "--name", "RustyGuiQA"]
        if getattr(self.args, "terrain", "vanilla") == "test" and not getattr(self, "native_terrain", False):
            command.append("--test-terrain")
        self.process = subprocess.Popen(command, cwd=self.home, env=env, stdout=self.log, stderr=subprocess.STDOUT,
                                        stdin=subprocess.DEVNULL, creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        self.wait_for(lambda: self.command("status", timeout=2),
                      lambda s: s.get("player") is not None and s.get("chunks", 0) >= 9 and s.get("screen") == "playing",
                      "fresh playing world with at least nine chunks", timeout=self.args.startup_timeout)
        time.sleep(0.6)
        self.passed("Fresh isolated offline world ready", account_loaded=False, minimum_loaded_chunks=9)

    def exercise(self):
        self.launch()
        self.phase = "native opening and continuing world simulation"
        inventory_before = canonical_digest(self.command("inventory"))
        self.key("key.keyboard.right.shift")
        state = self.wait_for(self.ui, lambda s: s.get("open") and bool(s["hits"]), "Right Shift native GUI")
        self.require(self.command("status")["screen"] == "mod", "GUI did not use the native over-world Mod screen")
        self.passed("Right Shift opens the real native Mod screen", visible_hit_count=len(state["hits"]))
        initial = self.command("status")["player"]
        tick_before = self.command("simulation")["game_time"]
        time.sleep(0.55)
        tick_after = self.command("simulation")["game_time"]
        self.require(tick_after > tick_before, "Single-player world simulation paused under the GUI")
        self.passed("World simulation continues under GUI", elapsed_game_ticks=tick_after - tick_before)
        self.shot("01-classic-open")

        self.phase = "search, categories, favorites and recency"
        self.search("night vision")
        self.click('Select("fullbright")')
        self.wait_for(self.ui, lambda s: s["selected"] == "fullbright", "alias search selection")
        self.passed("Alias search selects an existing module")
        self.search("x-ray context opacity")
        self.click('Select("xray")')
        self.hit('Numeric("context_opacity",')
        self.require(self.ui()["selected"] == "xray", "Setting search selected the wrong module")
        self.passed("Multiword setting search reveals the actual setting")
        self.shot("02-setting-search")
        self.search("xray")
        self.click('Favorite("xray")')
        self.wait_for(self.config, lambda c: "xray" in c["client"]["favorites"], "persisted favorite")
        self.click("Collection(Favorites)")
        state = self.wait_for(self.ui, lambda s: s["collection"] == "Favorites", "Favorites collection")
        self.require(bool(self.matching_hits(state, 'Select("xray")')), "Favorite module disappeared from Favorites")
        self.passed("Favorites filter and saved favorite use actual clicks")
        self.click("Collection(All)")
        self.search("fullbright")
        self.click('Select("fullbright")')
        self.click("Collection(Recent)")
        self.search("")
        self.wait_for(self.config, lambda c: c["client"]["recent"][:1] == ["fullbright"], "persisted most-recent selection")
        state = self.ui()
        self.require(self.matching_hits(state, 'Select("fullbright")') and self.matching_hits(state, 'Select("xray")'), "Recent collection lost visited modules")
        self.passed("Recent collection preserves visited modules and persisted order")
        self.click("Collection(All)")
        self.click("CategoryPicker")
        self.click("Category(Some(Render))")
        self.wait_for(self.ui, lambda s: s["category"] == "render", "Render category")
        self.click("CategoryPicker")
        self.click("Category(None)")
        self.wait_for(self.ui, lambda s: s["category"] is None, "all categories")
        self.passed("Nonempty category picker supports selection and All")

        self.phase = "themes, reduced motion and F1"
        for theme in ["Graphite", "Copper"]:
            self.click("Theme")
            self.wait_for(self.ui, lambda s, expected=theme: s["theme"] == expected, "theme " + theme)
            self.wait_for(self.config, lambda c, expected=theme.lower(): c["client"]["theme"] == expected, "saved theme")
            self.shot("03-theme-" + theme.lower())
        self.passed("Theme clicks persist Graphite and Copper")
        self.click("Motion")
        self.wait_for(self.config, lambda c: c["client"]["reduced_motion"] is True, "saved reduced motion")
        self.passed("Reduced motion preference persists")
        self.key("key.keyboard.f1")
        self.require(self.ui()["open"], "F1 closed the native GUI")
        self.shot("04-f1-gui")
        self.click("Theme")
        self.wait_for(self.ui, lambda s: s["theme"] == "Classic", "actionable GUI under F1")
        self.key("key.keyboard.f1")
        self.passed("F1 leaves GUI open and actionable", pixel_visibility="screenshot requires visual review")

        self.phase = "real slider endpoints, release and persistence"
        self.search("x-ray context opacity")
        original = self.config()["context_opacity"]
        for endpoint, value in [("minimum", 0), ("maximum", 255)]:
            rect = self.hit('Numeric("context_opacity",')["rect"]
            y = rect[1] + rect[3] / 2
            edge = rect[0] + (0.001 if value == 0 else rect[2] - 0.001)
            self.command(f"drag-at {rect[0]+rect[2]/2:.3f} {y:.3f} {edge:.3f} {y:.3f}")
            self.wait_for(self.config, lambda c, want=value: c["context_opacity"] == want, "saved slider " + endpoint)
            self.require(self.ui()["dirty"] is False, "Slider release did not flush its final value")
        self.passed("Numeric slider reaches both endpoints and flushes on release", setting="context_opacity", minimum=0, maximum=255)
        self.require(original == 255, "Fresh fixture had an unexpected initial opacity")

        self.phase = "failed settings write preserves the pending GUI edit"
        blocker = self.config_path.parent / "config.tmp"
        self.require(not blocker.exists(), "Fixture temporary save path is unexpectedly occupied")
        # This empty directory is an isolated failure fixture, not game state.
        # The edit and retry themselves still use real native pointer events.
        blocker.mkdir()
        try:
            rect = self.hit('Numeric("context_opacity",')["rect"]
            y = rect[1] + rect[3]/2
            x = rect[0] + rect[2] * 99/255
            self.command(f"drag-at {rect[0]+rect[2]/2:.3f} {y:.3f} {x:.3f} {y:.3f}")
            self.wait_for(self.ui, lambda s: s.get("ui_error") and s.get("dirty") and s.get("save_blocked"), "visible failed-save pending state")
            self.require(self.config()["context_opacity"] == 255, "Failed save replaced the last valid file")
            time.sleep(0.6)
            state = self.ui()
            self.require(state.get("setting_values", {}).get("context_opacity") == 99, "Failed save discarded the visible draft value")
            self.require(state.get("dirty") and state.get("save_blocked"), "Idle frames cleared the failed draft or its retry latch")
            self.shot("05-save-error-pending")
        finally:
            blocker.rmdir()
        self.click("Close")
        self.wait_for(self.ui, lambda s: not s["open"], "successful close retry")
        self.wait_for(self.config, lambda c: c["context_opacity"] == 99, "retried saved draft")
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "GUI after save retry")
        self.search("x-ray context opacity")
        rect = self.hit('Numeric("context_opacity",')["rect"]
        self.command(f"click-at {rect[0]+rect[2]-0.001:.3f} {rect[1]+rect[3]/2:.3f}")
        self.wait_for(self.config, lambda c: c["context_opacity"] == 255, "restored opacity")
        self.passed("Save failure preserves last valid file and editable draft; explicit Close retries")

        self.phase = "context help and configurable key capture"
        self.click('Help("Opacity of ordinary textured context')
        self.shot("05-context-help")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "GUI after closing help")
        self.passed("Contextual help opens and Escape returns to GUI")
        self.search("xray")
        self.top_details()
        previous = self.bindings().get("key.rusty.xray", "key.keyboard.x")
        self.click('Bind("xray")', pane="detail")
        self.wait_for(self.ui, lambda s: s["capture"] == "xray", "key capture")
        self.shot("06-key-capture")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: s["capture"] is None, "cancelled key capture")
        self.require(self.bindings().get("key.rusty.xray", "key.keyboard.x") == previous, "Cancelling capture changed the key")
        self.click('Bind("xray")')
        self.key("key.keyboard.k")
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.xray") == "key.keyboard.k", "saved captured key")
        self.require(not self.command("rusty-status")["xray"], "Captured key leaked into an X-ray toggle")
        self.click('Bind("xray")', right=True)
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.xray") == "key.keyboard.x", "restored default binding")
        self.passed("Key capture cancels, saves a real key, and resets with right-click")
        self.click('Bind("xray")')
        rect = self.hit("Search")["rect"]
        self.command(f"click-at {rect[0]+rect[2]/2:.3f} {rect[1]+rect[3]/2:.3f} right")
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.xray") == "key.mouse.right", "canonical mouse binding")
        self.require(not self.command("rusty-status")["xray"], "Mouse capture leaked into an X-ray action")
        self.click('Bind("xray")', right=True)
        self.wait_for(self.bindings, lambda b: b.get("key.rusty.xray") == "key.keyboard.x", "reset mouse binding")
        self.passed("Mouse capture saves canonical key.mouse.right and restores the default")

        self.phase = "captured movement, held-key closure and inventory integrity"
        before = self.command("status")["player"]
        for key in ["key.keyboard.w", "key.keyboard.a", "key.keyboard.s", "key.keyboard.d"]:
            self.key(key, "down")
            time.sleep(0.12)
            self.key(key, "up")
        after = self.command("status")["player"]
        self.assert_body(before, after)
        self.require(canonical_digest(self.command("inventory")) == inventory_before, "GUI interaction changed the fixture inventory")
        self.passed("GUI captures WASD without changing body, health, food or inventory")
        self.key("key.keyboard.w", "down")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "Escape closure")
        time.sleep(0.3)
        self.assert_body(after, self.command("status")["player"])
        self.key("key.keyboard.w", "repeat")
        time.sleep(0.25)
        self.assert_body(after, self.command("status")["player"])
        self.key("key.keyboard.w", "up")
        self.require(self.command("status")["screen"] == "playing", "Escape did not return to Playing")
        self.passed("Escape closes and held or repeated movement does not resume", body_position=after["position"])
        self.pad_acceptance()
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "reopening")
        self.shot("07-reopened-final")
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: not s["open"], "Right Shift closure")
        self.require(self.command("status")["screen"] == "playing", "Right Shift did not return to Playing")
        self.passed("Right Shift reopens and closes the native screen")
        self.assert_body(initial, self.command("status")["player"])
        self.report["final_fixture_preferences"] = self.config()["client"]

    def assert_body(self, before: dict, after: dict):
        sample = {"before": before["position"], "after": after["position"], "health_before": before["health"],
                  "health_after": after["health"], "food_before": before["food"], "food_after": after["food"]}
        self.report.setdefault("body_samples", []).append(sample)
        print("BODY " + json.dumps(sample, sort_keys=True), flush=True)
        self.require(all(abs(before["position"][i] - after["position"][i]) <= 1e-5 for i in [0, 2]), "Player X/Z moved during captured GUI input")
        self.require(before["health"] == after["health"] and before["food"] == after["food"], "Player health or food changed during GUI acceptance")

    def assert_view(self, before: dict, after: dict):
        self.assert_body(before["player"], after["player"])
        delta_yaw = (after["yaw"] - before["yaw"] + 180) % 360 - 180
        self.require(abs(delta_yaw) <= 1e-4 and abs(after["pitch"]-before["pitch"]) <= 1e-4,
                     "Held controller axes rotated the view after closing GUI")

    def pad_acceptance(self):
        self.phase = "controller close requires neutral before movement and look resume"
        baseline = self.command("status")
        for button in ["east", "start"]:
            self.key("key.keyboard.right.shift")
            self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "GUI before controller close")
            before = self.command("status")
            self.command("pad stick left 0 1")
            self.command("pad stick right 0.6 0")
            time.sleep(0.15)
            self.command("pad " + button + " tap")
            self.wait_for(self.ui, lambda s: not s["open"], "controller " + button + " closure")
            self.require(self.command("status")["screen"] == "playing", "Controller close did not return to Playing")
            time.sleep(0.35)
            after = self.command("status")
            self.assert_view(before, after)
            self.passed("Controller " + button + " closes without resuming held axes", body_before=before["player"]["position"],
                        body_after=after["player"]["position"], yaw_before=before["yaw"], yaw_after=after["yaw"])
            self.command("pad release")
            time.sleep(0.16)
        fresh = self.command("status")
        self.command("pad stick left 0 1")
        try:
            moved = self.wait_for(lambda: self.command("status"),
                                  lambda s: sum((s["player"]["position"][i]-fresh["player"]["position"][i])**2 for i in [0, 2]) > 0.0004,
                                  "fresh controller movement after neutral", timeout=2)
        finally:
            self.command("pad release")
        self.passed("Controller movement resumes after neutral and a fresh tilt", body_before=fresh["player"]["position"], body_after=moved["player"]["position"])
        # An ordinary command resets only this new disposable creative fixture.
        # No installed world, remote server or account is involved.
        p = baseline["player"]["position"]
        self.command(f"command tp @s {p[0]:.9f} {p[1]:.9f} {p[2]:.9f} {baseline['yaw']:.6f} {baseline['pitch']:.6f}")
        restored = self.wait_for(lambda: self.command("status"),
                                 lambda s: all(abs(s["player"]["position"][i]-p[i]) <= 1e-5 for i in [0, 2]),
                                 "fixture reset after fresh controller movement")
        time.sleep(0.15)
        self.assert_view(baseline, self.command("status"))
        self.passed("Disposable fixture restored after controller movement test", restored_position=restored["player"]["position"],
                    restored_yaw=restored["yaw"], restored_pitch=restored["pitch"])

    def finish(self):
        if self.process is not None and self.process.poll() is None:
            try:
                self.command("quit", timeout=5)
            except AcceptanceFailure:
                pass
            try:
                self.process.wait(timeout=20)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                try:
                    self.process.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait(timeout=8)
        if self.log is not None:
            self.log.close()
        # Ephemeral credentials belong to this fresh child only, never evidence.
        try:
            self.endpoint.unlink(missing_ok=True)
        except OSError:
            pass
        self.report["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        self.report["owned_process_stopped"] = self.process is None or self.process.poll() is not None
        report_path = self.output / "gui-acceptance.json"
        report_path.write_text(json.dumps(self.report, indent=2) + "\n", encoding="utf-8")
        print("Report: " + str(report_path), flush=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--resources", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path, help="New evidence directory; must not already exist")
    parser.add_argument("--width", default=960, type=int)
    parser.add_argument("--height", default=540, type=int)
    parser.add_argument("--startup-timeout", default=240.0, type=float)
    parser.add_argument("--terrain", choices=["vanilla", "test"], default="vanilla", help="Normal generation by default; synthetic terrain is diagnostic-only")
    args = parser.parse_args()
    if not 640 <= args.width <= 3840 or not 360 <= args.height <= 2160 or not 30 <= args.startup_timeout <= 900:
        parser.error("Window must be 640..3840 by 360..2160; startup timeout 30..900 seconds")
    smoke = None
    try:
        smoke = Smoke(args)
        smoke.exercise()
        smoke.report["passed"] = True
        return 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        message = str(exc) if isinstance(exc, AcceptanceFailure) else type(exc).__name__
        if smoke is not None:
            smoke.report["passed"] = False
            smoke.report["checks"].append({"name": smoke.phase, "passed": False, "reason": message})
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
