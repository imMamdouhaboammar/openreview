#!/usr/bin/env python3
"""Original, editable Rovi SVG universe. Source files preserved unchanged."""
from pathlib import Path
from html import escape
import json, math
R=Path(__file__).resolve().parents[1]
for d in ['character','character/expressions','character/poses','scenes','exports','motion','preview']: (R/d).mkdir(parents=True,exist_ok=True)
C={'paper':'#FAF7F0','ink':'#171717','white':'#FFFEFA','clay':'#CE7058','amber':'#E8BE70','sage':'#639781','mist':'#E4EBE5','peach':'#F3DDCE','shade':'#E9E2D7','deep':'#263C36','red':'#C76D5D','clayText':'#944B37','sageText':'#365E50'}
HEAD='M47 100C34 83 35 66 42 55c2-3 7 0 15 5-8-11-7-23 2-33 6-7 9 2 14 10 8 14 16 21 28 27 19-11 38-10 60-5 13-10 37-25 49-27 7-1 11 4 15 13l19 38c5 10-1 17-11 27 6 27 3 55-10 78-18 32-50 49-93 51-43 1-77-18-95-49-19-33-11-70 12-90Z'
def svg(content,w,h,title='Rovi — OpenReview',desc='Original vector illustration'):
 return '<svg xmlns="http://www.w3.org/2000/svg" width="'+str(w)+'" height="'+str(h)+'" viewBox="0 0 '+str(w)+' '+str(h)+'" role="img" aria-label="'+escape(title)+'"><title>'+escape(title)+'</title><desc>'+escape(desc)+'</desc>'+content+'</svg>'
def path(d,fill='none',stroke=None,sw=4,extra=''):
 return '<path d="'+d+'" fill="'+fill+'"'+(f' stroke="{stroke}" stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round"' if stroke else '')+' '+extra+'/>'
def text(x,y,msg,size=20,weight=600,color=None,spacing=None):
 effective=C['clayText'] if color==C['clay'] else (C['sageText'] if color==C['sage'] else (color or C['ink']))
 return '<text x="'+str(x)+'" y="'+str(y)+'" font-family="Arial,Helvetica,sans-serif" font-weight="'+str(weight)+'" font-size="'+str(size)+'" fill="'+effective+'"'+(f' letter-spacing="{spacing}"' if spacing else '')+'>'+escape(msg)+'</text>'
def rect(x,y,w,h,fill,rx=0,stroke=None,sw=2):
 return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}"'+(f' stroke="{stroke}" stroke-width="{sw}"' if stroke else '')+'/>'
def circle(x,y,r,fill,stroke=None,sw=2):
 return f'<circle cx="{x}" cy="{y}" r="{r}" fill="{fill}"'+(f' stroke="{stroke}" stroke-width="{sw}"' if stroke else '')+'/>'
def lin(x1,y1,x2,y2,color=None,w=3,dash=None):
 return f'<path d="M{x1} {y1}L{x2} {y2}" stroke="{color or C["ink"]}" stroke-width="{w}" stroke-linecap="round" fill="none"'+(f' stroke-dasharray="{dash}"' if dash else '')+'/>'
