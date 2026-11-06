#!/usr/bin/env python3
"""Build a self-contained, responsive local gallery and reference board."""
from pathlib import Path
from html import escape
import json
from build_identity import R,C,svg,rovi,head,rect,text,label,lin,circle,path
def save(rel,content):
 f=R/rel;f.parent.mkdir(parents=True,exist_ok=True);f.write_text(content,encoding='utf-8')
def motion():
 s='<style><![CDATA['
 s+='@keyframes rovi-peek{0%,18%,100%{transform:rotate(0deg)}35%,46%{transform:rotate(-4deg)}62%{transform:rotate(3deg)}}'
 s+='@keyframes note-enter{0%,12%{transform:translateX(24px);opacity:.25}30%,79%{transform:translateX(0);opacity:1}100%{transform:translateX(10px);opacity:.5}}'
 s+='@keyframes pulse{0%,100%{opacity:1}50%{opacity:.4}}'
 s+='#rovi-move{transform-origin:center;transform-box:fill-box;animation:rovi-peek 3.8s ease-in-out infinite}#note-slide{animation:note-enter 3.8s ease-in-out infinite}#cursor{animation:pulse 1.8s ease-in-out infinite}'
 s+='@media (prefers-reduced-motion: reduce){#rovi-move,#note-slide,#cursor{animation:none!important}}'
 s+=']]></style>'
 s+=rect(0,0,680,520,C['paper'])+rect(20,18,640,484,C['white'],22,C['ink'],4)
 s+=rect(20,18,640,57,C['ink'],20)
 s+=text(44,53,'OPENREVIEW / REVIEWING',22,700,C['white'])
 for i in range(8):
  y=120+i*43
  if i==3:s+=rect(48,y-6,445,31,C['peach'],4)
  s+=text(49,y+14,str(i+20),16,600,C['sage'])
  s+=rect(94,y+2,242-i*13,8,C['clay'] if i==3 else C['shade'],5)
 s+='<g id="note-slide">'+rect(278,360,220,86,C['white'],9,C['ink'],3)+label(295,385,'POTENTIAL ISSUE',C['clay'])+text(295,413,'Gathering evidence…',14,500,C['ink'])+'</g>'
 s+='<g transform="translate(400 108) scale(.54)"><g id="rovi-move">'+rovi('inspecting','curious')+'</g></g>'
 s+='<g id="cursor">'+circle(293,261,8,C['amber'],C['ink'],2)+'</g>'
 return svg(s,680,520,'Rovi review microinteraction, motion-aware')
def construction():
 o=rect(0,0,1400,885,C['paper'])+label(52,66,'ROVI / CONSTRUCTION BIBLE')+text(52,126,'One silhouette. Many decisions.',52,800)
 o+=rect(47,168,510,641,C['white'],18,C['shade'],2)
 o+='<g transform="translate(96 201) scale(1.1)">'+rovi('standing','neutral')+'</g>'
 o+=lin(102,234,485,234,C['sage'],2,'7 9')+lin(102,549,485,549,C['sage'],2,'7 9')+lin(102,720,485,720,C['sage'],2,'7 9')
 o+=text(102,788,'MASTER: FRONT / ALL POSES',19,700)
 o+=rect(586,168,763,641,C['white'],18,C['shade'],2)
 o+=text(620,224,'GEOMETRY',23,800)+text(620,263,'Head: retained from original logo path',21,500)
 o+=text(620,298,'Body: compact upright bean shape',21,500)
 o+=text(620,333,'Eye: integrated 28u lens, 9.5u outline',21,500)
 o+=text(620,368,'Feet: rounded ink ovals; hands: pale mittens',21,500)
 o+=text(620,403,'No mechanical joints. No generic robot details.',21,500)
 o+=lin(620,442,1311,442,C['shade'],3)
 o+='<g transform="translate(645 479) scale(1.1)">'+head('curious')+'</g>'
 o+=text(970,535,'ASYMMETRIC',22,800)+text(970,572,'Left flutter / right page-fold.',19,500)
 o+=text(970,612,'Preserve lens + check at all times.',19,500)
 o+=text(970,654,'Small sizes use the isolated icon.',19,500)
 o+=text(970,694,'Keep silhouette and expression readable.',19,500)
 return svg(o,1400,885,'Rovi anatomy and proportions — construction guide')
