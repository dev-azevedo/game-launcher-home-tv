/* Playground do mockup: controles, preview ao vivo (iframe escalado) e prompt copiável.
   Entrada: window.PG = { title, ca[], scenarios[[id,label]], options[{key,label,values[[v,label]]}], actions[{label,msg}], notes[] } */
(function(){
'use strict';
var PG=window.PG,$=function(s,r){return (r||document).querySelector(s)};
function h(sel,attrs){ var m=sel.match(/^([a-z0-9]*)((?:\.[\w-]+)*)$/i),e=document.createElement(m[1]||'div'); (m[2].match(/\.[\w-]+/g)||[]).forEach(function(c){e.classList.add(c.slice(1))});
  for(var k in (attrs||{})){ var v=attrs[k]; if(v==null)continue; if(k==='text')e.textContent=v; else e.setAttribute(k,v) }
  for(var i=2;i<arguments.length;i++){ var c=arguments[i]; if(c==null)continue; if(Array.isArray(c))c.forEach(function(x){e.append(x)}); else e.append(c) } return e }
var st={res:'1080',reduced:'no',guide:'no',scenario:PG.scenarios[0][0]},DEF={};
st.scenario=PG.scenarios[0][0]; DEF.scenario=st.scenario;
PG.options.forEach(function(o){ st[o.key]=o.values[0][0]; DEF[o.key]=o.values[0][0] });
(PG.presets||[]).length&&0;
var qs=new URLSearchParams(location.search); qs.forEach(function(v,k){ if(k in st)st[k]=v });

var frame=h('iframe',{title:'Prévia da tela de TV',allow:'gamepad',src:'about:blank'}),wrap=h('div.stagewrap',{}),hintEl=h('div.cover-hint',{text:'Clique na tela para usar o teclado. O controle Xbox funciona direto.'});
wrap.className='stagewrap'; wrap.append(frame,hintEl);
var ready=false;
function cfg(){ var c={scenario:st.scenario,reduced:st.reduced,guide:st.guide}; PG.options.forEach(function(o){c[o.key]=st[o.key]}); if(PG.extra)Object.assign(c,PG.extra); return c }
function post(m){ try{frame.contentWindow.postMessage(m,'*')}catch(e){} }
function qsOf(c){ return new URLSearchParams(c).toString() }
function layout(){
  var W=st.res==='4k'?3840:1920,H=st.res==='4k'?2160:1080,w=wrap.clientWidth,k=w/W;
  frame.style.width=W+'px';frame.style.height=H+'px';frame.style.transform='scale('+k+')';wrap.style.height=(H*k)+'px';
}
function apply(){ layout(); updatePrompt(); if(ready)post({t:'cfg',cfg:cfg()}) }
frame.addEventListener('load',function(){ ready=true; layout(); try{frame.contentWindow.focus()}catch(e){} });
frame.src='tv.html?'+qsOf(cfg());
new ResizeObserver(layout).observe(wrap);

/* painel */
function seg(key,values,col,get,set){
  var d=h('div.seg'+(col?'.col':'')); d.className='seg'+(col?' col':'');
  values.forEach(function(v){ var b=h('button',{type:'button','aria-pressed':String(get()===v[0]),text:v[1]}); b.addEventListener('click',function(){ set(v[0]); d.querySelectorAll('button').forEach(function(x,i){x.setAttribute('aria-pressed',String(values[i][0]===get()))}); apply(); try{frame.contentWindow.focus()}catch(e){} }); d.append(b) });
  return d;
}
function grp(t,el){ var g=h('div'); g.className='grp'; g.append(h('h2',{text:t}),el); return g }
var panel=h('aside'); panel.className='panel';
panel.append(h('a',{href:'index.html',class:'back',text:'← Índice dos mockups'}),h('h1',{text:PG.title}));
panel.append(grp('Estado da tela',seg('scenario',PG.scenarios,true,function(){return st.scenario},function(v){st.scenario=v})));
PG.options.forEach(function(o){ panel.append(grp(o.label,seg(o.key,o.values,false,function(){return st[o.key]},function(v){st[o.key]=v}))) });
panel.append(grp('Resolução da TV',seg('res',[['1080','1080p'],['4k','4K']],false,function(){return st.res},function(v){st.res=v})));
var chkA=h('label'); chkA.className='chk'; var ia=h('input',{type:'checkbox'}); chkA.append(ia,'Reduzir movimento'); ia.addEventListener('change',function(){st.reduced=ia.checked?'yes':'no';apply()});
var chkB=h('label'); chkB.className='chk'; var ib=h('input',{type:'checkbox'}); chkB.append(ib,'Mostrar área segura de 5%'); ib.addEventListener('change',function(){st.guide=ib.checked?'yes':'no';apply()});
panel.append(grp('Ver',h('div',{},chkA,chkB)));
if(PG.actions&&PG.actions.length){ var ac=h('div'); ac.className='acts'; PG.actions.forEach(function(a){ var b=h('button',{type:'button',text:a.label}); b.className='act'; b.addEventListener('click',function(){ post(a.msg); try{frame.contentWindow.focus()}catch(e){} }); ac.append(b) }); panel.append(grp('Simular',ac)) }

/* principal */
var top=h('div'); top.className='top'; top.append(h('span',{class:'ttl',text:PG.title})); PG.ca.forEach(function(c){top.append(h('span',{class:'ca',text:c}))}); top.append(h('span',{class:'spacer'}));
var open=h('a',{class:'open',target:'_blank',rel:'noopener',text:'Abrir em tela cheia (tecla F)'}); top.append(open);
var read=h('div'); read.className='read'; var rL=h('span',{},'Camada: ',h('b',{text:'-'})),rF=h('span',{},'Em foco: ',h('b',{text:'-'})),rC=h('span',{class:'badge',text:'-'}),rP=h('span',{text:'Controle: aperte um botão para ativar'}); read.append(rL,rF,rC,rP);
var keys=h('div'); keys.className='keys';
[['<kbd>←↑↓→</kbd> D-pad'],['<kbd>Enter</kbd> A (confirmar)'],['<kbd>Esc</kbd> B (voltar)'],['<kbd>M</kbd> ou <kbd>F1</kbd> Menu'],['<kbd>V</kbd> ou <kbd>F2</kbd> View (opções)'],['<kbd>PgUp</kbd> <kbd>PgDn</kbd> LB / RB'],['<kbd>[</kbd> <kbd>]</kbd> LT / RT (volume)'],['Segurar <kbd>V</kbd>+<kbd>M</kbd> por 1,5 s: combinação de retorno (só no jogo simulado)']].forEach(function(k){ var d=h('div'); d.innerHTML=k[0]; keys.append(d) });
var notes=PG.notes&&PG.notes.length?h('ul',{class:'notes'},PG.notes.map(function(n){return h('li',{text:n})})):null;

/* prompt */
var pb=h('div'); pb.className='promptbox';
var fb=h('textarea',{'aria-label':'Seu comentário sobre esta tela',placeholder:'Seu comentário: o que mudar, o que manter. Opcional.'});
var out=h('textarea',{'aria-label':'Prompt gerado',readonly:'readonly',rows:'4'});
var cp=h('button',{type:'button',text:'Copiar prompt'}); cp.className='copy'; var cd=h('span',{class:'copied',text:'Copiado'}); cd.className='copied';
pb.append(h('h2',{text:'Prompt para o feedback'}),fb,out,h('div',{class:'copyrow'},cp,cd));
function label(list,v){ var r=list.filter(function(x){return x[0]===v})[0]; return r?r[1]:v }
function updatePrompt(){
  var p=[];
  p.push('Ajuste o mockup "'+PG.title+'" do SofaLauncher (launcher-tv).');
  p.push('Estado em análise: '+label(PG.scenarios,st.scenario)+'.');
  var ch=[]; PG.options.forEach(function(o){ if(st[o.key]!==DEF[o.key])ch.push(o.label.toLowerCase()+': '+label(o.values,st[o.key])) });
  if(ch.length)p.push('Variações em teste: '+ch.join('; ')+'.');
  if(st.res==='4k')p.push('Avaliado em 4K.');
  if(st.reduced==='yes')p.push('Avaliado com movimento reduzido.');
  if(fb.value.trim())p.push('Comentário: '+fb.value.trim());
  else p.push('Sem comentário: o estado acima está aprovado como está.');
  p.push('Atualize o mockup e o handoff docs/design/launcher-tv.md juntos e mantenha um único item em foco por tela.');
  out.value=p.join(' ');
}
fb.addEventListener('input',updatePrompt);
cp.addEventListener('click',function(){ updatePrompt(); (navigator.clipboard&&navigator.clipboard.writeText?navigator.clipboard.writeText(out.value):Promise.reject()).catch(function(){ out.select(); document.execCommand('copy') }).then(function(){}); cd.classList.add('on'); setTimeout(function(){cd.classList.remove('on')},1600) });

var main=h('main'); main.className='main'; main.append(top,wrap,read,keys,notes,pb);
var root=h('div'); root.className='pg'; root.append(panel,main); document.body.append(root);

function setOpen(){ var c=cfg(); open.href='tv.html?'+qsOf(c) }
var _apply=apply; apply=function(){ _apply(); setOpen() }; setOpen();
layout(); updatePrompt();

/* mensagens da prévia */
window.addEventListener('message',function(ev){
  var d=ev.data; if(!d||typeof d!=='object')return;
  if(d.t==='status'){ rL.lastChild.textContent=d.layer||'-'; rF.lastChild.textContent=d.focus||'(nenhum)';
    if(d.sheet){rC.textContent='folha de componentes';rC.className='badge'}
    else if(d.focusCount===1){rC.textContent='1 item em foco';rC.className='badge ok'}
    else if(d.focusCount===0){rC.textContent='sem item em foco (tela passiva)';rC.className='badge'}
    else {rC.textContent=d.focusCount+' itens em foco';rC.className='badge bad'} }
  if(d.t==='error'){ rC.textContent='erro: '+d.msg; rC.className='badge bad' }
});
/* teclado: se o foco estiver na página, repassa para a prévia */
function relay(type){ return function(e){ var t=e.target; if(t&&/^(INPUT|TEXTAREA|SELECT|BUTTON|A)$/.test(t.tagName))return; if(e.ctrlKey||e.altKey||e.metaKey)return;
  var k=e.key; if(['ArrowUp','ArrowDown','ArrowLeft','ArrowRight','Enter','Escape','F1','F2','PageUp','PageDown',' ','[',']','m','M','v','V','ContextMenu'].indexOf(k)<0)return;
  e.preventDefault(); try{frame.contentWindow.focus()}catch(x){} post({t:'key',key:k,type:type}) } }
addEventListener('keydown',relay('down')); addEventListener('keyup',relay('up'));
/* controle Xbox: lido aqui e repassado */
if(window.Pad){ var pad=Pad.start(function(i){post({t:'intent',i:i})},function(v,m){post({t:'held',view:v,menu:m})});
  setInterval(function(){ rP.textContent=pad.connected()?'Controle: '+pad.name().slice(0,48):'Controle: aperte um botão para ativar' },600) }
wrap.addEventListener('click',function(){ hintEl.style.display='none' });
})();