def facial(expression='neutral'):
 e=expression
 brows={'neutral':'M137 99c14-8 28-12 43-10','curious':'M137 99q24-23 44-14',
 'suspicious':'M137 110q25-20 42-18','concentrating':'M137 106q20 1 42-2',
 'confused':'M134 94q11 13 27 0','concerned':'M137 105q13-24 43-13',
 'relieved':'M136 102q24-8 44-7','happy':'M137 98q22-17 42-7',
 'proud':'M137 101q20-6 42-11','sleepy':'M137 114q22-3 42-4'}
 out='<g fill="none" stroke="'+C['ink']+'" stroke-linecap="round" stroke-linejoin="round">'
 out+=path(brows.get(e,brows['neutral']),stroke=C['ink'],sw=7)
 if e in ('relieved','happy','sleepy'):
  out+=path('M71 136q12 '+('12' if e=='happy' else '3')+' 23 0',stroke=C['ink'],sw=7)
 else:
  out+='<ellipse cx="82.4" cy="136.3" rx="'+('8' if e=='suspicious' else '9.1')+'" ry="'+('7.5' if e=='suspicious' else '12.5')+'" transform="rotate(15 82.4 136.3)" fill="'+C['ink']+'"/>'
 # Magnifier lens is an integral eye feature; no duplicate handheld lens.
 out+='<circle cx="164" cy="142" r="28" stroke="'+C['ink']+'" stroke-width="9.5"/>'
 out+=path('m186 164 17 17',stroke=C['ink'],sw=10)
 if e=='sleepy':out+=path('M145 143q18 7 36 0',stroke=C['ink'],sw=6)
 elif e=='happy':out+=path('M149 144q14 14 27-1',stroke=C['ink'],sw=6)
 elif e=='confused':out+=path('M150 132q10-9 22 1',stroke=C['ink'],sw=5)+circle(166,148,7,C['ink'])
 else:
  out+=path('M149 140c6-6 16-9 25-4',stroke=C['ink'],sw=6)
  out+='<ellipse cx="'+('168' if e=='curious' else '160')+'" cy="145" rx="6.6" ry="9" transform="rotate(18 160 145)" fill="'+C['ink']+'"/>'
 out+=path('m101 149 10 12 14-15',stroke=C['ink'],sw=9)
 mouths={'neutral':'M137 199c11 1 21-4 28-13','curious':'M141 193q7-12 18-4','suspicious':'M139 195q16 4 24-3',
 'concentrating':'M140 192l19-2','confused':'M139 194q5-9 12 0t12 0','concerned':'M139 200q13-19 25 0',
 'relieved':'M137 190q14 17 30-2','happy':'M136 187q16 24 34-2','proud':'M140 187q13 18 27-2','sleepy':'M145 198q8 5 15 0'}
 out+=path(mouths.get(e,mouths['neutral']),stroke=C['ink'],sw=7)
 return out+'</g>'
def head(expression='neutral',view='front'):
 if view=='rear':
  return '<g>'+path(HEAD,C['white'],C['ink'],6)+path('M203 48q-3 22 30 43',C['mist'],C['ink'],5)+path('M54 94q-10 58 17 86',stroke=C['shade'],sw=8)+'</g>'
 if view=='side':
  return '<g transform="translate(35 5) scale(.78 1)">'+path(HEAD,C['white'],C['ink'],6)+path('M200 97q-10-10-21-1',stroke=C['ink'],sw=7)+circle(174,144,26,'none',C['ink'],9)+path('m192 163 17 17',stroke=C['ink'],sw=9)+circle(177,148,7,C['ink'])+path('m116 154 10 8 10-13',stroke=C['ink'],sw=8)+'</g>'
 if view=='three-quarter':
  return '<g transform="translate(14 2) scale(.92 1)">'+path(HEAD,C['white'],C['ink'],6)+facial(expression)+'</g>'
 return '<g>'+path(HEAD,C['white'],C['ink'],6)+path('M207 47c-1 13-4 24-2 30 2 7 14 9 27 14 4 1 1-6-2-11l-14-31c-3-5-7-6-9-2Z',C['ink'])+facial(expression)+'</g>'
POSES=['standing','sitting','walking','inspecting','pointing-code','reading-pr','writing-note','investigating-bug','applying-patch','waiting-tests','celebrating','resting']
EXPRS=['neutral','curious','suspicious','concentrating','confused','concerned','relieved','happy','proud','sleepy']
def limb(points):
 return path(points,stroke=C['ink'],sw=18)+path(points,stroke=C['white'],sw=10)