def card_for(path,kind,label_,desc,large=False):
 file=R/path
 assert file.exists(),str(file)
 href='../'+str(path).replace('\\','/')
 return '<article class="asset '+('feature' if large else '')+'" data-kind="'+kind+'"><div class="thumb"><img src="'+escape(href,quote=True)+'" loading="lazy" alt="'+escape(label_,quote=True)+'"/></div><div class="asset-info"><div><span class="asset-kind">'+kind.upper()+'</span><h3>'+escape(label_)+'</h3><p>'+escape(desc)+'</p></div><a href="'+escape(href,quote=True)+'" download aria-label="Download SVG: '+escape(label_,quote=True)+'">SVG ↗</a></div></article>'
def build():
 save('motion/rovi-motion.svg',motion())
 save('character/construction-sheet.svg',construction())
 catalog=[]
 def add(p,kind,label_,desc,large=False):catalog.append(card_for(p,kind,label_,desc,large))
 add('character/rovi-full-body-master.svg','character','Full-body master','The original Rovi silhouette, with a consistent small-body rig.',True)
 add('character/turnaround.svg','character','Turnaround sheet','Front, three-quarter, side and rear.')
 add('character/construction-sheet.svg','character','Construction','Proportions, eye construction, and continuity rules.',True)
 add('character/rovi-small-mark.svg','character','Small-size icon','Experimental compact mark for icon sizes.')
 add('character/rovi-horizontal-logo.svg','character','Wordmark lockup','Rovi + editable live type.')
 add('character/rovi-dark-variant.svg','character','Dark surface','Black background, white body.')
 add('character/rovi-light-variant.svg','character','Light surface','Warm paper background, ink strokes.')
 add('character/buglets-exploration.svg','character','Buglet family','Three small creatures, reserved for actual issue metaphors.')
 add('character/expression-sheet.svg','expressions','Expression sheet','All ten expressions share a single head outline.',True)
 for name in ['neutral','curious','suspicious','concentrating','confused','concerned','relieved','happy','proud','sleepy']:
  add('character/expressions/'+name+'.svg','expressions',name.capitalize(),'Editable facial controls and lens eye.')
 add('character/pose-sheet.svg','poses','Pose sheet','Twelve poses built from common geometry.',True)
 for name in ['standing','sitting','walking','inspecting','pointing-code','reading-pr','writing-note','investigating-bug','applying-patch','waiting-tests','celebrating','resting']:
  add('character/poses/'+name+'.svg','poses',name.replace('-',' ').title(),'Anatomy-preserving action illustration.')
 for name,t,d in [('hero','Hero — inspect PR','A closer look at a diff.'),
                  ('review','Review — suspicious line','Question before conclusion.'),
                  ('investigation','Investigation — evidence trail','Signal, minimal reproducer, and not-run test state.'),
                  ('fix-proposed','Fix — proposed patch','No publishing claim without authorization.'),
                  ('verify-pass','Verification — pass','Positive state only after actual evidence.'),
                  ('verify-fail','Verification — fail','Failures remain explicit.'),
                  ('empty-state','Empty review state','Quiet workspace and resting Rovi.'),
                  ('final-cta','Final call to action','Rovi directs the eye toward the next step.')]:
  add('scenes/'+name+'.svg','scenes',t,d,name in ('hero','review','investigation'))
 add('motion/rovi-motion.svg','motion','Attention and note movement','Subtle looping gesture; reduced-motion supported.',True)
 body=''.join(catalog)
 html='''<!doctype html><html lang="en"><head><meta charset="utf-8"/>
 <meta name="viewport" content="width=device-width,initial-scale=1"/>
 <meta name="color-scheme" content="light"/><title>Rovi — The Margins | OpenReview</title>
 <style>
 :root{--ink:#171717;--paper:#FAF7F0;--white:#FFFEFA;--sage:#639781;--clay:#CE7058;--amber:#E8BE70;--gray:#69645f}
 *{box-sizing:border-box}html{scroll-behavior:smooth}body{margin:0;background:var(--paper);color:var(--ink);font-family:system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;line-height:1.55}
 a{color:inherit}a:focus-visible,button:focus-visible{outline:4px solid var(--sage);outline-offset:4px}
 .wrap{width:min(1380px,calc(100% - 52px));margin:auto}.topbar{border-bottom:2px solid var(--ink);background:var(--paper);position:sticky;top:0;z-index:4}
 .topbar-inner{display:flex;justify-content:space-between;align-items:center;padding:14px 0;gap:16px}
 .wordmark{display:flex;align-items:center;gap:10px;font-weight:850;font-size:22px;text-decoration:none;letter-spacing:-.7px}
 .wordmark img{height:35px;width:35px;border-radius:9px}
 .top-meta{font:600 12px ui-monospace,SFMono-Regular,monospace;letter-spacing:1px;color:var(--gray)}
 .hero{display:grid;grid-template-columns:minmax(0,1.15fr) minmax(280px,.85fr);gap:45px;padding:76px 0 62px;align-items:center}
 .eyebrow{font:750 12px ui-monospace,monospace;letter-spacing:2px;text-transform:uppercase}
 h1{font-size:clamp(54px,6.8vw,118px);line-height:.96;letter-spacing:-.08em;margin:22px 0 26px;max-width:900px}
 h1 em{font-style:normal;color:#944B37}.lead{font-size:clamp(17px,1.5vw,22px);max-width:550px;color:#4b4844}
 .tiny-meta{display:flex;gap:12px;flex-wrap:wrap;margin-top:25px}
 .tiny-meta span{font:700 12px ui-monospace,monospace;border:1.5px solid var(--ink);padding:8px 12px;border-radius:22px}
 .hero-art{min-width:0;border:3px solid var(--ink);border-radius:22px;background:var(--white);position:relative;box-shadow:11px 11px 0 var(--ink);min-height:430px;display:flex;align-items:center;justify-content:center}
 .hero-art img{display:block;max-height:405px;width:100%;object-fit:contain}
 .hero-art .corner{position:absolute;bottom:18px;left:19px;font:700 11px ui-monospace,monospace;background:var(--amber);padding:7px 10px;border:1.5px solid var(--ink);border-radius:20px}
 .intro{border-block:2px solid var(--ink);display:flex;justify-content:space-between;gap:35px;padding:23px 0;align-items:center}
 .intro p{margin:0;max-width:780px;font-size:16px;color:#494740}
 .intro strong{white-space:normal;text-transform:uppercase;font:800 14px ui-monospace,monospace;overflow-wrap:anywhere}
 .gallery-top{display:flex;justify-content:space-between;align-items:end;gap:20px;padding-top:78px}
 h2{font-size:clamp(37px,4.2vw,70px);line-height:1;letter-spacing:-.065em;margin:0 0 8px}
 .gallery-top p{margin:0;color:var(--gray)}
 .filters{display:flex;flex-wrap:wrap;gap:9px;margin:28px 0 25px}
 button{border:2px solid var(--ink);background:transparent;color:var(--ink);padding:10px 18px;border-radius:24px;cursor:pointer;font:700 14px system-ui,sans-serif;transition:background .16s,color .16s}
 button:hover{background:#EEE8DD}button[aria-pressed=true]{background:var(--ink);color:var(--white)}
 .assets{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:18px;padding-bottom:70px}
 .asset{min-width:0;border:2px solid var(--ink);border-radius:17px;background:var(--white);overflow:hidden;display:flex;flex-direction:column}
 .asset[hidden]{display:none!important}.asset.feature{grid-column:span 2}
 .thumb{height:295px;display:flex;align-items:center;justify-content:center;overflow:hidden;background:#F3EFE6}
 .thumb img{width:100%;height:100%;object-fit:contain;padding:16px}.asset.feature .thumb{height:380px}
 .asset-info{display:flex;gap:12px;align-items:center;justify-content:space-between;padding:17px 20px 20px;border-top:2px solid var(--ink)}
 .asset-kind{font:750 10px ui-monospace,monospace;letter-spacing:1.4px;color:#944B37}
 h3{font-size:19px;line-height:1.14;margin:5px 0 7px;letter-spacing:-.02em}
 .asset-info p{font-size:13px;line-height:1.4;color:var(--gray);margin:0;max-width:340px}
 .asset-info a{flex-shrink:0;font:700 12px ui-monospace,monospace;padding:9px 12px;text-decoration:none;border:1.5px solid var(--ink);border-radius:25px;white-space:nowrap}
 .asset-info a:hover{background:var(--amber)}
 .endcap{background:var(--ink);color:var(--white);padding:66px 0}
 .endcap .wrap{display:grid;grid-template-columns:1.5fr 1fr;gap:25px}
 .endcap h2{max-width:780px}.endcap p{font-size:16px;color:#c7cac5}
 .endcap a{color:var(--white)}
 .footer{padding:22px 0;display:flex;justify-content:space-between;gap:15px;font:700 12px ui-monospace,monospace}
 @media(max-width:850px){.hero{grid-template-columns:1fr;padding:50px 0}.hero-art{min-height:320px}.hero-art img{max-height:320px}.assets{grid-template-columns:repeat(2,minmax(0,1fr))}.intro{align-items:start;flex-direction:column}.endcap .wrap{grid-template-columns:1fr}}
 @media(max-width:540px){.wrap{width:min(100% - 30px,1380px)}.top-meta{display:none}.hero{gap:28px}.hero-art{min-height:280px;box-shadow:6px 6px 0 var(--ink)}h1{font-size:clamp(46px,13.5vw,72px)}.assets{grid-template-columns:1fr}.asset.feature{grid-column:span 1}.asset.feature .thumb{height:270px}.gallery-top{padding-top:54px}.thumb{height:260px}.footer{flex-direction:column}}
 @media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}*,*::before,*::after{transition:none!important;animation:none!important}}
 </style></head><body>
 <header class="topbar"><div class="wrap topbar-inner">
 <a class="wordmark" href="#top" aria-label="Back to Rovi gallery top"><img src="../character/rovi-small-mark.svg" alt=""/> OpenReview<span style="font-weight:400"> / Rovi</span></a>
 <span class="top-meta">THE MARGINS · ORIGINAL CHARACTER UNIVERSE · V1.0</span></div></header>
 <main id="top"><div class="wrap">
 <section class="hero" aria-labelledby="hero-title"><div>
 <div class="eyebrow">OPENREVIEW / CHARACTER WORLD / 2026</div>
 <h1 id="hero-title">Every change <em>deserves a closer look.</em></h1>
 <p class="lead">Meet Rovi: curious about the diff, careful with the evidence, and quietly delighted when the tests actually prove something.</p>
 <div class="tiny-meta"><span>10 EXPRESSIONS</span><span>12 POSES</span><span>ORIGINAL VECTOR ART</span></div>
 </div><div class="hero-art"><img src="../character/rovi-full-body-master.svg" alt="Original white Rovi mascot, full body, with lens eye and folded-page head"/>
 <span class="corner">ROVI / FRONT MASTER</span></div></section>
 <section class="intro"><strong>One character. One construction system.</strong><p>This local workspace keeps every drawing editable and every claim honest. Buglets represent possible problems—not verified vulnerabilities. Source icon files have not been overwritten.</p></section>
 <div class="gallery-top"><div><div class="eyebrow">ASSET LIBRARY / CURRENT BUILD</div><h2>Meet the whole cast.</h2><p>Inspect, compare, and download the editable masters.</p></div><div class="eyebrow" id="visible-count" aria-live="polite">ALL ASSETS</div></div>
 <nav class="filters" aria-label="Asset categories">
 <button type="button" data-filter="all" aria-pressed="true">All assets</button>
 <button type="button" data-filter="character" aria-pressed="false">Characters</button>
 <button type="button" data-filter="expressions" aria-pressed="false">Expressions</button>
 <button type="button" data-filter="poses" aria-pressed="false">Poses</button>
 <button type="button" data-filter="scenes" aria-pressed="false">Scenes</button>
 <button type="button" data-filter="motion" aria-pressed="false">Motion</button>
 </nav>
 <div class="assets" id="assets">'''+body+'''</div>
 </div><section class="endcap"><div class="wrap"><div><div class="eyebrow">THE MARGINS / GROW WITH THE PRODUCT</div><h2>Evidence makes the character trustworthy.</h2></div>
 <div><p>All claims in the illustration captions describe intended visual states, not evidence of real program execution. Keep review, investigation, authorized Fix, and verification distinct.</p>
 <p>Open the <a href="../IDENTITY_GUIDELINES.md">identity guidelines</a> or the <a href="../PLAYBOOK.md">production playbook</a>.</p></div></div></section>
 <div class="wrap footer"><span>OPENREVIEW · ROVI V1 / LOCAL ART DIRECTION</span><span>NO CDN · NO EXTERNAL FONTS · NO THIRD-PARTY ARTWORK</span></div></main>
 <script>
 const buttons=[...document.querySelectorAll('[data-filter]')];
 const cards=[...document.querySelectorAll('.asset')];
 function select(kind){
  buttons.forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.filter===kind)));
  let n=0;
  cards.forEach(card=>{card.hidden=kind!=='all'&&card.dataset.kind!==kind;if(!card.hidden)n++;});
  document.getElementById('visible-count').textContent=n+' ASSETS / '+kind.toUpperCase();
 }
 buttons.forEach(b=>b.addEventListener('click',()=>select(b.dataset.filter)));
 select('all');
 </script></body></html>'''
 save('preview/index.html',html)
 # Internal-only photographic research screenshots, never distributed as production art.
 data=json.loads((R/'research/capy_inventory.json').read_text())
 samples=['https://capy.ai','https://capy.ai/pricing','https://capy.ai/security','https://capy.ai/about',
 'https://capy.ai/features/automations','https://capy.ai/integrations/github',
 'https://capy.ai/customers','https://capy.ai/changelog','https://capy.ai/blog']
 board=[]
 for url in samples:
  entry=next((x for x in data['pages'] if x['url']==url),None)
  if entry and entry.get('desktop'):
   board.append('<figure><img src="./'+escape(entry['desktop'].replace('research/',''),quote=True)+'" alt="Capy captured browser page"/><figcaption>'+escape(url)+'</figcaption></figure>')
 for x in data.get('references',[]):
  if x.get('screenshots'):
   board.append('<figure><img src="./'+escape(x['screenshots'][1].replace('research/',''),quote=True)+'" alt="Reference screen"/><figcaption>'+escape(x['url'])+'</figcaption></figure>')
 boardhtml='<!doctype html><meta charset="utf-8"><title>Reference audit board / Internal only</title><style>body{background:#faf7f0;color:#171717;font:16px system-ui;margin:40px auto;max-width:1440px;padding:0 24px}h1{font-size:50px;letter-spacing:-2px}p{max-width:780px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(270px,1fr));gap:20px}figure{margin:0;border:2px solid #171717;border-radius:14px;overflow:hidden;background:white}img{width:100%;height:230px;object-fit:cover;object-position:top}figcaption{padding:12px;font:12px monospace;overflow-wrap:anywhere}</style><h1>Reference study / not production assets</h1><p>Recorded browser screenshots. Internal art-direction comparisons only; Capy illustrations and distinctive compositions must not be reused in OpenReview production.</p><main>'+''.join(board)+'</main>'
 save('research/REFERENCE_BOARD.html',boardhtml)
 print('GALLERY BUILT',len(catalog),'assets',len(html),'HTML bytes')
if __name__=='__main__':build()
