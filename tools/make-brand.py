"""Deterministic vector mark and raster icons. No external image assets."""
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
root=Path(__file__).resolve().parents[1]
brand=root/'assets/brand'
brand.mkdir(exist_ok=True)
segments=[((12,38),(12,12)),((12,12),(38,12)),((62,12),(88,12)),((88,12),(88,38)),((88,62),(88,88)),((88,88),(62,88)),((38,88),(12,88)),((12,88),(12,62)),((38,62),(62,38))]
paths=' '.join(f'M {a[0]} {a[1]} L {b[0]} {b[1]}' for a,b in segments)
svg=f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><title>Aperture</title><path d="{paths}" fill="none" stroke="#BED9C5" stroke-width="5.6" stroke-linecap="round"/></svg>\n'
(brand/'mark.svg').write_text(svg,encoding='utf-8')
(brand/'mark-light.svg').write_text(svg.replace('#BED9C5','#1B3023'),encoding='utf-8')
size=1024
im=Image.new('RGBA',(size,size),(16,18,19,255))
d=ImageDraw.Draw(im)
for a,b in segments:
    a=tuple(round(v*size/100) for v in a); b=tuple(round(v*size/100) for v in b)
    width=round(5.6*size/100)
    d.line([a,b],fill='#BED9C5',width=width)
    for x,y in (a,b): d.ellipse((x-width/2,y-width/2,x+width/2,y+width/2),fill='#BED9C5')
im.resize((256,256),Image.Resampling.LANCZOS).save(root/'assets/studio-icon.png')
im.resize((256,256),Image.Resampling.LANCZOS).save(brand/'aperture.ico',sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])
im.resize((512,512),Image.Resampling.LANCZOS).save(brand/'mark.png')
banner=Image.new('RGB',(1600,480),'#101213')
banner.paste(im.resize((144,144),Image.Resampling.LANCZOS),(92,84))
d=ImageDraw.Draw(banner)
font=ImageFont.truetype(str(root/'assets/Inter-Light-subset.ttf'),108)
small=ImageFont.truetype(str(root/'assets/Inter-Regular-subset.ttf'),26)
d.text((282,82),'Aperture',font=font,fill='#F2F3EF')
d.text((290,213),'A quieter view of your system.',font=small,fill='#A8B0B2')
d.line([(96,325),(1504,325)],fill='#343A3D',width=1)
d.text((96,365),'SYSTEM DISPLAY / TROFEO VISION 9.16',font=small,fill='#BED9C5')
d.text((1130,365),'DESIGNED BY HUY',font=small,fill='#899496')
(root/'docs/img/aperture').mkdir(parents=True,exist_ok=True)
banner.save(root/'docs/img/aperture/brand.png')
print('Brand assets generated.')