def rovi(pose='standing',expression='neutral',view='front'):
 e=expression
 if pose=='celebrating':e='happy'
 if pose=='waiting-tests':e='concerned'
 if pose=='resting':e='sleepy'
 if pose=='investigating-bug':e='suspicious'
 # Space 320 wide x 470 high. All views share torso and underlying rig.
 body=path('M123 242Q164 216 221 245Q244 289 232 362Q201 387 139 368Q112 336 123 242Z',C['white'],C['ink'],6)
 body+=path('M157 274q29-8 55 0',stroke=C['shade'],sw=5)
 sitting=pose in ('sitting','resting')
 if sitting:
  legs=limb('M146 359q-26 30-42 35')+limb('M207 363q28 27 47 31')
  feet='<ellipse cx="98" cy="397" rx="22" ry="10" fill="'+C['ink']+'"/><ellipse cx="257" cy="399" rx="22" ry="10" fill="'+C['ink']+'"/>'
 elif pose=='walking':
  legs=limb('M153 353q-22 21-43 55')+limb('M209 353q16 29 50 40')
  feet='<ellipse cx="104" cy="411" rx="26" ry="10" fill="'+C['ink']+'"/><ellipse cx="260" cy="395" rx="22" ry="10" fill="'+C['ink']+'"/>'
 elif pose=='celebrating':
  legs=limb('M158 357q-18 30-31 48')+limb('M207 356q14 26 29 48')
  feet='<ellipse cx="124" cy="412" rx="24" ry="9" fill="'+C['ink']+'"/><ellipse cx="239" cy="411" rx="23" ry="9" fill="'+C['ink']+'"/>'
 else:
  legs=limb('M150 354q-7 29-15 50')+limb('M207 355q8 27 13 50')
  feet='<ellipse cx="127" cy="412" rx="25" ry="10" fill="'+C['ink']+'"/><ellipse cx="226" cy="412" rx="25" ry="10" fill="'+C['ink']+'"/>'
 armL='M127 279Q107 303 106 342'
 armR='M227 281Q252 313 250 344'
 if pose=='inspecting':armR='M228 278Q260 262 282 233'
 if pose=='pointing-code':armR='M230 282Q272 277 306 252'
 if pose=='reading-pr':armL='M126 287Q144 318 161 322';armR='M227 284Q212 312 207 319'
 if pose=='writing-note':armL='M126 286Q141 307 162 319';armR='M225 281Q234 295 218 320'
 if pose=='investigating-bug':armL='M123 282Q95 317 80 342';armR='M229 284Q260 310 291 317'
 if pose=='applying-patch':armL='M127 284Q158 312 170 322';armR='M230 281Q266 289 286 281'
 if pose=='waiting-tests':armL='M127 282Q155 333 172 338';armR='M230 281Q210 328 185 338'
 if pose=='celebrating':armL='M127 285Q106 231 85 209';armR='M229 284Q262 223 288 211'
 if pose=='resting':armL='M124 281Q100 302 92 332';armR='M229 283Q251 301 254 328'
 arms=limb(armL)+limb(armR)
 hands=''
 def last_point(d):
  # parse known endpoints using final integer pair
  arr=list(map(int,__import__('re').findall(r'-?\d+',d)));return arr[-2],arr[-1]
 for v in [armL,armR]:
  px,py=last_point(v);hands+=circle(px,py,12,C['white'],C['ink'],5)
 props=''
 if pose=='reading-pr':
  props+=rect(150,291,67,68,C['paper'],6,C['ink'],4)+rect(159,300,48,7,C['sage'],3)
  props+=lin(159,318,201,318,C['ink'],3)+lin(159,330,194,330,C['shade'],4)+lin(159,342,205,342,C['shade'],4)
 if pose=='writing-note':
  props+=rect(146,303,76,49,C['paper'],6,C['ink'],3)+lin(156,324,198,324,C['shade'],3)+lin(156,335,190,335,C['shade'],3)
  props+=path('M207 306l17-25',stroke=C['clay'],sw=6)
 if pose=='investigating-bug': props+=circle(291,326,14,C['clay'],C['ink'],3)+circle(287,324,2,C['ink'])+circle(295,324,2,C['ink'])
 if pose=='applying-patch':props+=rect(273,265,41,33,C['sage'],4,C['ink'],3)+text(280,287,'+',22,800,C['white'])
 if pose=='waiting-tests':props+=rect(144,314,71,38,C['amber'],9,C['ink'],3)+text(158,339,'...',18,700)
 if pose=='sitting':props+=rect(110,368,149,15,C['amber'],6,C['ink'],3)
 if pose=='resting':props+=rect(111,368,150,15,C['amber'],6,C['ink'],3)+path('M256 210q26-10 32 8',stroke=C['shade'],sw=5)+text(268,195,'z',19,700,C['deep'])
 angle={'curious':-4,'confused':5,'sleepy':4,'concerned':3}.get(e,0)
 headpart='<g transform="translate(36 22) rotate('+str(angle)+' 140 140)">'+head(e,view)+'</g>'
 return '<g class="rovi rovi-'+pose+'" data-expression="'+e+'">'+legs+feet+arms+body+hands+headpart+props+'</g>'
