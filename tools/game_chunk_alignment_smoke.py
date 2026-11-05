#!/usr/bin/env python3
"""Measure native chunk lines against received, server-verified pillar corners."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import sys
import time
from game_feature_smoke import FeatureSmoke, png_rgb
from game_gui_smoke import Smoke, AcceptanceFailure, canonical_digest, digest


def alignment_pixels(width, height, on, off):
    """Find the two-face corner using OFF alone, then compare the ON gold line.

    Concrete face shading supplies an independent geometric edge. The enabled
    image never selects the expected edge, and gold elsewhere cannot pass.
    """
    rows = []
    for y in range(int(height*.15), int(height*.8)):
        spans = []; start = None
        for x in range(int(width*.35), int(width*.65)):
            k = (y*width+x)*3; r,g,b = off[k:k+3]
            magenta = r > g*1.5 and b > g*1.4 and r-g > 25 and b-g > 20
            if magenta and start is None: start = x
            if start is not None and (not magenta or x == int(width*.65)-1):
                end = x if not magenta else x+1
                if end-start >= 12 and abs((start+end)/2-width/2) < width*.08:
                    spans.append((start,end))
                start = None
        if not spans: continue
        left,right = max(spans,key=lambda s:s[1]-s[0]); margin=max(4,int((right-left)*.2))
        candidates = []
        for x in range(left+margin,right-margin):
            a = [statistics.median(off[(y*width+t)*3+c] for t in range(x-3,x)) for c in range(3)]
            b = [statistics.median(off[(y*width+t)*3+c] for t in range(x,x+3)) for c in range(3)]
            score = max(abs(a[c]-b[c]) for c in range(3))
            candidates.append((score,-abs(x-(left+right)/2),x))
        if not candidates: continue
        score,_,edge = max(candidates)
        if score < 12: continue
        gold = []
        for x in range(edge-3,edge+4):
            k=(y*width+x)*3
            if max(abs(on[k+c]-[237,199,125][c]) for c in range(3)) <= 20 and max(abs(on[k+c]-off[k+c]) for c in range(3)) > 20:
                gold.append(x)
        rows.append({"y":y,"off_corner_x":edge,"off_face_transition":score,"magenta_span":[left,right],"gold_x":gold,"error_pixels":None if not gold else min(abs(x-edge) for x in gold)})
    matched=[row for row in rows if row["gold_x"]]
    span=0 if not matched else matched[-1]["y"]-matched[0]["y"]
    fraction=len(matched)/max(1,len(rows))
    return {"passed":len(matched)>=12 and span>=20 and fraction>=.8,
            "expected_gold_rgb":[237,199,125],"color_tolerance":20,"alignment_tolerance_pixels":3,
            "required_rows":12,"required_vertical_span":20,"required_fraction":.8,
            "independent_off_corner_rows":len(rows),"matched_rows":len(matched),"matched_fraction":fraction,
            "matched_vertical_span":span,"rows":rows}


class ChunkAlignmentSmoke(FeatureSmoke):
    native_terrain=True

    def __init__(self,args):
        self.budget_active=False;super().__init__(args)
        self.report.update(suite="native-chunk-corner-alignment",script_sha256=digest(Path(__file__)),
            claims="Matched native chunk-border gold line and independently detected magenta pillar face corner; received coordinates and independent integrated-server predicates.",
            not_exercised=["Other graphics backends, resource packs and distant/unloaded chunks.","Independent server predicates run in the disposable integrated server, not a separate Java process."])
        self.report.pop("pixel_contract",None);self.report.pop("fixture_correction",None)

    def command(self,*args,**kwargs):
        if self.budget_active and time.monotonic()>self.deadline:raise AcceptanceFailure("Chunk alignment exceeded180-second post-launch budget")
        return super().command(*args,**kwargs)

    def scene(self,negative=False):
        self.phase="negative chunk corner" if negative else "positive chunk corner"
        # Hidden chat suppresses received server messages as well as painting;
        # restore it before the authoritative predicate, hide only for photos.
        self.command("options-set chatVisibility 0")
        pillars=[(-17,-17),(-1,-17),(-1,-1),(-17,-1)] if negative else [(-1,-1),(16,-1),(16,16),(-1,16)]
        position=[-8.5,129,-8.5] if negative else [8.5,129,8.5]
        yaw=135 if negative else -45
        bounds=(-20,-20,20,20)
        self.load_area(*bounds,y=145)
        for text in ["gamerule advance_time false","gamerule advance_weather false","gamerule spawn_mobs false","time set noon","weather clear","clear @s","kill @e[type=minecraft:item]",
                     "fill -20 129 -20 20 140 20 air","fill -20 128 -20 20 128 20 stone"]:
            self.server(text)
        for x,z in pillars:self.server(f"fill {x} 129 {z} {x} 133 {z} minecraft:magenta_concrete")
        received=[]
        for x,z in pillars:
            for y in [129,131,133]:
                block=self.wait_for(lambda x=x,y=y,z=z:self.command(f"block {x} {y} {z}"),lambda b:b["loaded"] and b["block"]=="minecraft:magenta_concrete","received exact magenta pillar")
                received.append({"position":[x,y,z],"block":block})
        predicate=" ".join(f"if block {x} {y} {z} minecraft:magenta_concrete" for x,z in pillars for y in range(129,134))
        marker="__NEGATIVE_CHUNK_PILLARS_VERIFIED__" if negative else "__POSITIVE_CHUNK_PILLARS_VERIFIED__"
        self.server("execute "+predicate+" run say "+marker)
        server=self.wait_for(lambda:self.command("messages 50"),lambda m:marker in json.dumps(m),"independent server pillar predicates")
        self.wait_for(lambda:self.command(f"block {int(position[0]//1)} 128 {int(position[2]//1)}"),lambda b:b["block"]=="minecraft:stone","received support floor")
        self.settle_fixture_body(*position)
        self.command(f"look {yaw} -5")
        reference=self.camera_pose();inventory=canonical_digest(self.command("inventory"))
        source=canonical_digest(self.command("blocks -20 128 -20 20 134 20"))
        self.command("options-set chatVisibility 2")
        self.require(self.command("options-get chatVisibility")["options"]["chatVisibility"]["value"]==2,"Fixture chat was not hidden through ordinary options")
        self.set_module("chunk_borders",["awareness","chunk_borders"],False)
        off=self.photo(("03" if negative else "01")+"-chunk-off",reference)
        self.set_module("chunk_borders",["awareness","chunk_borders"])
        on=self.photo(("04" if negative else "02")+"-chunk-on",reference)
        w,h,a=png_rgb(on);w2,h2,b=png_rgb(off)
        self.require((w,h)==(w2,h2),"Matched chunk image sizes differ")
        measured=alignment_pixels(w,h,a,b)
        self.report.setdefault("alignment_scenes",[]).append({"negative":negative,"body":reference,"pillar_origins":pillars,"known_nearest_corner":[-16,-16] if negative else [16,16],"received":received,"server_predicate":server,"pixels":measured,"off_sha256":digest(off),"on_sha256":digest(on)})
        self.require(source==canonical_digest(self.command("blocks -20 128 -20 20 134 20")),"Chunk rendering changed received fixture blocks")
        self.require(inventory==canonical_digest(self.command("inventory")),"Chunk rendering changed inventory")
        self.require(measured["passed"],"Gold chunk boundary did not align with the independently detected magenta two-face corner")
        self.passed(self.phase+" boundary aligns to known16-block coordinate",matched_rows=measured["matched_rows"],vertical_span=measured["matched_vertical_span"],matched_fraction=measured["matched_fraction"],source_unchanged=True,inventory_unchanged=True)
        self.set_module("chunk_borders",["awareness","chunk_borders"],False)

    def exercise(self):
        self.launch();self.deadline=time.monotonic()+180;self.budget_active=True
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"initial browser")
        self.search("chunk borders");self.click('Select("chunk_borders")');self.key("key.keyboard.escape")
        self.wait_for(self.config,lambda c:c["schema"]==1,"initial saved preferences")
        for name in ["safe_walk","auto_jump","auto_tool","trigger_bot","auto_eat","auto_fish","auto_totem","armor_assist","refill"]:
            self.require(not self.config()["assistance"][name],"Fresh action module was enabled: "+name)
        self.scene();self.scene(negative=True)

    def finish(self):
        self.budget_active=False;Smoke.finish(self)
        path=self.output/"gui-acceptance.json"
        if path.exists():path.replace(self.output/"chunk-alignment-acceptance.json")


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ["binary","resources","output"]:parser.add_argument("--"+name,required=True,type=Path)
    parser.add_argument("--width",type=int,default=1280);parser.add_argument("--height",type=int,default=720);parser.add_argument("--startup-timeout",type=float,default=300)
    args=parser.parse_args();smoke=None
    try:
        smoke=ChunkAlignmentSmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:smoke.report["passed"]=False;smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()


if __name__=="__main__":raise SystemExit(main())
