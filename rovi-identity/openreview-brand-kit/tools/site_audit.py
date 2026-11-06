#!/usr/bin/env python3
import json,re,hashlib,urllib.request,urllib.parse,xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from datetime import datetime,timezone
from playwright.sync_api import sync_playwright
ROOT=Path(__file__).resolve().parents[1]
RES=ROOT/'research'
SCREEN=RES/'screenshots'/'capy'
REF=RES/'screenshots'/'creative'
SCREEN.mkdir(parents=True,exist_ok=True); REF.mkdir(parents=True,exist_ok=True)
SKIP=('/project/','/settings/','/admin/','/repos/','/invites/','/evals/','/logs/','/r/','/share/','/sign-in','/sign-up','/templates','/api/','/no-access','/unsubscribe/','/maintenance','/new','/sink/')
def sitemap(url):
 req=urllib.request.Request(url,headers={'User-Agent':'Mozilla/5.0'})
 data=urllib.request.urlopen(req,timeout=20).read()
 return list(dict.fromkeys((node.text or '').rstrip('/') for node in ET.fromstring(data).iter() if node.tag.endswith('loc') and node.text))
def public(url):
 p=urllib.parse.urlparse(url)
 return p.netloc in ('capy.ai','www.capy.ai') and not any(p.path.startswith(x) for x in SKIP)
def name(url):
 p=urllib.parse.urlparse(url)
 return re.sub('[^A-Za-z0-9-]','-',p.path.strip('/').replace('/','--') or 'home')[:65]+'-'+hashlib.sha1(url.encode()).hexdigest()[:6]
