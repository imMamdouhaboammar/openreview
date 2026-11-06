#!/usr/bin/env python3
"""Browser acceptance checks for the local, no-dependency mascot gallery."""
from pathlib import Path
from playwright.sync_api import sync_playwright
ROOT=Path(__file__).resolve().parents[1]
GALLERY=ROOT/'preview'/'index.html'
MOTION=ROOT/'motion'/'rovi-motion.svg'
def main():
 assert GALLERY.exists(), 'RED: preview/index.html does not exist'
 assert MOTION.exists(), 'RED: motion/rovi-motion.svg does not exist'
 with sync_playwright() as p:
  browser=p.chromium.launch(headless=True,executable_path='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome')
  page=browser.new_page(viewport={'width':1440,'height':900})
  errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
  page.goto(GALLERY.as_uri(),wait_until='load')
  assert 'Rovi' in page.title()
  assert page.locator('[data-filter]').count()>=4
  assert page.locator('.asset').count()>=18
  assert page.locator('img').count()>=15
  # Lazy images below the fold are not loaded until requested; force loading for a real reference check.
  page.locator('img').evaluate_all('(imgs)=>imgs.forEach(i=>i.loading="eager")')
  page.wait_for_function('Array.from(document.images).every(i=>i.complete)',timeout=20000)
  assert page.locator('img').evaluate_all('(imgs)=>imgs.every(i=>i.naturalWidth>0)'), 'Broken asset reference'
  page.get_by_role('button',name='Expressions').click()
  assert page.locator('[data-filter="expressions"]').get_attribute('aria-pressed')=='true'
  assert page.locator('.asset:visible').count()>=10
  assert page.locator('.asset[data-kind="scenes"]:visible').count()==0
  page.get_by_role('button',name='Scenes').click()
  assert page.locator('.asset[data-kind="scenes"]:visible').count()>=3
  assert page.locator('.asset[data-kind="expressions"]:visible').count()==0
  page.get_by_role('button',name='All assets').click()
  assert page.locator('.asset:visible').count()>=18
  assert not errors,errors
  for width,height in [(1440,900),(390,844),(320,690)]:
   page.set_viewport_size({'width':width,'height':height})
   page.wait_for_timeout(120)
   over=page.evaluate('document.documentElement.scrollWidth > innerWidth + 2')
   assert not over,f'Horizontal overflow at {width}px'
   page.evaluate('window.scrollTo(0,0)')
   page.wait_for_timeout(90)
   page.screenshot(path=str(ROOT/'preview'/('gallery-'+str(width)+'.png')),full_page=False,animations='disabled')
  # Internal-only research board must show the captured local screenshots.
  board=ROOT/'research'/'REFERENCE_BOARD.html'
  assert board.exists()
  page.goto(board.as_uri(),wait_until='load')
  assert page.locator('img').count()>=8
  page.locator('img').evaluate_all('(imgs)=>imgs.forEach(i=>i.loading="eager")')
  page.wait_for_function('Array.from(document.images).every(i=>i.complete)',timeout=12000)
  assert page.locator('img').evaluate_all('(imgs)=>imgs.every(i=>i.naturalWidth>0)'), 'Reference board screenshot broken'
  # Test actual animated SVG independently with and without reduced-motion.
  for reduce in ['no-preference','reduce']:
   ctx=browser.new_context(viewport={'width':520,'height':430},reduced_motion=reduce)
   pg=ctx.new_page()
   pg.goto(MOTION.as_uri(),wait_until='load')
   anim=pg.locator('#rovi-move').evaluate('(e)=>getComputedStyle(e).animationName')
   if reduce=='reduce': assert anim=='none',f'Reduced-motion still animated: {anim}'
   else: assert anim!='none','Expected visible animation in normal motion mode'
   bbox=pg.locator('#rovi-move').bounding_box()
   assert bbox and bbox['x']>360 and bbox['x']+bbox['width']<675 and bbox['y']>70 and bbox['y']+bbox['height']<510, f'Rovi is out of its intended right-side panel: {reduce} {bbox}'
   at_start=pg.locator('#rovi-move').evaluate('(e)=>getComputedStyle(e).transform')
   pg.wait_for_timeout(1150)
   at_later=pg.locator('#rovi-move').evaluate('(e)=>getComputedStyle(e).transform')
   if reduce=='reduce': assert at_start==at_later, 'Motion must remain still under reduce'
   else: assert at_start!=at_later, 'Motion must visibly change over time'
   pg.screenshot(path=str(ROOT/'motion'/('motion-'+reduce+'.png')),animations='disabled')
   ctx.close()
  browser.close()
 print('GALLERY PASS: all images loaded, filter buttons work, 3 breakpoints fit, reduced-motion works, normal animation active')
if __name__=='__main__':main()
