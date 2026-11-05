#!/usr/bin/env python3
"""Separate native host/guest acceptance, using two fresh offline instances.

Run from the repository root after other game fixtures have stopped:
  python tools/game_multiplayer_smoke.py --binary PATH --resources PATH
         --output target/qa/multiplayer-NEW

The guest connects only to this fixture's loopback LAN host, with guest server
commands disabled. All authoritative edits/teleports are ordinary host commands.
No installed worlds, accounts, external servers or existing settings are used.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
from game_gui_smoke import AcceptanceFailure, Smoke, canonical_digest, digest
from game_feature_smoke import FeatureSmoke

GUEST_NAME = "RustyLanGuest"


class Peer(FeatureSmoke):
    def __init__(self, args, role):
        self.role = role
        self.deadline = None
        self.stopping = False
        super().__init__(args)
        self.report.update({"suite": "multiplayer-peer", "role": role,
                            "script_sha256": digest(Path(__file__)),
                            "claims": "Peer receipt; the parent multiplayer-acceptance.json states the actual checks.",
                            "not_exercised": ["See parent multiplayer acceptance receipt for scope and limitations."]})

    def command(self, line, timeout=15):
        if self.role == "guest" and line.split(" ", 1)[0] in {"command", "chat", "tp", "fill", "give", "time"}:
            raise AcceptanceFailure("Guest fixture attempted an authoritative command")
        if self.deadline is not None and not self.stopping:
            remaining = self.deadline - time.monotonic()
            if remaining <= 0:
                raise AcceptanceFailure("Overall multiplayer fixture deadline expired")
            timeout = min(timeout, remaining)
        return super().command(line, timeout)

    def wait_for(self, reader, predicate, description, timeout=8):
        if self.deadline is not None:
            timeout = min(timeout, max(0, self.deadline - time.monotonic()))
        return super().wait_for(reader, predicate, description, timeout)

    def wait_render_ready(self, timeout=120):
        if self.deadline is not None:
            timeout = min(timeout, max(0, self.deadline - time.monotonic()))
        return super().wait_render_ready(timeout)

    def server(self, text):
        if self.role != "host":
            raise AcceptanceFailure("Only the owned host may issue server commands")
        return self.command("command " + text, timeout=30)

    def launch(self, address=None):
        for name in ["home", "launcher", "xdg-data", "xdg-config", "xdg-cache", "runtime", "temp", "appdata", "localappdata"]:
            (self.private / name).mkdir()
        env = dict(os.environ)
        for key in ("HORIZON_CONTROL", "HORIZON_HOME", "HORIZON_LAUNCHER_HOME"):
            env.pop(key, None)
        env.update({"HOME": str(self.private / "home"), "USERPROFILE": str(self.private / "home"),
                    "APPDATA": str(self.private / "appdata"), "LOCALAPPDATA": str(self.private / "localappdata"),
                    "HORIZON_HOME": str(self.home), "HORIZON_LAUNCHER_HOME": str(self.private / "launcher"),
                    "XDG_DATA_HOME": str(self.private / "xdg-data"), "XDG_CONFIG_HOME": str(self.private / "xdg-config"),
                    "XDG_CACHE_HOME": str(self.private / "xdg-cache"), "XDG_RUNTIME_DIR": str(self.private / "runtime"),
                    "TEMP": str(self.private / "temp"), "TMP": str(self.private / "temp"),
                    "HORIZON_NO_AUDIO": "1", "HORIZON_LAUNCHER_NO_SELF_UPDATE": "1"})
        (self.home / "options.txt").write_text("lang:en_us\npauseOnLostFocus:false\nmaxFps:60\nrenderDistance:4\ntutorialStep:none\n", encoding="utf-8")
        self.log = (self.private / "process.log").open("wb")
        command = [str(self.binary), "--resources", str(self.resources), "--game-dir", str(self.home), "--home", str(self.home),
                   "--player-dir", str(self.home), "--offscreen", "--control", "--control-socket", str(self.endpoint),
                   "--width", str(self.args.width), "--height", str(self.args.height), "--render-distance", "4"]
        if self.role == "host":
            command += ["--world", "RustyMultiplayerQA", "--seed", "1", "--creative", "--name", "RustyLanHost"]
            if getattr(self.args,"terrain","vanilla") == "test":
                command.append("--test-terrain")
        else:
            if not address or not address.startswith("127.0.0.1:"):
                raise AcceptanceFailure("Guest may connect only to the owned loopback host")
            command += ["--server", address, "--name", GUEST_NAME]
        self.process = subprocess.Popen(command, cwd=self.home, env=env, stdout=self.log, stderr=subprocess.STDOUT,
                                        stdin=subprocess.DEVNULL, creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        return self.ready()

    def ready(self):
        return self.wait_for(lambda: self.command("status", timeout=3),
                             lambda s: s.get("player") is not None and s.get("chunks", 0) >= 9 and s.get("screen") == "playing"
                             and (s.get("server") or {}).get("loaded"),
                             "owned " + self.role + " playing with received chunks", timeout=self.args.startup_timeout)

    def finish(self):
        self.stopping = True  # Cleanup must still quit owned children after the deadline.
        try:
            Smoke.finish(self)
        finally:
            # Preserve secret cleanup even if an exceptional process wait fails.
            self.endpoint.unlink(missing_ok=True)
        (self.output / "gui-acceptance.json").replace(self.output / "peer-receipt.json")


class Multiplayer:
    def __init__(self, args):
        self.args = args
        self.output = args.output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.phase = "initialization"
        self.peers = []
        self.deadline = time.monotonic() + args.timeout
        self.report = {"schema": 1, "suite": "separate-client-multiplayer", "checks": [],
                       "binary_sha256": digest(args.binary.resolve(strict=True)), "script_sha256": digest(Path(__file__)),
                       "harness_sha256": {"gui": digest(TOOLS / "game_gui_smoke.py"), "features": digest(TOOLS / "game_feature_smoke.py")},
                       "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                       "isolated_offline_host_and_guest": True, "guest_commands_enabled": False,
                       "fixture_correction": "Earlier outside-script screenshots placed host and guest at the same coordinates, so the host model occluded terrain. Those frames do not establish ore visibility. This fixture parks the host behind the guest before LAN join; new screenshots still require visual review.",
                       "not_exercised": ["External/dedicated servers, accounts and server-specific obfuscation/anti-cheat are not exercised.",
                                         "No server bypass or access to terrain the guest has not received is claimed.",
                                         "Screenshots require visual review; received-source equality does not by itself prove rendered face correctness.",
                                         "Separate host/guest process restart, Freecam networking, controller/focus lifecycle and utility modules are outside this fixture."]}

    def checkpoint(self, phase):
        self.phase = phase
        if time.monotonic() >= self.deadline:
            raise AcceptanceFailure("Overall multiplayer fixture deadline expired")

    def require(self, condition, message):
        if not condition:
            raise AcceptanceFailure(message)

    def passed(self, name, **details):
        self.report["checks"].append({"name": name, "passed": True, **details})
        print("PASS " + name, flush=True)

    def peer(self, role):
        options = argparse.Namespace(**vars(self.args))
        options.output = self.output / role
        peer = Peer(options, role)
        peer.deadline = self.deadline
        self.peers.append(peer)
        self.require(peer.report["binary_sha256"] == self.report["binary_sha256"], "Candidate binary changed between fixture peers")
        return peer

    def snapshot(self, guest):
        status = guest.command("status")
        server = status.get("server") or {}
        self.require(server.get("decode_errors") == 0, "Guest protocol reported a packet decode error")
        return {"dimension": status["dimension"], "chunks": status["chunks"], "pending_chunks": status["pending_chunks"],
                "pending_uploads": status["pending_uploads"], "meshing": status["streaming"]["meshing"],
                "game_mode": server.get("game_mode"), "decode_errors": server.get("decode_errors"),
                "position": status["player"]["position"], "native": guest.state()}

    def source(self, peer):
        return peer.command("blocks 4 95 8 17 100 12")

    def received_ore(self, peer, ore="minecraft:diamond_ore"):
        return peer.wait_for(lambda: peer.command("block 8 97 10"),
                             lambda b: b["loaded"] and b["block"] == ore, "guest received authoritative ore state", timeout=60)

    def teleport(self, host, guest, dimension, x=8.5, z=3.5):
        host.server(f"execute in {dimension} run tp {GUEST_NAME} {x} 95 {z} 0 0")
        guest.wait_for(lambda: guest.command("status"),
                       lambda s: s["dimension"] == dimension and s.get("player") is not None
                       and abs(s["player"]["position"][0]-x) < .05 and abs(s["player"]["position"][2]-z) < .05,
                       "authoritative guest teleport", timeout=60)
        guest.ready()
        return guest.wait_render_ready()

    def prepare_area(self, host, dimension, offset=0, ores=False):
        host.load_area(offset-16,offset-16,offset+31,offset+31,y=120,dimension=dimension)
        host.server(f"execute in {dimension} run tp @s {offset+8.5} 105 {offset+3.5} 0 0")
        host.wait_for(lambda: host.command("status"),
                      lambda s: s["dimension"] == dimension and abs(s["player"]["position"][0]-offset-8.5) < .05 and s["chunks"] >= 9,
                      "host loads disposable preparation area", timeout=90)
        host.wait_render_ready()
        for command in host.context_clear_commands(offset):host.server(command)
        for x in [-8,8,24]:
            for z in [-8,8,24]:
                for y in [88,104]:
                    host.wait_for(lambda x=x,y=y,z=z:host.command(f"block {offset+x} {y} {offset+z}"),lambda b:b["loaded"] and b["block"]=="minecraft:air","received multiplayer whole-section context clearing")
        host.server(f"fill {offset} 95 {offset} {offset+31} 104 {offset+31} air")
        host.server(f"fill {offset} 94 {offset} {offset+31} 94 {offset+31} stone")
        for x,z in [(offset,offset),(offset+31,offset),(offset,offset+31),(offset+31,offset+31)]:
            host.wait_for(lambda x=x,z=z:host.command(f"block {x} 94 {z}"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","received multiplayer preparation floor")
        if ores:
            host.server("fill 4 95 8 12 100 12 stone")
            host.server("fill 14 95 8 17 100 12 stone")
            for x in [8, 15, 16]:
                host.server(f"setblock {x} 97 10 diamond_ore")
        # Keep the host behind the guest's unchanged (8.5, 95, 3.5) view.
        # Overlapping player models otherwise fill the guest's screenshots.
        host.server("gamemode creative @s")
        host.server(f"tp @s {offset+.5} 95 {offset+.5} 0 0")
        host.wait_for(lambda: host.command("status"),
                      lambda s: s["dimension"] == dimension
                      and abs(s["player"]["position"][0]-offset-.5) < .05
                      and abs(s["player"]["position"][2]-offset-.5) < .05,
                      "host parked behind guest fixture view", timeout=30)
        host.wait_render_ready()

    def scope(self, guest, name):
        data = guest.wait_waypoint(name)
        scope = next(s for s in data["scopes"] if any(p["name"] == name for p in s["waypoints"]))
        point = next(p for p in scope["waypoints"] if p["name"] == name)
        return scope, point

    def run(self):
        self.checkpoint("fresh host and deterministic preparation")
        host = self.peer("host")
        host.launch()
        host.server("gamerule advance_time false")
        host.server("gamerule advance_weather false")
        host.server("gamerule spawn_mobs false")
        host.server("time set noon")
        host.server("difficulty peaceful")
        self.prepare_area(host, "minecraft:overworld", 1024)
        self.prepare_area(host, "minecraft:the_nether")
        self.prepare_area(host, "minecraft:overworld", ores=True)
        host.command("save", timeout=45)
        opened = host.command("lan open 0")  # Deliberately no `commands` or forced Creative mode.
        port = opened["port"]
        self.require(type(port) is int and 1 <= port <= 65535, "Owned host returned no usable LAN port")
        address = "127.0.0.1:" + str(port)
        self.checkpoint("separate ordinary Survival guest")
        guest = self.peer("guest")
        guest.launch(address)
        host.server("gamemode survival " + GUEST_NAME)
        self.teleport(host, guest, "minecraft:overworld")
        guest.wait_for(lambda: guest.command("state"), lambda s: s["gamemode"] == "survival", "guest Survival mode")
        lan = host.command("lan status")
        self.require(any(g["name"] == GUEST_NAME and g["loaded"] and g["game_mode"] == 0 for g in lan["guests"]), "Host did not observe the loaded Survival guest")
        for x in [8, 15, 16]:
            guest.wait_for(lambda at=x: guest.command(f"block {at} 97 10"),
                           lambda b: b["loaded"] and b["block"] == "minecraft:diamond_ore", "separate guest receives enclosed/border ore")
        guest.configure_scan_fixture()
        original = self.source(guest)
        source_hash = canonical_digest(original)
        self.require(canonical_digest(self.source(host)) == source_hash, "Host and guest received fixture sources differ")
        guest.photo("01-guest-normal-source")
        self.passed("Separate loaded Survival guest receives known host terrain", status=self.snapshot(guest), source_sha256=source_hash)

        self.checkpoint("remote Xray source preservation and authoritative update")
        self.require(not guest.state()["xray"], "Guest Xray did not start off")
        guest.key("key.keyboard.x")
        guest.wait_for(guest.state, lambda s: s["xray"], "guest Xray enabled")
        scan = guest.scan_ready(3)
        guest.photo("02-guest-xray-received-ore")
        self.require(canonical_digest(self.source(guest)) == source_hash, "Guest Xray changed received blocks")
        self.require(canonical_digest(self.source(host)) == source_hash, "Guest Xray changed host received blocks")
        host.server("setblock 8 97 10 stone")
        self.received_ore(guest, "minecraft:stone")
        guest.wait_for(guest.state, lambda s: s["pending"] == 0 and s["targets"] == scan["targets"]-1,
                       "remote cache refresh after host block update", timeout=120)
        host.server("setblock 8 97 10 diamond_ore")
        self.received_ore(guest)
        guest.wait_for(guest.state, lambda s: s["pending"] == 0 and s["targets"] == scan["targets"],
                       "remote cache refresh after host restoration", timeout=120)
        guest.key("key.keyboard.x")
        guest.wait_for(guest.state, lambda s: not s["xray"], "guest Xray disabled")
        guest.photo("03-guest-xray-off-restored")
        self.require(canonical_digest(self.source(guest)) == source_hash and canonical_digest(self.source(host)) == source_hash,
                     "Received fixture source did not restore after host update")
        self.passed("Remote Xray toggles preserve source and refresh after ordinary host updates", targets=scan["targets"], update_delta=-1,
                    status=self.snapshot(guest), source_sha256=source_hash, rendered_face_review="pending screenshots")
        guest.typed_command("rusty waypoint add QA_LAN_Origin")
        origin_scope, origin_point = self.scope(guest, "QA_LAN_Origin")
        self.require(origin_scope["scope"]["identity"] == "server:" + address, "Guest waypoint did not use the actual remote endpoint scope")

        self.checkpoint("distant remote chunk unload and reload")
        guest.key("key.keyboard.x")
        guest.wait_for(guest.state, lambda s: s["xray"], "Xray before distant transition")
        self.teleport(host, guest, "minecraft:overworld", 1032.5, 1027.5)
        guest.wait_for(lambda: guest.command("block 8 97 10"), lambda b: not b["loaded"], "old fixture chunk unloaded on separate guest", timeout=90)
        guest.wait_for(lambda: guest.command("block 1032 94 1027"), lambda b: b["loaded"] and b["block"] == "minecraft:stone", "distant prepared platform received", timeout=60)
        distant = self.snapshot(guest)
        guest.photo("04-guest-distant-chunks")
        self.teleport(host, guest, "minecraft:overworld")
        self.received_ore(guest)
        guest.scan_ready(3)
        self.require(canonical_digest(self.source(guest)) == source_hash, "Remote chunk reload changed received original fixture")
        self.passed("Separate guest unloads old chunk, receives distant terrain and reloads original terrain", distant_status=distant,
                    returned_status=self.snapshot(guest), restored_source_sha256=source_hash)

        self.checkpoint("remote dimension lifecycle and scoped waypoints")
        self.require(guest.state()["xray"], "Dimension reset check requires an active Xray state before transition")
        self.teleport(host, guest, "minecraft:the_nether")
        guest.wait_for(guest.state, lambda s: not s["xray"], "native activation state reset on remote dimension change")
        guest.typed_command("rusty waypoint add QA_LAN_Nether")
        nether_scope, nether_point = self.scope(guest, "QA_LAN_Nether")
        self.require(nether_scope["scope"]["identity"] == origin_scope["scope"]["identity"]
                     and nether_scope["scope"]["dimension"] == "minecraft:the_nether"
                     and all(p["name"] != "QA_LAN_Origin" for p in nether_scope["waypoints"]), "Remote waypoint dimension isolation failed")
        guest.photo("05-guest-nether-transition")
        self.teleport(host, guest, "minecraft:overworld")
        self.received_ore(guest)
        guest.typed_command(f"rusty waypoint select {origin_point['id']}")
        guest.wait_for(guest.navigation, lambda d: next(s for s in d["scopes"] if s["scope"] == origin_scope["scope"])["selected"] == origin_point["id"], "original remote waypoint selection restored")
        self.passed("Remote dimension transition resets native activation and isolates waypoint scopes", status=self.snapshot(guest),
                    overworld_waypoint_id=origin_point["id"], nether_waypoint_id=nether_point["id"])

        self.checkpoint("separate guest disconnect and same-endpoint reconnect")
        guest.command("disconnect", timeout=30)
        guest.wait_for(lambda: guest.command("status"), lambda s: s["screen"] == "title" and s["server"] is None, "guest disconnected", timeout=45)
        host.wait_for(lambda: host.command("lan status"), lambda s: all(g["name"] != GUEST_NAME for g in s["guests"]), "host observed guest departure", timeout=45)
        guest.command("connect " + address + " " + GUEST_NAME, timeout=30)
        guest.ready()
        host.server("gamemode survival " + GUEST_NAME)
        self.teleport(host, guest, "minecraft:overworld")
        guest.wait_for(lambda: guest.command("state"), lambda s: s["gamemode"] == "survival", "reconnected guest Survival mode")
        self.received_ore(guest)
        self.require(canonical_digest(self.source(guest)) == source_hash, "Reconnected guest received different original terrain")
        guest.typed_command("rusty waypoint list")
        guest.wait_for(lambda: guest.command("messages 1"), lambda s: "QA_LAN_Origin" in json.dumps(s["styled"]), "reconnected native waypoint list")
        again_scope, _ = self.scope(guest, "QA_LAN_Origin")
        self.require(again_scope["scope"] == origin_scope["scope"] and again_scope["selected"] == origin_point["id"], "Remote waypoint scope/selection changed across reconnect")
        guest.photo("06-guest-reconnected-original-world")
        self.passed("Same guest reconnects to the owned host with intact received terrain and scoped waypoint selection", status=self.snapshot(guest),
                    restored_source_sha256=source_hash, waypoint_id=origin_point["id"])
        self.checkpoint("complete")

    def finish(self):
        errors = []
        for peer in reversed(self.peers):
            try:
                peer.finish()
            except (OSError, ValueError, subprocess.SubprocessError) as exc:
                errors.append({"role": peer.role, "reason": type(exc).__name__})
        self.report["cleanup_errors"] = errors
        self.report["owned_processes_stopped"] = all(p.process is None or p.process.poll() is not None for p in self.peers)
        if errors or not self.report["owned_processes_stopped"]:
            self.report["passed"] = False
        self.report["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        path = self.output / "multiplayer-acceptance.json"
        path.write_text(json.dumps(self.report, indent=2) + "\n", encoding="utf-8")
        print("Report: " + str(path), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--resources", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path, help="New evidence directory; must not exist")
    parser.add_argument("--width", default=960, type=int)
    parser.add_argument("--height", default=540, type=int)
    parser.add_argument("--startup-timeout", default=240., type=float)
    parser.add_argument("--terrain", choices=["vanilla", "test"], default="vanilla", help="Normal generation by default; synthetic terrain is diagnostic-only")
    parser.add_argument("--timeout", default=1500., type=float, help="Overall phase deadline in seconds")
    args = parser.parse_args()
    if not 640 <= args.width <= 3840 or not 360 <= args.height <= 2160 or not 30 <= args.startup_timeout <= 900 or not 120 <= args.timeout <= 3600:
        parser.error("Window must be 640..3840 by 360..2160; startup timeout 30..900; overall timeout 120..3600 seconds")
    suite = None
    result = 1
    try:
        suite = Multiplayer(args)
        suite.run()
        suite.report["passed"] = True
        result = 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError, KeyboardInterrupt) as exc:
        reason = str(exc) if isinstance(exc, AcceptanceFailure) else type(exc).__name__
        if suite is not None:
            suite.report["passed"] = False
            suite.report["checks"].append({"name": suite.phase, "passed": False, "reason": reason})
            for peer in suite.peers:
                try:
                    if peer.process is not None and peer.process.poll() is None:
                        peer.shot("failure")
                except (AcceptanceFailure, OSError, ValueError):
                    pass
        print("FAIL " + reason, file=sys.stderr, flush=True)
    finally:
        if suite is not None:
            suite.finish()
            if not suite.report.get("passed"):
                result = 1
    return result


if __name__ == "__main__":
    raise SystemExit(main())