SNAP="""() => {
 const style=e=>e?getComputedStyle(e):null, h=document.querySelector('h1'), main=document.querySelector('main')||document.body, st=style(main);
 return {h1:h?.innerText?.slice(0,220)||'',h2:[...document.querySelectorAll('h2')].slice(0,8).map(e=>e.innerText.slice(0,100)),nav:(document.querySelector('header')?.innerText||'').slice(0,500),
 typography:{heading:h?style(h).fontFamily:'',headingSize:h?style(h).fontSize:'',body:st?.fontFamily||''},
 colors:{text:st?.color||'',background:style(document.body)?.backgroundColor||''},height:document.documentElement.scrollHeight,
 images:[...document.images].filter(e=>/capy|illustrat|mascot|character|hero|comic|avatar|feature/i.test(e.alt+' '+e.currentSrc)).slice(0,15).map(e=>({alt:e.alt,src:e.currentSrc.slice(0,210)})),
 motion:[...document.querySelectorAll('*')].filter(e=>{let s=style(e);return s.animationName!=='none'||parseFloat(s.transitionDuration)>0.15}).slice(0,10).map(e=>({tag:e.tagName,animation:style(e).animationName,transition:style(e).transitionDuration})),
 links:[...document.querySelectorAll('a[href]')].map(e=>e.href).filter(Boolean)};
}"""
def collect():
 main=sitemap('https://capy.ai/sitemap.xml')
 docs=[u for u in sitemap('https://docs.capy.ai/sitemap.xml') if '/og/' not in u and not u.endswith('.png')]
 d={'date':datetime.now(timezone.utc).isoformat(),'main_sitemap_count':len(main),'docs_pages':docs,'pages':[],'references':[],'discovered':[]}
 queue=list(main);known=set(queue);found=set()
 with sync_playwright() as p:
  b=p.chromium.launch(headless=True,executable_path='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',args=['--no-sandbox'])
  ctx=b.new_context(viewport={'width':1440,'height':900},device_scale_factor=1,reduced_motion='reduce')
  pg=ctx.new_page();pg.set_default_timeout(8000)
  for i,u in enumerate(queue):
   if not public(u):continue
   item={'url':u}
   try:
    response=pg.goto(u,wait_until='domcontentloaded',timeout=17000)
    pg.wait_for_timeout(100)
    item.update({'status':response.status if response else None,'title':pg.title(),'final_url':pg.url})
    if item['status'] and item['status']>=400:raise RuntimeError('HTTP '+str(item['status']))
    item.update(pg.evaluate(SNAP))
    n=name(u)
    path=SCREEN/('d-'+n+'.jpg')
    pg.screenshot(path=str(path),type='jpeg',quality=70,animations='disabled',timeout=9000)
    item['desktop']=str(path.relative_to(ROOT))
    pg.set_viewport_size({'width':390,'height':844}); pg.wait_for_timeout(100)
    path=SCREEN/('m-'+n+'.jpg')
    pg.screenshot(path=str(path),type='jpeg',quality=67,animations='disabled',timeout=9000)
    item['mobile']=str(path.relative_to(ROOT))
    pg.set_viewport_size({'width':1440,'height':900})
    if i<32 or any(x in u for x in ['/features/','/customers/','/integrations/']):
     pg.evaluate('window.scrollTo(0,document.documentElement.scrollHeight*0.54)')
     pg.wait_for_timeout(80)
     path=SCREEN/('scroll-'+n+'.jpg')
     pg.screenshot(path=str(path),type='jpeg',quality=64,animations='disabled',timeout=8000)
     item['scroll']=str(path.relative_to(ROOT))
    for link in item.get('links',[]):
     parts=urllib.parse.urlparse(link)
     clean=urllib.parse.urlunparse(parts._replace(query='',fragment='')).rstrip('/')
     if public(clean) and clean not in known and not re.search(r'\.(png|jpg|svg|webp|gif|pdf|zip|dmg)$',clean,re.I):
      found.add(clean);known.add(clean)
   except Exception as e:item['error']=str(e)[:280]
   d['pages'].append(item)
   if (i+1)%10==0:print('SITE PROGRESS',i+1,'of',len(queue),'ok',sum(not x.get('error') for x in d['pages']),flush=True)
  d['discovered']=sorted(found)
  for key,url in [('contra','https://contra.com/p/g37kMveR-capyai-branding-illustrations-project?utm_source=chatgpt.com&r=mamdouh_aboammar_oi71x6ol'),('launch','https://www.pattytanch.com/work/capy-ai')]:
   item={'key':key,'url':url,'screenshots':[]}
   try:
    response=pg.goto(url,wait_until='domcontentloaded',timeout=19000)
    pg.wait_for_timeout(450)
    item.update({'status':response.status if response else None,'title':pg.title(),'final_url':pg.url})
    item.update(pg.evaluate(SNAP))
    for fraction in [0,.25,.50,.75,.95]:
     pg.evaluate('(f)=>scrollTo(0,document.documentElement.scrollHeight*f)',fraction);pg.wait_for_timeout(130)
     path=REF/(key+'-'+str(int(fraction*100))+'.jpg')
     pg.screenshot(path=str(path),type='jpeg',quality=70,animations='disabled',timeout=9000)
     item['screenshots'].append(str(path.relative_to(ROOT)))
    pg.set_viewport_size({'width':390,'height':844});pg.wait_for_timeout(120)
    path=REF/(key+'-mobile.jpg');pg.screenshot(path=str(path),type='jpeg',quality=66,animations='disabled')
    item['mobile']=str(path.relative_to(ROOT));pg.set_viewport_size({'width':1440,'height':900})
   except Exception as e:item['error']=str(e)[:280]
   d['references'].append(item)
  try:
   pg.goto('https://capy.ai/',wait_until='domcontentloaded',timeout=15000)
   nav=pg.locator('header a,header button,nav a,nav button')
   d['hover']=[]
   for i in range(min(nav.count(),20)):
    el=nav.nth(i)
    try:
     label=el.inner_text(timeout=700).strip()[:55]
     if not label:continue
     el.hover(timeout=700);pg.wait_for_timeout(70)
     d['hover'].append({'label':label,'menus':pg.locator('[role="menu"],[data-state="open"]').all_inner_texts()[:3]})
     if len(d['hover'])<=5:pg.screenshot(path=str(SCREEN/('hover-'+str(len(d['hover']))+'.jpg')),type='jpeg',quality=66)
    except Exception:pass
  except Exception as e:d['hover_error']=str(e)[:180]
  b.close()
 (RES/'capy_inventory.json').write_text(json.dumps(d,indent=2,ensure_ascii=False),encoding='utf-8')
 lines=['# Capy Website — Public Visual Audit','',
 'Evidence: Chrome screenshots captured on a real macOS browser at desktop 1440x900 and mobile 390x844. Source art is for internal research only.','',
 '## Inventory and coverage', '',
 'Main public sitemap URLs: '+str(len(main)),
 'Attempted public sitemap pages: '+str(len(d['pages'])),
 'Browser pages with screenshots: '+str(sum(bool(x.get('desktop')) for x in d['pages'])),
 'Individual page errors: '+str(sum(bool(x.get('error')) for x in d['pages'])),
 'Docs HTML pages discovered: '+str(len(docs))+'; enumerated in capy_inventory.json, not audited one by one.',
 'Additional linked public paths not listed in sitemap: '+str(len(found)),
 'Excluded disallowed/private routes per robots.txt.', '',
 '## Creative observations', '',
 'Capy couples warm comic-driven mascot illustration with comparatively sober UI, using consistent line quality and recurring contextual props. Treat this as a high-level principle, not a compositional template.',
 'Transfer: character pauses at boundaries and helps explain states. Do not copy the capybara silhouette, eyewear, branded scenes, color blocking, or proprietary layouts.',
 'Motion observed below is limited to DOM-computed transition/animation candidates and screenshot states; not all scroll-triggered motion has been individually verified.',
 'Accessibility check: maintain meaningful status labels independent of illustrations, visible focus, reduced-motion behavior and legible small sizes.','',
 '## Page-by-page visual evidence']
 for x in d['pages']:
  lines.extend(['','### '+x['url'],
   'HTTP: '+str(x.get('status'))+'; final '+x.get('final_url','unknown'),
   'Title: '+x.get('title','unknown'),
   'Hierarchy: '+x.get('h1','').replace('\n',' ')+' | sections: '+', '.join(x.get('h2',[])[:4]).replace('\n',' '),
   'Typography: '+str(x.get('typography',{}))+'; palette: '+str(x.get('colors',{})),
   'Character/illustration image metadata: '+str(x.get('images',[])[:4]),
   'Motion candidate nodes: '+str(x.get('motion',[])[:4]),
   'Desktop evidence: '+str(x.get('desktop'))+'; mobile evidence: '+str(x.get('mobile'))+'; scrolled: '+str(x.get('scroll')),
   'Issue: '+str(x.get('error','none'))])
 lines+=['','## Navigation hover','',json.dumps(d.get('hover',[]),indent=2,ensure_ascii=False),
 '','## External creative references']
 for r in d['references']:lines+=['',r['key']+': '+r['url'],'Status '+str(r.get('status'))+'; screenshots '+str(r.get('screenshots'))+'; blocker '+str(r.get('error','none'))]
 lines+=['','## Coverage limits','Every sitemap marketing URL was attempted, but discovery can reveal additional paths; those are explicitly listed in capy_inventory.json. Docs are inventory-only. Screenshots are viewport samples rather than proof of every hover, scroll frame, or every UI state. No authentication barriers were bypassed.']
 (RES/'CAPY_SITE_AUDIT.md').write_text('\n'.join(lines),encoding='utf-8')
 print('AUDIT COMPLETE',len(d['pages']),sum(bool(x.get('desktop')) for x in d['pages']),'new links',len(found),flush=True)
if __name__=='__main__':collect()
