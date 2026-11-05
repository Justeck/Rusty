#!/usr/bin/env python3
"""Bounded native RSS/CPU observation with real settings and world transitions.

Only new offline worlds and private settings are used. Ten minutes of measured
behavior cannot prove the absence of memory leaks, establish long-session
stability, or replace separate functional and performance acceptance.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import time

import psutil

from game_feature_smoke import FeatureSmoke
from game_gui_smoke import AcceptanceFailure, Smoke, digest


class EnduranceSmoke(FeatureSmoke):
    native_terrain = True

    def __init__(self, args):
        super().__init__(args)
        self.measure_deadline = None
        self.started = None
        self.observed = None
        self.context = "original_overworld"
        self.report.update(suite="bounded-native-endurance", script_sha256=digest(Path(__file__)),
            release_accepted=False, duration_seconds_requested=args.duration_seconds,
            sample_interval_seconds=15, samples=[], operations=[],
            conditions={"terrain":"vanilla","initial_seed":42,"render_distance":4,"fps_cap_requested":60,
                        "inactivity_limit_requested":"minimized","offscreen":True,"fresh_private_worlds":2},
            claims="Observed native process RSS/CPU and actual bounded input/context operations; no leak-free or long-session claim.",
            not_exercised=["Online servers, authenticated accounts, plugins, latency and arbitrary resource packs.",
                           "No allocator-level allocation counts are exposed or measured.",
                           "RSS includes allocator reservations, world caches and renderer resources; a short RSS trend is not a leak diagnosis.",
                           "This observation supplies no reference-client or maximum-throughput comparison."])
        for key in ["pixel_contract", "fixture_correction", "photo_readiness", "paired_camera_receipts", "hud_input_transitions"]:
            self.report.pop(key, None)

    def remaining(self):
        if self.measure_deadline is None:
            return None
        remaining = self.measure_deadline - time.monotonic()
        if remaining <= 0:
            raise AcceptanceFailure("An operation exceeded the bounded measurement deadline")
        return remaining

    def command(self, line, timeout=15):
        remaining = self.remaining()
        if remaining is not None:
            timeout = min(timeout, remaining)
        return super().command(line, timeout)

    def wait_for(self, reader, predicate, description, timeout=8):
        remaining = self.remaining()
        if remaining is not None:
            timeout = min(timeout, remaining)
        return super().wait_for(reader, predicate, description, timeout)

    def launch(self):
        # The shared GUI harness does not set the AFK option. This explicit
        # launch keeps its isolated paths while removing that measurement confound.
        for name in ["home", "launcher", "xdg-data", "xdg-config", "xdg-cache", "runtime", "temp", "appdata", "localappdata"]:
            (self.private / name).mkdir()
        env = dict(os.environ)
        for key in ("HORIZON_CONTROL", "HORIZON_HOME", "HORIZON_LAUNCHER_HOME"):
            env.pop(key, None)
        for key, folder in {"HOME":"home", "USERPROFILE":"home", "APPDATA":"appdata", "LOCALAPPDATA":"localappdata",
                            "HORIZON_LAUNCHER_HOME":"launcher", "XDG_DATA_HOME":"xdg-data", "XDG_CONFIG_HOME":"xdg-config",
                            "XDG_CACHE_HOME":"xdg-cache", "XDG_RUNTIME_DIR":"runtime", "TEMP":"temp", "TMP":"temp"}.items():
            env[key] = str(self.private / folder)
        env.update(HORIZON_HOME=str(self.home), HORIZON_NO_AUDIO="1", HORIZON_LAUNCHER_NO_SELF_UPDATE="1")
        (self.home / "options.txt").write_text('lang:en_us\npauseOnLostFocus:false\nmaxFps:60\nrenderDistance:4\ntutorialStep:none\ninactivityFpsLimit:"minimized"\n')
        other = ["appleskin", "armor_status", "biomecompass", "chatheads", "coords_hud", "distant_horizons",
                 "eatinganimation", "enchantmenthints", "fps_overlay", "itemnew", "rightharvest", "serverify"]
        (self.home / "horizon-mods.txt").write_text("mod.rusty:true\n" + "".join(f"mod.{name}:false\n" for name in other))
        self.report["other_mod_settings_disabled"] = other
        self.log = (self.private / "process.log").open("wb")
        player_name="RustyEndureQA"
        self.require(player_name.isascii() and 1<=len(player_name)<=16,"Fixture player name exceeds the pinned login protocol bound")
        self.process = subprocess.Popen([str(self.binary), "--resources", str(self.resources), "--game-dir", str(self.home),
            "--home", str(self.home), "--player-dir", str(self.home), "--offscreen", "--control", "--control-socket", str(self.endpoint),
            "--width", str(self.args.width), "--height", str(self.args.height), "--render-distance", "4", "--world", "RustyEndurance",
            "--seed", "42", "--creative", "--name", player_name], cwd=self.home, env=env,
            stdout=self.log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        self.wait_for(lambda:self.command("status", 2),
                      lambda s:s.get("player") is not None and s.get("chunks", 0)>=9 and s.get("screen")=="playing",
                      "fresh offline endurance world", self.args.startup_timeout)
        options=(self.home/"options.txt").read_text()
        self.require('inactivityFpsLimit:"minimized"' in options,"Endurance AFK policy readback differed")
        self.report["conditions"]["inactivity_limit_readback"]="minimized"
        self.observed = psutil.Process(self.process.pid)
        self.passed("Fresh isolated vanilla endurance world ready", account_loaded=False)

    def prepare_scene(self):
        self.load_area(0, 0, 15, 15, y=205)
        for command in ["gamerule advance_time false", "gamerule advance_weather false", "gamerule spawn_mobs false",
                        "time set noon", "weather clear", "fill 0 200 0 15 206 15 air", "fill 0 199 0 15 199 15 stone",
                        "setblock 8 201 10 diamond_ore", "tp @s 8.5 200 3.5 0 0", "gamemode creative @s"]:
            self.server(command)
        self.wait_for(lambda:self.command("block 8 201 10"),
                      lambda b:b["loaded"] and b["block"]=="minecraft:diamond_ore", "received endurance ore")
        self.wait_for(lambda:self.command("status"),
                      lambda s:s.get("player") and abs(s["player"]["position"][1]-200)<.05,
                      "supported endurance body")
        self.wait_render_ready(timeout=60)

    def toggle_xray(self):
        desired = not self.state()["xray"]
        self.key("key.keyboard.x")
        self.wait_for(self.state, lambda s:s["xray"]==desired and s["pending"]==0, "Xray toggle drained", timeout=30)
        return {"xray":desired}

    def settings_cycle(self):
        self.set_module("fullbright", ["fullbright"], True)
        self.wait_for(self.state, lambda s:s["fullbright"], "Fullbright renderer settings on")
        self.set_module("fullbright", ["fullbright"], False)
        self.wait_for(self.state, lambda s:not s["fullbright"], "Fullbright renderer settings restored")
        return {"saved_and_observed_fullbright_restored":True,"native_menu_visits":2}

    def dimension(self, target):
        self.server(f"execute in {target} run tp @s 8.5 205 3.5 0 0")
        self.wait_for(lambda:self.command("status"), lambda s:s["dimension"]==target and s.get("chunks", 0)>=9,
                      "endurance dimension transition", timeout=60)
        self.wait_for(self.state, lambda s:not s["xray"], "dimension clears Xray activation")
        self.context = "original_nether" if target.endswith("the_nether") else "original_overworld"
        self.prepare_scene()
        return {"dimension":target,"activation_reset":True}

    def world(self, original):
        self.command("save", 30)
        self.command("disconnect", 30)
        self.wait_for(lambda:self.command("status"), lambda s:s["screen"]=="title", "endurance world disconnect", timeout=30)
        name, seed = ("RustyEndurance",42) if original else ("RustyEnduranceOther",43)
        self.command(f"open {name} {seed} creative", 30)
        self.wait_for(lambda:self.command("status"), lambda s:s["screen"]=="playing" and s.get("player") and s.get("chunks", 0)>=9,
                      "endurance second world or original-world return", timeout=90)
        self.wait_for(self.state, lambda s:not s["xray"], "world change clears Xray activation")
        self.context = "original_overworld" if original else "other_overworld"
        self.prepare_scene()
        return {"world":"original" if original else "second_fresh_world","activation_reset":True}

    def restore_idle(self):
        if self.state()["xray"]:
            self.toggle_xray()
        self.require(not self.state()["fullbright"] and not self.ui()["open"], "Final original scene was not restored")
        return {"xray":False,"fullbright":False,"menu_open":False}

    def sample(self):
        status = self.command("status")
        state = self.state()
        memory = self.observed.memory_info()
        cpu = self.observed.cpu_times()
        server = status.get("server") or {}
        renderer = status.get("renderer") or {}
        decode = server.get("decode_errors")
        self.require(decode is None or decode==0, "Endurance observed packet decoding errors")
        row = {"elapsed_seconds":time.monotonic()-self.started,"context":self.context,
            "rss_bytes":memory.rss,"private_bytes":getattr(memory,"private",None),
            "process_cpu_seconds":cpu.user+cpu.system,"frames":status["frames"],"chunks":status["chunks"],
            "pending_chunks":status["pending_chunks"],"pending_uploads":status["pending_uploads"],
            "dimension":status["dimension"],"xray":state["xray"],"fullbright":state["fullbright"],
            "scan_pending":state["pending"],"scan_cache":state.get("scan_cache"),"decode_errors":decode,
            "renderer":{key:renderer.get(key) for key in ["resident_sections","drawn_sections","arena_bytes",
                "section_vertex_bytes","gpu_allocated_bytes","gpu_reserved_bytes","uploaded_bytes"]}}
        self.report["samples"].append(row)
        (self.output/"endurance-progress.json").write_text(json.dumps(self.report,indent=2)+"\n")

    def exercise(self):
        self.launch()
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda state:state["open"] and bool(state["hits"]),"initial saved endurance profile")
        self.search("safe walk");self.click('Select("safe_walk")');self.key("key.keyboard.escape")
        self.wait_for(self.config,lambda config:config["schema"]==1,"initial endurance preferences save")
        self.prepare_scene()
        self.settings_cycle()
        self.wait_render_ready(timeout=60)
        time.sleep(15)
        self.started = time.monotonic()
        self.measure_deadline = self.started+self.args.duration_seconds
        events = [(float(t),"xray_toggle",self.toggle_xray) for t in range(30,481,30)]
        events += [(float(t),"saved_settings_cycle",self.settings_cycle) for t in [60,120,300,480]]
        events += [(180.,"nether_transition",lambda:self.dimension("minecraft:the_nether")),
                   (240.,"overworld_return",lambda:self.dimension("minecraft:overworld")),
                   (360.,"second_world",lambda:self.world(False)),(420.,"original_world_return",lambda:self.world(True)),
                   (495.,"restore_idle_original_scene",self.restore_idle)]
        events = sorted((event for event in events if event[0]<self.args.duration_seconds-15), key=lambda e:e[0])
        next_sample = next_progress = 0.
        self.sample()
        next_sample = 15.
        while True:
            elapsed = time.monotonic()-self.started
            if elapsed>=self.args.duration_seconds:
                break
            while events and events[0][0]<=elapsed:
                scheduled, name, action = events.pop(0)
                self.phase=name
                start=time.monotonic()-self.started
                details=action()
                self.report["operations"].append({"name":name,"scheduled_seconds":scheduled,"started_seconds":start,
                    "finished_seconds":time.monotonic()-self.started,"passed":True,**details})
                elapsed=time.monotonic()-self.started
            if elapsed>=next_sample:
                self.sample()
                while next_sample<=elapsed:next_sample+=15
            if elapsed>=next_progress:
                print(f"Endurance observed {int(elapsed)} / {self.args.duration_seconds} seconds",flush=True)
                next_progress+=60
            time.sleep(min(.2,max(0,self.measure_deadline-time.monotonic())))
        self.measure_deadline=None
        self.sample()
        self.report["measured_seconds"]=time.monotonic()-self.started
        rows=self.report["samples"]
        first=[r["rss_bytes"] for r in rows if r["elapsed_seconds"]<=60]
        last=[r["rss_bytes"] for r in rows if r["elapsed_seconds"]>=self.args.duration_seconds-60]
        idle=[r for r in rows if r["elapsed_seconds"]>=510 and r["context"]=="original_overworld" and not r["xray"] and not r["fullbright"]]
        self.report["observed_memory"]={"minimum_rss_bytes":min(r["rss_bytes"] for r in rows),
            "maximum_rss_bytes":max(r["rss_bytes"] for r in rows),"first_minute_median_rss_bytes":statistics.median(first),
            "last_minute_median_rss_bytes":statistics.median(last),
            "median_change_bytes":statistics.median(last)-statistics.median(first),
            "last_restored_idle_samples":len(idle),
            "idle_first_rss_bytes":idle[0]["rss_bytes"] if idle else None,
            "idle_last_rss_bytes":idle[-1]["rss_bytes"] if idle else None,
            "interpretation":"Observed resident memory only. No allocation-free, leak-free or endurance beyond this bounded interval claim."}
        self.require(len(rows)>=max(3,self.args.duration_seconds//30), "Endurance sampling coverage was insufficient")
        self.require(self.report["measured_seconds"]>=self.args.duration_seconds, "Endurance duration was incomplete")
        if self.args.duration_seconds==600:
            names=[event["name"] for event in self.report["operations"]]
            self.require(all(name in names for name in ["nether_transition","overworld_return","second_world","original_world_return","restore_idle_original_scene"]),
                         "A required endurance transition was not exercised")
            self.require(len(idle)>=5,"Restored original-scene idle observation was insufficient")
        self.passed("Bounded process observation and scheduled operations completed",sample_count=len(rows),duration_seconds=self.report["measured_seconds"])

    def finish(self):
        self.measure_deadline=None
        Smoke.finish(self)
        text=(self.private/"process.log").read_text(encoding="utf-8",errors="replace") if self.process is not None else ""
        self.report["panic_or_fatal_observed"]=any(marker in text for marker in ["panicked at", "fatal runtime error"])
        if self.report["panic_or_fatal_observed"] or not self.report["owned_process_stopped"]:
            self.report["passed"]=False
        (self.output/"gui-acceptance.json").unlink(missing_ok=True)
        (self.output/"endurance-acceptance.json").write_text(json.dumps(self.report,indent=2)+"\n")


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ["binary","resources","output"]:parser.add_argument("--"+name,required=True,type=Path)
    parser.add_argument("--width",type=int,default=960);parser.add_argument("--height",type=int,default=540)
    parser.add_argument("--startup-timeout",type=float,default=240)
    parser.add_argument("--duration-seconds",type=int,default=600)
    args=parser.parse_args()
    if not 60<=args.duration_seconds<=600:parser.error("duration-seconds must be between60and600")
    smoke=None;result=1
    try:
        smoke=EnduranceSmoke(args);smoke.exercise();smoke.report["passed"]=True;result=0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,psutil.Error,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:
            smoke.report["passed"]=False;smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True)
    finally:
        if smoke:smoke.finish()
    return result if smoke is not None and smoke.report.get("passed") else 1


if __name__=="__main__":raise SystemExit(main())
