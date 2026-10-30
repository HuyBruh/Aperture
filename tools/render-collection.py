"""Render fixtures through the native executable; fail on any preview error."""
import json,subprocess,sys,hashlib,os
from pathlib import Path
root=Path(__file__).resolve().parents[1]
spec=json.loads((root/'design/collection.json').read_text(encoding='utf-8'))
exe=root/'target/release'/('aperture-display.exe' if sys.platform=='win32' else 'aperture-display')
out=root/'docs/img/aperture'
out.mkdir(parents=True,exist_ok=True)
scratch=root/'target/design-check'
scratch.mkdir(parents=True,exist_ok=True)
base=(root/'windows/trofeo.conf').read_text(encoding='utf-8')
results=[]
for layout in [l['key'] for l in spec['layouts']]+['focus','grid']:
    for palette in spec['palettes']:
        for state in (['music','idle','empty'] if palette=='mineral' else ['music']):
            cfg=base.replace('pro_layout = axis',f'pro_layout = {layout}').replace('pro_theme = mineral',f'pro_theme = {palette}')
            config=scratch/f'{layout}-{palette}-{state}.conf'
            config.write_text(cfg,encoding='utf-8')
            target=out/(f'{layout}.png' if palette=='mineral' and state=='music' else f'{layout}-{palette}-{state}.png')
            env=os.environ.copy()
            env.pop('APERTURE_PREVIEW_EMPTY',None)
            if state=='empty':env['APERTURE_PREVIEW_EMPTY']='1'
            result=subprocess.run([str(exe),'--config',str(config),'--preview-state','idle' if state=='empty' else state,'--preview-png',str(target)],cwd=root,capture_output=True,text=True,timeout=30,env=env)
            if result.returncode!=0 or not target.exists():raise RuntimeError(result.stderr+result.stdout)
            from PIL import Image
            im=Image.open(target).convert('RGB')
            assert im.size==(1920,462),(layout,palette,im.size)
            assert len(im.getcolors(1920*462))>100,'Blank/unrendered image'
            results.append({'layout':layout,'palette':palette,'state':state,'sha256':hashlib.sha256(target.read_bytes()).hexdigest()})
    print('Rendered',layout,flush=True)
assert len(set(r['sha256'] for r in results))==len(results),'Some combinations produced identical frames'
(scratch/'render-results.json').write_text(json.dumps(results,indent=2))
print('Native preview renders passed:',len(results))
