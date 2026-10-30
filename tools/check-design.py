"""Contract checks for the actual layout source; no device or network required."""
import json,math
from pathlib import Path
root=Path(__file__).resolve().parents[1]
spec=json.loads((root/'design/collection.json').read_text(encoding='utf-8'))
W,H=spec['canvas']
keys=set()
for l in spec['layouts']:
    assert l['key'] not in keys
    keys.add(l['key'])
    widgets=l['widgets']
    assert sorted(w[0] for w in widgets)==sorted(['cpu','gpu','mem','time','weather','music'])
    for i,(kind,x,y,w,h,mode) in enumerate(widgets):
        assert x>=0 and y>=0 and x+w<=W and y+h<=H,(l['key'],kind,'outside canvas')
        assert w>0 and h>0
        for other in widgets[i+1:]:
            _,ox,oy,ow,oh,_=other
            assert x+w<=ox or ox+ow<=x or y+h<=oy or oy+oh<=y,(l['key'],kind,other[0],'overlap')
def linear(v):
    v=v/255
    return v/12.92 if v<=0.04045 else ((v+0.055)/1.055)**2.4
def lum(hex):return sum(a*linear(int(hex[i:i+2],16)) for a,i in zip([.2126,.7152,.0722],[0,2,4]))
for key,p in spec['palettes'].items():
    for fg in ['text','muted','dim','accent']:
        contrast=(lum(p[fg])+.05)/(lum(p['panel'])+.05)
        assert contrast>=4.5,(key,fg,contrast)
print(f'{len(keys)} layouts: complete information, no overlapping panels; {len(spec["palettes"])} palettes pass 4.5:1 text contrast.')
