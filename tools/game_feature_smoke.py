#!/usr/bin/env python3
"""Native feature acceptance in a new disposable offline creative world.

Example: python tools/game_feature_smoke.py --binary PATH --resources PATH
         --output target/qa/feature-report

No installed instance, account, existing world or settings are loaded. This
script launches one owned process through game_gui_smoke.Smoke. Evidence is
written only to a new output folder; screenshots still require human review.
Use the exact candidate binary and prepared legal resource directory. No LAN
guest or external server is contacted. This is intentionally a bounded fixture,
not a claim that every module or every rendering configuration is accepted.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import struct
import subprocess
import sys
import time
import zlib

from game_gui_smoke import Smoke, AcceptanceFailure, canonical_digest, digest


def png_rgb(path: Path) -> tuple[int, int, bytes]:
    """Decode native screenshot RGB/RGBA PNGs without optional image packages.

    Reject unsupported layouts explicitly rather than silently comparing the
    compressed file bytes (metadata/compression changes are not pixel changes).
    """
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise AcceptanceFailure("Pixel comparison requires a PNG")
    offset, compressed, size = 8, bytearray(), None
    while offset + 12 <= len(data):
        length = struct.unpack_from(">I", data, offset)[0]
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + length]
        if offset + length + 12 > len(data):
            raise AcceptanceFailure("Truncated screenshot PNG")
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", payload)
            if depth != 8 or color not in (2, 6) or compression or filtering or interlace:
                raise AcceptanceFailure("Screenshot PNG layout is unsupported for pixel assertions")
            if width * height > 3840 * 2160:
                raise AcceptanceFailure("Screenshot exceeds bounded pixel comparison size")
            size = width, height, 3 if color == 2 else 4
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            break
        offset += length + 12
    if size is None:
        raise AcceptanceFailure("Screenshot has no PNG dimensions")
    width, height, channels = size
    raw = zlib.decompress(compressed)
    stride = width * channels
    if len(raw) != height * (stride + 1):
        raise AcceptanceFailure("Screenshot has invalid decoded PNG length")
    previous = bytearray(stride)
    rgb = bytearray()
    for y in range(height):
        start = y * (stride + 1)
        mode, row = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        if mode > 4:
            raise AcceptanceFailure("Screenshot PNG has an invalid row filter")
        for i in range(stride):
            left = row[i - channels] if i >= channels else 0
            up = previous[i]
            upper_left = previous[i - channels] if i >= channels else 0
            if mode == 1:
                value = left
            elif mode == 2:
                value = up
            elif mode == 3:
                value = (left + up) // 2
            elif mode == 4:
                p = left + up - upper_left
                distances = abs(p - left), abs(p - up), abs(p - upper_left)
                value = (left, up, upper_left)[distances.index(min(distances))]
            else:
                value = 0
            row[i] = (row[i] + value) & 255
        if channels == 3:
            rgb.extend(row)
        else:
            for i in range(0, stride, 4):
                rgb.extend(row[i:i + 3])
        previous = row
    return width, height, bytes(rgb)


def pixel_difference(a: Path, b: Path) -> dict:
    aw, ah, first = png_rgb(a)
    bw, bh, second = png_rgb(b)
    if (aw, ah) != (bw, bh):
        raise AcceptanceFailure("Compared screenshots have different dimensions")
    # Paired captures hide the HUD through actual F1 input. This region alone
    # does not exclude chat: the earlier run's fading chat contaminated it.
    bounds = [int(aw * .35), int(ah * .42), int(aw * .65), int(ah * .68)]
    total = changed = absolute = 0
    for y in range(bounds[1], bounds[3]):
        for x in range(bounds[0], bounds[2]):
            i = (y * aw + x) * 3
            delta = [abs(first[i + c] - second[i + c]) for c in range(3)]
            changed += max(delta) > 12
            absolute += sum(delta)
            total += 1
    return {"region_xyxy": bounds, "changed_fraction": changed / total,
            "mean_absolute_channel_difference": absolute / (total * 3), "channel_tolerance": 12}


class FeatureSmoke(Smoke):
    def __init__(self, args):
        super().__init__(args)
        self.report.update({
            "suite": "native-features", "script_sha256": digest(Path(__file__)),
            "shared_harness_sha256": digest(Path(__file__).with_name("game_gui_smoke.py")),
            "claims": "Control/data assertions and bounded pixel comparisons; screenshot text/art review remains pending.",
            "not_exercised": [
                "NoFog, ClearWeather, ESP, AutoSprint and NoHurt behavior are not exercised.",
                "External servers, accounts, a LAN guest and reconnecting a separate remote client are not exercised.",
                "Integrated single-player receives authoritative chunk changes through its normal remote client path; this is not separate-server acceptance.",
                "Breadcrumb count/cap has no native status accessor. Only observed movement and screenshots are recorded; bounded capacity remains covered by the pure harness/manual native review.",
                "Waypoint persistence is checked across disconnect/reopen and dimensions, not an operating-system process restart.",
                "Utility numeric labels, container rows, compass bearing and breadcrumb visibility require review of the saved native screenshots.",
                "Window focus loss, live resize, other graphics backends and resource packs are not covered here.",
            ],
            "pixel_contract": "Fixed camera, frozen daytime, HUD hidden through F1, central wall region; change >2%, restoration <=1% above 12 RGB levels. Render queues must be zero for two distinct frames and remain zero after a 0.5s cooldown. This measures this scene only.",
            "fixture_correction": "The earlier reviewed failure included fading chat text/background in the pixel region. It does not establish a terrain restoration defect. This fixture hides HUD for paired comparisons, records queue readiness, and retains the original thresholds. Earlier failure evidence is preserved separately.",
            "photo_readiness": [],
            "paired_camera_receipts": [],
            "hud_input_transitions": [],
        })
        self.navigation_path = self.config_path.parent / "navigation.json"
        # Smoke.launch creates a fresh instance with the default visible HUD.
        # API359 exposes no hide_gui status flag; actual F1 input controls this
        # paired fixture, and the saved screenshots remain the visual evidence.
        self.hud_hidden = False

    def state(self):
        return self.command("rusty-status")

    @staticmethod
    def render_ready(status):
        values = (status.get("pending_chunks"), status.get("pending_uploads"),
                  status.get("streaming", {}).get("meshing"))
        return all(type(value) is int and value == 0 for value in values)

    def wait_render_ready(self, timeout=120):
        until = time.monotonic() + timeout
        stable = 0
        previous_frame = None
        while time.monotonic() < until:
            if self.process is not None and self.process.poll() is not None:
                raise AcceptanceFailure("Owned game exited while waiting for render queues")
            status = self.command("status")
            if not self.render_ready(status):
                stable = 0
                previous_frame = None
            elif status["frames"] != previous_frame:
                stable += 1
                previous_frame = status["frames"]
            if stable >= 2:
                time.sleep(.5)
                cooled = self.command("status")
                if self.render_ready(cooled) and cooled["frames"] > status["frames"]:
                    return cooled
                stable = 0
                previous_frame = None
            time.sleep(.08)
        raise AcceptanceFailure("Timed out waiting for zero chunk/upload/meshing queues across two frames and cooldown")

    def set_hud_hidden(self, hidden):
        if self.hud_hidden != hidden:
            self.require(self.command("status")["screen"] == "playing", "HUD fixture transition requires the Playing screen")
            self.key("key.keyboard.f1")
            self.hud_hidden = hidden
            self.report["hud_input_transitions"].append({"input": "F1 tap", "requested_hidden": hidden,
                                                         "verification": "saved screenshots; API359 has no hide_gui status flag"})

    def camera_pose(self):
        state = self.command("state")
        self.require(state.get("player") is not None, "Fixture camera has no body")
        player = state["player"]
        return {"position": player["position"], "yaw": player["yaw"], "pitch": player["pitch"]}

    @staticmethod
    def same_camera_pose(first, second, tolerance=.01):
        dyaw = (second["yaw"] - first["yaw"] + 180) % 360 - 180
        return (all(abs(first["position"][i] - second["position"][i]) <= tolerance for i in range(3))
                and abs(dyaw) <= .001 and abs(first["pitch"] - second["pitch"]) <= .001)

    def photo(self, name, expected_pose=None):
        status = self.wait_render_ready()
        self.report["photo_readiness"].append({"name": name, "pending_chunks": status["pending_chunks"],
                                               "pending_uploads": status["pending_uploads"],
                                               "meshing": status["streaming"]["meshing"], "frame": status["frames"],
                                               "stable_distinct_frames": 2, "cooldown_seconds": .5,
                                               "hud_requested_hidden": self.hud_hidden})
        receipt = None
        if expected_pose is not None:
            before = self.camera_pose()
            receipt = {"name": name, "reference": expected_pose, "before": before,
                       "before_matches_reference": self.same_camera_pose(expected_pose, before),
                       "position_tolerance": .01, "angle_tolerance_degrees": .001}
            self.report["paired_camera_receipts"].append(receipt)
            self.require(receipt["before_matches_reference"], "Camera/body changed before paired capture " + name)
        self.shot(name)
        if receipt is not None:
            after = self.camera_pose()
            receipt.update({"after": after, "after_matches_reference": self.same_camera_pose(expected_pose, after)})
            self.require(receipt["after_matches_reference"], "Camera/body changed during paired capture " + name)
        return self.output / "screenshots" / (name + ".png")

    def server(self, text):
        return self.command("command " + text, timeout=30)

    def load_area(self, x0, z0, x1, z1, y=110, dimension=None):
        """Load every touched chunk while spectator, before authoritative edits.

        A control command being dispatched does not prove that /fill succeeded.
        Concrete received block assertions still verify each fixture afterward.
        """
        if x1 < x0 or z1 < z0 or (x1>>4)-(x0>>4)>8 or (z1>>4)-(z0>>4)>8:
            raise AcceptanceFailure("Fixture load area exceeds the bounded view")
        center=[(x0+x1+1)/2,y,(z0+z1+1)/2]
        self.server("gamemode spectator @s")
        prefix=f"execute in {dimension} run " if dimension else ""
        self.server(prefix+f"tp @s {center[0]} {center[1]} {center[2]} 0 0")
        self.wait_for(lambda:self.command("state"),
                      lambda s:s.get("player") is not None and all(abs(s["player"]["position"][i]-center[i])<.05 for i in range(3)),
                      "spectator loads fixture center",timeout=60)
        chunks=[]
        for cx in range(x0>>4,(x1>>4)+1):
            for cz in range(z0>>4,(z1>>4)+1):
                seen=self.wait_for(lambda cx=cx,cz=cz:self.command(f"block {cx*16} {y} {cz*16}"),
                                   lambda block:block["loaded"],"received fixture area chunk",timeout=60)
                chunks.append({"chunk":[cx,cz],"loaded":seen["loaded"]})
        self.report.setdefault("fixture_area_receipts",[]).append({"bounds_xz":[x0,z0,x1,z1],"center":center,"dimension":dimension,"chunks":chunks})

    def settle_fixture_body(self, x, y, z, creative=True):
        self.server(f"tp @s {x} {y} {z} 0 0")
        self.server("gamemode survival @s")
        self.wait_for(lambda:self.command("state")["player"],
                      lambda p:p["on_ground"] and not p["flying"] and all(abs(p["position"][i]-[x,y,z][i])<.01 for i in range(3)),
                      "received supported fixture body",timeout=30)
        time.sleep(.25)
        stable=self.command("state")["player"]
        self.require(stable["on_ground"] and all(abs(stable["position"][i]-[x,y,z][i])<.01 for i in range(3)),"Fixture body did not remain supported across physics ticks")
        if creative:
            self.server("gamemode creative @s")

    def typed_command(self, text):
        # `chat`/`command` control shortcuts send directly to the integrated
        # server. Native mod commands must traverse ChatScreen's Enter route.
        self.key("key.keyboard.t")
        self.wait_for(lambda: self.command("status"), lambda s: s["screen"] == "chat", "normal chat screen")
        self.command("type /" + text)
        self.key("key.keyboard.enter")
        self.wait_for(lambda: self.command("status"), lambda s: s["screen"] == "playing", "chat command completion")

    def set_module(self, module, path, enabled=True):
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "module browser")
        self.search(module.replace("_", " "))
        self.click(f'Select("{module}")')
        current = self.config()
        for part in path:
            current = current[part]
        if current != enabled:
            self.click(f'Toggle("{module}")')
        def saved(value):
            for part in path:
                value = value[part]
            return value == enabled
        self.wait_for(self.config, saved, "saved module " + module)
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "closed module browser")

    def texture_endpoint(self, maximum):
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui, lambda s: s["open"] and bool(s["hits"]), "Xray settings")
        self.search("texture visibility")
        self.click('Select("xray")')
        self.top_details()
        rect = self.hit('Numeric("texture_visibility",', pane="detail")["rect"]
        y = rect[1] + rect[3] / 2
        x = rect[0] + (rect[2] - .001 if maximum else .001)
        self.command(f"drag-at {rect[0]+rect[2]/2:.3f} {y:.3f} {x:.3f} {y:.3f}")
        self.wait_for(self.config, lambda c: c["context_border_width"] >= 49.99 if maximum else c["context_border_width"] <= .11,
                      "saved texture visibility endpoint")
        self.require(not self.ui()["dirty"], "Texture slider release did not flush")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui, lambda s: not s["open"], "closed texture settings")

    def configure_scan_fixture(self):
        """Bound this known three-ore scene without accepting a partial scan.

        Ordinary vanilla generation contains many legitimate ores outside the
        scene. These are actual saved GUI controls, not a production default or
        a target-count/pixel-threshold exemption.
        """
        before=self.camera_pose()
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"controlled scanner browser")
        self.search("finder");self.click('Select("finder")')
        for key,value,minimum,maximum in [("scan_radius",12,1,128),("max_targets",4096,1,4096)]:
            self.top_details()
            rect=self.hit(f'Numeric("{key}",',pane="detail")["rect"]
            fraction=(value-minimum)/(maximum-minimum)
            x=rect[0]+max(.001,min(rect[2]-.001,rect[2]*fraction));y=rect[1]+rect[3]/2
            self.command(f"drag-at {rect[0]+rect[2]/2:.3f} {y:.3f} {x:.3f} {y:.3f}")
            self.wait_for(self.config,lambda c,k=key,v=value:c[k]==v,"saved controlled scanner "+key)
        self.search("xray");self.click('Select("xray")');self.top_details()
        rect=self.hit('Numeric("context_radius",',pane="detail")["rect"]
        x=rect[0]+rect[2]*(12-1)/(48-1);y=rect[1]+rect[3]/2
        self.command(f"drag-at {rect[0]+rect[2]/2:.3f} {y:.3f} {x:.3f} {y:.3f}")
        self.wait_for(self.config,lambda c:c["context_radius"]==12,"saved controlled textured-context radius")
        self.require(self.config()["context_outlines"] and self.config()["context_border_width"]<50,"Fixture disabled textured context instead of bounding its geometry")
        self.require(not self.ui()["dirty"],"Controlled scanner settings did not flush")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui,lambda s:not s["open"],"closed controlled scanner settings")
        after=self.camera_pose()
        self.require(self.same_camera_pose(before,after),"Scanner configuration changed the known fixture pose")
        self.report["scanner_fixture"]={"scan_radius":12,"max_targets":4096,"context_radius":12,
            "method":"actual native GUI Numeric controls",
            "known_ore_positions":[[8,97,10],[15,97,10],[16,97,10]],
            "farthest_known_horizontal_distance":(8**2+7**2)**.5,
            "pose_before":before,"pose_after":after,
            "partial_scan_accepted":False}

    @staticmethod
    def context_clear_commands(offset=0):
        """Clear whole potential context sections before the known geometry.

        Context caches count whole16^3 sections before sphere filtering. Four
        18432-block fills stay below vanilla's32768-block command limit.
        """
        return [f"fill {offset+x} {y} {offset-16} {offset+x+23} {y+15} {offset+31} air"
                for x in [-16,8] for y in [80,96]]

    def source_snapshot(self):
        return self.command("blocks 0 94 0 31 101 15")

    def scan_ready(self, minimum=0):
        return self.wait_for(self.state, lambda s: s["pending"] == 0 and s["targets"] >= minimum and not s["partial"],
                             "fully drained received-section scan", timeout=120)

    def navigation(self):
        def read():
            if self.navigation_path.stat().st_size > 4 * 1024 * 1024:
                raise AcceptanceFailure("Navigation fixture JSON exceeded its bound")
            return json.loads(self.navigation_path.read_text(encoding="utf-8"))
        try:
            return read()
        except (OSError, ValueError):
            raise AcceptanceFailure("Private waypoint persistence is not available") from None

    def wait_waypoint(self, name):
        return self.wait_for(self.navigation,
                             lambda d: any(p["name"] == name for s in d["scopes"] for p in s["waypoints"]),
                             "persisted waypoint " + name)

    def exercise(self):
        self.launch()
        self.phase = "deterministic authoritative fixture"
        self.load_area(-16,-16,31,31,y=120)
        for command in self.context_clear_commands():self.server(command)
        for x in [-8,8,24]:
            for z in [-8,8,24]:
                for y in [88,104]:
                    self.wait_for(lambda x=x,y=y,z=z:self.command(f"block {x} {y} {z}"),lambda b:b["loaded"] and b["block"]=="minecraft:air","received whole-section context clearing")
        self.report["controlled_context_sections"]={"minimum":[-16,80,-16],"maximum":[31,111,31],"section_centers_verified":18,"fill_volume_each":18432}
        for command in ["gamerule advance_time false", "gamerule advance_weather false", "gamerule spawn_mobs false", "time set noon", "weather clear",
                        "fill 0 94 0 31 94 31 stone", "fill 0 95 0 31 105 31 air", "fill 4 95 8 12 100 12 stone",
                        "fill 14 95 8 17 100 12 stone", "setblock 8 97 10 diamond_ore",
                        "setblock 15 97 10 diamond_ore", "setblock 16 97 10 diamond_ore",
                        "setblock 8 98 8 red_concrete", "tp @s 8.5 95 3.5 0 0"]:
            self.server(command)
        for x,z in [(0,0),(31,0),(0,31),(31,31),(8,3)]:
            self.wait_for(lambda x=x,z=z:self.command(f"block {x} 94 {z}"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","received platform floor")
        self.settle_fixture_body(8.5,95,3.5)
        self.command("release")
        expected_pose = {"position": [8.5, 95, 3.5], "yaw": 0, "pitch": 0}
        scene_pose = self.wait_for(self.camera_pose, lambda s: self.same_camera_pose(expected_pose, s),
                                   "settled platform body XYZ and commanded view")
        for pos in [(8, 97, 10), (15, 97, 10), (16, 97, 10)]:
            self.wait_for(lambda p=pos: self.command("block " + " ".join(map(str, p))),
                          lambda b: b["loaded"] and b["block"] == "minecraft:diamond_ore", "received enclosed/border ore")
        self.configure_scan_fixture()
        original = self.source_snapshot()
        source_hash = canonical_digest(original)
        self.passed("Known enclosed ore and both sides of chunk boundary received", ore_positions=[[8,97,10],[15,97,10],[16,97,10]], source_sha256=source_hash)

        self.phase = "Fullbright renderer hotkey"
        self.require(not self.state()["xray"] and not self.state()["fullbright"], "Fresh scene did not start with Xray and Fullbright off")
        self.key("key.keyboard.b")
        self.wait_for(self.state, lambda s: s["fullbright"], "Fullbright renderer flag on")
        self.wait_for(self.config, lambda c: c["fullbright"], "persisted Fullbright on")
        self.key("key.keyboard.b")
        self.wait_for(self.state, lambda s: not s["fullbright"], "Fullbright renderer flag off")
        self.wait_for(self.config, lambda c: not c["fullbright"], "persisted Fullbright off")
        self.key("key.keyboard.b")
        self.wait_for(self.state, lambda s: s["fullbright"], "fixed brightness for paired render comparisons")
        self.passed("B toggles saved Fullbright and actual renderer flag in both directions")

        self.phase = "Xray native terrain and received updates"
        self.set_hud_hidden(True)
        before = self.photo("01-xray-off-native-terrain", scene_pose)
        self.key("key.keyboard.x")
        self.wait_for(self.state, lambda s: s["xray"], "Xray hotkey on")
        scanned = self.scan_ready(3)
        revealed = self.photo("02-xray-on-enclosed-and-border-ore", scene_pose)
        difference = pixel_difference(before, revealed)
        self.require(difference["changed_fraction"] > .02, "Xray did not visibly change the fixed wall region")
        self.require(canonical_digest(self.source_snapshot()) == source_hash, "Xray changed received source blocks")
        self.texture_endpoint(True)
        restored = self.photo("03-xray-100-percent-native-terrain", scene_pose)
        restoration = pixel_difference(before, restored)
        self.require(restoration["changed_fraction"] <= .01, "100% texture endpoint did not restore the fixed native terrain region")
        self.require(canonical_digest(self.source_snapshot()) == source_hash, "100% endpoint changed received source blocks")
        self.texture_endpoint(False)
        self.scan_ready(3)
        # This ordinary authoritative edit must arrive through integrated
        # single-player's remote world packet path and invalidate scan caches.
        count = self.state()["targets"]
        self.server("setblock 8 97 10 stone")
        self.wait_for(lambda: self.command("block 8 97 10"), lambda b: b["block"] == "minecraft:stone", "received authoritative ore replacement")
        self.wait_for(self.state, lambda s: s["pending"] == 0 and s["targets"] == count - 1, "scan invalidation after received block update", timeout=120)
        self.server("setblock 8 97 10 diamond_ore")
        self.wait_for(lambda: self.command("block 8 97 10"), lambda b: b["block"] == "minecraft:diamond_ore", "received authoritative ore restoration")
        self.wait_for(self.state, lambda s: s["pending"] == 0 and s["targets"] == count, "scan target restoration", timeout=120)
        self.key("key.keyboard.x")
        self.wait_for(self.state, lambda s: not s["xray"], "Xray off after authoritative update")
        off_again = self.photo("04-xray-off-after-update", scene_pose)
        off_difference = pixel_difference(before, off_again)
        self.require(off_difference["changed_fraction"] <= .01, "Xray off did not restore the fixed native wall region")
        self.require(canonical_digest(self.source_snapshot()) == source_hash, "Fixture source did not restore after authoritative edits")
        self.passed("Xray visibly reveals terrain, restores at 100% and off, preserves received source, and refreshes after authoritative update",
                    scan_targets=scanned["targets"], xray_pixel_difference=difference,
                    full_texture_restoration=restoration, off_restoration=off_difference,
                    source_sha256=source_hash, received_update_target_delta=-1)
        self.set_hud_hidden(False)

        self.phase = "Finder hotkey independent of Xray"
        self.key("key.keyboard.v")
        self.wait_for(self.config, lambda c: c["finder_enabled"], "Finder V saved on")
        found = self.scan_ready(3)
        self.require(not found["xray"], "Finder unexpectedly enabled Xray")
        self.require(found["nearest"] is not None, "Finder did not expose a nearest received target")
        self.photo("05-finder-v-with-xray-off")
        self.key("key.keyboard.v")
        self.wait_for(self.config, lambda c: not c["finder_enabled"], "Finder V saved off")
        self.passed("V toggles Finder with Xray off and a fully drained nonempty received-target scan", targets=found["targets"], nearest=found["nearest"])

        self.phase = "held zoom and key release"
        self.set_hud_hidden(True)
        zoom_pose = self.camera_pose()
        zoom_before = self.photo("06-zoom-released-before", zoom_pose)
        self.key("key.keyboard.c", "down")
        try:
            zoomed = self.photo("07-zoom-c-held", zoom_pose)
        finally:
            self.key("key.keyboard.c", "up")
        zoom_after = self.photo("08-zoom-c-released", zoom_pose)
        zoom_change = pixel_difference(zoom_before, zoomed)
        zoom_restore = pixel_difference(zoom_before, zoom_after)
        self.require(zoom_change["changed_fraction"] > .02, "Holding C did not visibly magnify/change the known wall region")
        self.require(zoom_restore["changed_fraction"] <= .01, "Releasing C did not restore the fixed view")
        self.passed("Held C changes the known rendered view and release restores it", held_difference=zoom_change, released_restoration=zoom_restore)
        self.set_hud_hidden(False)

        self.phase = "detached camera motion and body authority"
        body_before = self.command("status")
        self.key("key.keyboard.u")
        entered = self.wait_for(self.state, lambda s: s["freecam"] and s["camera"] is not None, "Freecam U entered")
        self.key("key.keyboard.w", "down")
        try:
            moved = self.wait_for(self.state, lambda s: sum((s["camera"][0][i]-entered["camera"][0][i])**2 for i in (0,2)) > .25,
                                  "detached camera movement", timeout=4)
            self.command("turn 25 -10")
        finally:
            self.key("key.keyboard.w", "up")
        turned = self.wait_for(self.state, lambda s: abs(s["camera"][1]-moved["camera"][1]) > 10, "detached camera turn")
        body_after = self.command("status")
        self.require(all(abs(body_before["player"]["position"][i]-body_after["player"]["position"][i]) <= 1e-5 for i in (0,2)), "Freecam moved the authoritative body XZ")
        self.require(abs(body_before["yaw"]-body_after["yaw"]) <= 1e-5 and abs(body_before["pitch"]-body_after["pitch"]) <= 1e-5, "Freecam turn changed body rotation")
        self.photo("09-freecam-detached")
        self.key("key.keyboard.u")
        self.wait_for(self.state, lambda s: not s["freecam"] and s["camera"] is None, "Freecam U exit")
        self.command("release")
        self.passed("U detaches a moving and turning camera while body XZ and rotation remain unchanged, then exits",
                    body_xz_before=[body_before["player"]["position"][i] for i in (0,2)],
                    body_xz_after=[body_after["player"]["position"][i] for i in (0,2)],
                    camera_before=entered["camera"], camera_after=turned["camera"])

        self.navigation_fixture()
        self.utility_fixture()

    def navigation_fixture(self):
        self.phase = "world and dimension scoped waypoint persistence"
        self.typed_command("rusty waypoint add QA_Overworld")
        first = self.wait_waypoint("QA_Overworld")
        first_scope = next(s for s in first["scopes"] if any(p["name"] == "QA_Overworld" for p in s["waypoints"]))
        point = next(p for p in first_scope["waypoints"] if p["name"] == "QA_Overworld")
        self.require(first_scope["scope"]["dimension"] == "minecraft:overworld", "Overworld waypoint has wrong dimension")
        self.require(first_scope["selected"] == point["id"], "New waypoint was not selected")
        self.typed_command("rusty waypoint select none")
        self.wait_for(self.navigation, lambda d: next(s for s in d["scopes"] if s["scope"] == first_scope["scope"])["selected"] is None, "waypoint deselection persistence")
        self.typed_command(f"rusty waypoint select {point['id']}")
        self.wait_for(self.navigation, lambda d: next(s for s in d["scopes"] if s["scope"] == first_scope["scope"])["selected"] == point["id"], "waypoint selection persistence")
        self.set_module("compass", ["navigation", "compass_enabled"])
        self.set_module("waypoints", ["navigation", "waypoint_hud"])
        self.set_module("breadcrumbs", ["navigation", "breadcrumbs_enabled"])
        self.photo("10-waypoint-compass-origin")
        observed = [self.command("status")["player"]["position"]]
        self.command("look 180 0")  # Fresh physical movement away from the wall.
        self.key("key.keyboard.w", "down")
        try:
            start = time.monotonic()
            while time.monotonic() - start < .65:
                observed.append(self.command("status")["player"]["position"])
                time.sleep(.08)
        finally:
            self.key("key.keyboard.w", "up")
        self.command("look 0 15")
        self.photo("11-breadcrumbs-after-observed-body-movement")
        self.require(sum((observed[-1][i]-observed[0][i])**2 for i in (0,2)) > 1, "Breadcrumb scene did not observe ordinary body movement")
        # Commands are ordinary authority-controlled local dimension changes.
        self.server("execute in minecraft:the_nether run tp @s 8.5 96 3.5 0 0")
        self.wait_for(lambda: self.command("status"), lambda s: s["dimension"] == "minecraft:the_nether" and s["chunks"] >= 9, "local Nether transition", timeout=60)
        self.typed_command("rusty waypoint add QA_Nether")
        second = self.wait_waypoint("QA_Nether")
        nether = next(s for s in second["scopes"] if any(p["name"] == "QA_Nether" for p in s["waypoints"]))
        self.require(nether["scope"]["dimension"] == "minecraft:the_nether" and nether["scope"]["identity"] == first_scope["scope"]["identity"], "Nether scope did not separate dimension under the same verified world")
        self.require(all(p["name"] != "QA_Overworld" for p in nether["waypoints"]), "Overworld waypoint leaked into Nether scope")
        self.photo("12-nether-waypoint-isolated")
        self.server("execute in minecraft:overworld run tp @s 8.5 95 3.5 0 0")
        self.wait_for(lambda: self.command("status"), lambda s: s["dimension"] == "minecraft:overworld", "return to Overworld", timeout=60)
        self.command("save", timeout=45)
        self.command("disconnect", timeout=45)
        self.wait_for(lambda: self.command("status"), lambda s: s["screen"] == "title", "saved world disconnect", timeout=45)
        self.command("open RustyFeatureOther 2 creative", timeout=45)
        self.wait_for(lambda: self.command("status"), lambda s: s["screen"] == "playing" and s["player"] is not None, "second disposable world", timeout=90)
        self.typed_command("rusty waypoint add QA_OtherWorld")
        third = self.wait_waypoint("QA_OtherWorld")
        other = next(s for s in third["scopes"] if any(p["name"] == "QA_OtherWorld" for p in s["waypoints"]))
        self.require(other["scope"]["identity"] != first_scope["scope"]["identity"], "Two disposable worlds shared a navigation identity")
        self.command("save", timeout=45)
        self.command("disconnect", timeout=45)
        self.wait_for(lambda: self.command("status"), lambda s: s["screen"] == "title", "second world disconnect", timeout=45)
        # Smoke.launch names its fresh world RustyGuiSmoke.
        self.command("open RustyGuiSmoke", timeout=45)
        self.wait_for(lambda: self.command("status"), lambda s: s["screen"] == "playing" and s["chunks"] >= 9, "original world reopen", timeout=90)
        self.typed_command("rusty waypoint list")
        # `messages.chat` on a remote connection contains server messages;
        # native_chat uses the displayed client component history instead.
        self.wait_for(lambda: self.command("messages 1"), lambda m: "QA_Overworld" in json.dumps(m["styled"]), "reloaded current-scope waypoint list")
        reloaded = self.navigation()
        self.require(next(s for s in reloaded["scopes"] if s["scope"] == first_scope["scope"])["selected"] == point["id"], "Selected waypoint did not persist across reopen")
        self.photo("13-waypoint-restored-after-reopen")
        # Private identity paths never enter the public receipt.
        self.passed("Typed waypoint commands persist selection, isolate dimensions and isolate separate worlds across disconnect/reopen",
                    scopes=len(reloaded["scopes"]), selected_id=point["id"], dimensions=sorted({s["scope"]["dimension"] for s in reloaded["scopes"]}),
                    observed_body_positions=observed, trail_assertion="body movement only; rendered trail/cap remains pending visual/manual review")

    def utility_fixture(self):
        self.phase = "received equipment and supplied container component HUD"
        for module in ["equipment_hud", "environment_hud", "container_preview"]:
            self.set_module(module, ["utilities", module])
        self.server("item replace entity @s armor.head with minecraft:diamond_helmet[minecraft:damage=100] 1")
        self.server("item replace entity @s weapon.mainhand with minecraft:diamond_pickaxe[minecraft:damage=1500] 1")
        tool = self.wait_for(lambda: self.command("inventory"),
                             lambda v: (v.get("held") or {}).get("item") == "minecraft:diamond_pickaxe" and v["held"].get("damage") == 1500
                             and any(a.get("part") == "head" and a.get("item") == "minecraft:diamond_helmet" and a.get("damage") == 100 for a in v["armor"]),
                             "received damaged tool and armor")
        self.photo("14-utility-known-durability-and-environment")
        self.server('item replace entity @s weapon.mainhand with minecraft:shulker_box[minecraft:container=[{slot:0,item:{id:"minecraft:diamond",count:7}},{slot:5,item:{id:"minecraft:stone",count:32}}]] 1')
        container = self.wait_for(lambda: self.command("inventory"),
                                  lambda v: (v.get("held") or {}).get("item") == "minecraft:shulker_box" and "minecraft:diamond" in v["held"].get("components", "") and "minecraft:stone" in v["held"].get("components", ""),
                                  "received supplied container components")
        self.photo("15-utility-supplied-shulker-contents")
        self.server("item replace entity @s weapon.mainhand with minecraft:shulker_box 1")
        self.wait_for(lambda: self.command("inventory"), lambda v: (v.get("held") or {}).get("item") == "minecraft:shulker_box" and "minecraft:container" not in v["held"].get("components", ""), "container without provided contents")
        self.photo("16-utility-contents-empty-or-not-provided")
        self.passed("Ordinary local commands provide received damaged armor/tool and shulker components for native HUD review",
                    armor_damage=100, tool_damage=tool["held"]["damage"], supplied_container_components=container["held"].get("components"),
                    expected_visual_labels=["Head: 263/363 (72%)", "Hand: 61/1561 (3%)", "1: x7 diamond", "6: x32 stone", "empty or not provided"],
                    numeric_label_assertion="pending screenshot review; data presence only asserted")

    def finish(self):
        super().finish()
        # The inherited implementation owns process cleanup and token removal.
        # Rename only this new suite's evidence file inside its fresh output.
        source = self.output / "gui-acceptance.json"
        destination = self.output / "feature-acceptance.json"
        source.replace(destination)
        print("Feature report: " + str(destination), flush=True)


def main():
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
        smoke = FeatureSmoke(args)
        smoke.exercise()
        smoke.report["passed"] = True
        return 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, zlib.error, subprocess.SubprocessError, KeyboardInterrupt) as exc:
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