def card(x,y,w,h,fill=None,shadow=9):
 s=rect(x+shadow,y+shadow,w,h,C['ink'],19)
 s+=rect(x,y,w,h,fill or C['white'],19,C['ink'],3)
 return s
def label(x,y,msg,color=None):
 return text(x,y,msg,17,700,color or C['ink'],1.4)
def pill(x,y,w,name,fill=None):
 return rect(x,y,w,34,fill or C['mist'],17,C['ink'],2)+text(x+15,y+24,name,15,700)
def lines(x,y,widths,spacing=31,accent=None):
 o=''
 for i,w in enumerate(widths):
  o+=rect(x,y+i*spacing,w,9,(accent if i==2 and accent else '#D4D6D0'),5)
 return o
def buglet(x,y,variant=0,scale=1):
 bodies=[
 '<path d="M6 41Q-1 21 15 9Q35 -4 51 12Q68 32 56 50Q44 66 22 60Q4 57 6 41Z" fill="'+C['clay']+'" stroke="'+C['ink']+'" stroke-width="4"/>',
 '<path d="M10 15Q33-2 54 20Q69 42 52 59Q18 66 5 42Q-3 28 10 15Z" fill="'+C['amber']+'" stroke="'+C['ink']+'" stroke-width="4"/>',
 '<path d="M8 47Q-4 31 15 15Q27 2 46 12Q65 22 58 48Q54 64 27 61Q14 59 8 47Z" fill="'+C['sage']+'" stroke="'+C['ink']+'" stroke-width="4"/>']
 return '<g transform="translate('+str(x)+' '+str(y)+') scale('+str(scale)+')">'+bodies[variant%3]+circle(24,33,4,C['ink'])+circle(44,33,4,C['ink'])+path('M25 48q10 5 18-2',stroke=C['ink'],sw=3)+'</g>'
def scene_base():
 return rect(0,0,1600,900,C['paper'])+circle(1490,80,220,C['peach'])+circle(90,825,150,C['mist'])
def scene_hero():
 a=scene_base()+label(84,90,'OPENREVIEW / THE MARGINS')
 a+=text(84,173,'Every change',74,800)+text(84,246,'deserves a closer look.',74,800)
 a+=text(88,294,'Find what matters. Show what the evidence says.',24,400,C['deep'])
 a+=card(94,354,1040,425)
 a+=rect(96,354,1036,60,C['ink'],17)+circle(124,384,8,C['clay'])+circle(148,384,8,C['amber'])+circle(172,384,8,C['sage'])
 a+=text(219,391,'pull / 042  •  change-set',18,600,C['white'])
 a+=pill(959,366,148,'REVIEWING',C['amber'])
 for idx in range(9):
  y=445+idx*31
  a+=text(124,y+10,str(idx+78),16,600,C['sage'])
  if idx in [4,5]: a+=rect(173,y-11,894,28,C['peach'],6)
  a+=rect(199,y, [440,360,505,294,690,580,425,480,320][idx],8,C['shade'] if idx not in [4,5] else C['clay'],4)
 a+=card(720,601,355,126,C['white'],5)+label(749,639,'OBSERVATION',C['clay'])+text(749,672,'Worth investigating',20,700)+text(749,700,'No claim without evidence',14,400,C['deep'])
 a+='<g transform="translate(1065 337) scale(1.10)">'+rovi('inspecting','curious')+'</g>'
 a+=path('M1314 562 Q1220 607 1078 585',stroke=C['clay'],sw=5)+circle(1080,584,10,C['amber'],C['ink'],3)
 a+=text(94,840,'THE MARGINS — CHARACTER UNIVERSE',16,700,C['deep'],1.1)
 return svg(a,1600,900,'OpenReview hero — Rovi inspects a PR')
