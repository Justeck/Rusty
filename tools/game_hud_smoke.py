#!/usr/bin/env python3
"""Bounded native HUD observations in a fresh private world.

Counters and native minimap descriptors are asserted. Screenshot labels,
rendered map tiles and breadcrumb geometry require explicit image review.
"""
from __future__ import annotations
import argparse
from pathlib import Path
import subprocess
import sys
import time
from game_inventory_smoke import InventorySmoke
from game_gui_smoke import Smoke, AcceptanceFailure, digest, canonical_digest


class HudSmoke(InventorySmoke):
    def __init__(self,args):
        super().__init__(args)
        self.hud_budget_active=False
        # Assistance fixtures omit capture metadata; this HUD fixture uses
        # FeatureSmoke.photo and must retain its real queue/pose receipts.
        for key in ("photo_readiness", "paired_camera_receipts", "hud_input_transitions"):
            self.report[key] = []
        self.report.update(suite="native-hud-supplement",script_sha256=digest(Path(__file__)),
            claims="Actual click counter increase/decay, native minimap descriptor/control transitions and staged native HUD screenshots.",
            manual_review="pending",not_exercised=["Screenshot text, map tiles and trail appearance require explicit image review; configuration is not rendering acceptance.","Physical controller hardware, high latency and other resource packs are not tested."])
        self.report["expected_image_content"]={}

    def command(self,*args,**kwargs):
        if self.hud_budget_active and time.monotonic()>self.hud_deadline:
            raise AcceptanceFailure("HUD supplement exceeded its 150-second post-launch budget")
        return super().command(*args,**kwargs)

    def photo_expected(self,name,expected):
        result=self.photo(name)
        self.report["expected_image_content"][name]={"expected":expected,"review":"pending"}
        return result

    def aware(self,key,on=True):self.set_module(key,["awareness",key],on)

    def minimap_controls(self):
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"native minimap controls")
        self.search("minimap");self.click('Select("minimap")')
        if self.config()["awareness"]["minimap_round"]:self.click('Bool("minimap_round")',pane="detail")
        for key,maximum,expected in [("minimap_size",True,160),("minimap_scale",False,.5)]:
            # The round switch can require scrolling below these earlier rows.
            # The bounded hit search scrolls down only; return to the top first.
            self.top_details()
            rect=self.hit(f'Numeric("{key}",',pane="detail")["rect"]
            x=rect[0]+(rect[2]-.001 if maximum else .001);y=rect[1]+rect[3]/2
            self.command(f"drag-at {rect[0]+rect[2]/2:.3f} {y:.3f} {x:.3f} {y:.3f}")
            self.wait_for(self.config,lambda c,k=key,e=expected:abs(c["awareness"][k]-e)<.001,"saved minimap endpoint "+key)
        self.require(not self.ui()["dirty"],"Minimap endpoint left an unsaved draft")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui,lambda s:not s["open"],"closed minimap settings")

    def exercise(self):
        self.launch();self.hud_deadline=time.monotonic()+150;self.hud_budget_active=True
        self.phase="native Help overlay ownership"
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"initial HUD browser")
        self.search("auto eat");self.click('Select("auto_eat")')
        self.wait_for(self.config,lambda c:"auto_eat" in c["assistance"],"persisted isolated profile")
        self.click('Help("Search names, aliases, descriptions')
        self.wait_for(self.ui,lambda s:s["open"] and not s["hits"],"help overlay owns and clears actionable GUI hits")
        self.shot("00-help-overlay")
        self.report["expected_image_content"]["00-help-overlay"]={"expected":["HELP - Escape to dismiss","Readable search/control explanation"],"review":"pending"}
        self.key("key.keyboard.escape")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"Escape dismisses help and restores actionable GUI")
        self.passed("Help overlay disables background hit actions and Escape restores them",text="pending explicit screenshot review")
        self.key("key.keyboard.escape");self.off();self.setup()
        for rule in ["gamerule advance_time false","gamerule advance_weather false","time set 6000","weather clear"]:self.server(rule)
        actions=["safe_walk","auto_jump","auto_tool","trigger_bot","auto_eat","auto_fish","auto_totem","armor_assist","refill"]
        self.require(not any(self.config()["assistance"][key] for key in actions),"Action module remained enabled in read-only HUD fixture")
        original=canonical_digest(self.command("inventory"))
        self.phase="real click counter and decay"
        self.command("look 0 -60");self.aware("clicks_hud")
        self.wait_for(lambda:self.state()["awareness"]["clicks"],lambda c:c["left"]==0 and c["right"]==0,"settled click counter",timeout=4)
        for _ in range(3):self.command("mouse left click");time.sleep(.06)
        clicks=self.wait_for(lambda:self.state()["awareness"]["clicks"],lambda c:c["left"]>=1,"actual ordinary left-click counter",timeout=3)
        self.shot("01-cps-after-real-clicks")
        self.report["expected_image_content"]["01-cps-after-real-clicks"]={"expected":"Clicks/s left count positive, right zero","review":"pending"}
        decay=self.wait_for(lambda:self.state()["awareness"]["clicks"],lambda c:c["left"]==0 and c["right"]==0,"recent click counter decays to zero",timeout=4)
        self.require(canonical_digest(self.command("inventory"))==original,"Click-counter fixture changed inventory")
        self.passed("Click Counter observes actual ordinary clicks and decays without item actions",positive=clicks,decayed=decay)
        self.aware("clicks_hud",False);self.command("look 0 25")
        self.phase="native minimap descriptors, controls and tile images"
        for text in ["fill -4 199 3 -1 199 6 red_concrete","fill 1 199 3 4 199 6 blue_concrete"]:self.server(text)
        for x,name in [(-2,"minecraft:red_concrete"),(2,"minecraft:blue_concrete")]:self.wait_for(lambda x=x:self.command(f"block {x} 199 4"),lambda b,n=name:b["block"]==n,"received distinguishable map terrain")
        self.photo_expected("02-map-off",["No minimap"])
        self.aware("minimap")
        initial=self.wait_for(self.state,lambda s:s.get("minimap") is not None,"native minimap descriptor")["minimap"]
        self.photo_expected("03-map-round-default",["Rendered received-terrain minimap","Round shape, default size96/scale1","Red/blue fixture terrain where sampled"])
        self.minimap_controls()
        changed=self.wait_for(self.state,lambda s:s.get("minimap") is not None and s["minimap"]["size"]==160 and abs(s["minimap"]["scale"]-.5)<.001 and not s["minimap"]["round"],"native square minimap at actual saved endpoints")["minimap"]
        self.photo_expected("04-map-square-160-scale-half",["Larger square native map","Received red/blue terrain tiles","No opaque blank substitute for map"])
        self.aware("minimap",False)
        self.wait_for(self.state,lambda s:s.get("minimap") is None,"native minimap removed")
        self.photo_expected("05-map-restored-off",["Minimap absent again"])
        self.passed("Minimap uses native on/off descriptors and real round/size/scale controls",initial=initial,changed=changed,rendered_tiles="pending explicit screenshot review")
        self.phase="known context HUD labels"
        for key in ["fps_hud","target_info","portal_coordinates","clock_hud"]:self.aware(key)
        self.set_module("coordinates",["coords_hud"]);self.set_module("compass",["navigation","compass_enabled"])
        self.command("look 0 60")
        target=self.command("target")
        self.photo_expected("06-context-south",["S 0 degrees","Body .5 / 200 / .5 in Overworld","Nether XZ 0.1 / 0.1","World 12:00","FPS row with native values","Received stone target and coordinates"])
        self.command("look 90 60")
        self.photo_expected("07-context-west",["W 90 degrees","FPS/clock/portal rows retained","Normally reachable received stone target"])
        self.report["hud_source_observations"]={"target":target,"body":self.player(),"native_status":self.state()}
        self.utility_fixture()
        self.report["expected_image_content"].update({"14-utility-known-durability-and-environment":{"expected":["Head: 263/363 (72%)","Hand: 61/1561 (3%) with warning","Biome and received light row"],"review":"pending"},"15-utility-supplied-shulker-contents":{"expected":["1: x7 diamond","6: x32 stone"],"review":"pending"},"16-utility-contents-empty-or-not-provided":{"expected":["Held contents: empty or not provided"],"review":"pending"}})
        self.phase="observed breadcrumb trail and selected marker images"
        self.set_module("breadcrumbs",["navigation","breadcrumbs_enabled"])
        self.typed_command("rusty waypoint add HUD_Origin");self.set_module("waypoints",["navigation","waypoint_hud"])
        before=self.player()["position"];self.command("look 180 0");self.key("key.keyboard.w","down")
        try:time.sleep(.6)
        finally:self.key("key.keyboard.w","up")
        after=self.player()["position"]
        self.require(sum((a-b)**2 for a,b in zip(before,after))>1,"Breadcrumb positive control did not move")
        self.command("look 0 25")
        self.photo_expected("08-trail-and-selected-marker",["Visible tan breadcrumb path toward prior position","HUD_Origin distance/bearing follows actual displacement"])
        self.set_module("breadcrumbs",["navigation","breadcrumbs_enabled"],False)
        self.photo_expected("09-trail-disabled",["Tan trail removed at unchanged body/camera","Waypoint remains selected"])
        self.passed("HUD known-data labels and trail off/on screenshots captured",movement_before=before,movement_after=after,labels="pending explicit image review")
        self.report["post_launch_elapsed_seconds"]=round(150-(self.hud_deadline-time.monotonic()),3)

    def finish(self):
        self.hud_budget_active=False;Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"hud-acceptance.json")


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for key in ["binary","resources","output"]:p.add_argument("--"+key,required=True,type=Path)
    p.add_argument("--width",type=int,default=1280);p.add_argument("--height",type=int,default=720);p.add_argument("--startup-timeout",type=float,default=300)
    args=p.parse_args();args.cases=[];smoke=None
    try:smoke=HudSmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:smoke.report["passed"]=False;smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()

if __name__=="__main__":raise SystemExit(main())
