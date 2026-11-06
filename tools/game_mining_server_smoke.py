#!/usr/bin/env python3
"""Native Auto Mine against an owned, unmodified Java26.3 loopback server."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import subprocess
import sys
from game_java_server_smoke import JavaServer, JavaPeer
from game_mining_smoke import MiningSmoke
from game_gui_smoke import AcceptanceFailure, Smoke, digest


class RemoteMining(JavaPeer):
    miner=MiningSmoke.miner
    player=MiningSmoke.player
    fixture=MiningSmoke.fixture
    begin=MiningSmoke.begin
    exercise=MiningSmoke.exercise

    def __init__(self,args,server):
        super().__init__(args,server)
        self.report.update(suite="native-auto-mine-java-server",script_sha256=digest(Path(__file__)),
            isolated_offline_world=False,isolated_loopback_server=True,
            claims="Native client joins a separate unmodified owned Java server. Console readbacks independently verify ore removal and collected drops.",
            not_exercised=["Public/authenticated servers, server anti-Xray, server anticheats and arbitrary latency are not covered."])
        self.config_path.parent.mkdir(parents=True,exist_ok=True)
        self.config_path.write_text(json.dumps({"mining":{"radius":8,"goal":1}}))

    def completed(self):
        state=MiningSmoke.completed(self)
        rows=self.java.run("execute if block 0 200 7 minecraft:air run say RUSTY_AUTO_MINE_AIR_CONFIRMED")
        self.require(any("RUSTY_AUTO_MINE_AIR_CONFIRMED" in row for row in rows),"Authoritative server did not confirm the empty ore block")
        self.report.setdefault("server_inventory_readbacks",[]).append(self.java.inventory_receipt("auto-mine-"+str(len(self.report.get("server_inventory_readbacks",[])))))
        self.wait_for(lambda:self.java.item_count("minecraft:diamond"),lambda n:n>=1,"authoritative collected diamond",timeout=5)
        return state


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ["binary","resources","server-jar","java","output"]:p.add_argument("--"+name,required=True,type=Path)
    p.add_argument("--width",default=960,type=int);p.add_argument("--height",default=540,type=int);p.add_argument("--startup-timeout",default=240,type=float)
    args=p.parse_args();server=peer=None
    try:
        for name in ["binary","resources","server_jar","java"]:setattr(args,name,getattr(args,name).resolve(strict=True))
        # JavaServer owns a child directory; the peer creates the output root separately.
        parent=args.output.resolve().parent;parent.mkdir(parents=True,exist_ok=True)
        args.output.resolve().mkdir(exist_ok=False)
        server=JavaServer(args,args.output.resolve());server.start()
        peer_args=argparse.Namespace(**vars(args));peer_args.output=args.output.resolve()/"client"
        peer=RemoteMining(peer_args,server);peer.exercise();peer.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if peer:peer.report["passed"]=False;peer.report["checks"].append({"name":peer.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if peer:
            Smoke.finish(peer)
            (peer.output/"gui-acceptance.json").replace(peer.output/"mining-server-acceptance.json")
        if server:server.stop()

if __name__=="__main__":raise SystemExit(main())
