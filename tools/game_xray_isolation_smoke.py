#!/usr/bin/env python3
"""Actual rendered X-ray/Finder independence in a new disposable offline world.

Use one accepted native executable and prepared legal resources. The paired
images keep F1/HUD hiding OFF: the Finder-on positive control must visibly
render lines, so a hidden overlay layer cannot produce a false pass.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys

from game_gui_smoke import Smoke, AcceptanceFailure, canonical_digest
from game_feature_smoke import FeatureSmoke, pixel_difference


class IsolationSmoke(FeatureSmoke):
    def __init__(self,args):
        super().__init__(args)
        self.report.update(suite='xray-finder-independence', claims='Actual input, saved settings, received data and bounded paired rendered pixels.',
            not_exercised=['Remote servers, other graphics backends and unrelated modules are not exercised.'],
            pixel_contract='F1 remains off. Status HUD and drifting clouds are disabled separately. Fixed body/camera, frozen daytime, drained render queues. Finder-on positive control must show a line difference.')

    def option(self,key,value):
        self.key('key.keyboard.right.shift')
        self.wait_for(self.ui,lambda s:s['open'] and bool(s['hits']),'options browser')
        self.search('finder');self.click('Select("finder")');self.top_details()
        if self.config()[key] != value:self.click(f'Bool("{key}")',pane='detail')
        self.wait_for(self.config,lambda s:s[key]==value,'saved '+key)
        self.key('key.keyboard.escape')
        self.wait_for(self.ui,lambda s:not s['open'],'closed options')

    def paired(self,name,pose):
        # Deliberately do not call set_hud_hidden: on_top gizmos require visible GUI.
        return self.photo(name,pose)

    def exercise(self):
        self.launch();self.phase='received deterministic scene'
        # Hide only fading chat, not F1: the on-top line layer must remain visible.
        self.command('options-set chatVisibility 2')
        # Clouds drift independently of the frozen world clock and would change
        # pixels seen through the terrain even when no Finder geometry exists.
        self.command('options-set renderClouds false')
        self.load_area(-16,-16,31,31,y=120)
        for command in self.context_clear_commands():self.server(command)
        for command in ['gamerule advance_time false','gamerule advance_weather false','gamerule spawn_mobs false','time set noon','weather clear',
                        'fill 0 94 0 31 94 31 stone','fill 0 95 0 31 105 31 air','fill 4 95 8 12 100 12 stone',
                        'fill 14 95 8 17 100 12 stone','setblock 8 97 10 diamond_ore','setblock 15 97 10 diamond_ore','setblock 16 97 10 diamond_ore']:
            self.server(command)
        self.wait_for(lambda:self.command('block 8 97 10'),lambda b:b['loaded'] and b['block']=='minecraft:diamond_ore','received enclosed ore')
        self.settle_fixture_body(8.5,95,3.5)
        self.configure_scan_fixture()
        self.set_module('status_hud',['hud'],False)
        self.key('key.keyboard.b')
        self.wait_for(self.state,lambda s:s['fullbright'],'fixed brightness')
        self.command('release');pose=self.camera_pose()
        source=canonical_digest(self.source_snapshot())
        self.require(self.config()['tracers'] and self.config()['block_outlines'] and not self.config()['finder_enabled'],'Fixture must preserve existing enabled line preferences with Finder off')
        self.key('key.keyboard.x');self.wait_for(self.state,lambda s:s['xray'],'X hotkey on');self.scan_ready(3)
        self.require(not self.config()['finder_enabled'],'X enabled Finder')
        enabled_preferences=self.paired('01-xray-only-saved-lines-on',pose)
        self.option('tracers',False);self.option('block_outlines',False)
        disabled_preferences=self.paired('02-xray-only-saved-lines-off',pose)
        diff=pixel_difference(enabled_preferences,disabled_preferences)
        self.report['xray_only_pixel_difference']=diff
        self.require(diff['changed_fraction']<=.00001,'X-ray alone rendered Finder lines/boxes from saved preferences')
        self.passed('X-ray alone renders identically with saved block lines/boxes on or off',pixel_difference=diff)
        self.phase='Finder opt-in rendered positive control'
        self.key('key.keyboard.v');self.wait_for(self.config,lambda c:c['finder_enabled'],'Finder enabled');self.scan_ready(3)
        no_lines=self.paired('03-finder-on-lines-off',pose)
        self.option('tracers',True)
        with_lines=self.paired('04-finder-on-lines-on',pose)
        visible=pixel_difference(no_lines,with_lines)
        self.require(visible['changed_fraction']>.0001,'Finder positive control did not visibly draw tracers')
        self.passed('Finder explicitly enables visible tracer rendering while X-ray remains on',pixel_difference=visible)
        self.key('key.keyboard.x');self.wait_for(self.state,lambda s:not s['xray'],'X-ray off independently')
        self.require(self.config()['finder_enabled'] and self.config()['tracers'],'X-ray off changed independent Finder preferences')
        ordinary_with_lines=self.paired('05-normal-terrain-finder-lines',pose)
        self.option('tracers',False)
        ordinary_no_lines=self.paired('06-normal-terrain-lines-off',pose)
        ordinary=pixel_difference(ordinary_with_lines,ordinary_no_lines)
        self.require(ordinary['changed_fraction']>.0001,'Finder lines stopped working with X-ray off')
        self.passed('Finder keeps working with ordinary terrain after X-ray is switched off',pixel_difference=ordinary)
        self.option('tracers',True);self.option('block_outlines',True)
        self.key('key.keyboard.v');self.wait_for(self.config,lambda c:not c['finder_enabled'],'Finder off')
        self.key('key.keyboard.x');self.wait_for(self.state,lambda s:s['xray'],'X-ray on after Finder off')
        final=self.paired('07-xray-only-after-independent-toggles',pose)
        restored=pixel_difference(disabled_preferences,final)
        self.require(restored['changed_fraction']<=.00001,'Saved Finder lines leaked back into X-ray after independent toggles')
        self.require(canonical_digest(self.source_snapshot())==source,'Visual toggles changed received blocks')
        self.passed('Independent hotkey toggles preserve line preferences and received block data',pixel_difference=restored)

    def finish(self):
        Smoke.finish(self)
        (self.output/'gui-acceptance.json').replace(self.output/'xray-isolation-acceptance.json')

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('binary','resources','output'):p.add_argument('--'+name,required=True,type=Path)
    p.add_argument('--width',type=int,default=1200);p.add_argument('--height',type=int,default=675)
    p.add_argument('--startup-timeout',type=float,default=300);p.add_argument('--terrain',default='vanilla',choices=['vanilla','test'])
    args=p.parse_args();s=None
    try:
        s=IsolationSmoke(args);s.exercise();s.report['passed']=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as e:
        if s:s.report['passed']=False;s.report['checks'].append(dict(name=s.phase,passed=False,reason=str(e)))
        print('FAIL '+str(e),file=sys.stderr,flush=True);return 1
    finally:
        if s:s.finish()

if __name__=='__main__':raise SystemExit(main())
