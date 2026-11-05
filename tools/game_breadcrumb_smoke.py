#!/usr/bin/env python3
"""Discriminate native breadcrumb geometry from a matched elevated view.

Observe at least six blocks of ordinary body walking, then move only Freecam.
The trail is tan ARGB0xffdca56f, not cyan. No production patch is assumed.
"""
from __future__ import annotations
import argparse
from pathlib import Path
import subprocess
import sys
import time
from game_inventory_smoke import InventorySmoke
from game_feature_smoke import png_rgb
from game_gui_smoke import Smoke, AcceptanceFailure, canonical_digest, digest


def added_tan_pixels(on,off):
    w,h,a=png_rgb(on);w2,h2,b=png_rgb(off)
    if (w,h)!=(w2,h2):raise AcceptanceFailure("Breadcrumb image sizes differ")
    bounds=[int(w*.35),int(h*.25),int(w*.65),int(h*.75)];points=[]
    for y in range(bounds[1],bounds[3]):
        for x in range(bounds[0],bounds[2]):
            k=(y*w+x)*3;r,g,blue=a[k:k+3]
            if r>150 and r>g+20 and g>blue+20 and max(abs(a[k+i]-b[k+i]) for i in range(3))>20:points.append((x,y))
    return {"region_xyxy":bounds,"added_tan_pixels":len(points),"observed_bounds":None if not points else [min(p[0] for p in points),min(p[1] for p in points),max(p[0] for p in points),max(p[1] for p in points)],"expected_argb":"0xffdca56f"}


class BreadcrumbSmoke(InventorySmoke):
    def __init__(self,args):
        self.breadcrumb_budget_active=False;super().__init__(args)
        self.report.update(suite="native-breadcrumb-visibility",script_sha256=digest(Path(__file__)),
            claims="Actual ordinary six-block body movement, matched elevated Freecam on/off images and native tan-line pixel discrimination; inventory and received source preserved.",
            not_exercised=["Trail512-point capacity and unloaded/discontinuous paths have pure tests, not this short scene.","Different graphics backends and resource-pack line shaders are not covered."])
        for key in ["photo_readiness","paired_camera_receipts","hud_input_transitions"]:self.report.setdefault(key,[])

    def command(self,*args,**kwargs):
        if self.breadcrumb_budget_active and time.monotonic()>self.breadcrumb_deadline:raise AcceptanceFailure("Breadcrumb supplement exceeded120-second post-launch budget")
        return super().command(*args,**kwargs)

    @staticmethod
    def same_view(a,b):return all(abs(x-y)<1e-4 for x,y in zip(a[0],b[0])) and abs(a[1]-b[1])<1e-4 and abs(a[2]-b[2])<1e-4

    def exercise(self):
        self.launch();self.breadcrumb_deadline=time.monotonic()+120;self.breadcrumb_budget_active=True
        self.phase="controlled breadcrumb platform"
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"initial breadcrumb browser")
        self.search("breadcrumbs");self.click('Select("breadcrumbs")');self.key("key.keyboard.escape")
        self.off();self.setup()
        for text in ["gamerule spawn_mobs false","gamerule advance_time false","time set noon","weather clear"]:self.server(text)
        self.set_module("breadcrumbs",["navigation","breadcrumbs_enabled"])
        self.require(self.config()["navigation"]["sample_distance"]==2,"Fresh trail sample distance must be two blocks")
        source=canonical_digest(self.command("blocks -6 198 -6 6 201 12"));inventory=canonical_digest(self.command("inventory"))
        initial=self.player()["position"];self.command("look 0 0");self.key("key.keyboard.w","down")
        try:self.wait_for(self.player,lambda p:p["position"][2]-initial[2]>=6.3,"ordinary six-block trail path",timeout=6)
        finally:self.key("key.keyboard.w","up")
        prior=[None]
        def stationary():
            position=self.player()["position"];old=prior[0];prior[0]=position
            return {"position":position,"delta":float("inf") if old is None else sum((a-b)**2 for a,b in zip(position,old))}
        final=self.wait_for(stationary,lambda s:s["delta"]<1e-7,"body motion settles before Freecam",timeout=5)["position"]
        self.require(final[2]-initial[2]>=6,"Trail lacked enough ordinary movement for at least three default-spaced samples")
        self.phase="elevated detached trail view"
        self.key("key.keyboard.u")
        self.wait_for(self.state,lambda s:s["freecam"] and s["camera"] is not None,"normal Freecam entered")
        self.key("key.keyboard.space","down")
        try:self.wait_for(self.state,lambda s:s["camera"] is not None and s["camera"][0][1]>=final[1]+8.5,"detached camera above entire trail",timeout=9)
        finally:self.key("key.keyboard.space","up")
        view=self.state()["camera"];self.command(f"turn {180-view[1]:.6f} {65-view[2]:.6f}")
        self.wait_for(self.state,lambda s:s["camera"] is not None and abs(s["camera"][1]-180)<.1 and abs(s["camera"][2]-65)<.1,"detached camera faces the whole walked path")
        frame=self.command("status")["frames"]
        self.wait_for(lambda:self.command("status"),lambda s:s["frames"]>=frame+2,"stationary detached camera interpolation")
        reference=self.state()["camera"]
        on=self.photo("01-tan-trail-on-overview")
        self.require(self.same_view(reference,self.state()["camera"]),"Freecam moved during enabled capture")
        self.set_module("breadcrumbs",["navigation","breadcrumbs_enabled"],False)
        self.require(self.same_view(reference,self.state()["camera"]),"Trail toggle changed the detached camera")
        off=self.photo("02-tan-trail-off-overview")
        self.require(self.same_view(reference,self.state()["camera"]),"Freecam moved during disabled capture")
        self.require(all(abs(a-b)<.01 for a,b in zip(final,self.player()["position"])),"Detached view moved the body")
        self.require(source==canonical_digest(self.command("blocks -6 198 -6 6 201 12")),"Trail/view changed received platform")
        self.require(inventory==canonical_digest(self.command("inventory")),"Trail/view changed inventory")
        difference=added_tan_pixels(on,off);self.report["trail_pixel_difference"]=difference
        self.require(difference["added_tan_pixels"]>20,"Enabled trail added no discriminating tan line from the matched elevated view")
        self.passed("Native breadcrumb path is visible from a matched elevated view and clears when disabled",body_initial=initial,body_final=final,freecam_view=reference,pixels=difference,source_unchanged=True,inventory_unchanged=True)
        self.key("key.keyboard.u")

    def finish(self):
        self.breadcrumb_budget_active=False;Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"breadcrumb-acceptance.json")


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for key in ["binary","resources","output"]:p.add_argument("--"+key,required=True,type=Path)
    p.add_argument("--width",type=int,default=1280);p.add_argument("--height",type=int,default=720);p.add_argument("--startup-timeout",type=float,default=300)
    args=p.parse_args();args.cases=[];smoke=None
    try:smoke=BreadcrumbSmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:smoke.report["passed"]=False;smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()

if __name__=="__main__":raise SystemExit(main())
