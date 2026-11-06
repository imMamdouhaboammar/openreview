#!/usr/bin/env python3
"""Raster exports and objective pixel checks from actual SVG rendering in Chrome."""
from pathlib import Path
from playwright.sync_api import sync_playwright
from PIL import Image,ImageDraw,ImageFont
import xml.etree.ElementTree as ET
R=Path(__file__).resolve().parents[1];O=R/'exports';O.mkdir(exist_ok=True)
sources=list((R/'character').rglob('*.svg'))+list((R/'scenes').glob('*.svg'))+[R/'motion/rovi-motion.svg']
def main():
 for f in sources:ET.parse(f)
 print('XML VALID',len(sources),flush=True)
 rend=[]
 with sync_playwright() as p:
  b=p.chromium.launch(headless=True,executable_path='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome')
  page=b.new_page(viewport={'width':1800,'height':1200},device_scale_factor=1)
  def export(src,dest,w=None,h=None):
   page.goto(src.as_uri(),wait_until='load')
   element=page.locator('svg')
   if w is not None:
    element.evaluate('(e,p)=>{e.setAttribute("width",p[0]);e.setAttribute("height",p[1]);e.style.width=p[0]+"px";e.style.height=p[1]+"px"}',[w,h or w])
   element.screenshot(path=str(dest),omit_background=True,animations='disabled',timeout=18000)
   im=Image.open(dest)
   assert im.width==(w or int(element.get_attribute('width'))) and im.height==(h or int(element.get_attribute('height'))),(dest,im.size)
   assert im.mode in ('RGBA','RGB'),(dest,im.mode)
   assert im.getbbox() is not None,(dest,'all transparent')
   rend.append((str(dest.relative_to(R)),im.size,im.mode))
   return im
  for sz in [24,32,64,128,256,512]:
   export(R/'character/rovi-small-mark.svg',O/f'avatar-{sz}.png',sz,sz)
   export(R/'openreview-mascot.svg',O/f'legacy-avatar-{sz}.png',sz,sz)
  im=export(R/'character/rovi-full-body-master.svg',O/'rovi-full-body-transparent.png',400,520)
  assert im.mode=='RGBA' and im.getpixel((0,0))[3]==0,'Full body PNG should have transparent margins'
  im=export(R/'character/rovi-horizontal-logo.svg',O/'rovi-horizontal-logo.png',1020,255)
  for name in ['hero','review','investigation','fix-proposed','verify-pass','verify-fail','empty-state','final-cta']:
   png=O/('scene-'+name+'.png')
   im=export(R/'scenes'/f'{name}.svg',png,1600,900)
   im.convert('RGB').save(O/('scene-'+name+'.webp'),format='WEBP',quality=84,method=5)
  # Visual inspection chart with actual raster pixels (not SVG proxies).
  board=Image.new('RGB',(1280,720),'#FAF7F0')
  draw=ImageDraw.Draw(board)
  draw.text((32,18),'ROVI ICON SIZE QA — original vs compact candidate',fill='#171717')
  sizes=[24,32,64,128,256,512]
  X=[24,100,200,340,540,790]
  for i,s in enumerate(sizes):
   x=X[i];draw.text((x,62),str(s)+'px',fill='#171717')
   src=Image.open(O/f'avatar-{s}.png').convert('RGBA')
   if s>=256:src.thumbnail((220,220))
   board.paste(src,(x,100),src)
   old=Image.open(O/f'legacy-avatar-{s}.png').convert('RGBA')
   if s>=256:old.thumbnail((220,220))
   board.paste(old,(x,382),old)
  draw.text((25,342),'COMPACT CANDIDATE / TOP  —  ORIGINAL LEGACY / BOTTOM',fill='#171717')
  board.save(O/'avatar-size-matrix.png')
  print('RASTER VERIFIED',len(rend),'PNG renders + 8 WebP scenes',flush=True)
  for file,size,mode in rend:print(file,size,mode,flush=True)
  b.close()
if __name__=='__main__':main()
