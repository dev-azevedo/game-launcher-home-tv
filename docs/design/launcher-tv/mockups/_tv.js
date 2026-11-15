/* SofaLauncher, protótipo da UI de TV (mockup). Referência visual, não é código de produção.
   Todo dado externo (nome de jogo, caminho, nome de monitor) entra como texto (textContent), como manda SEC-07. */
(function(){
'use strict';
var BRAND='../../../brand/sofa/assets/';
window.onerror=function(m,s,l){ try{ parent.postMessage({t:'error',msg:m+' @'+l},'*') }catch(e){} };
var $=function(s,r){return (r||document).querySelector(s)};

/* ---------- DOM seguro ---------- */
function add(e,k){ if(k==null||k===false)return; if(Array.isArray(k))return k.forEach(function(x){add(e,x)}); e.append(k) }
function h(sel,attrs){
  var m=sel.match(/^([a-z0-9]*)((?:[.#][\w-]+)*)$/i);
  var e=document.createElement(m[1]||'div');
  (m[2].match(/[.#][\w-]+/g)||[]).forEach(function(x){ if(x[0]==='.')e.classList.add(x.slice(1)); else e.id=x.slice(1) });
  if(attrs){ for(var k in attrs){ var v=attrs[k]; if(v==null||v===false)continue;
    if(k==='text')e.textContent=v; else if(k==='style')e.style.cssText=v; else e.setAttribute(k,v===true?'':v) } }
  for(var i=2;i<arguments.length;i++)add(e,arguments[i]);
  return e;
}
var ICONS={
 play:'<path d="M8 5.5v13l11-6.5z" fill="currentColor"/>',
 vol:'<path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5H4z" fill="currentColor" stroke="none"/><path d="M15.5 9a4.2 4.2 0 010 6M18 6.5a8 8 0 010 11"/>',
 mute:'<path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5H4z" fill="currentColor" stroke="none"/><path d="M16 9.5l5 5M21 9.5l-5 5"/>',
 gamepad:'<path d="M7.5 7.5h9a4.5 4.5 0 014.4 5.4l-.7 3.3a2.6 2.6 0 01-4.4 1.3L14 15.5h-4l-1.8 2a2.6 2.6 0 01-4.4-1.3l-.7-3.3a4.5 4.5 0 014.4-5.4z"/><path d="M8 10.5v3M6.5 12h3"/><circle cx="15.5" cy="11" r=".6"/><circle cx="17.5" cy="13" r=".6"/>',
 power:'<path d="M12 3.5v8"/><path d="M6.8 6.9a7.5 7.5 0 1010.4 0"/>',
 restart:'<path d="M19.5 12a7.5 7.5 0 11-2.4-5.5"/><path d="M19.5 4.5v4.5H15"/>',
 moon:'<path d="M20 14.5A8 8 0 019.5 4a8 8 0 1010.5 10.5z"/>',
 sliders:'<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2"/><circle cx="9" cy="17" r="2"/>',
 exit:'<path d="M10 4.5H5.5v15H10"/><path d="M14.5 8l4 4-4 4M18.5 12H9.5"/>',
 eyeoff:'<path d="M3.5 3.5l17 17"/><path d="M10 6.2A8.6 8.6 0 0112 6c4 0 7 3.2 9 6a15 15 0 01-3 3.4M6.4 7.4C4.5 8.7 3 10.5 2.5 12c1.8 3 5 6 9.5 6 1.2 0 2.3-.2 3.3-.6"/><path d="M9.9 10a3 3 0 004.1 4.1"/>',
 pencil:'<path d="M4 20l1-4L16 5a2.1 2.1 0 013 3L8 19l-4 1z"/>',
 folder:'<path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/>',
 drive:'<rect x="3" y="12" width="18" height="7" rx="2"/><path d="M5 12l2-6h10l2 6"/><circle cx="7" cy="15.5" r=".7"/>',
 exe:'<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M4 9h16"/><path d="M8 13l2 2-2 2M12 17h4"/>',
 check:'<path d="M5 12.5l4.5 4.5L19 7.5"/>',
 alert:'<path d="M12 4l9 16H3L12 4z"/><path d="M12 10v4.5M12 17.5h.01"/>',
 shield:'<path d="M12 3.5l7.5 3v5.5c0 4.6-3.2 7.6-7.5 8.5-4.3-.9-7.5-3.9-7.5-8.5V6.5L12 3.5z"/><path d="M12 8.5v4M12 15.5h.01"/>',
 plus:'<path d="M12 5v14M5 12h14"/>',
 chevron:'<path d="M9 6l6 6-6 6"/>',
 monitor:'<rect x="3" y="4.5" width="18" height="12" rx="1.5"/><path d="M8.5 20h7M12 16.5V20"/>',
 grid:'<rect x="4" y="4" width="6.5" height="6.5" rx="1"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1"/>',
 clock:'<circle cx="12" cy="12" r="8"/><path d="M12 7.5V12l3 2"/>',
 view:'<rect x="4" y="7" width="11" height="11" rx="1.5"/><path d="M9 7V5.5A1.5 1.5 0 0110.5 4H18a2 2 0 012 2v7.5a1.5 1.5 0 01-1.5 1.5H17"/>',
 menu:'<path d="M5 8h14M5 12h14M5 16h14"/>',
 undo:'<path d="M9 6L4 11l5 5"/><path d="M4 11h10a5 5 0 110 10h-3"/>',
 cloud:'<path d="M7 18a4 4 0 01-.5-8 5.5 5.5 0 0110.7 1.2A3.5 3.5 0 0117 18H7z"/>'
};
ICONS.refresh=ICONS.restart;
function ic(n,c){ var t=document.createElement('template');
  t.innerHTML='<svg class="ic '+(c||'')+'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'+ICONS[n]+'</svg>';
  return t.content.firstChild }
function g(k){
  if(k==='view'||k==='menu')return h('span.g',{},ic(k));
  if(['LB','RB','LT','RT'].indexOf(k)>=0)return h('span.g.pill',{text:k});
  if(k==='dpad')return h('span.g.pill',{text:'D-pad'});
  return h('span.g',{text:k});
}
function hints(list){ return h('div.hints',{},list.map(function(x){ return h('span.hint',{},[].concat(x[0]).map(g),h('span',{text:x[1]})) })) }

/* ---------- capas procedurais (no app: cover:// com imagem local da loja) ---------- */
function artUrl(hue,seed){
  var a='hsl('+hue+',72%,58%)',b='hsl('+((hue+35)%360)+',65%,22%)',c='hsl('+((hue+180)%360)+',80%,72%)',d='hsl('+hue+',50%,12%)',m='',i;
  switch(seed%4){
    case 0: m="<circle cx='100' cy='105' r='56' fill='"+c+"'/><path d='M0 220L55 150L100 205L150 140L200 210V300H0Z' fill='"+d+"'/><path d='M0 255L70 205L130 250L200 215V300H0Z' fill='"+b+"' opacity='.9'/>";break;
    case 1: m="<g transform='rotate(-24 100 150)'>";for(i=0;i<6;i++)m+="<rect x='-40' y='"+(i*58-20)+"' width='300' height='22' fill='"+(i%2?c:a)+"' opacity='"+(.55+(i%3)*.15)+"'/>";m+="</g><rect x='30' y='200' width='140' height='70' rx='6' fill='"+d+"' opacity='.85'/>";break;
    case 2: [90,70,50,30,12].forEach(function(r,k){m+="<circle cx='100' cy='120' r='"+r+"' fill='"+(k%2?a:c)+"' opacity='"+(.95-k*.1)+"'/>"});m+="<rect y='230' width='200' height='70' fill='"+d+"'/>";break;
    default: m="<rect x='72' y='60' width='56' height='190' rx='4' fill='"+d+"'/><rect x='30' y='120' width='36' height='130' rx='4' fill='"+b+"'/><rect x='134' y='100' width='40' height='150' rx='4' fill='"+b+"'/>";for(i=0;i<5;i++)m+="<rect x='84' y='"+(80+i*32)+"' width='32' height='10' fill='"+c+"'/>";m+="<circle cx='150' cy='60' r='22' fill='"+c+"'/>";
  }
  var svg="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 300' preserveAspectRatio='xMidYMid slice'><defs><linearGradient id='g' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='"+a+"'/><stop offset='1' stop-color='"+b+"'/></linearGradient></defs><rect width='200' height='300' fill='url(#g)'/>"+m+"</svg>";
  return 'url("data:image/svg+xml,'+encodeURIComponent(svg)+'")';
}

/* ---------- dados de exemplo ---------- */
function G(id,name,origin,o){ return Object.assign({id:id,name:name,origin:origin,cover:'art',hue:262,seed:0,last:null,launch:'ok'},o) }
function base(){ return [
  G('steam:1245','Farol do Fim','Steam',{hue:262,seed:0,last:0}),
  G('epic:turbo','Turbo Rota 66','Epic',{hue:18,seed:1,last:1}),
  G('gog:orion','Cinzas de Orion','GOG',{hue:340,seed:2,last:2}),
  G('steam:3301','Vale dos Ventos','Steam',{hue:140,seed:3,last:3,orig:'ValeDosVentos-Win64'}),
  G('manual:minerio','Minério','Manual',{cover:'icon',last:5,launch:'network'}),
  G('steam:5520','Pixel Pescaria','Steam',{hue:190,seed:1,last:7}),
  G('epic:parada','Última Parada','Epic',{cover:'none',last:9,launch:'unconfirmed'}),
  G('steam:7788','Arena Cósmica','Steam',{hue:285,seed:2,last:12}),
  G('gog:chuva','Guarda-Chuva Noturno','GOG',{hue:215,seed:3,launch:'admin'}),
  G('manual:bruma','Bruma','Manual',{cover:'none',launch:'missing'}),
  G('steam:9001','Dragões de Papel','Steam',{hue:48,seed:0}),
  G('epic:fut24','Campeonato de Futebol 24','Epic',{hue:120,seed:2}),
  G('steam:9150','Casa de Areia','Steam',{hue:30,seed:3})
]}
var W1=['Torre','Vale','Reino','Mina','Porto','Ilha','Cidade','Rota','Jardim','Estação','Ponte','Cofre'];
var W2=['de Vidro','Perdido','Esquecido','Elétrico','Dourado','da Névoa','Distante','Vermelho','do Eco','Lunar','Selvagem','Sem Fim'];
function lib(kind){
  var b=base();
  if(kind==='vazia'||kind==='carregando')return [];
  if(kind==='unico')return [b[0]];
  if(kind==='longo'){
    b.push(G('steam:long1','Zero Gravidade Deluxe Edition: Complete Collection of All Expansions and Original Soundtrack Bonus','Steam',{hue:200,seed:2,last:1}));
    b.push(G('manual:html','<img src=x onerror=alert(1)>','Manual',{cover:'none'}));
    b.push(G('epic:nospace','SuperHiperMegaUltraDeLongaDuracaoSemEspacosNenhumQueSejaQuebravelNoMeioDoCard','Epic',{hue:75,seed:1}));
  }
  if(kind==='muitos'){
    for(var i=0;i<66;i++){ var o=['Steam','Epic','GOG'][i%3];
      b.push(G('gen:'+i,W1[i%12]+' '+W2[(i*5)%12]+(i>23?' '+(1+i%4):''),o,{hue:(i*47)%360,seed:i,last:i<2?(i+14):null,cover:i%11===0?'none':'art'})) }
  }
  return b;
}
function hiddenList(kind){
  var out=[],n=kind==='vazio'?0:kind==='longa'?30:4;
  var seed=[['Steamworks Common Redistributables','Steam'],['Proton Experimental','Steam'],['Trilha Sonora Original: Farol do Fim','Steam'],['Launcher antigo','Manual']];
  for(var i=0;i<n;i++){ var s=i<4?seed[i]:[W1[i%12]+' '+W2[(i*7)%12]+' (demo)',['Steam','Epic','GOG','Manual'][i%4]];
    out.push(G('hid:'+i,s[0],s[1],{hue:(i*61)%360,seed:i,cover:'none'})) }
  return out;
}
var coll=new Intl.Collator('pt-BR',{sensitivity:'base'});
function lastText(x){ return x.last==null?'Nunca jogado':x.last===0?'Jogado hoje':x.last===1?'Jogado ontem':'Jogado há '+x.last+' dias' }

/* ---------- estado ---------- */
var DEF={scenario:'home',lib:'cheia',controller:'ok',audio:'ok',suspend:'ok',hidden:'4',monitors:'dois',autostart:'on',autostartFail:'no',trava:'no',fgDenied:'no',forceDenied:'no',fallback:'no',reduced:'no',guide:'no',group:'foco',autoflow:'no',bootHold:'no'};
var C=Object.assign({},DEF);
var S={timers:[],stack:[]};
function timer(fn,ms){ var id=setTimeout(fn,ms); S.timers.push(id); return id }
function clearTimers(){ S.timers.forEach(clearTimeout); S.timers=[] }
function top(){ return S.stack[S.stack.length-1] }
function gameById(id){ return S.games.filter(function(x){return x.id===id})[0] }
function recents(){ return S.games.filter(function(x){return x.last!=null}).sort(function(a,b){return a.last-b.last}).slice(0,8) }
function alpha(){ return S.games.slice().sort(function(a,b){return coll.compare(a.name,b.name)}) }
function beh(instant){ return (instant||C.reduced==='yes')?'auto':'smooth' }
function px(n){ return n*window.innerHeight/1080 }

/* ---------- pilha de camadas ---------- */
function mount(){
  var st=S.stack,si=0,i,root=$('#layers');
  for(i=st.length-1;i>=0;i--){ if(st[i].kind==='screen'){si=i;break} }
  st.forEach(function(L,k){ if(k<si&&L.el){L.el.remove();L.el=null} });
  st.slice(si).forEach(function(L){
    if(!L.el){ L.el=L.build(L); L.el.classList.add('layer'); root.append(L.el); L.mounted&&L.mounted(L) }
    L.el.classList.toggle('top',L===top());
  });
  st.forEach(function(L){ if(L.el&&L!==top())L.el.classList.remove('top') });
}
function push(L){ L.born=performance.now(); S.stack.push(L); mount(); applyFocus(true) }
function pop(){
  if(S.stack.length<2)return;
  var L=S.stack.pop(); if(L.el)L.el.remove(); L.onClose&&L.onClose();
  mount(); var T=top(); T.onResume&&T.onResume(T); applyFocus(true);
}
function replaceTop(L){ var o=S.stack.pop(); if(o&&o.el)o.el.remove(); push(L) }
function resetStack(){ S.stack.forEach(function(L){ if(L.el){L.el.remove();L.el=null} }); S.stack=[]; $('#layers').replaceChildren() }
function rerender(L){
  if(!L.el)return; var o=L.el; L.el=L.build(L); L.el.classList.add('layer'); L.el.classList.toggle('top',L===top());
  o.replaceWith(L.el); L.mounted&&L.mounted(L); applyFocus(true);
}
function toHome(focusId){
  var b=S.stack.filter(function(l){return l.name==='Início'})[0];
  resetStack(); b=home({focusGame:focusId}); push(b);
}

/* ---------- foco ---------- */
function applyFocus(instant){
  var T=top(); if(!T||!T.el||T.inert){report();return}
  S.stack.forEach(function(L){ if(L.el&&L!==T)L.el.querySelectorAll('[data-fid]').forEach(function(e){e.setAttribute('data-focused','false')}) });
  var els=T.el.querySelectorAll('[data-fid]');
  if(els.length&&!T.el.querySelector('[data-fid="'+T.focus+'"]'))T.focus=els[0].getAttribute('data-fid');
  els.forEach(function(e){ var on=e.getAttribute('data-fid')===T.focus; e.setAttribute('data-focused',on?'true':'false'); var cd=e.closest('.card'); if(cd)cd.classList.toggle('on',on) });
  if(els.length)T.el.setAttribute('data-hasfocus',''); else T.el.removeAttribute('data-hasfocus');
  var cur=T.el.querySelector('[data-focused="true"]');
  if(cur){ if(T.reveal)T.reveal(cur,instant,T); else keepVisible(cur.closest('.card')||cur,instant) }
  T.onFocus&&T.onFocus(T.focus,cur,instant,T);
  report();
}
function keepVisible(el,instant){
  var sc=el.closest('[data-scroll]'); if(!sc)return;
  var r=el.getBoundingClientRect(),s=sc.getBoundingClientRect(),cs=getComputedStyle(sc),pad=px(10),d=(r.top-pad)-(s.top+parseFloat(cs.paddingTop)),e=(r.bottom+pad)-(s.bottom-parseFloat(cs.paddingBottom)),to=sc.scrollTop;
  if(d<0)to+=d; else if(e>0)to+=e; else return;
  sc.scrollTo({top:Math.max(0,to),behavior:beh(instant)});
}
function setFocus(L,id){ L.focus=id; applyFocus(false) }
function nav(L,dir){
  var rows=L.rows(); if(!rows.length)return;
  var r=-1,c=0; rows.forEach(function(row,i){ var k=row.indexOf(L.focus); if(k>=0){r=i;c=k} });
  if(r<0){ setFocus(L,rows[0][0]); return }
  if(dir==='Left'||dir==='Right'){
    var nc=c+(dir==='Right'?1:-1); if(nc<0||nc>=rows[r].length)return;
    L.col=nc; setFocus(L,rows[r][nc]);
  } else {
    var nr=r+(dir==='Down'?1:-1); if(nr<0||nr>=rows.length)return;
    var col=Math.min(L.col==null?c:L.col,rows[nr].length-1); L.col=col; setFocus(L,rows[nr][col]);
  }
}

/* ---------- intenções ---------- */
function intent(i){
  var L=top(); if(!L||L.inert)return;
  var on=L.on||{};
  if(i==='VolUp'||i==='VolDown'){ if(L.volKeys)volStep(i==='VolUp'?1:-1); return }
  if(i==='Up'||i==='Down'||i==='Left'||i==='Right'){ if(on.dir&&on.dir(i,L))return; nav(L,i); return }
  if(i==='PagePrev'||i==='PageNext'){ if(on.page)on.page(i,L); return }
  if(i==='Confirm'){ if(L.guard&&performance.now()-L.born<L.guard)return; on.Confirm&&on.Confirm(L.focus,L); return }
  if(i==='Back'){ if(on.Back)on.Back(L); else if(!L.noBack)pop(); return }
  if(i==='Menu'){ on.Menu&&on.Menu(L); return }
  if(i==='View'){ on.View&&on.View(L.focus,L); return }
}
var KEYS={ArrowUp:'Up',ArrowDown:'Down',ArrowLeft:'Left',ArrowRight:'Right',Enter:'Confirm',' ':'Confirm',Escape:'Back',ContextMenu:'Menu',F1:'Menu',m:'Menu',M:'Menu',v:'View',V:'View',F2:'View',PageUp:'PagePrev',PageDown:'PageNext','[':'VolDown',']':'VolUp'};
var kdown={};
function onKey(e,type){
  var tgt=e.target&&e.target.tagName;
  if(type==='up'){ if(e.key==='v'||e.key==='V')kdown.v=false; if(e.key==='m'||e.key==='M')kdown.m=false; held(!!kdown.v,!!kdown.m); return }
  if(e.key==='v'||e.key==='V')kdown.v=true; if(e.key==='m'||e.key==='M')kdown.m=true;
  if(top()&&top().inert){ held(!!kdown.v,!!kdown.m); e.preventDefault&&e.preventDefault(); return }
  if(tgt==='INPUT'){
    if(e.key==='Enter'){e.preventDefault();intent('Confirm')}
    else if(e.key==='Escape'){e.preventDefault();intent('Back')}
    else if(e.key==='ArrowDown'){e.preventDefault();intent('Down')}
    else if(e.key==='ArrowUp'){e.preventDefault();intent('Up')}
    return;
  }
  var L=top();
  if(L&&L.uac&&(e.key==='u'||e.key==='U')){L.uac('accept');return}
  if(L&&L.uac&&(e.key==='n'||e.key==='N')){L.uac('deny');return}
  var it=KEYS[e.key]; if(it){ e.preventDefault&&e.preventDefault(); if(e.repeat&&['Up','Down','Left','Right'].indexOf(it)<0)return; intent(it) }
}
addEventListener('keydown',function(e){ onKey(e,'down') });
addEventListener('keyup',function(e){ onKey(e,'up') });
addEventListener('click',function(e){
  var f=e.target.closest&&e.target.closest('[data-fid]'),L=top(); if(!f||!L||!L.el||!L.el.contains(f))return;
  L.focus=f.getAttribute('data-fid'); applyFocus(true); intent('Confirm');
});
var idle; function wake(){ document.body.classList.remove('nocursor'); clearTimeout(idle); idle=setTimeout(function(){document.body.classList.add('nocursor')},3000) }
addEventListener('mousemove',wake); wake();

/* combinação de retorno: View + Menu por 1,5 s (RN-11) */
var holdT=null,holdRaf=0;
function held(v,m){
  var L=top(); var ring=document.querySelector('.fakegame .ring i');
  if(L&&L.inert&&v&&m){
    if(holdT!=null)return; holdT=performance.now();
    (function tick(){ if(holdT==null)return; var p=Math.min(1,(performance.now()-holdT)/1500); if(ring)ring.style.width=(p*100)+'%';
      if(p>=1){holdT=null;if(ring)ring.style.width='0';combo();return} holdRaf=requestAnimationFrame(tick) })();
  } else { holdT=null; cancelAnimationFrame(holdRaf); if(ring)ring.style.width='0' }
}
function combo(){ if(S.session&&top()&&top().name==='Jogo (simulado)')push(sessionPanel(C.fallback==='yes')) }

/* ---------- avisos, volume, toast ---------- */
var TXT={
  ctrlNone:'Nenhum controle conectado. Ligue o controle ou conecte o cabo.',
  ctrlLost:'Controle desconectado. Ligue o controle ou conecte o cabo.',
  ctrlBack:'Controle conectado.'
};
function setCtrl(v){
  S.ctrl=v==='back'?'ok':v; var b=$('#banner'); b.replaceChildren(); clearTimeout(S.bannerT);
  var m={none:['gamepad',TXT.ctrlNone,''],lost:['gamepad',TXT.ctrlLost,'.lost'],back:['check',TXT.ctrlBack,'.ok']}[v];
  if(m)b.append(h('div.banner'+m[2],{role:'status'},ic(m[0]),h('span',{text:m[1]})));
  if(v==='back')S.bannerT=setTimeout(function(){b.replaceChildren()},1500);
  syncChips();
}
function syncChips(){
  var bad=S.ctrl==='none'||S.ctrl==='lost';
  document.querySelectorAll('[data-chip=ctrl]').forEach(function(e){ e.classList.toggle('bad',bad); e.querySelector('span').textContent=bad?'Sem controle':'Controle conectado' });
  document.querySelectorAll('[data-chip=vol]').forEach(function(e){ e.replaceChildren(ic(S.muted?'mute':'vol','s'),h('span',{text:S.audio==='none'?'Sem áudio':S.muted?'Mudo':String(S.volume)})) });
  document.querySelectorAll('[data-chip=sync]').forEach(function(e){ e.style.display=S.refreshing?'':'none' });
}
function volStep(dir,silent){
  var hud=$('#hud'); clearTimeout(S.hudT);
  if(S.audio==='none'){ hud.replaceChildren(h('div.vol',{role:'status'},ic('mute'),h('span.lbl',{text:'Sem áudio',style:'text-align:left'}))); S.hudT=setTimeout(function(){hud.replaceChildren()},2000); return }
  if(S.muted&&dir>0){S.muted=false;S.volume=Math.min(100,S.volume+5)}
  else S.volume=Math.max(0,Math.min(100,S.volume+dir*5));
  if(!silent)showVol(); syncChips(); var L=top(); L&&L.syncVol&&L.syncVol(L);
}
function showVol(){
  var hud=$('#hud'); clearTimeout(S.hudT);
  hud.replaceChildren(h('div.vol'+(S.muted?'.muted':''),{role:'status'},ic(S.muted?'mute':'vol'),h('div.slider'+(S.muted?'.m':''),{},h('i',{style:'width:'+S.volume+'%'})),h('span.lbl',{text:S.muted?'Mudo':String(S.volume)})));
  S.hudT=setTimeout(function(){hud.replaceChildren()},2000);
}
function toast(msg){
  var t=$('#toast'); clearTimeout(S.toastT); t.replaceChildren(h('div.toast',{role:'status'},ic('check'),h('span',{text:msg}))); S.toastT=setTimeout(function(){t.replaceChildren()},3200);
}
function report(){
  var T=top(),cur=T&&T.el&&T.el.querySelector('[data-focused="true"]');
  var n=document.querySelectorAll('[data-focused="true"]').length;
  try{ parent.postMessage({t:'status',layer:T?T.name:'',focus:cur?(cur.getAttribute('aria-label')||(cur.textContent||'').trim().slice(0,60)||cur.getAttribute('data-fid')):'',focusCount:n,stack:S.stack.map(function(l){return l.name}),sheet:!!S.sheet},'*') }catch(e){}
}

/* ---------- componentes visuais ---------- */
function header(){
  return h('div.hdr',{},h('img.logo',{src:BRAND+'logo-horizontal.svg',alt:'SofaLauncher'}),
    h('div.chips',{},
      h('span.chip',{'data-chip':'sync',style:'display:none'},ic('refresh','s'),h('span',{text:'Atualizando biblioteca…'})),
      h('span.chip',{'data-chip':'ctrl'},ic('gamepad','s'),h('span',{text:'Controle conectado'})),
      h('span.chip',{'data-chip':'vol'}),
      h('span.chip',{},ic('clock','s'),h('span',{text:'21:47'}))));
}
function poster(x,o){
  o=o||{}; var cls='div.poster.focusable'+(x.cover==='none'?'.ph':x.cover==='icon'?'.icon':'');
  var p=h(cls,{'data-fid':o.fid,role:'button','aria-label':x.name+', '+x.origin});
  if(x.cover==='art')p.append(h('div.art',{style:'background-image:'+artUrl(x.hue,x.seed)}));
  if(x.cover==='none')p.append(h('div.ph-name',{},h('span',{text:x.name})));
  if(x.cover==='icon')p.append(ic('exe','xl'));
  p.append(h('div.scrim'),h('span.pill.glass.b-origin',{text:x.origin}));
  if(o.run)p.append(h('span.pill.amber.b-prog',{text:'Em andamento'}));
  return p;
}
function card(x,fid){ var run=S.session&&S.session.id===x.id; return h('div.card',{'data-gid':x.id},poster(x,{fid:fid,run:run}),h('div.nm.clamp1',{text:x.name})) }
function allCard(n){ return h('div.card',{},h('div.poster.focusable.all',{'data-fid':'all'},ic('grid','l'),h('span',{text:'Todos os jogos'}),h('span.muted',{text:String(n)})),h('div.nm',{text:' '})) }
function btn(id,label,kind,icon){ return h('button.btn.focusable'+(kind?'.'+kind:''),{'data-fid':id,type:'button'},icon?ic(icon):null,label) }
function glyphHints(){ return hints([['A','Jogar'],['view','Opções'],['menu','Menu'],[['LT','RT'],'Volume']]) }

/* ---------- diálogo genérico ---------- */
function dlg(c){
  var L={name:c.name||'Diálogo',kind:'overlay',focus:c.focus||c.buttons[0].id,guard:c.guard||0,uac:c.uac,
    rows:function(){return [c.buttons.map(function(b){return b.id})]},
    build:function(){
      var side=c.cover?h('div.side',{},h('div.thumb',{},h('div',{style:'position:absolute;inset:0;background-size:cover;background-position:center;'+(c.cover.cover==='art'?'background-image:'+artUrl(c.cover.hue,c.cover.seed):'background:linear-gradient(165deg,#322a68,#1b1540)')}),c.cover.cover==='none'?h('span',{style:'position:absolute;inset:0;display:grid;place-items:center;font:800 calc(56*var(--u))/1 var(--font-display);color:var(--brand-text)',text:Array.from(c.cover.name)[0]||''}):null,c.cover.cover==='icon'?h('span',{style:'position:absolute;inset:0;display:grid;place-items:center;color:var(--brand-text-muted)'},ic('exe','l')):null)):
        h('div.side',{},h('div.badge-ic'+(c.tone==='danger'?'.danger':c.tone==='ok'?'.ok':''),{},ic(c.icon||'alert')));
      var body=h('div.dc',{},h('h2',{id:'dt',text:c.title}));
      (c.body||[]).forEach(function(b){ body.append(h('p.body',{},b)) });
      if(c.path)body.append(h('div.path',{text:c.path}));
      if(c.warn)body.append(h('div.warnline'+(c.warnTone==='amber'?'.amber':''),{role:'alert'},ic('alert','s'),h('span',{text:c.warn})));
      if(c.spin)body.append(h('div.rowspin',{},h('div.spinner'),h('span',{text:c.spin})));
      if(c.sim)body.append(h('div.uac',{},ic('shield'),h('span',{},h('b',{text:'Janela do Windows (UAC). '}),c.sim)));
      body.append(h('div.btnrow',{},c.buttons.map(function(b){ return btn(b.id,b.label,b.kind,b.icon) })));
      return h('div.layer',{role:'alertdialog','aria-labelledby':'dt','aria-modal':'true'},h('div.scrim-l'),h('div.center',{},h('div.dialog',{},side,body)));
    },
    on:{
      Confirm:function(id){ var b=c.buttons.filter(function(x){return x.id===id})[0]; b&&b.run&&b.run(L) },
      Back:function(){ var b=c.buttons.filter(function(x){return x.back})[0]; if(b){b.run?b.run(L):pop()} else pop() }
    }};
  return L;
}
function B(id,label,run,o){ return Object.assign({id:id,label:label,run:run},o||{}) }

/* ---------- menu genérico (opções do jogo, painel de sessão) ---------- */
function menuL(c){
  var L={name:c.name,kind:'overlay',focus:c.focus||c.items[0].id,
    rows:function(){return c.items.filter(function(i){return !i.sep&&!i.disabled}).map(function(i){return [i.id]})},
    build:function(){
      var m=h('div.menu',{role:'menu','aria-label':c.name},c.head,c.items.map(function(i){
        if(i.sep)return h('div.sep');
        return h('div.mi.focusable'+(i.danger?'.danger':''),{'data-fid':i.id,role:'menuitem','aria-disabled':i.disabled?'true':null},
          ic(i.icon),h('div.grow',{},h('span.clamp1',{text:i.label,style:'display:block'}),i.small?h('span.small',{text:i.small}):null),i.right||null) }),
        c.foot||null, hints(c.hints||[['A','Escolher'],['B','Voltar']]));
      return h('div.layer',{},h('div.scrim-l'),h('div.center',{},m));
    },
    on:{Confirm:function(id){ var i=c.items.filter(function(x){return x.id===id})[0]; i&&i.run&&i.run(L) },Back:c.onBack||function(){pop()},Menu:c.onMenu}};
  return L;
}

/* ---------- abertura ---------- */
function boot(){
  return {name:'Abertura',kind:'screen',focus:null,noBack:true,rows:function(){return []},
    build:function(){ return h('div.layer.boot'+(C.bootHold==='yes'?'.hold':''),{},h('img.splash',{src:BRAND+'splash.svg',alt:'SofaLauncher'}),
      h('div.st',{role:'status'},h('div.bar',{},h('i')),h('span',{text:'Carregando biblioteca…'}))) }};
}

/* ---------- início ---------- */
function home(o){
  o=o||{};
  var L={name:'Início',kind:'screen',volKeys:true,noBack:true,focus:null,spot:30};
  L.rows=function(){
    if(!S.games.length)return [C.lib==='carregando'?['add']:['add','refresh']];
    var rows=[],rc=recents(); if(rc.length)rows.push(rc.map(function(x){return 'r:'+x.id}));
    rows.push(alpha().slice(0,12).map(function(x){return 'a:'+x.id}).concat(['all'])); return rows;
  };
  function initialFocus(){
    if(!S.games.length)return 'add';
    var rc=recents(),want=o.focusGame||L.focusGame;
    if(want&&gameById(want)){ var g1=gameById(want); return (g1.last!=null&&rc.indexOf(g1)>=0?'r:':'a:')+want }
    return rc.length?'r:'+rc[0].id:'a:'+alpha()[0].id;
  }
  L.focus=initialFocus();
  L.build=function(){
    var el=h('div.layer.home',{'data-screen':'home'},h('div.bg'),h('div.spot'),h('div.veil'));
    var safe=h('div.safe'); el.append(safe); safe.append(header());
    if(!S.games.length){
      if(C.lib==='carregando'){
        safe.append(h('div.empty',{role:'status'},h('div.big',{text:'Procurando jogos…'}),h('p',{text:'Lendo a Steam, a Epic e o GOG deste PC.'}),
          h('div.row',{style:'padding:0;margin:0;'},[0,1,2,3,4,5].map(function(){return h('div.card',{},h('div.poster.sk'))})),
          h('div.btnrow',{},btn('add','Adicionar jogo','','plus'))));
      } else {
        safe.append(h('div.empty',{},h('div.big',{text:'Nenhum jogo por aqui'}),h('p',{text:'Instale jogos na Steam, na Epic ou no GOG, ou adicione um executável.'}),
          h('div.btnrow',{},btn('add','Adicionar jogo','primary','plus'),btn('refresh','Atualizar biblioteca','','refresh'))));
      }
    } else {
      safe.append(h('div.hero',{},h('div.eyebrow'),h('div.name-big.clamp2',{'aria-live':'polite'})));
      var rows=h('div.rows',{'data-scroll':''}); var rc=recents(),al=alpha();
      if(rc.length)rows.append(h('section.rowblock',{'data-row':'r'},h('div.rowtitle',{text:'Recentes'}),h('div.row',{},rc.map(function(x){return card(x,'r:'+x.id)}))));
      rows.append(h('section.rowblock',{'data-row':'a'},h('div.rowtitle',{text:'Biblioteca'}),h('div.row',{},al.slice(0,12).map(function(x){return card(x,'a:'+x.id)}).concat([allCard(S.games.length)]))));
      safe.append(rows);
    }
    safe.append(S.games.length?glyphHints():hints([['A','Escolher'],['menu','Menu']]));
    return el;
  };
  L.mounted=function(){ syncChips(); var r=L.el.querySelector('.rows'); if(r)L.reveal(L.el.querySelector('[data-focused="true"]')||L.el.querySelector('[data-fid="'+L.focus+'"]'),true,L) };
  L.reveal=function(cur,instant){
    if(!cur)return; var row=cur.closest('.row'),block=cur.closest('.rowblock'),rows=L.el.querySelector('.rows');
    if(row){ var cd=cur.closest('.card'); var t=Math.max(0,Math.min(row.scrollWidth-row.clientWidth,cd.offsetLeft-(row.clientWidth-cd.offsetWidth)/2));
      row.scrollTo({left:t,behavior:beh(instant)});
      var x=(row.getBoundingClientRect().left+cd.offsetLeft-t+cd.offsetWidth/2)/innerWidth*100; L.el.style.setProperty('--spot-x',Math.max(12,Math.min(88,x))+'%') }
    if(block&&rows)rows.scrollTo({top:block.offsetTop-parseFloat(getComputedStyle(rows).paddingTop),behavior:beh(instant)});
  };
  L.onFocus=function(fid){
    var el=L.el; if(!el)return; var hero=el.querySelector('.hero'); if(!hero)return;
    var x=fid&&fid.indexOf(':')>0?gameById(fid.slice(2)):null, eb=hero.querySelector('.eyebrow'),nm=hero.querySelector('.name-big'),bg=el.querySelector('.bg');
    if(x){ var run=S.session&&S.session.id===x.id;
      eb.replaceChildren(h('span.pill',{text:x.origin}),h('span',{text:run?'Em andamento. Aperte A para voltar ao jogo.':lastText(x)}));
      nm.textContent=x.name; bg.style.backgroundImage=x.cover==='art'?artUrl(x.hue,x.seed):'none'; bg.style.opacity=x.cover==='art'?'.26':'0';
    } else if(fid==='all'){ eb.replaceChildren(h('span',{text:'Biblioteca'})); nm.textContent='Todos os jogos'; bg.style.opacity='0' }
  };
  L.on={
    Confirm:function(fid){
      if(fid==='all')return push(allGames(L.focus));
      if(fid==='add')return openFolder('');
      if(fid==='refresh')return doRefresh();
      launch(gameById(fid.slice(2)));
    },
    View:function(fid){ if(fid&&fid.indexOf(':')>0)push(options(gameById(fid.slice(2)))) },
    Menu:function(){ push(quick()) },
    page:function(i,l){ nav(l,i==='PageNext'?'Down':'Up') }
  };
  return L;
}
function doRefresh(done){
  if(S.refreshing)return; S.refreshing=true; syncChips(); var L=top(); L.refreshUi&&L.refreshUi(L);
  timer(function(){ S.refreshing=false; syncChips(); toast(S.games.length?'Biblioteca atualizada.':'Nenhum jogo novo.'); var T=top(); T&&T.refreshUi&&T.refreshUi(T) },1800);
}

/* ---------- todos os jogos ---------- */
function allGames(focusFid){
  var L={name:'Todos os jogos',kind:'screen',volKeys:true,focus:null};
  var list=alpha(); var COLS=7;
  L.rows=function(){ var r=[]; for(var i=0;i<list.length;i+=COLS)r.push(list.slice(i,i+COLS).map(function(x){return 'g:'+x.id})); return r };
  L.focus=(focusFid&&focusFid.indexOf(':')>0&&gameById(focusFid.slice(2))?'g:'+focusFid.slice(2):'g:'+list[0].id);
  L.build=function(){
    var el=h('div.layer.home',{},h('div.spotlight'),h('div.safe',{},header(),
      h('div',{style:'display:flex;align-items:baseline;gap:var(--space-3);margin-top:var(--space-2)'},h('div.title',{text:'Todos os jogos'}),h('div.sub',{text:list.length+' jogos, de A a Z'})),
      h('div.grid',{'data-scroll':''},h('div.gi',{},list.map(function(x){return card(x,'g:'+x.id)}))),
      hints([['A','Jogar'],['view','Opções'],[['LB','RB'],'Pular linhas'],['B','Voltar']])));
    return el;
  };
  L.mounted=function(){ syncChips(); var c=L.el.querySelector('[data-fid="'+L.focus+'"]'); if(c)keepVisible(c.closest('.card'),true) };
  L.on={Confirm:function(fid){launch(gameById(fid.slice(2)))},View:function(fid){push(options(gameById(fid.slice(2))))},Menu:function(){push(quick())},
    page:function(i,l){ for(var k=0;k<3;k++)nav(l,i==='PageNext'?'Down':'Up') }};
  L.onResume=function(l){ rerender(l) };
  return L;
}

/* ---------- opções do jogo, renomear, ocultar ---------- */
function options(x){
  var run=S.session&&S.session.id===x.id;
  var items=[{id:'play',label:run?'Voltar ao jogo':'Jogar',icon:'play',run:function(){pop();launch(x)}},
    {id:'rename',label:'Renomear',icon:'pencil',small:'Pelo teclado do PC',run:function(){push(rename(x))}}];
  if(x.orig)items.push({id:'restore',label:'Restaurar nome original',icon:'undo',small:x.orig,run:function(){ x.name=x.orig; delete x.orig; pop(); refreshBase(); toast('Nome original restaurado.') }});
  items.push({id:'hide',label:'Ocultar',icon:'eyeoff',small:'Nenhum arquivo é apagado',danger:true,run:function(){push(hideConfirm(x))}});
  return menuL({name:'Opções do jogo',items:items,focus:'play',
    head:h('div.mh',{},h('div.thumb',{style:'width:'+'calc(64*var(--u))'+';height:calc(96*var(--u));border-radius:calc(10*var(--u))'},h('div',{style:'position:absolute;inset:0;background-size:cover;'+(x.cover==='art'?'background-image:'+artUrl(x.hue,x.seed):'background:linear-gradient(165deg,#322a68,#1b1540)')})),
      h('div',{style:'min-width:0'},h('h2.clamp2',{text:x.name}),h('div.sub',{text:x.origin+' · '+lastText(x)})))});
}
function refreshBase(){ var b=S.stack[0]; if(b&&b.name==='Início'){ if(b.el)rerender(b) } S.stack.forEach(function(l){ if(l!==top()&&l.name==='Todos os jogos'&&l.el)rerender(l) }) }
function hideConfirm(x){
  return dlg({name:'Ocultar jogo',icon:'eyeoff',tone:'danger',title:'Ocultar '+x.name+'?',
    body:['O jogo sai da biblioteca e não volta nas atualizações. Nenhum arquivo é apagado.','Para restaurar, abra Configurações e escolha Itens ocultos.'],focus:'cancel',
    buttons:[B('cancel','Cancelar',function(){pop()},{back:true}),B('ok','Ocultar',function(){
      x.hidden=true; S.games=S.games.filter(function(y){return y!==x}); S.hidden.push(x);
      var nextFocus=null; var al=alpha(); nextFocus=al[0]&&al[0].id;
      S.stack=S.stack.filter(function(l){ if(l.name==='Diálogo'||l.name==='Ocultar jogo'||l.name==='Opções do jogo'){ if(l.el)l.el.remove(); return false } return true });
      var T=top(); if(T.name==='Início'){ T.focus=al.length?(recents().length?'r:'+recents()[0].id:'a:'+nextFocus):'add'; rerender(T) } else if(T.name==='Todos os jogos'){ T.onResume(T) }
      mount(); applyFocus(true); toast(x.name+' oculto. Restaure em Configurações, Itens ocultos.');
    },{kind:'danger',icon:'eyeoff'})]});
}
function rename(x){
  var L={name:'Renomear jogo',kind:'overlay',focus:'input',value:x.name,error:'',rows:function(){return [['input'],['cancel','save']]}};
  function save(){
    var v=(L.value||'').replace(/[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069]/g,'').replace(/\s+/g,' ').trim();
    if(!v){ L.error='O nome não pode ficar vazio.'; rerender(L); L.focus='input'; applyFocus(true); return }
    if(!x.orig&&v!==x.name)x.orig=x.name; x.name=v.slice(0,256); if(x.orig===x.name)delete x.orig;
    pop(); pop(); refreshBase(); toast('Nome salvo.');
  }
  L.build=function(){
    var inp=h('input',{type:'text',value:L.value,'aria-label':'Novo nome do jogo',maxlength:'256',spellcheck:'false',autocomplete:'off',style:'width:100%;background:transparent;border:0;outline:0;color:var(--brand-text);font:600 var(--text-card-title)/1.2 var(--font-text);padding:0'});
    inp.addEventListener('input',function(){ L.value=inp.value });
    var wrap=h('div.focusable',{'data-fid':'input',style:'background:var(--brand-surface);border-radius:var(--radius-button);padding:var(--space-3) var(--space-3);'+(L.error?'outline:calc(3*var(--u)) solid var(--brand-danger)':'')},inp);
    return h('div.layer',{role:'dialog','aria-labelledby':'dt','aria-modal':'true'},h('div.scrim-l'),h('div.center',{},h('div.dialog',{style:'width:calc(1100*var(--u))'},
      h('div.side',{},h('div.badge-ic',{},ic('pencil'))),
      h('div.dc',{},h('h2',{id:'dt',text:'Renomear jogo'}),h('p.body',{text:'Nome original: '+(x.orig||x.name)}),wrap,
        L.error?h('div.warnline',{role:'alert'},ic('alert','s'),h('span',{text:L.error})):null,
        h('p.body',{text:'Digite no teclado do PC. Enter salva, Esc cancela.'}),
        h('div.btnrow',{},btn('cancel','Cancelar'),btn('save','Salvar','primary','check'))))));
  };
  L.onFocus=function(fid){ var inp=L.el&&L.el.querySelector('input'); if(!inp)return; if(fid==='input'){inp.focus();inp.setSelectionRange(inp.value.length,inp.value.length)} else inp.blur() };
  L.on={Confirm:function(id){ if(id==='cancel')pop(); else save() },Back:function(){pop()}};
  return L;
}

/* ---------- lançar, sessão ---------- */
function startRunning(x,elev){ S.session={id:x.id,state:'running',since:Date.now()-72*60000,elevated:!!elev}; x.last=0 }
function launch(x){
  if(!x)return;
  if(S.session){ if(S.session.id===x.id)return returnToGame(); return push(switchDlg(x)) }
  startLaunch(x);
}
function launchingDlg(x){
  return dlg({name:'Abrindo',cover:x,title:'Abrindo '+x.name+'…',body:['Aguarde. O jogo vai aparecer na frente.'],spin:'Iniciando',guard:600,
    buttons:[B('back','Voltar',function(){ S.session=null; pop() },{back:true})]});
}
function startLaunch(x,hold){
  S.session={id:x.id,state:'launching'}; push(launchingDlg(x)); if(hold)return;
  var m=x.launch;
  function still(){ return S.session&&S.session.id===x.id&&S.session.state==='launching'&&top().name==='Abrindo' }
  if(m==='missing')timer(function(){ if(!still())return; S.session=null; replaceTop(errDlg('GameFileMissing',x)) },800);
  else if(m==='network')timer(function(){ if(!still())return; S.session=null; replaceTop(errDlg('PathNetwork',x)) },800);
  else if(m==='admin')timer(function(){ if(!still())return; S.session={id:x.id,state:'needsElevation'}; replaceTop(adminDlg(x)) },800);
  else if(m==='unconfirmed')timer(function(){ if(!still())return; replaceTop(errDlg('NotConfirmed',x)) },3500);
  else timer(function(){ if(!still())return; startRunning(x); enterGame() },1800);
}
function enterGame(){
  var b=S.stack.filter(function(l){return l.name==='Início'})[0]||home(); var x=gameById(S.session.id);
  resetStack(); b.focus=(x&&x.last===0&&recents().indexOf(x)>=0)?'r:'+x.id:b.focus; push(b); push(ingame());
}
function returnToGame(){
  if(C.fgDenied==='yes'&&S.session.state!=='x')return push(errDlg('ForegroundDenied',gameById(S.session.id)));
  enterGame();
}
function ingame(){
  var x=gameById(S.session.id);
  return {name:'Jogo (simulado)',kind:'screen',inert:true,focus:null,rows:function(){return []},
    build:function(){ return h('div.layer.fakegame',{},h('div.art',{style:x.cover==='art'?'background-image:'+artUrl(x.hue,x.seed):'background:#1b1540'}),h('div.vig'),h('div.hud',{},h('i'),h('i')),
      h('div.sim',{},ic('gamepad','s'),h('span',{text:'Simulação: o jogo está na frente e o SofaLauncher em segundo plano. Segure View + Menu por 1,5 s (no teclado, V + M).'})),h('div.ring',{},h('i'))) }};
}
function sessionPanel(fallback){
  var x=gameById(S.session.id);
  var note=fallback?'O Windows não deixou o SofaLauncher passar na frente. O jogo foi minimizado. Voltar ao jogo restaura a janela.':(S.session.elevated?'Este jogo roda como administrador. A combinação de retorno pode falhar nele.':null);
  var mins=Math.round((Date.now()-S.session.since)/60000),since=mins<1?'Jogando agora':mins<60?'Jogando há '+mins+' min':'Jogando há '+Math.floor(mins/60)+' h '+(mins%60)+' min';
  var L=menuL({name:'Jogo em andamento',focus:'back',
    head:h('div.panel-hero',{},h('div.thumb',{},h('div',{style:'position:absolute;inset:0;background-size:cover;'+(x.cover==='art'?'background-image:'+artUrl(x.hue,x.seed):'background:linear-gradient(165deg,#322a68,#1b1540)')})),
      h('div.ph1',{},h('div.lab',{text:'Em andamento'}),h('h2.clamp2',{text:x.name}),h('div.sub',{text:since}))),
    items:[{id:'back',label:'Voltar ao jogo',icon:'play',run:function(){pop()}},
      {id:'close',label:'Fechar jogo',icon:'power',danger:true,run:function(){push(closeConfirm(x))}},
      {id:'lib',label:'Ir para a biblioteca',icon:'grid',small:'O jogo continua aberto',run:function(){toHome(x.id)}},
      {id:'quick',label:'Menu rápido',icon:'sliders',run:function(){push(quick({fromSession:true}))}}],
    foot:note?h('div.minnote',{text:note}):null,
    hints:[['A','Escolher'],['B','Voltar ao jogo'],['menu','Menu rápido']],
    onBack:function(){pop()},onMenu:function(){push(quick({fromSession:true}))}});
  return L;
}
function closeConfirm(x){
  return dlg({name:'Fechar jogo',icon:'power',tone:'danger',title:'Fechar '+x.name+'?',body:['O SofaLauncher pede ao jogo para fechar.'],warn:'O progresso não salvo pode ser perdido.',focus:'cancel',
    buttons:[B('cancel','Cancelar',function(){pop()},{back:true}),B('ok','Fechar jogo',function(){ doClose(x) },{kind:'danger',icon:'power'})]});
}
function doClose(x){
  S.session.state='closing'; var cl=dlg({name:'Fechando',cover:x,title:'Fechando '+x.name+'…',body:['Esperando o jogo fechar. Pode levar alguns segundos.'],spin:'Fechando',guard:600,
    buttons:[B('lib','Ir para a biblioteca',function(){ toHome(x.id) },{back:true})]});
  replaceTop(cl); S.closingL=cl;
  if(C.trava==='yes')timer(function(){ var f=forceOffer(x); if(top()===cl)replaceTop(f); else push(f) },3500);
  else timer(function(){ gameClosed(x) },2200);
}
function forceOffer(x){
  S.session.state='overdue';
  return dlg({name:'Forçar fechamento',cover:x,title:x.name+' não fechou',body:['O jogo não respondeu ao pedido para fechar.'],warn:'O progresso não salvo será perdido.',focus:'cancel',
    buttons:[B('cancel','Cancelar',function(){ S.session.state='running'; pop() },{back:true}),
      B('force','Forçar fechamento',function(){ if(C.forceDenied==='yes')replaceTop(errDlg('ForceDenied',x)); else gameClosed(x) },{kind:'danger',icon:'power'})]});
}
function gameClosed(x){ S.session=null; toHome(x.id); toast(x.name+' fechado.') }
function switchDlg(nw){
  var cur=gameById(S.session.id);
  return dlg({name:'Trocar de jogo',icon:'restart',tone:'danger',title:cur.name+' está aberto. Fechar e abrir '+nw.name+'?',body:['O progresso não salvo de '+cur.name+' pode ser perdido.'],focus:'cancel',
    buttons:[B('cancel','Cancelar',function(){pop()},{back:true}),B('ok','Fechar e abrir',function(){ var x=cur; S.session=null; S.stack.pop().el.remove(); push(closingSwap(x,nw)) },{kind:'danger'})]});
}
function closingSwap(old,nw){
  var l=dlg({name:'Fechando',cover:old,title:'Fechando '+old.name+'…',body:['Depois o SofaLauncher abre '+nw.name+'.'],spin:'Fechando',guard:600,buttons:[B('x','Voltar',function(){pop()},{back:true})]});
  timer(function(){ if(top()===l){ pop(); startLaunch(nw) } },1800); return l;
}
function adminDlg(x){
  return dlg({name:'Pede administrador',icon:'shield',tone:'warn',title:x.name,body:[x.name+' precisa de permissão de administrador para abrir. O Windows vai pedir a sua confirmação.'],focus:'go',
    buttons:[B('back','Voltar',function(){ S.session=null; pop() },{back:true}),B('go','Continuar',function(){ replaceTop(adminWait(x)) },{kind:'primary'})]});
}
function adminWait(x){
  var d=dlg({name:'AdminWait',icon:'shield',tone:'warn',title:x.name,
    body:[x.name+' pede permissão de administrador. Confirme no Windows com teclado ou mouse. A combinação de retorno pode não funcionar neste jogo.'],spin:'Esperando o Windows',
    sim:'Quer permitir que este app faça alterações no dispositivo? (Aceitar: tecla U. Recusar: tecla N.)',focus:'back',
    buttons:[B('back','Voltar',function(){ S.session=null; pop() },{back:true})]});
  d.uac=function(v){ if(v==='accept'){ startRunning(x,true); enterGame() } else { S.session=null; replaceTop(errDlg('UacDenied',x)) } };
  S.uacL=d; return d;
}

/* ---------- catálogo de texto por código (espelha microcopy.md) ---------- */
function errDlg(code,x){
  var nm=x?x.name:'', V=B('back','Voltar',function(){pop()},{back:true});
  var map={
    GameFileMissing:function(){ return {icon:'alert',cover:x,title:nm,body:['Não encontramos o arquivo do jogo em '+'D:\\Jogos\\'+nm+'\\'+nm+'.exe'],focus:'back',
      buttons:[V,B('hide','Ocultar',function(){ pop(); push(hideConfirm(x)) },{icon:'eyeoff'})]} },
    PathNetwork:function(){ return {icon:'alert',cover:x,title:nm,body:['Jogos em pastas de rede não são aceitos. Copie o jogo para um disco deste PC.'],focus:'back',
      buttons:[V,B('hide','Ocultar',function(){ pop(); push(hideConfirm(x)) },{icon:'eyeoff'})]} },
    NotConfirmed:function(){ return {icon:'alert',cover:x,title:nm,body:['Não foi possível confirmar que '+nm+' abriu'],focus:'retry',
      buttons:[B('back','Voltar',function(){ S.session=null; pop() },{back:true}),B('retry','Tentar de novo',function(){ pop(); startLaunch(x) },{kind:'primary',icon:'restart'})]} },
    UacDenied:function(){ return {icon:'shield',tone:'danger',cover:x,title:nm,body:['O Windows não deu permissão para abrir '+nm],focus:'back',buttons:[B('back','Voltar',function(){pop()},{back:true})]} },
    StoreNotInstalled:function(){ return {icon:'alert',cover:x,title:nm,body:['Não encontramos a '+(x&&x.origin||'loja')+' neste PC. Instale a loja para abrir este jogo.'],focus:'back',buttons:[V]} },
    ForegroundDenied:function(){ return {icon:'alert',cover:x,title:nm,body:['O Windows não deixou voltar ao jogo. Tente de novo.'],focus:'retry',
      buttons:[B('back','Ir para a biblioteca',function(){ toHome(x.id) },{back:true}),B('retry','Tentar de novo',function(){ pop(); C.fgDenied='no'; returnToGame() },{kind:'primary'})]} },
    ForceDenied:function(){ return {icon:'shield',tone:'danger',cover:x,title:nm,body:['Não foi possível fechar '+nm+'. O jogo roda como administrador. Feche pelo menu do próprio jogo.'],focus:'back',
      buttons:[B('back','Voltar',function(){ S.session.state='running'; pop() },{back:true})]} },
    Busy:function(){ return {icon:'alert',title:'Um momento',body:['O SofaLauncher ainda está terminando outra ação. Tente de novo em instantes.'],focus:'back',buttons:[V]} }
  };
  var c=map[code](); c.name='Aviso: '+code; return dlg(c);
}

/* ---------- menu rápido ---------- */
function quick(o){
  o=o||{};
  var items=['vol','mute','off','restart','suspend','exit','settings'];
  var L={name:'Menu rápido',kind:'overlay',focus:S.audio==='none'?'off':'vol'};
  var dis={vol:S.audio==='none',mute:S.audio==='none',suspend:C.suspend==='no'};
  L.rows=function(){ return items.filter(function(i){return !dis[i]}).map(function(i){return [i]}) };
  function volRow(){
    return h('div.mi.focusable',{'data-fid':'vol',role:'slider','aria-label':'Volume','aria-valuemin':'0','aria-valuemax':'100','aria-valuenow':String(S.volume),'aria-disabled':dis.vol?'true':null},
      ic(S.muted?'mute':'vol'),h('div.grow',{},h('span',{text:'Volume'}),dis.vol?h('span.small',{text:'Nenhum dispositivo de áudio encontrado'}):null),
      dis.vol?null:h('div.slider'+(S.muted?'.m':''),{style:'max-width:calc(260*var(--u));min-width:calc(200*var(--u))'},h('i',{style:'width:'+S.volume+'%'})),
      dis.vol?null:h('span.num',{text:String(S.volume)}));
  }
  function row(id,icon,label,extra,danger,small){
    return h('div.mi.focusable'+(danger?'.danger':''),{'data-fid':id,role:'menuitem','aria-disabled':dis[id]?'true':null},ic(icon),h('div.grow',{},h('span',{text:label}),small?h('span.small',{text:small}):null),extra||null);
  }
  L.build=function(){
    var m=h('div.menu.right',{role:'menu','aria-label':'Menu rápido'},
      h('div.mh',{},h('img',{src:BRAND+'logo-horizontal.svg',alt:'SofaLauncher',style:'height:calc(44*var(--u))'}),
        S.session?h('span.pill.amber',{style:'margin-left:auto',text:'Em andamento'}):null),
      volRow(),
      row('mute',S.muted?'mute':'vol','Mudo',h('span.sw'+(S.muted?'.on':'')),false),
      h('div.note',{},g('LT'),g('RT'),h('span',{text:'Na biblioteca, LT diminui e RT aumenta o volume.'})),
      h('div.sep'),
      row('off','power','Desligar',null,true),row('restart','restart','Reiniciar',null,true),
      row('suspend','moon','Suspender',null,true,dis.suspend?'Suspensão indisponível neste PC':null),
      row('exit','exit','Fechar o launcher'),
      h('div.sep'),
      row('settings','sliders','Configurações'),
      hints([['dpad','Ajustar volume'],['A','Escolher'],['B','Fechar']]));
    return h('div.layer',{},h('div.scrim-l'),m);
  };
  function paint(){ var el=L.el; if(!el)return; var v=el.querySelector('[data-fid="vol"]');
    if(v){ v.setAttribute('aria-valuenow',String(S.volume)); var i=v.querySelector('.slider i'); if(i)i.style.width=S.volume+'%'; var n=v.querySelector('.num'); if(n)n.textContent=String(S.volume);
      var sl=v.querySelector('.slider'); if(sl)sl.classList.toggle('m',S.muted); v.querySelector('svg').replaceWith(ic(S.muted?'mute':'vol')) }
    var m=el.querySelector('[data-fid="mute"]'); if(m){ m.querySelector('.sw').classList.toggle('on',S.muted); m.querySelector('svg').replaceWith(ic(S.muted?'mute':'vol')) } }
  L.syncVol=paint;
  L.on={
    dir:function(d,l){ if(l.focus==='vol'&&(d==='Left'||d==='Right')){ if(S.audio==='none')return true; volStep(d==='Right'?1:-1,true); return true } return false },
    Confirm:function(id){
      if(id==='mute'){ S.muted=!S.muted; syncChips(); paint() }
      else if(id==='off')push(power('shutdown'));
      else if(id==='restart')push(power('restart'));
      else if(id==='suspend')push(power('suspend'));
      else if(id==='settings'){ pop(); push(settings({})) }
      else if(id==='exit')push(exitDlg());
    },
    Menu:function(){pop()}
  };
  return L;
}
function power(kind){
  var m={shutdown:['Desligar o PC?','O Windows desliga do jeito normal. Se outro app tiver trabalho não salvo, o Windows avisa.','Desligar','power','Desligando…','O Windows está encerrando.'],
    restart:['Reiniciar o PC?','O Windows reinicia do jeito normal.'+(S.autostart?' O SofaLauncher abre de novo quando você entrar.':''),'Reiniciar','restart','Reiniciando…','O Windows está reiniciando.'],
    suspend:['Suspender o PC?','O PC entra em suspensão. Ao acordar, o SofaLauncher volta para esta tela.','Suspender','moon','Suspendendo…','O PC está entrando em suspensão.']}[kind];
  var x=S.session&&gameById(S.session.id);
  return dlg({name:'Confirmar energia',icon:m[3],tone:'danger',title:m[0],body:[m[1]],warn:x?x.name+' está aberto. O progresso não salvo pode ser perdido.':null,focus:'cancel',
    buttons:[B('cancel','Cancelar',function(){pop()},{back:true}),B('ok',m[2],function(){
      replaceTop(dlg({name:'Executando energia',icon:m[3],title:m[4],body:[m[5]],spin:'Aguardando o Windows',buttons:[B('back','Voltar',function(){pop()},{back:true})]})) },{kind:'danger',icon:m[3]})]});
}
function exitDlg(){
  var x=S.session&&gameById(S.session.id);
  var reopen=x?'Para reabrir, use o atalho do SofaLauncher ou reinicie o PC depois de sair do jogo.':'Para reabrir, use o atalho do SofaLauncher (com teclado ou mouse) ou reinicie o PC.';
  return dlg({name:'Fechar o launcher',icon:'exit',title:'Fechar o launcher?',body:[x?'Só o SofaLauncher fecha.':'O SofaLauncher fecha e o Windows mostra a área de trabalho.',reopen],warn:x?x.name+' continuará aberto. Nenhum progresso se perde.':null,warnTone:'amber',focus:'cancel',
    buttons:[B('cancel','Cancelar',function(){pop()},{back:true}),B('ok','Fechar o launcher',function(){
      resetStack(); push({name:'Área de trabalho',kind:'screen',focus:'again',noBack:true,rows:function(){return [['again']]},
        build:function(){ return h('div.layer.bye',{},h('div',{},h('div',{text:'Simulação: o SofaLauncher fechou e o Windows mostra a área de trabalho.'+(x?' '+x.name+' segue aberto.':'')}),btn('again','Abrir o SofaLauncher','primary'))) },
        on:{Confirm:function(){ apply(C) }}}) },{icon:'exit'})]});
}


/* ---------- configurações ---------- */
function monitors(){
  if(C.monitors==='um')return {list:[{id:'m1',n:'Monitor Dell U2720Q',res:'2560 × 1440',main:true}],sel:'m1',missing:false};
  var l=[{id:'m1',n:'Monitor Dell U2720Q',res:'2560 × 1440',main:true},{id:'m2',n:'TV Sala (HDMI)',res:'3840 × 2160',main:false}];
  if(C.monitors==='ausente')return {list:[l[0]],sel:'m1',missing:true};
  return {list:l,sel:S.monitor||'m2',missing:false};
}
function settings(o){
  var L={name:'Configurações',kind:'screen',focus:'t:'+(o.tab||'tela'),tab:o.tab||'tela',zone:'tabs',paneIds:[]};
  var TABS=[['tela','Tela','monitor'],['biblioteca','Biblioteca','grid'],['geral','Geral','sliders'],['ocultos','Itens ocultos','eyeoff']];
  function pane(){
    var p=h('div.pane',{role:'region','aria-label':L.tab}); var ids=[];
    function mi(id,icon,label,small,right,disabled){ ids.push(id); return h('div.mi.focusable',{'data-fid':id,role:'button','aria-disabled':disabled?'true':null},ic(icon),h('div.grow',{},h('span.clamp1',{text:label,style:'display:block'}),small?h('span.small',{text:small}):null),right||null) }
    if(L.tab==='tela'){
      var M=monitors(); S.monitor=M.sel;
      p.append(h('h3',{text:'Monitor do launcher'}),h('div.lead',{text:'O SofaLauncher abre em tela cheia no monitor escolhido.'}));
      if(M.missing)p.append(h('div.callout',{role:'status'},ic('alert'),h('span',{text:'O monitor escolhido não está conectado. O SofaLauncher está no monitor principal e volta ao monitor escolhido quando ele reconectar.'})));
      p.append(h('div.list',{'data-scroll':''},M.list.map(function(m){ return mi('p:'+m.id,'monitor',m.n,(m.main?'Principal':null),m.id===M.sel?h('span.tagsm',{},h('span',{text:'Em uso '}),ic('check','s')):null) })));
      if(M.list.length===1)p.append(h('div.sub',{text:'Só há um monitor conectado.'}));
    } else if(L.tab==='biblioteca'){
      var n=S.games.length,cnt={};S.games.forEach(function(x){cnt[x.origin]=(cnt[x.origin]||0)+1});
      var sum=n+' jogos'+(n?': '+Object.keys(cnt).map(function(k){return cnt[k]+' '+k}).join(', '):'');
      p.append(h('h3',{text:'Biblioteca'}),h('div.lead',{text:sum}),h('div.list',{'data-scroll':''},
        mi('p:refresh','refresh','Atualizar biblioteca','Procura jogos novos na Steam, na Epic e no GOG.',S.refreshing?h('span.tagsm',{},h('div.spinner',{style:'width:calc(32*var(--u));height:calc(32*var(--u));display:inline-block;vertical-align:middle'})):null),
        mi('p:add','plus','Adicionar jogo','Escolha um arquivo .exe em um disco deste PC.',ic('chevron'))));
    } else if(L.tab==='geral'){
      p.append(h('h3',{text:'Geral'}),h('div.list',{'data-scroll':''},
        mi('p:auto','power','Iniciar com o Windows','Abre o SofaLauncher quando você entra no Windows. Vale só para o seu usuário.',h('span.sw'+(S.autostart?'.on':''),{role:'switch','aria-checked':String(S.autostart)})),
        L.autoErr?h('div.errline',{role:'alert'},ic('alert','s'),h('span',{text:'Não foi possível alterar a inicialização com o Windows. Tente de novo.'})):null,
        h('div.callout',{style:'margin-top:var(--space-2)'},ic('gamepad'),h('span',{text:'Para ligar o PC e jogar sem teclado, configure o logon automático ou o Windows Hello no Windows. O SofaLauncher começa na área de trabalho.'})),
        h('div.callout',{},ic('shield'),h('span',{text:'O SofaLauncher nunca roda como administrador. Jogos que exigem administrador pedem confirmação no Windows, com teclado ou mouse.'}))),
        h('div.sub',{text:'SofaLauncher 0.2.0 · Fontes Sora e Figtree sob licença OFL'}));
    } else {
      var hd=S.hidden;
      p.append(h('h3',{text:'Itens ocultos'}));
      if(!hd.length){ p.append(h('div.empty-s',{},ic('eyeoff','xl'),h('b',{text:'Nenhum item oculto'}),h('span',{text:'Os jogos que você ocultar aparecem aqui.'}))) }
      else p.append(h('div.lead',{text:'Escolha um item para voltar à biblioteca.'}),h('div.list',{'data-scroll':''},hd.map(function(x){ return mi('p:h:'+x.id,'eyeoff',x.name,null,h('span.tagsm',{text:x.origin})) })));
    }
    L.paneIds=ids; return p;
  }
  L.rows=function(){ return L.zone==='tabs'?TABS.map(function(t){return ['t:'+t[0]]}):L.paneIds.map(function(i){return [i]}) };
  L.build=function(){
    var tabs=h('div.tabs',{role:'tablist'},TABS.map(function(t){
      return h('div.tab.focusable'+(L.tab===t[0]?'.sel':''),{'data-fid':'t:'+t[0],role:'tab','aria-selected':String(L.tab===t[0])},ic(t[2]),h('span',{text:t[1]}),t[0]==='ocultos'&&S.hidden.length?h('span.cnt',{text:String(S.hidden.length)}):null) }));
    return h('div.layer.settings.home',{},h('div.spotlight'),h('div.safe',{},header(),h('div.title',{text:'Configurações',style:'margin-top:var(--space-2)'}),
      h('div.cols',{},tabs,pane()),hints(L.zone==='tabs'?[['A','Entrar'],['B','Voltar']]:[['A','Escolher'],['B','Abas']])));
  };
  L.mounted=function(){ syncChips() };
  function swapPane(){ var old=L.el.querySelector('.pane'); var np=pane(); old.replaceWith(np);
    L.el.querySelectorAll('.tab').forEach(function(t){ var on=t.getAttribute('data-fid')==='t:'+L.tab; t.classList.toggle('sel',on); t.setAttribute('aria-selected',String(on)) }) }
  L.onFocus=function(fid){ if(fid&&fid.indexOf('t:')===0&&fid!=='t:'+L.tab){ L.tab=fid.slice(2); swapPane(); var f=L.el.querySelector('[data-fid="'+fid+'"]'); } };
  L.refreshUi=function(){ if(L.tab==='biblioteca'&&L.zone==='pane'){ swapPane(); applyFocus(true) } };
  L.onResume=function(l){ var f=l.focus; rerender(l); };
  L.on={
    dir:function(d,l){
      if(l.zone==='tabs'&&d==='Right'){ if(l.paneIds.length){ l.zone='pane'; l.focus=l.paneIds[0]; l.col=0; rerenderHints(l); applyFocus(true) } return true }
      if(l.zone==='pane'&&d==='Left'){ l.zone='tabs'; l.focus='t:'+l.tab; rerenderHints(l); applyFocus(true); return true }
      return false;
    },
    Back:function(l){ if(l.zone==='pane'){ l.zone='tabs'; l.focus='t:'+l.tab; rerenderHints(l); applyFocus(true) } else pop() },
    Confirm:function(id,l){
      if(l.zone==='tabs'){ if(l.paneIds.length){ l.zone='pane'; l.focus=l.paneIds[0]; rerenderHints(l); applyFocus(true) } return }
      if(id.indexOf('p:m')===0){ var m=monitors().list.filter(function(x){return 'p:'+x.id===id})[0]; S.monitor=m.id; C.monitors=C.monitors==='ausente'?'dois':C.monitors; swapPane(); applyFocus(true); toast('SofaLauncher movido para '+m.n+'.') }
      else if(id==='p:refresh')doRefresh();
      else if(id==='p:add')openFolder('');
      else if(id==='p:auto'){ if(C.autostartFail==='yes'){ l.autoErr=true } else { l.autoErr=false; S.autostart=!S.autostart } swapPane(); applyFocus(true) }
      else if(id.indexOf('p:h:')===0){ var x=S.hidden.filter(function(y){return 'p:h:'+y.id===id})[0]; push(restoreDlg(x)) }
    }
  };
  function rerenderHints(l){ var hb=l.el.querySelector('.hints'); var nb=hints(l.zone==='tabs'?[['A','Entrar'],['B','Voltar']]:[['A','Escolher'],['B','Abas']]); hb.replaceWith(nb) }
  return L;
}
function restoreDlg(x){
  return dlg({name:'Restaurar item',icon:'undo',tone:'ok',title:'Restaurar '+x.name+'?',body:['O jogo volta para a biblioteca com o nome e a data de último jogo que tinha.'],focus:'ok',
    buttons:[B('cancel','Cancelar',function(){pop()},{back:true}),B('ok','Restaurar',function(){ S.hidden=S.hidden.filter(function(y){return y!==x}); S.games.push(x); pop(); var T=top(); if(T.name==='Configurações'){ rerender(T); T.zone='pane'; T.focus=T.paneIds[0]||'t:ocultos'; if(!T.paneIds.length){T.zone='tabs'} applyFocus(true) } toast(x.name+' voltou para a biblioteca.') },{kind:'primary',icon:'undo'})]});
}

/* ---------- navegador de pastas ---------- */
function D(n,k,p){ return {n:n,k:k,p:p} }
var FS={
  '':[D('Disco local (C:)','Drive','C:'),D('Jogos (D:)','Drive','D:'),D('Pen drive (E:)','Drive','E:')],
  'C:':[D('Jogos','Dir','C:\\Jogos'),D('Program Files','Dir','C:\\Program Files'),D('Users','Dir','C:\\Users'),D('instalador-antigo.exe','Exe')],
  'C:\\Jogos':[D('Aventura','Dir','C:\\Jogos\\Aventura'),D('Aventura.exe','Exe'),D('Bruma.exe','Exe'),D('leiame.exe','Exe')],
  'C:\\Jogos\\Aventura':[D('Aventura.exe','Exe'),D('Config.exe','Exe')],
  'C:\\Program Files':[D('Steam','Dir','C:\\Program Files\\Steam')],
  'C:\\Program Files\\Steam':[D('steam.exe','Exe')],
  'D:':[D('Jogos','Dir','D:\\Jogos'),D('Coleção','Dir','D:\\Coleção'),D('setup-antigo.exe','Exe')],
  'D:\\Jogos':[D('Aventura.exe','Exe'),D('Bruma.exe','Exe'),D('leiame.exe','Exe')],
  'E:':[]
};
(function(){ var a=[]; for(var i=0;i<60;i++)a.push(i%5===0?D('Pasta '+(i/5+1),'Dir','D:\\Coleção\\P'+i):D('Jogo '+W1[i%12]+' '+W2[(i*5)%12]+'.exe','Exe')); FS['D:\\Coleção']=a })();
function parentOf(p){ if(!p)return null; var i=p.lastIndexOf('\\'); return i<0?'':p.slice(0,i) }
function openFolder(path){ push(folder(path)) }
function folder(path0){
  var L={name:'Adicionar jogo',kind:'screen',path:path0||'',focus:null,err:null};
  function entries(){ return (FS[L.path]||[]).slice().sort(function(a,b){ var A=a.k==='Exe'?1:0,B2=b.k==='Exe'?1:0; return A-B2||coll.compare(a.n,b.n) }) }
  L.rows=function(){ if(L.err||!entries().length)return [['fback']]; return entries().map(function(e,i){return ['f:'+i]}) };
  function setPath(p,focusName){ L.path=p; L.err=null; rerender(L); var es=entries(),i=0; es.forEach(function(e,k){ if(focusName!=null&&(e.p===focusName||e.n===focusName))i=k }); L.focus=es.length?'f:'+i:'fback'; L.col=0; applyFocus(true) }
  L.focus=entries().length?'f:0':'fback';
  L.build=function(){
    var es=entries(),crumb=h('div.crumb',{},ic(L.path===''?'drive':'folder'),h('span',{text:L.path===''?'Este computador':L.path}));
    var body;
    if(L.err)body=h('div.empty-s',{role:'alert'},ic('alert','xl'),h('b',{text:L.err==='AccessDenied'?'Sem permissão para abrir esta pasta':'Não encontramos esta pasta'}),h('span',{text:L.err==='AccessDenied'?'O Windows não deixa o SofaLauncher ver o que há aqui.':'Ela pode ter sido removida, ou o disco foi desconectado.'}),btn('fback','Voltar','','chevron'));
    else if(!es.length)body=h('div.empty-s',{},ic('folder','xl'),h('b',{text:'Nenhuma pasta ou .exe aqui'}),h('span',{text:'Esta pasta não tem pastas nem arquivos .exe.'}),btn('fback','Voltar'));
    else body=h('div.list',{'data-scroll':''},es.map(function(e,i){
      return h('div.mi.focusable',{'data-fid':'f:'+i,role:'button'},ic(e.k==='Drive'?'drive':e.k==='Dir'?'folder':'exe'),h('div.grow',{},h('span.clamp1',{text:e.n,style:'display:block'})),e.k==='Exe'?h('span.tagsm',{text:'Adicionar'}):ic('chevron')) }));
    return h('div.layer.home.fb',{},h('div.spotlight'),h('div.safe',{},header(),h('div.title',{text:'Adicionar jogo',style:'margin-top:var(--space-2)'}),h('div.sub',{text:'Escolha o arquivo .exe do jogo. Só aparecem discos deste PC.'}),crumb,body,
      hints([['A',es.length&&!L.err?'Abrir ou escolher':'Abrir'],['B',L.path===''?'Voltar':'Pasta anterior']])));
  };
  L.mounted=function(){ syncChips(); var c=L.el.querySelector('[data-fid="'+L.focus+'"]'); if(c)keepVisible(c,true) };
  L.on={
    Confirm:function(id,l){
      if(!id)return; if(id==='fback')return l.on.Back(l); var e=entries()[+id.slice(2)];
      if(e.k==='Exe')return push(addConfirm(e,L));
      if(e.p==='C:\\Users'){ L.err='AccessDenied'; L.focus='fback'; rerender(L); return }
      setPath(e.p);
    },
    Back:function(l){ if(l.err){ l.err=null; setPath(l.path); return } if(l.path===''){pop();return} var cur=l.path,p=parentOf(l.path); setPath(p,cur) }
  };
  return L;
}
function addConfirm(e,fl){
  var name=e.n.replace(/\.exe$/i,''), full=(fl.path?fl.path+'\\':'')+e.n;
  return dlg({name:'Adicionar jogo?',icon:'plus',tone:'ok',title:'Adicionar este jogo?',body:['O jogo entra na biblioteca como '+name+'. Dá para renomear depois.'],path:full,focus:'ok',
    buttons:[B('cancel','Voltar',function(){pop()},{back:true}),B('ok','Adicionar',function(){
      if(e.n==='Bruma.exe'){ replaceTop(dlg({name:'Duplicado',icon:'alert',title:'Este jogo já está na biblioteca',body:['Não criamos uma segunda cópia.'],focus:'back',buttons:[B('back','Voltar',function(){pop()},{back:true})]})); return }
      if(e.n==='leiame.exe'){ replaceTop(dlg({name:'Executável inválido',icon:'alert',tone:'danger',title:'Este arquivo não é um executável válido',body:['O arquivo tem a extensão .exe, mas o Windows não o reconhece como programa. Escolha outro arquivo.'],focus:'back',buttons:[B('back','Voltar',function(){pop()},{back:true})]})); return }
      S.games.push(G('manual:'+full,name,'Manual',{cover:'icon'})); pop(); pop(); toast(name+' adicionado.');
    },{kind:'primary',icon:'plus'})]});
}

/* ---------- folha de componentes ---------- */
function sheet(group){
  S.sheet=true; var x=gameById('steam:1245')||base()[0],y=base()[6],z=base()[4];
  var L={name:'Componentes',kind:'screen',inert:true,focus:null,rows:function(){return []}};
  function lab(t){ return h('div.lab',{text:t}) }
  function stack(el,t){ return h('div',{},el,lab(t)) }
  var content;
  if(group==='foco'){
    var cNorm=h('div.card',{},poster(x,{})),cFoc=h('div.card',{},poster(x,{}));cFoc.firstChild.setAttribute('data-focused','true');
    var cDim=h('div.card',{},poster(x,{}));cDim.firstChild.style.opacity='.72';
    var btnA=btn('a','Cancelar'),btnB=btn('b','Cancelar');btnB.setAttribute('data-focused','true');var btnC=btn('c','Desligar','danger','power');
    var mi1=h('div.mi.focusable',{},ic('restart'),h('span',{text:'Reiniciar'})),mi2=h('div.mi.focusable',{},ic('restart'),h('span',{text:'Reiniciar'}));mi2.setAttribute('data-focused','true');
    content=h('div',{style:'display:grid;gap:var(--space-6)'},h('h1',{text:'Foco e estados'}),
      h('div.grp.box',{},stack(cNorm,'Card em repouso'),stack(cFoc,'Card em foco: anel duplo, halo e escala 1,06'),stack(cDim,'Os demais cards: 72% de opacidade'),
        h('div',{style:'display:grid;gap:var(--space-2);width:calc(560*var(--u))'},mi1,mi2,lab('Item de menu: em repouso, em foco (preenchimento âmbar)'))),
      h('div.grp.box',{},stack(btnA,'Botão em repouso'),stack(btnB,'Botão em foco'),stack(btnC,'Botão destrutivo em repouso'),
        stack(h('div.mi.focusable',{'aria-disabled':'true',style:'width:calc(560*var(--u))'},ic('moon'),h('div.grow',{},h('span',{text:'Suspender'}),h('span.small',{text:'Suspensão indisponível neste PC'}))),'Desabilitado: 55% de opacidade, pulado pelo foco e com o motivo escrito')));
  } else if(group==='capas'){
    var run={id:x.id};S.session=run;
    content=h('div',{style:'display:grid;gap:var(--space-4)'},h('h1',{text:'Capas, origem e nomes'}),
      h('div.grp.box.tight',{},stack(h('div.card',{},poster(x,{})),'Capa da loja com selo de origem'),stack(h('div.card',{},poster(x,{run:true})),'Selo Em andamento (CA-36)'),stack(h('div.card',{},poster(z,{})),'Ícone do executável (jogo manual)'),
        stack(h('div.card',{},poster(y,{})),'Sem capa: gradiente e nome em Sora 700 (CA-24)'),
        stack(h('div.card',{},poster(G('x','Zero Gravidade Deluxe Edition: Complete Collection of All Expansions','Steam',{cover:'none'}),{})),'Placeholder com nome longo: 5 linhas e corta'),
        stack(h('div.card',{},h('div.poster.sk')),'Carregando')),
      h('div.grp.box',{},stack(h('div.card',{style:'width:calc(220*var(--u))'},h('div.nm.clamp1',{text:'Zero Gravidade Deluxe Edition: Complete Collection'})),'Nome no card: 1 linha com reticências (E-13). O nome inteiro aparece no herói'),
        stack(h('div.name-big.clamp2',{style:'width:calc(900*var(--u))',text:'Zero Gravidade Deluxe Edition: Complete Collection of All Expansions and Original Soundtrack Bonus'}),'Herói: 2 linhas e reticências'),
        stack(h('div.card',{},h('div.nm',{text:'<img src=x onerror=alert(1)>'})),'Texto de fora aparece literal (SEC-07)')));
    S.session=null;
  } else if(group==='avisos'){
    content=h('div',{style:'display:grid;gap:var(--space-4)'},h('h1',{text:'Avisos e indicadores'}),
      h('div',{style:'display:grid;gap:var(--space-3);justify-items:start'},
        h('div.banner',{style:'animation:none'},ic('gamepad'),h('span',{text:TXT.ctrlNone})),
        h('div.banner.lost',{style:'animation:none'},ic('gamepad'),h('span',{text:TXT.ctrlLost})),
        h('div.banner.ok',{style:'animation:none'},ic('check'),h('span',{text:TXT.ctrlBack}))),
      h('div.grp',{},h('div.vol',{style:'animation:none'},ic('vol'),h('div.slider',{},h('i',{style:'width:45%'})),h('span.lbl',{text:'45'})),
        h('div.vol.muted',{style:'animation:none'},ic('mute'),h('div.slider.m',{},h('i',{style:'width:45%'})),h('span.lbl',{text:'Mudo'})),
        h('div.vol',{style:'animation:none'},ic('mute'),h('span.lbl',{text:'Sem áudio',style:'text-align:left'}))),
      h('div.grp',{},h('span.pill.glass',{text:'Steam'}),h('span.pill.glass',{text:'Epic'}),h('span.pill.glass',{text:'GOG'}),h('span.pill.glass',{text:'Manual'}),h('span.pill.amber',{text:'Em andamento'})),
      h('div.toast',{style:'animation:none'},ic('check'),h('span',{text:'Nome salvo.'})));
  } else if(group==='dialogos'){
    var fake=dlg({icon:'power',tone:'danger',title:'Desligar o PC?',body:['O Windows desliga do jeito normal. Se outro app tiver trabalho não salvo, o Windows avisa.'],warn:'Farol do Fim está aberto. O progresso não salvo pode ser perdido.',focus:'cancel',
      buttons:[B('cancel','Cancelar'),B('ok','Desligar',null,{kind:'danger',icon:'power'})]});
    var d=fake.build().querySelector('.dialog'); d.querySelector('[data-fid=cancel]').setAttribute('data-focused','true');
    content=h('div',{style:'display:grid;gap:var(--space-4)'},h('h1',{text:'Diálogo de confirmação'}),h('div',{},d,lab('Cancelar em foco por padrão (RN-14). B cancela. Ação destrutiva em vermelho.')));
  } else {
    content=h('div',{style:'display:grid;gap:var(--space-3)'},h('h1',{text:'Escala de 10 pés'}),
      [['Herói','--text-hero','96','800','Sora'],['Destaque','--text-display','72','800','Sora'],['Título de tela','--text-heading','48','700','Sora'],['Título de card','--text-card-title','40','700','Sora'],['Rótulo','--text-label','32','600','Figtree'],['Corpo','--text-body','28','400','Figtree'],['Legenda (mínimo)','--text-caption','24','600','Figtree']].map(function(t){
        return h('div',{style:'display:flex;align-items:baseline;gap:var(--space-4)'},h('span',{style:'width:calc(320*var(--u));font:600 var(--text-caption)/1 var(--font-text);color:var(--brand-text-muted)',text:t[0]+' · '+t[2]+' px'}),h('span',{style:'font:'+t[3]+' var('+t[1]+')/1.1 '+(t[4]==='Sora'?'var(--font-display)':'var(--font-text)'),text:t[4]==='Sora'?'Farol do Fim':'Segure View + Menu para voltar'})) }),
      h('div.sub',{text:'Valores a 1080p. Em 4K, o dobro (a escala usa 100vh ÷ 1080). Texto nunca abaixo de 24 px.'}));
  }
  L.build=function(){ return h('div.layer',{},h('div.spotlight'),h('div.sheet',{},content)) };
  return L;
}

/* ---------- cenários ---------- */
function gm(id){ return gameById(id) }
function sessionStack(g,fallback,elev){
  startRunning(g,elev); var b=home({focusGame:g.id}); resetStack(); push(b); push(ingame()); push(sessionPanel(fallback));
}
var SCN={
  boot:function(){ push(boot()); if(C.autoflow==='yes')timer(function(){ toHome() },2400) },
  home:function(){ push(home()) },
  andamento:function(){ startRunning(gm('steam:1245')); push(home({focusGame:'steam:1245'})) },
  all:function(){ push(home()); push(allGames('r:'+(recents()[0]||alpha()[0]).id)) },
  options:function(){ push(home()); push(options(recents()[0]||alpha()[0])) },
  'options-renamed':function(){ push(home({focusGame:'steam:3301'})); push(options(gm('steam:3301'))) },
  rename:function(){ push(home()); var x=recents()[0]||alpha()[0]; push(options(x)); push(rename(x)) },
  'rename-empty':function(){ push(home()); var x=recents()[0]||alpha()[0]; push(options(x)); var r=rename(x); r.value=''; r.error='O nome não pode ficar vazio.'; push(r) },
  hide:function(){ push(home()); var x=recents()[0]||alpha()[0]; push(options(x)); push(hideConfirm(x)) },
  launching:function(){ push(home()); startLaunch(gm('steam:1245')||S.games[0],true) },
  notfound:function(){ push(home({focusGame:'manual:bruma'})); push(errDlg('GameFileMissing',gm('manual:bruma'))) },
  network:function(){ push(home({focusGame:'manual:minerio'})); push(errDlg('PathNetwork',gm('manual:minerio'))) },
  unconfirmed:function(){ push(home({focusGame:'epic:parada'})); push(errDlg('NotConfirmed',gm('epic:parada'))) },
  storemissing:function(){ push(home()); push(errDlg('StoreNotInstalled',gm('steam:1245'))) },
  admin:function(){ push(home({focusGame:'gog:chuva'})); S.session={id:'gog:chuva',state:'needsElevation'}; push(adminDlg(gm('gog:chuva'))) },
  'admin-wait':function(){ push(home({focusGame:'gog:chuva'})); S.session={id:'gog:chuva',state:'needsElevation'}; push(adminWait(gm('gog:chuva'))) },
  'admin-denied':function(){ push(home({focusGame:'gog:chuva'})); push(errDlg('UacDenied',gm('gog:chuva'))) },
  switch:function(){ startRunning(gm('steam:1245')); push(home({focusGame:'epic:turbo'})); push(switchDlg(gm('epic:turbo'))) },
  ingame:function(){ startRunning(gm('steam:1245')); push(home({focusGame:'steam:1245'})); push(ingame()) },
  session:function(){ sessionStack(gm('steam:1245'),false) },
  'session-fallback':function(){ sessionStack(gm('steam:1245'),true) },
  'session-admin':function(){ sessionStack(gm('gog:chuva'),false,true) },
  'close-confirm':function(){ sessionStack(gm('steam:1245'),false); push(closeConfirm(gm('steam:1245'))) },
  closing:function(){ sessionStack(gm('steam:1245'),false); S.session.state='closing'; push(dlg({name:'Fechando',cover:gm('steam:1245'),title:'Fechando Farol do Fim…',body:['Esperando o jogo fechar. Pode levar alguns segundos.'],spin:'Fechando',buttons:[B('lib','Ir para a biblioteca',function(){toHome('steam:1245')},{back:true})]})) },
  force:function(){ sessionStack(gm('steam:1245'),false); push(forceOffer(gm('steam:1245'))) },
  'force-denied':function(){ sessionStack(gm('gog:chuva'),false,true); push(errDlg('ForceDenied',gm('gog:chuva'))) },
  'return-denied':function(){ startRunning(gm('steam:1245')); push(home({focusGame:'steam:1245'})); push(errDlg('ForegroundDenied',gm('steam:1245'))) },
  quick:function(){ push(home()); push(quick()) },
  'quick-session':function(){ sessionStack(gm('steam:1245'),false); push(quick({fromSession:true})) },
  'power-shutdown':function(){ push(home()); push(quick()); push(power('shutdown')) },
  'power-restart':function(){ push(home()); push(quick()); push(power('restart')) },
  'power-suspend':function(){ push(home()); push(quick()); push(power('suspend')) },
  'power-session':function(){ startRunning(gm('steam:1245')); push(home({focusGame:'steam:1245'})); push(quick()); push(power('shutdown')) },
  exit:function(){ push(home()); push(quick()); push(exitDlg()) },
  'exit-session':function(){ startRunning(gm('steam:1245')); push(home({focusGame:'steam:1245'})); push(quick()); push(exitDlg()) },
  'settings-tela':function(){ push(home()); push(settings({tab:'tela'})) },
  'settings-biblioteca':function(){ push(home()); push(settings({tab:'biblioteca'})) },
  'settings-geral':function(){ push(home()); push(settings({tab:'geral'})) },
  'settings-ocultos':function(){ push(home()); push(settings({tab:'ocultos'})) },
  folder:function(){ push(home()); push(settings({tab:'biblioteca'})); push(folder('')) },
  'folder-dir':function(){ push(home()); push(folder('C:\\Jogos')) },
  'folder-empty':function(){ push(home()); push(folder('E:')) },
  'folder-denied':function(){ push(home()); var f=folder('C:'); push(f); f.focus='f:2'; applyFocus(true); f.on.Confirm('f:2') },
  'folder-long':function(){ push(home()); push(folder('D:\\Coleção')) },
  'folder-add':function(){ push(home()); var f=folder('C:\\Jogos'); push(f); push(addConfirm(FS['C:\\Jogos'][1],f)) },
  'folder-dup':function(){ push(home()); var f=folder('C:\\Jogos'); push(f); push(addConfirm(FS['C:\\Jogos'][2],f)); top().buttons; top().on.Confirm('ok') },
  'folder-invalid':function(){ push(home()); var f=folder('C:\\Jogos'); push(f); push(addConfirm(FS['C:\\Jogos'][3],f)); top().on.Confirm('ok') },
  'ctrl-lost':function(){ push(home()); setCtrl('lost') },
  'ctrl-back':function(){ push(home()); setCtrl('back'); clearTimeout(S.bannerT) },
  components:function(){ push(sheet(C.group)) }
};
function reset(){
  clearTimers(); S.sheet=false; S.games=lib(C.lib); S.hidden=hiddenList(C.hidden); S.session=null; S.volume=40; S.muted=false; S.audio=C.audio;
  S.autostart=C.autostart==='on'; S.monitor=null; S.refreshing=false; holdT=null; kdown={};
  $('#hud').replaceChildren(); $('#toast').replaceChildren(); resetStack(); setCtrl(C.controller==='none'?'none':C.controller==='lost'?'lost':'ok');
}
function apply(cfg){
  C=Object.assign({},DEF,cfg||{}); document.documentElement.classList.toggle('reduced',C.reduced==='yes');
  $('#guide').style.display=C.guide==='yes'?'block':'none';
  reset(); try{ (SCN[C.scenario]||SCN.home)() }catch(err){ resetStack(); S.session=null; push(home()) } syncChips();
}
window.addEventListener('message',function(ev){
  var d=ev.data; if(!d||typeof d!=='object')return;
  if(d.t==='cfg')apply(d.cfg);
  else if(d.t==='intent')intent(d.i);
  else if(d.t==='held')held(d.view,d.menu);
  else if(d.t==='key')onKey({key:d.key,repeat:false,target:document.body,preventDefault:function(){}},d.type||'down');
  else if(d.t==='ctrl')setCtrl(d.v);
  else if(d.t==='uac'){ var L=top(); L&&L.uac&&L.uac(d.v) }
  else if(d.t==='guide'){ C.guide=d.v; $('#guide').style.display=d.v==='yes'?'block':'none' }
});
window.SofaTV={apply:apply,intent:intent};
document.addEventListener('DOMContentLoaded',function(){
  var cfg={},q=new URLSearchParams(location.search); q.forEach(function(v,k){cfg[k]=v});
  if(window.top===window){
    if(!cfg.scenario){cfg.scenario='boot';cfg.autoflow='yes'}
    if(window.Pad)Pad.start(function(i){intent(i)},function(v,m){held(v,m)});
    addEventListener('keydown',function(e){ if((e.key==='f'||e.key==='F')&&!(e.target&&e.target.tagName==='INPUT')){ if(!document.fullscreenElement)document.documentElement.requestFullscreen&&document.documentElement.requestFullscreen(); else document.exitFullscreen() } });
  }
  apply(cfg);
});
})();