def scene_review():
 a=scene_base()+label(82,83,'02 / REVIEW')
 a+=text(80,159,'Curiosity before certainty.',63,800)
 a+=text(81,207,'A suspicious line is a question, not a verdict.',24,400,C['deep'])
 a+=card(76,280,1002,525)
 a+=rect(79,282,995,58,C['ink'],18)+text(118,320,'src/workflow/verification.rs',21,600,C['white'])+pill(849,291,195,'UNDER REVIEW',C['amber'])
 for i in range(10):
  y=382+i*35
  a+=text(107,y+11,str(i+111),16,600,C['sage'])
  if i==4:a+=rect(153,y-12,878,34,C['peach'],5)
  a+=rect(180,y,[350,610,465,680,570,450,605,405,522,353][i],9,C['clay'] if i==4 else C['shade'],4)
 a+=circle(878,523,28,'none',C['clay'],6)+path('M890 547l26 24',stroke=C['clay'],sw=7)
 a+=card(565,622,487,151,C['white'],4)+label(587,653,'DRAFT REVIEW NOTE',C['clay'])
 a+=text(587,691,'Potential unchecked branch',21,700)+text(587,727,'Confirm with a minimal reproducer.',18,400)
 a+='<g transform="translate(1082 319) scale(.98)">'+rovi('writing-note','suspicious')+'</g>'
 a+=buglet(965,475,0,.75)+path('M990 465q50-60 106-64',stroke=C['clay'],sw=3,extra='stroke-dasharray="7 10"')
 a+=text(83,853,'STATUS: UNVERIFIED · INVESTIGATION REQUIRED',17,700,C['deep'],1)
 return svg(a,1600,900,'Review illustration — suspicious line and draft note')
def scene_investigation():
 a=scene_base()+label(84,84,'03 / INVESTIGATE')
 a+=text(81,167,'Follow the evidence.',70,800)
 a+=text(84,215,'A trace, a reproducer, and a result — kept distinct.',25,400,C['deep'])
 a+=card(71,296,441,465,C['white'],7)+pill(100,323,150,'SIGNAL',C['amber'])
 a+=text(105,404,'Unexpected behavior',25,700)+lines(106,450,[320,270,295,170],39)
 a+=buglet(369,617,1,1.15)+text(106,712,'Observed, not yet proven',18,500,C['deep'])
 a+=path('M525 517H644',stroke=C['ink'],sw=5)+path('m626 500 19 17-19 17',stroke=C['ink'],sw=5)
 a+=card(650,296,412,465,C['white'],7)+pill(682,323,156,'REPRO',C['peach'])
 a+=text(684,404,'Minimal example',26,700)+lines(686,450,[289,240,190,269,225],37,C['clay'])
 a+=pill(685,690,202,'READY TO TEST',C['mist'])
 a+=path('M1080 517H1160',stroke=C['ink'],sw=5)+path('m1142 500 19 17-19 17',stroke=C['ink'],sw=5)
 a+=card(1182,337,342,360,C['white'],8)+pill(1208,367,189,'TEST RESULT',C['mist'])
 a+=text(1215,451,'NOT RUN',39,800)+text(1216,504,'No invented proof.',20,400,C['deep'])
 a+=circle(1353,588,32,C['amber'],C['ink'],4)+text(1340,599,'?',35,800)
 a+='<g transform="translate(1010 464) scale(.74)">'+rovi('investigating-bug','concentrating')+'</g>'
 a+=text(84,842,'THE EVIDENCE TRAIL / OPENREVIEW',16,700,C['deep'],1.3)
 return svg(a,1600,900,'Investigation scene — signal, reproducer and not-run result')
