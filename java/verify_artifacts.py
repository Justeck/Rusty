from pathlib import Path
import zipfile,json,hashlib
b=Path(__file__).resolve().parent
version='1.3.0-beta.5'
rows=[]
for loader in ['fabric','neoforge']:
 for suffix in ['', '-sources']:
  p=b/loader/'build/libs'/f'rusty-{loader}-{version}{suffix}.jar'
  assert p.is_file(),f'Missing {p}'
  with zipfile.ZipFile(p) as z:
   names=z.namelist()
   assert 'LICENSE' in names and 'NOTICE' in names,'License notices missing'
   assert not any('/qa/' in n or 'rusty-qa' in n or 'GameAcceptance' in n for n in names),'Development fixture leaked into download'
   assert not any(n.startswith(('net/minecraft/','net/fabricmc/','net/neoforged/','com/mojang/','org/spongepowered/')) for n in names),'Game/loader dependencies must not be bundled'
   if suffix:
    assert 'dev/justeck/rusty/RustyClient.java' in names and 'dev/justeck/rusty/core/Settings.java' in names
    assert not any(n.endswith('.class') for n in names)
   else:
    assert 'dev/justeck/rusty/RustyClient.class' in names
    assert 'dev/justeck/rusty/core/Settings.class' in names
    assert 'dev/justeck/rusty/network/ConsentPayload.class' in names
    assert json.loads(z.read('rusty.mixins.json'))['required'] is True
    if loader=='fabric':
     mod=json.loads(z.read('fabric.mod.json'));assert mod['id']=='rusty' and mod['version']==version and mod['depends']['minecraft']=='26.3'
    else:
     import tomllib
     mod=tomllib.loads(z.read('META-INF/neoforge.mods.toml').decode());assert mod['mods'][0]['modId']=='rusty' and mod['mods'][0]['version']==version
     assert all(dep['side']=='BOTH' for dep in mod['dependencies']['rusty']),'Server version bounds must also be enforced'
  data=p.read_bytes();rows.append({'file':p.name,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
(b/'SHA256SUMS').write_text(''.join(r['sha256']+'  '+r['file']+'\n' for r in rows))
print(json.dumps({'verified_packages':rows},indent=2))
