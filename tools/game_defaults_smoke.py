#!/usr/bin/env python3
"""Actual defaults/list controls in one owned disposable offline world."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

from game_gui_smoke import Smoke, AcceptanceFailure, canonical_digest, digest
from game_ui12_smoke import Ui12Smoke


class DefaultsSmoke(Ui12Smoke):
    def __init__(self,args):
        super().__init__(args)
        for key in ['pixel_contract','fixture_correction','photo_readiness','paired_camera_receipts','hud_input_transitions']:
            self.report.pop(key,None)
        self.report.update(suite='native-defaults-active-list',script_sha256=digest(Path(__file__)),
            claims='Actual native Default buttons, advanced target-field repair, confirmed reset and retained saved data; active-list state and captured gameplay.',
            not_exercised=['Physical device input, public servers, alternate graphics backends and unrelated modules are not accepted by this bounded fixture.','Screenshots require visual review; diagnostic active names alone do not prove rendered text.'])

    def exercise(self):
        self.launch()
        self.phase = 'individual default controls'
        self.command('options-set guiScale 2')
        self.command('options-set chatVisibility 2')
        self.command('options-set renderClouds false')
        self.open_browser(); self.select('fullbright')
        self.click('Toggle("fullbright")', pane='detail')
        self.wait_for(self.config, lambda c:c['fullbright'], 'modified boolean')
        self.require(any(h['action']=='DefaultSetting("fullbright")' for h in self.ui()['hits']),
                     'The actual menu has no individual Default setting control')
        self.click('DefaultSetting("fullbright")', pane='detail')
        self.wait_for(self.config, lambda c:not c['fullbright'], 'boolean default restored')
        self.passed('Individual boolean Default restores its actual configured default')
        self.select('finder')
        self.slider('scan_radius', 1, lambda c:c['scan_radius']==128, 'changed radius', pane='detail')
        self.click('DefaultSetting("scan_radius")', pane='detail')
        self.wait_for(self.config, lambda c:c['scan_radius']==64, 'radius default')
        self.click('Bool("tracers")', pane='detail')
        self.wait_for(self.config, lambda c:not c['tracers'], 'changed tracer preference')
        self.click('DefaultSetting("tracers")', pane='detail')
        self.wait_for(self.config, lambda c:c['tracers'], 'true boolean default')
        self.passed('Numeric and true-by-default controls restore independently')
        self.select('fullbright')
        self.click('Bind("fullbright")'); self.key('key.keyboard.f10')
        self.wait_for(self.bindings, lambda b:b.get('key.rusty.fullbright')=='key.keyboard.f10', 'custom binding')
        self.click('DefaultBind("fullbright")', pane='detail')
        self.wait_for(self.bindings, lambda b:b.get('key.rusty.fullbright')=='key.keyboard.b', 'binding default')
        self.passed('Default keybind restores B after a saved custom shortcut')
        self.select('enabled_mods')
        self.slider('hud:enabled_mods:x', .4, lambda c:abs(c['client']['hud_layout'].get('enabled_mods',{}).get('x',-99)-.4)<.001, 'moved list X', pane='detail')
        self.click('DefaultSetting("hud:enabled_mods:x")', pane='detail')
        self.wait_for(self.config, lambda c:abs(c['client']['hud_layout'].get('enabled_mods',{}).get('x',-99))<.001, 'default list X')
        self.passed('New list panel has individual position defaults')
        self.phase='advanced target defaults'
        self.click('Advanced')
        self.wait_for(lambda:self.command('status'),lambda s:s['screen']=='options','advanced editor')
        self.widget('Targets')
        self.editor_click('Edit(0)')
        editor=self.wait_for(lambda:self.command('rusty-editor-ui'),lambda s:'rule_color' in s['fields'],'target color field')
        field=editor['field_order'].index('rule_color')
        self.editor_click(f'Field({field})')
        self.key('key.keyboard.left.control','down')
        try:self.key('key.keyboard.a')
        finally:self.key('key.keyboard.left.control','up')
        self.command('type invalid-color')
        self.wait_for(lambda:self.command('rusty-editor-ui'),lambda s:'invalid-color' in s['fields']['rule_color'],'invalid color entered')
        self.editor_click('Default("rule_color")')
        self.wait_for(lambda:self.command('rusty-editor-ui'),lambda s:len(s['fields']['rule_color'])==8 and s['status']=='Setting restored; Apply to save','default repairs invalid field')
        self.editor_click('Apply')
        self.wait_for(lambda:self.command('rusty-editor-ui'),lambda s:not s['dirty'],'Advanced defaults saved')
        self.key('key.keyboard.escape')
        self.wait_for(lambda:self.command('status'),lambda s:s['screen']=='playing','advanced editor closed')
        self.open_browser();self.select('enabled_mods')
        self.passed('Advanced target Default repairs invalid text and applies valid settings')
        self.phase='confirmed full reset'
        self.close_browser()
        self.command('options-set chatVisibility 0')
        self.typed_command('rusty waypoint add Keep')
        self.command('options-set chatVisibility 2')
        nav=self.config_path.parent/'navigation.json'
        # Use the actual file emitted by the normal waypoint command.
        nav_files=list(self.config_path.parent.glob('*waypoint*'))+list(self.config_path.parent.glob('*navigation*'))
        self.require(bool(nav_files),'Normal waypoint command did not save waypoint data')
        nav=nav_files[0]; nav_before=nav.read_bytes()
        self.key('key.keyboard.x')
        self.wait_for(lambda:self.command('rusty-status'),lambda s:s['xray'],'active Xray before reset')
        self.open_browser(); self.select('freecam');self.click('Toggle("freecam")',pane='detail')
        self.wait_for(lambda:self.command('rusty-status'),lambda s:s['freecam'],'active Freecam before reset')
        self.open_browser();self.select('enabled_mods')
        self.slider('menu_scale', .6, lambda c:abs(c['client']['menu_scale']-1.4)<.001, 'modified menu size')
        self.click('Bind("menu")'); self.key('key.keyboard.f9')
        self.wait_for(self.bindings, lambda b:b.get('key.rusty.menu')=='key.keyboard.f9', 'custom menu shortcut')
        presets=self.config_path.parent/'presets'; presets.mkdir(exist_ok=True)
        saved=presets/'keep.json';saved.write_text('{"schema":1}')
        before=canonical_digest(self.config())
        self.click('RestoreDefaults')
        self.wait_for(self.ui,lambda u:u['confirm_defaults'],'reset confirmation')
        self.capture('01-confirm-defaults','Rusty reset confirmation with Restore and Cancel')
        self.click('CancelDefaults')
        self.require(canonical_digest(self.config())==before,'Cancel changed settings')
        self.passed('Cancel in the full reset confirmation leaves all settings unchanged')
        self.click('RestoreDefaults');self.wait_for(self.ui,lambda u:u['confirm_defaults'],'second confirmation')
        self.click('ConfirmDefaults')
        self.wait_for(self.config,lambda c:abs(c['client']['menu_scale']-.7)<.001 and c['client']['theme']=='classic' and not c['client']['hud_layout'] and not c['fullbright'] and not c['finder_enabled'],'full saved reset')
        self.wait_for(self.bindings,lambda b:b.get('key.rusty.menu')=='key.keyboard.right.shift','full reset binding')
        self.require(saved.read_text()=='{"schema":1}','Reset changed a saved profile file')
        self.require(nav.read_bytes()==nav_before,'Reset changed saved waypoint data')
        self.wait_for(lambda:self.command('rusty-status'),lambda s:not s['xray'] and not s['freecam'],'reset runtime features')
        self.passed('Confirmed reset restores configuration and keybind defaults while keeping saved profiles and waypoints')
        self.select('xray')
        self.capture('00-default-controls','Minecraft-style native menu with individual Default controls and Restore defaults')
        self.phase='vertical active list'
        self.close_browser()
        self.set_module('status_hud',['hud'],False)
        self.set_module('enabled_mods',['enabled_mods'],True)
        self.set_module('fullbright',['fullbright'],True)
        self.set_module('no_fog',['no_fog'],True)
        self.wait_for(lambda:self.command('rusty-status'),lambda s:s['active_modules']==['Fullbright','No Fog'],'live list names')
        self.capture('02-enabled-mods-top-left','Fullbright and No Fog in separate top-left rows, with other HUD panels disabled')
        self.set_module('fullbright',['fullbright'],False)
        self.wait_for(lambda:self.command('rusty-status'),lambda s:s['active_modules']==['No Fog'],'disabled name removed')
        self.capture('03-list-updated','Only No Fog remains in the active list')
        self.set_module('enabled_mods',['enabled_mods'],False)
        self.capture('04-list-off','Active-list overlay is off')
        self.passed('Enabled Mods tracks active names in stable vertical order and can be independently disabled')

    def editor_click(self,action):
        state=self.wait_for(lambda:self.command('rusty-editor-ui'),lambda s:s['open'] and any(h['action']==action for h in s['hits']),'advanced action '+action)
        hit=next(h for h in state['hits'] if h['action']==action)
        x,y,w,h=hit['rect'];self.command(f'click-at {x+w/2:.3f} {y+h/2:.3f}');time.sleep(.15)

    def finish(self):
        self.deadline=None
        Smoke.finish(self)
        (self.output/'gui-acceptance.json').replace(self.output/'defaults-acceptance.json')

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','resources','output']:p.add_argument('--'+name,required=True,type=Path)
    p.add_argument('--width',type=int,default=1280);p.add_argument('--height',type=int,default=720)
    p.add_argument('--startup-timeout',type=float,default=300);p.add_argument('--budget',type=float,default=900)
    args=p.parse_args();s=None
    try:
        s=DefaultsSmoke(args);s.exercise();s.report['passed']=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as e:
        if s:s.report['passed']=False;s.report['checks'].append({'name':s.phase,'passed':False,'reason':str(e)})
        if s:
            try:s.report['failure_state']=s.command('status');s.report['failure_ui']=s.command('ui');s.shot('failure')
            except (AcceptanceFailure,OSError,ValueError):pass
        print('FAIL '+str(e),file=sys.stderr,flush=True);return 1
    finally:
        if s:s.finish()

if __name__=='__main__':raise SystemExit(main())
