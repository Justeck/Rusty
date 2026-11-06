#!/usr/bin/env python3
"""Area Clear on an owned, separate unmodified Java26.3 loopback server."""
from __future__ import annotations
import argparse
from pathlib import Path
import subprocess
import sys
from game_area_clear_smoke import AreaClearSmoke
from game_java_server_smoke import JavaPeer, JavaServer
from game_gui_smoke import Smoke, AcceptanceFailure, digest


class RemoteAreaClear(JavaPeer):
    clear = AreaClearSmoke.clear
    player = AreaClearSmoke.player
    fixture = AreaClearSmoke.fixture
    corner = AreaClearSmoke.corner
    begin = AreaClearSmoke.begin
    exercise = AreaClearSmoke.exercise

    def __init__(self, args, server):
        super().__init__(args, server)
        self.report.update(suite="native-area-clear-java-server",
            script_sha256=digest(Path(__file__)), isolated_offline_world=False,
            isolated_loopback_server=True,
            claims="Normal native survival excavation on a separate owned vanilla Java server; server console independently checks selected air and outside blocks.")

    def ended(self):
        result = AreaClearSmoke.ended(self)
        if self.phase=="protected named wall sign":
            rows=self.java.run("data get block 1 200 3 front_text")
            self.require(any("KEEP RUSTY" in row for row in rows),"Server lost protected sign text")
            self.report.setdefault("authoritative_checks",[]).append("Protected wall sign text KEEP RUSTY and its support survive")
        if result["result"] == "Area cleared":
            for x in range(2):
                for y in range(200, 202):
                    for z in range(3, 5):
                        marker = f"RUSTY_AREA_AIR_{x}_{y}_{z}"
                        rows = self.java.run(f"execute if block {x} {y} {z} air run say {marker}")
                        self.require(any(marker in row for row in rows), "Server did not confirm selected air")
            self.report.setdefault("authoritative_checks", []).append("All eight selected cells are server-confirmed air")
        return result

    def outside(self):
        AreaClearSmoke.outside(self)
        for x, y, z in [(-1, 200, 3), (2, 200, 3), (0, 199, 3), (1, 199, 4)]:
            block = "bedrock" if y == 199 else "stone"
            marker = f"RUSTY_AREA_OUTSIDE_{x}_{y}_{z}"
            rows = self.java.run(f"execute if block {x} {y} {z} {block} run say {marker}")
            self.require(any(marker in row for row in rows), "Server did not confirm outside/support preservation")
        self.report.setdefault("authoritative_checks", []).append("Adjacent stone and supporting bedrock remain server-side")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ["binary", "resources", "server-jar", "java", "output"]:
        p.add_argument("--" + name, required=True, type=Path)
    p.add_argument("--width", default=960, type=int)
    p.add_argument("--height", default=540, type=int)
    p.add_argument("--startup-timeout", default=240, type=float)
    args = p.parse_args()
    server = peer = None
    try:
        for name in ["binary", "resources", "server_jar", "java"]:
            setattr(args, name, getattr(args, name).resolve(strict=True))
        args.output = args.output.resolve()
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.mkdir(exist_ok=False)
        server = JavaServer(args, args.output)
        server.start()
        peer_args = argparse.Namespace(**vars(args))
        peer_args.output = args.output / "client"
        peer = RemoteAreaClear(peer_args, server)
        peer.exercise()
        peer.report["passed"] = True
        return 0
    except (AcceptanceFailure, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError, KeyboardInterrupt) as e:
        if peer:
            peer.report["passed"] = False
            peer.report["checks"].append({"name": peer.phase, "passed": False, "reason": str(e)})
        print("FAIL " + str(e), file=sys.stderr, flush=True)
        return 1
    finally:
        if peer:
            Smoke.finish(peer)
            (peer.output / "gui-acceptance.json").replace(peer.output / "area-clear-server-acceptance.json")
        if server:
            server.stop()


if __name__ == "__main__":
    raise SystemExit(main())
