/* Gamepad API (mapeamento "standard" do Xbox) -> intenções do Sofá. Só para o mockup.
   Direções repetem após 400 ms (CA-07); zona morta de 25% (RN-06); gatilhos contam 1 vez por puxada (RN-23). */
(function(){
  var BTN={0:'Confirm',1:'Back',9:'Menu',8:'View',4:'PagePrev',5:'PageNext',12:'Up',13:'Down',14:'Left',15:'Right'};
  var DIRS={Up:1,Down:1,Left:1,Right:1};
  function start(emit,onHeld){
    var st={},lt=false,rt=false,lastHeld='',connected=false,name='';
    function press(k,on,t){
      var s=st[k]||(st[k]={on:false,t:0,n:0});
      if(on&&!s.on){s.on=true;s.t=t;s.n=0;emit(k,false)}
      else if(on&&s.on&&DIRS[k]){var wait=s.n===0?400:110;if(t-s.t>=wait){s.t=t;s.n++;emit(k,true)}}
      else if(!on&&s.on){s.on=false}
    }
    function poll(t){
      var pads=(navigator.getGamepads?Array.prototype.slice.call(navigator.getGamepads()):[]).filter(Boolean);
      connected=pads.length>0;name=connected?pads[0].id:'';
      var d={},tl=false,tr=false;
      pads.forEach(function(p){
        for(var b in BTN){if(p.buttons[b]&&p.buttons[b].pressed)d[BTN[b]]=true}
        var x=p.axes[0]||0,y=p.axes[1]||0,m=Math.hypot(x,y);
        if(m>0.25){ if(Math.abs(x)>Math.abs(y)){d[x>0?'Right':'Left']=true}else{d[y>0?'Down':'Up']=true} }
        if(p.buttons[6]&&p.buttons[6].value>0.5)tl=true;
        if(p.buttons[7]&&p.buttons[7].value>0.5)tr=true;
      });
      ['Up','Down','Left','Right','Confirm','Back','Menu','View','PagePrev','PageNext'].forEach(function(k){press(k,!!d[k],t)});
      if(tl&&!lt)emit('VolDown',false);
      if(tr&&!rt)emit('VolUp',false);
      lt=tl;rt=tr;
      var h=(d.View?'1':'0')+(d.Menu?'1':'0');
      if(h!==lastHeld){lastHeld=h;onHeld&&onHeld(!!d.View,!!d.Menu)}
      requestAnimationFrame(poll);
    }
    requestAnimationFrame(poll);
    return {connected:function(){return connected},name:function(){return name}};
  }
  window.Pad={start:start};
})();