def scene_fix():
 a=scene_base()+label(80,88,'04 / AUTHORIZED FIX')+text(82,174,'Small patch. Clear scope.',66,800)
 a+=card(95,265,1110,505)+text(137,329,'PATCH PREVIEW / AUTHORIZATION REQUIRED',20,700)
 for i in range(9):
  y=389+i*35
  a+=rect(143,y,670-i*29,11,C['shade'] if i not in [4,5] else C['sage'],5)
 a+=pill(143,708,270,'PROPOSED — NOT PUSHED',C['amber'])
 a+='<g transform="translate(1150 303) scale(.95)">'+rovi('applying-patch','concentrating')+'</g>'
 return svg(a,1600,900,'Authorized fix — proposed patch')
def scene_verify(ok=True):
 a=scene_base()+label(80,88,'05 / VERIFICATION')+text(81,174,'Proof changes everything.',66,800)
 a+=card(95,284,1135,456)+text(140,352,'TEST RUNNER',22,700)
 a+=rect(145,403,1020,113,C['mist'] if ok else C['peach'],10)
 a+=text(174,476,'PASS — evidence recorded' if ok else 'FAIL — patch not verified',39,800,C['deep'] if ok else C['clay'])
 a+=lines(158,552,[830,670,740],40)
 a+='<g transform="translate(1155 286) scale(.89)">'+rovi('celebrating' if ok else 'waiting-tests','happy' if ok else 'concerned')+'</g>'
 return svg(a,1600,900,'Verification state — '+('Pass' if ok else 'Fail'))
def scene_empty():
 a=scene_base()+label(80,88,'06 / EMPTY STATE')+text(80,167,'A quiet moment.',67,800)
 a+=card(200,320,1210,370)+text(248,421,'Nothing waiting for review',43,800)+text(251,471,'When a PR arrives, the margins come alive.',23,400,C['deep'])
 a+='<g transform="translate(974 307) scale(.91)">'+rovi('resting','sleepy')+'</g>'
 return svg(a,1600,900,'Empty state — Rovi resting')
def scene_cta():
 a=scene_base()+label(80,88,'07 / NEXT STEP')+text(82,190,'Ready for a closer look?',72,800)
 a+=card(90,327,1130,366)+text(134,438,'Start with the diff.',50,800)
 a+=rect(139,501,303,78,C['ink'],15)+text(172,550,'Review a pull request',23,700,C['white'])
 a+='<g transform="translate(1124 278) scale(.94)">'+rovi('pointing-code','curious')+'</g>'
 return svg(a,1600,900,'CTA — Rovi points toward review action')
