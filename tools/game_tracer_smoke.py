#!/usr/bin/env python3
"""Actual independent tracer controls in a new, owned offline native world."""
import argparse
from pathlib import Path
import subprocess
import sys

from game_gui_smoke import AcceptanceFailure, Smoke, digest
from game_feature_smoke import FeatureSmoke


class TracerSmoke(FeatureSmoke):
    def __init__(self, args):
        super().__init__(args)
        self.report.update(suite='native-independent-tracers',
            script_sha256=digest(Path(__file__)),
            feature_harness_sha256=digest(Path(__file__).with_name('game_feature_smoke.py')),
            claims='Actual menu clicks, saved target choices, received pig/item positive and negative controls, and captured gameplay.',
            not_exercised=['A second player, public servers, other graphics backends and unrelated modules are not exercised. Player selection is covered by the pure combination tests.',
                'Screenshots require visual review; line counts alone do not establish visible geometry.'])

    def filters(self, module, values):
        self.key('key.keyboard.right.shift')
        self.wait_for(self.ui, lambda s:s['open'] and bool(s['hits']), 'target menu')
        self.search(module.replace('_', ' ')); self.click(f'Select("{module}")'); self.top_details()
        for field, wanted in values.items():
            def saved(c):
                return (c['awareness'] if field.startswith('tracer_') else c)[field]
            if saved(self.config()) != wanted:
                self.click(f'Bool("{field}")', pane='detail')
            self.wait_for(self.config, lambda c:saved(c)==wanted, 'saved '+field)
        self.key('key.keyboard.escape')
        self.wait_for(self.ui, lambda s:not s['open'], 'closed target menu')

    def lines(self, wanted):
        self.wait_for(self.state, lambda s:s['awareness']['entity_tracer_lines']==wanted,
                      f'{wanted} rendered tracer lines')

    def exercise(self):
        self.launch(); self.phase='deterministic received entities'
        self.command('options-set chatVisibility 2'); self.command('options-set renderClouds false')
        self.load_area(0,0,15,15,y=110)
        for text in ['gamerule advance_time false', 'gamerule advance_weather false',
                     'gamerule spawn_mobs false', 'time set noon', 'weather clear',
                     'fill 0 94 0 15 94 15 stone', 'fill 0 95 0 15 108 15 air',
                     'kill @e[type=!minecraft:player]',
                     'summon pig 5.5 95 8.5 {NoAI:1b,NoGravity:1b}',
                     'summon item 11.5 96 8.5 {NoGravity:1b,PickupDelay:32767s,Item:{id:"minecraft:diamond",count:1}}']:
            self.server(text)
        self.settle_fixture_body(8.5,95,3.5)
        # A brand-new home has no settings file until the first actual change.
        self.key('key.keyboard.right.shift')
        self.wait_for(self.ui, lambda s:s['open'] and bool(s['hits']), 'initial settings')
        self.search('status hud'); self.click('Select("status_hud")')
        self.click('Toggle("status_hud")')
        self.wait_for(self.config, lambda c:not c['hud'], 'first saved settings')
        self.key('key.keyboard.escape')
        self.wait_for(self.ui, lambda s:not s['open'], 'closed initial settings')
        self.set_module('entity_tracers',['awareness','entity_tracers'],True)
        self.filters('entity_tracers',{'tracer_players':False,'tracer_mobs':False,'tracer_items':False})
        self.lines(0)
        self.passed('Empty tracer target selection draws no lines')
        self.photo('01-no-targets')
        self.filters('entity_tracers',{'tracer_mobs':True})
        self.lines(1); self.photo('02-mobs-only')
        self.passed('Mobs-only selection renders one received pig tracer')
        self.filters('entity_tracers',{'tracer_mobs':False,'tracer_items':True})
        self.lines(1); self.photo('03-items-only')
        self.passed('Dropped-items-only selection renders one received item tracer')
        self.filters('entity_esp',{'esp_players':False,'esp_mobs':True,'esp_items':False})
        self.set_module('entity_esp',['entity_esp'],True)
        self.lines(1); self.photo('04-mob-box-item-tracer')
        cfg=self.config()
        self.require(cfg['esp_mobs'] and not cfg['esp_items'] and not cfg['awareness']['tracer_mobs'] and cfg['awareness']['tracer_items'], 'ESP controls changed tracer choices')
        self.passed('Mobs-only ESP coexists with items-only tracers without shared target flags')
        self.filters('entity_tracers',{'tracer_mobs':True,'tracer_items':True})
        self.lines(2); self.photo('05-both-targets')
        self.require(self.config()['esp_mobs'] and not self.config()['esp_items'], 'Tracer controls changed ESP choices')
        self.passed('Both tracer targets render while ESP still selects mobs only')
        self.set_module('entity_tracers',['awareness','entity_tracers'],False)
        self.lines(0); self.photo('06-tracers-off-esp-on')
        self.passed('Turning off tracers clears all lines while Entity ESP remains enabled')
        self.key('key.keyboard.right.shift')
        self.wait_for(self.ui, lambda s:s['open'], 'tracer settings screenshot')
        self.search('entity tracers'); self.click('Select("entity_tracers")'); self.top_details()
        self.click('DefaultSetting("tracer_mobs")', pane='detail')
        self.wait_for(self.config, lambda c:not c['awareness']['tracer_mobs'] and c['esp_mobs'], 'independent tracer Default')
        self.shot('07-tracer-target-settings')
        self.passed('Individual tracer Default restores its target without changing ESP')

    def finish(self):
        Smoke.finish(self)
        (self.output/'gui-acceptance.json').replace(self.output/'tracer-acceptance.json')


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','resources','output']:p.add_argument('--'+name,required=True,type=Path)
    p.add_argument('--width',type=int,default=1280); p.add_argument('--height',type=int,default=720)
    p.add_argument('--startup-timeout',type=float,default=300)
    args=p.parse_args(); smoke=None
    try:
        smoke=TracerSmoke(args); smoke.exercise(); smoke.report['passed']=True; return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        if smoke:
            smoke.report['passed']=False;smoke.report['checks'].append({'name':smoke.phase,'passed':False,'reason':str(error)})
            try:smoke.shot('failure')
            except (AcceptanceFailure,OSError,ValueError):pass
        print('FAIL '+str(error),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()


if __name__=='__main__':raise SystemExit(main())