def sheet(items,cols,cellw,cellh,title,mode):
 rows=math.ceil(len(items)/cols);w=cols*cellw+80;h=rows*cellh+160
 out=rect(0,0,w,h,C['paper'])+label(44,46,'OPENREVIEW / ROVI CHARACTER BIBLE')
 out+=text(44,97,title,41,800)
 for i,item in enumerate(items):
  x=40+(i%cols)*cellw;y=126+(i//cols)*cellh
  out+=rect(x,y,cellw-12,cellh-12,C['white'],17,C['shade'],2)
  if mode=='expression':out+='<g transform="translate('+str(x+35)+' '+str(y+45)+') scale(.66)">'+head(item)+'</g>'
  else:out+='<g transform="translate('+str(x+26)+' '+str(y+8)+') scale(.63)">'+rovi(item,'neutral')+'</g>'
  out+=text(x+20,y+cellh-38,str(i+1).zfill(2)+' / '+item.upper().replace('-',' '),17,700)
 return svg(out,w,h,title)
def turnkey():
 out=rect(0,0,1430,610,C['paper'])+label(40,53,'ROVI / TURNAROUND / V1.0')
 for i,(view,labelname) in enumerate([('front','FRONT'),('three-quarter','3/4'),('side','SIDE'),('rear','REAR')]):
  x=38+i*350
  out+=rect(x,78,325,478,C['white'],20,C['shade'],2)
  out+='<g transform="translate('+str(x+3)+' '+str(95)+') scale(.90)">'+rovi('standing','neutral',view)+'</g>'
  out+=text(x+127,544,labelname,19,800)
 return svg(out,1430,610,'Rovi turnaround — four views')
def logo_mark():
 # Not replacing original mark, simplified small-size variant in standalone 128 artboard.
 o=rect(0,0,128,128,C['ink'],24)+path('M23 43Q11 28 25 12L39 28Q65 10 89 27L105 12Q113 11 117 23L125 48Q115 99 71 112Q29 115 17 76Q13 55 23 43Z',C['white'])
 o+=circle(80,64,17,'none',C['ink'],7)+circle(79,65,5,C['ink'])+path('m91 77 10 11',stroke=C['ink'],sw=7)+circle(44,63,5,C['ink'])+path('m52 75 9 8 12-12',stroke=C['ink'],sw=6)
 return svg(o,128,128,'Rovi small-size mark — exploratory v1')
def horizontal_logo():
 o=rect(0,0,1020,255,C['paper'])
 o+='<g transform="translate(13 9) scale(.75)">'+rovi('standing','curious')+'</g>'
 o+=text(260,137,'OpenReview',99,800)+text(266,177,'REVIEW WITH EVIDENCE. FIX WITH PROOF.',20,700,C['deep'],1)
 return svg(o,1020,255,'OpenReview Rovi horizontal lockup')
def write(path,content): 
 f=R/path;f.parent.mkdir(parents=True,exist_ok=True);f.write_text(content,encoding='utf-8')
def build():
 write('character/rovi-full-body-master.svg',svg('<g transform="translate(40 33)">'+rovi('standing','neutral')+'</g>',400,520,'Rovi full-body master, front'))
 write('character/turnaround.svg',turnkey())
 write('character/expression-sheet.svg',sheet(EXPRS,5,255,305,'Ten expressions / same head geometry','expression'))
 write('character/pose-sheet.svg',sheet(POSES,4,292,370,'Twelve reusable poses / one rig','pose'))
 for e in EXPRS:write('character/expressions/'+e+'.svg',svg('<g transform="translate(15 15)">'+head(e)+'</g>',290,290,'Rovi expression — '+e))
 for po in POSES:write('character/poses/'+po+'.svg',svg('<g transform="translate(12 6)">'+rovi(po)+'</g>',340,445,'Rovi pose — '+po))
 write('character/buglets-exploration.svg',svg(rect(0,0,600,240,C['paper'])+buglet(84,80,0,1.6)+buglet(268,80,1,1.6)+buglet(450,80,2,1.6)+text(100,217,'SNEAKY',16,700)+text(275,217,'NERVOUS',16,700)+text(462,217,'SHY',16,700),600,240,'Buglet exploration — three original creatures'))
 write('character/rovi-small-mark.svg',logo_mark())
 write('character/rovi-horizontal-logo.svg',horizontal_logo())
 write('character/rovi-dark-variant.svg',svg(rect(0,0,400,520,C['ink'])+'<g transform="translate(40 33)">'+rovi('standing','curious')+'</g>',400,520,'Rovi on dark'))
 write('character/rovi-light-variant.svg',svg(rect(0,0,400,520,C['paper'])+'<g transform="translate(40 33)">'+rovi('standing','curious')+'</g>',400,520,'Rovi on light'))
 scene_files={'hero':scene_hero(),'review':scene_review(),'investigation':scene_investigation(),'fix-proposed':scene_fix(),'verify-pass':scene_verify(True),'verify-fail':scene_verify(False),'empty-state':scene_empty(),'final-cta':scene_cta()}
 for n,data in scene_files.items():write('scenes/'+n+'.svg',data)
 write('character/construction.json',json.dumps({'original_head_path':HEAD,'geometry':'Original Rovi head vector retained in all frontal poses. Artboard 320x470, torso x123..244, legs from y354, head translated (36,22).','palette':C,'expressions':EXPRS,'poses':POSES,'small_size':'Use the original square mark, or experimental rovi-small-mark.svg at 24px/32px; do not shrink a full-body character into a favicon.'},indent=2))
 print('ART BUILD',len(list((R/'character').rglob('*.svg'))),'character svgs',len(scene_files),'scene svgs',flush=True)
if __name__=='__main__':build()
