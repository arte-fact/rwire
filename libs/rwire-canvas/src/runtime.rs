//! Canvas JS runtime — browser-side opcode dispatcher + input + interpolation.
//!
//! This is the equivalent of rwire's RUNTIME_JS but for Canvas 2D.
//! It dispatches binary canvas opcodes, manages entity interpolation,
//! and collects input to send back to the server.

/// The complete canvas runtime JS, minified.
///
/// Injected into the capsule HTML. Handles:
/// - WebSocket connection to server
/// - Binary canvas opcode dispatcher (immediate-mode + retained-mode)
/// - Texture/sprite/font/color table management
/// - Retained sprite table with 60fps interpolation
/// - Layer caching with offscreen canvases
/// - Keyboard and touch input collection at 20Hz
pub const CANVAS_RUNTIME_JS: &str = r#"
(function(){
'use strict';
const cvs=document.getElementById('game-canvas');
let ctx=cvs.getContext('2d');
let mainCtx=ctx;
ctx.imageSmoothingEnabled=false;
let W=cvs.width,H=cvs.height;

// DPR-aware canvas sizing — fills viewport at native resolution
function resizeCanvas(){
  const dpr=window.devicePixelRatio||1;
  const cw=cvs.clientWidth||960;
  const ch=cvs.clientHeight||640;
  const pw=Math.round(cw*dpr);
  const ph=Math.round(ch*dpr);
  if(pw!==W||ph!==H){
    cvs.width=pw;cvs.height=ph;
    W=pw;H=ph;
    ctx=cvs.getContext('2d');
    mainCtx=ctx;
    ctx.imageSmoothingEnabled=false;
  }
}
resizeCanvas();
window.addEventListener('resize',resizeCanvas);

// Tables
let tex={},spr=[],fonts=[],colors=[],ents={};
let prevEnts={},lastTick=0,frameTime=0,tickInterval=50;
let loaded=0,totalTex=0,ready=false;
let pendingSetup=null; // buffered setup bytes to replay after textures load

// Retained-mode scene state
let layers={};       // layer_id -> {cvs,ctx,flags,sprites:[]}
let rSprites={};     // id -> {layer,sp,x,y,px,py,fl,alpha}
let cam={x:0,y:0,z:256,px:0,py:0,pz:256};
let sceneMode=false; // true once first SCENE_TICK received
let lastSceneTime=0;
let overlayBuf=null; // stored overlay opcode bytes for replay

// Input state
let keys=0,keys2=0,touchX=0,touchY=0,touching=0;

// Text align/baseline maps
const TA=['left','center','right'];
const TB=['top','middle','bottom','alphabetic'];
const CO=['source-over','multiply','screen'];

// Read i16 from DataView
function i16(v,o){return v.getInt16(o)}
function u16(v,o){return v.getUint16(o)}
function u32(v,o){return v.getUint32(o)}

// Dispatch canvas opcodes from binary data
function exec(d){
  const v=new DataView(d.buffer,d.byteOffset,d.byteLength);
  let i=0;
  while(i<d.length){
    const o=d[i++];
    if(o===0x00){lastTick=u32(v,i);i+=4;frameTime=performance.now()}
    else if(o===0x01){/* FRAME_END */}
    else if(o===0x02){ctx.clearRect(0,0,W,H)}
    else if(o===0x10){ctx.save()}
    else if(o===0x11){ctx.restore()}
    else if(o===0x12){ctx.translate(i16(v,i),i16(v,i+2));i+=4}
    else if(o===0x13){ctx.scale(i16(v,i)/256,i16(v,i+2)/256);i+=4}
    else if(o===0x14){ctx.rotate(u16(v,i)/10430.378);i+=2}
    else if(o===0x20){ctx.fillRect(i16(v,i),i16(v,i+2),u16(v,i+4),u16(v,i+6));i+=8}
    else if(o===0x21){ctx.strokeRect(i16(v,i),i16(v,i+2),u16(v,i+4),u16(v,i+6));i+=8}
    else if(o===0x23){ctx.beginPath()}
    else if(o===0x24){ctx.moveTo(i16(v,i),i16(v,i+2));i+=4}
    else if(o===0x25){ctx.lineTo(i16(v,i),i16(v,i+2));i+=4}
    else if(o===0x26){ctx.arc(i16(v,i),i16(v,i+2),u16(v,i+4),u16(v,i+6)/10430.378,u16(v,i+8)/10430.378);i+=10}
    else if(o===0x27){ctx.fill()}
    else if(o===0x28){ctx.stroke()}
    else if(o===0x29){ctx.closePath()}
    else if(o===0x2A){const x=i16(v,i),y=i16(v,i+2),w=u16(v,i+4),h=u16(v,i+6),r=d[i+8];i+=9;ctx.beginPath();ctx.roundRect(x,y,w,h,r);ctx.fill()}
    else if(o===0x30){const t=d[i++],sx=u16(v,i),sy=u16(v,i+2),sw=u16(v,i+4),sh=u16(v,i+6),dx=i16(v,i+8),dy=i16(v,i+10),dw=u16(v,i+12),dh=u16(v,i+14);i+=16;if(tex[t])ctx.drawImage(tex[t],sx,sy,sw,sh,dx,dy,dw,dh)}
    else if(o===0x31){const t=d[i++],dx=i16(v,i),dy=i16(v,i+2),dw=u16(v,i+4),dh=u16(v,i+6);i+=8;if(tex[t])ctx.drawImage(tex[t],dx,dy,dw,dh)}
    else if(o===0x32){const sid=u16(v,i),dx=i16(v,i+2),dy=i16(v,i+4);i+=6;const s=spr[sid];if(s&&tex[s[0]])ctx.drawImage(tex[s[0]],s[1],s[2],s[3],s[4],dx,dy,s[3],s[4])}
    else if(o===0x40){const x=i16(v,i),y=i16(v,i+2),len=d[i+4];i+=5;const t=new TextDecoder().decode(d.subarray(i,i+len));i+=len;ctx.fillText(t,x,y)}
    else if(o===0x41){const x=i16(v,i),y=i16(v,i+2),len=d[i+4];i+=5;const t=new TextDecoder().decode(d.subarray(i,i+len));i+=len;ctx.strokeText(t,x,y)}
    else if(o===0x50){ctx.fillStyle=`rgba(${d[i]},${d[i+1]},${d[i+2]},${d[i+3]/255})`;i+=4}
    else if(o===0x51){const c=colors[d[i++]];if(c)ctx.fillStyle=c}
    else if(o===0x52){ctx.strokeStyle=`rgba(${d[i]},${d[i+1]},${d[i+2]},${d[i+3]/255})`;i+=4}
    else if(o===0x53){const c=colors[d[i++]];if(c)ctx.strokeStyle=c}
    else if(o===0x54){ctx.globalAlpha=d[i++]/255}
    else if(o===0x55){ctx.lineWidth=d[i++]/4}
    else if(o===0x56){const n=d[i++];const segs=[];for(let j=0;j<n;j++)segs.push(d[i++]);ctx.setLineDash(segs)}
    else if(o===0x57){const f=fonts[d[i++]];if(f)ctx.font=f}
    else if(o===0x58){ctx.textAlign=TA[d[i++]]||'left'}
    else if(o===0x59){ctx.textBaseline=TB[d[i++]]||'top'}
    else if(o===0x5A){ctx.imageSmoothingEnabled=!!d[i++]}
    else if(o===0x5B){ctx.globalCompositeOperation=CO[d[i++]]||'source-over'}
    // Scene control (0x60-0x6F)
    else if(o===0x60){const l=d[i++],fl=d[i++];layers[l]={flags:fl,sprites:[],cvs:null,ctx:null};
      if(fl&1){const c=document.createElement('canvas');c.width=12800;c.height=12800;const lc=c.getContext('2d');lc.imageSmoothingEnabled=false;layers[l].cvs=c;layers[l].ctx=lc}}
    else if(o===0x61){const l=d[i++];if(layers[l]&&layers[l].cvs){layers[l].ctx.clearRect(0,0,layers[l].cvs.width,layers[l].cvs.height)}}
    else if(o===0x62){cam.px=cam.x;cam.py=cam.y;cam.pz=cam.z;cam.x=i16(v,i);cam.y=i16(v,i+2);cam.z=u16(v,i+4);i+=6}
    else if(o===0x67){const ms=u16(v,i);i+=2;if(ms!==tickInterval){tickInterval=ms;if(window.__inputIv){clearInterval(window.__inputIv)}window.__inputIv=setInterval(sendInput,ms)}}
    else if(o===0x68){const l=d[i++];if(layers[l]&&layers[l].cvs){const si=ctx.imageSmoothingEnabled;ctx.imageSmoothingEnabled=false;ctx.drawImage(layers[l].cvs,0,0);ctx.imageSmoothingEnabled=si}}
    else if(o===0x65){const l=d[i++];if(!ready){
      // Textures not loaded yet — find matching LAYER_TARGET_MAIN and buffer everything between
      const start=i-2;let depth=1;while(i<d.length){const op=d[i];if(op===0x65){depth++;i+=2}else if(op===0x66){depth--;i++;if(depth===0)break}else{
        // Skip opcode args (simplified: scan for next known opcode)
        // Instead, buffer the entire remaining setup and replay later
        pendingSetup=new Uint8Array(d.buffer.slice(d.byteOffset+start,d.byteOffset+d.byteLength));i=d.length;break}}
      }else if(layers[l]&&layers[l].ctx){ctx=layers[l].ctx}}
    else if(o===0x66){ctx=mainCtx}
    else if(o===0x63){sceneMode=true;lastSceneTime=performance.now();
      for(const id in rSprites){const s=rSprites[id];s.px=s.x;s.py=s.y}
      i+=4}
    else if(o===0x64){/* SCENE_END — remaining bytes are overlay, store for replay only */
      overlayBuf=new Uint8Array(d.buffer.slice(d.byteOffset+i,d.byteOffset+d.byteLength));break}
    // Retained sprites (0x70-0x77)
    else if(o===0x70){const id=u16(v,i),l=d[i+2],sp=u16(v,i+3),x=i16(v,i+5),y=i16(v,i+7),fl=d[i+9];i+=10;
      rSprites[id]={layer:l,sp:sp,x:x,y:y,px:x,py:y,fl:fl,alpha:255};
      if(layers[l])layers[l].sprites.push(id)}
    else if(o===0x71){const id=u16(v,i);i+=2;const s=rSprites[id];
      if(s&&layers[s.layer]){const a=layers[s.layer].sprites;const j=a.indexOf(id);if(j>=0)a.splice(j,1)}
      delete rSprites[id]}
    else if(o===0x72){const id=u16(v,i),x=i16(v,i+2),y=i16(v,i+4);i+=6;
      const s=rSprites[id];if(s){s.x=x;s.y=y}}
    else if(o===0x73){const id=u16(v,i),sp=u16(v,i+2);i+=4;
      const s=rSprites[id];if(s)s.sp=sp}
    else if(o===0x74){const id=u16(v,i),x=i16(v,i+2),y=i16(v,i+4),sp=u16(v,i+6),fl=d[i+8];i+=9;
      const s=rSprites[id];if(s){s.x=x;s.y=y;s.sp=sp;s.fl=fl}}
    else if(o===0x75){const id=u16(v,i),a=d[i+2];i+=3;
      const s=rSprites[id];if(s)s.alpha=a}
    else if(o===0x76){const n=d[i++];for(let j=0;j<n;j++){
      const id=u16(v,i),x=i16(v,i+2),y=i16(v,i+4);i+=6;
      const s=rSprites[id];if(s){s.x=x;s.y=y}}}
    else if(o===0x77){const n=d[i++];for(let j=0;j<n;j++){
      const id=u16(v,i),x=i16(v,i+2),y=i16(v,i+4),sp=u16(v,i+6),fl=d[i+8];i+=9;
      const s=rSprites[id];if(s){s.x=x;s.y=y;s.sp=sp;s.fl=fl}}}
    // Tilemap region (0x78)
    else if(o===0x78){const l=d[i++],gx=u16(v,i),gy=u16(v,i+2),gw=u16(v,i+4),gh=u16(v,i+6),ts=u16(v,i+8);i+=10;
      const lay=layers[l];if(lay&&lay.ctx){const lc=lay.ctx;
      for(let ty=0;ty<gh;ty++)for(let tx=0;tx<gw;tx++){
        const sp_id=u16(v,i),fl=d[i+2];i+=3;const s=spr[sp_id];
        if(s&&tex[s[0]]){
          if(fl&1){lc.save();lc.translate((gx+tx)*ts+ts,0);lc.scale(-1,1);lc.drawImage(tex[s[0]],s[1],s[2],s[3],s[4],(gx+tx)*ts,(gy+ty)*ts,ts,ts);lc.restore()}
          else{lc.drawImage(tex[s[0]],s[1],s[2],s[3],s[4],(gx+tx)*ts,(gy+ty)*ts,ts,ts)}
        }}}}
    // Draw retained sprites on layer (0x7D) — y-sorted, viewport culled
    else if(o===0x7D){const tl=d[i++];
      const z=cam.z/256||1;const margin=256;
      const vl=cam.x-W/(2*z)-margin,vr=cam.x+W/(2*z)+margin;
      const vt=cam.y-H/(2*z)-margin,vb=cam.y+H/(2*z)+margin;
      const ids=[];
      for(const id in rSprites){const s=rSprites[id];
        if(s.anim||!(s.fl&2)||s.layer!==tl)continue;
        if(s.x<vl||s.x>vr||s.y<vt||s.y>vb)continue;
        ids.push(id)}
      ids.sort((a,b)=>rSprites[a].y-rSprites[b].y);
      for(const id of ids){const s=rSprites[id];
        const sp=spr[s.sp];if(!sp||!tex[sp[0]])continue;
        if(s.alpha<255)ctx.globalAlpha=s.alpha/255;
        const hw=sp[3]/2,hh=sp[4]/2;
        if(s.fl&1){ctx.save();ctx.translate(s.x,s.y);ctx.scale(-1,1);ctx.drawImage(tex[sp[0]],sp[1],sp[2],sp[3],sp[4],-hw,-hh,sp[3],sp[4]);ctx.restore()}
        else{ctx.drawImage(tex[sp[0]],sp[1],sp[2],sp[3],sp[4],s.x-hw,s.y-hh,sp[3],sp[4])}
        if(s.alpha<255)ctx.globalAlpha=1}}
    // Draw animated sprites on layer (0x7C) — inline in current transform, viewport culled
    else if(o===0x7C){const tl=d[i++];const nowSec=performance.now()*0.001;
      // Viewport bounds in world space (with 256px margin for large sprites)
      const z=cam.z/256||1;const margin=256;
      const vl=cam.x-W/(2*z)-margin,vr=cam.x+W/(2*z)+margin;
      const vt=cam.y-H/(2*z)-margin,vb=cam.y+H/(2*z)+margin;
      for(const id in rSprites){const s=rSprites[id];
        if(!s.anim||!(s.fl&2)||s.layer!==tl)continue;
        if(s.x<vl||s.x>vr||s.y<vt||s.y>vb)continue;
        let frame=0;
        if(s.fl&4){const wave=Math.sin(nowSec*0.94+s.phase);frame=wave>0.3?Math.floor(nowSec*s.fps)%s.fc:0}
        else{frame=Math.floor(nowSec*s.fps)%s.fc}
        const spId=s.firstSp+frame;const sp=spr[spId];
        if(!sp||!tex[sp[0]])continue;
        if(s.alpha<255)ctx.globalAlpha=s.alpha/255;
        if(s.fl&1){ctx.save();ctx.translate(s.x+sp[3],s.y);ctx.scale(-1,1);ctx.drawImage(tex[sp[0]],sp[1],sp[2],sp[3],sp[4],0,0,sp[3],sp[4]);ctx.restore()}
        else{ctx.drawImage(tex[sp[0]],sp[1],sp[2],sp[3],sp[4],s.x,s.y,sp[3],sp[4])}
        if(s.alpha<255)ctx.globalAlpha=1}}
    // Animated sprite (0x7B) — client auto-cycles frames
    else if(o===0x7B){const id=u16(v,i),l=d[i+2],fsp=u16(v,i+3),fc=d[i+5],fps=d[i+6],ph=d[i+7],x=i16(v,i+8),y=i16(v,i+10),fl=d[i+12];i+=13;
      rSprites[id]={layer:l,sp:fsp,x:x,y:y,px:x,py:y,fl:fl,alpha:255,anim:1,firstSp:fsp,fc:fc,fps:fps,phase:ph*(Math.PI*2/255)};
      if(layers[l])layers[l].sprites.push(id)}
    // Minimap (0x79 = store, 0x7A = draw)
    else if(o===0x79){const mx=i16(v,i),my=i16(v,i+2),mw=u16(v,i+4),mh=u16(v,i+6);i+=8;if(!window.__miniCvs||window.__miniCvs.width!==mw||window.__miniCvs.height!==mh){window.__miniCvs=document.createElement('canvas');window.__miniCvs.width=mw;window.__miniCvs.height=mh}const mc=window.__miniCvs.getContext('2d');const id2=mc.createImageData(mw,mh);for(let j=0;j<mw*mh;j++){id2.data[j*4]=d[i++];id2.data[j*4+1]=d[i++];id2.data[j*4+2]=d[i++];id2.data[j*4+3]=255}mc.putImageData(id2,0,0)}
    else if(o===0x7A){const dx=i16(v,i),dy=i16(v,i+2),dw=u16(v,i+4),dh=u16(v,i+6);i+=8;if(window.__miniCvs){const si=ctx.imageSmoothingEnabled;ctx.imageSmoothingEnabled=false;ctx.drawImage(window.__miniCvs,dx,dy,dw,dh);ctx.imageSmoothingEnabled=si}}
    // Tables
    else if(o===0xF0){const n=d[i++];totalTex=n;loaded=0;for(let j=0;j<n;j++){const id=d[i++],pl=d[i++];const p=new TextDecoder().decode(d.subarray(i,i+pl));i+=pl;const img=new Image();img.onload=()=>{tex[id]=img;loaded++;if(loaded>=totalTex){ready=true;if(pendingSetup){exec(pendingSetup);pendingSetup=null}}};img.src=p}}
    else if(o===0xF1){const n=d[i++];colors=[];for(let j=0;j<n;j++){colors.push(`rgba(${d[i]},${d[i+1]},${d[i+2]},${d[i+3]/255})`);i+=4}}
    else if(o===0xF2){const n=d[i++];fonts=[];for(let j=0;j<n;j++){const fl=d[i++];fonts.push(new TextDecoder().decode(d.subarray(i,i+fl)));i+=fl}}
    else if(o===0xF3){const n=u16(v,i);i+=2;spr=[];for(let j=0;j<n;j++){spr.push([d[i],u16(v,i+1),u16(v,i+3),u16(v,i+5),u16(v,i+7)]);i+=9}}
    else if(o===0xF8){const wx=i16(v,i),wy=i16(v,i+2),gw=u16(v,i+4),gh=u16(v,i+6),ts=d[i+8];i+=9;if(!window.__fogCvs||window.__fogCvs.width!==gw||window.__fogCvs.height!==gh){window.__fogCvs=document.createElement('canvas');window.__fogCvs.width=gw;window.__fogCvs.height=gh}const fc=window.__fogCvs.getContext('2d');const id=fc.createImageData(gw,gh);for(let j=0;j<gw*gh;j++){const a=d[i++];id.data[j*4]=12;id.data[j*4+1]=12;id.data[j*4+2]=22;id.data[j*4+3]=a}fc.putImageData(id,0,0);const si=ctx.imageSmoothingEnabled;ctx.imageSmoothingEnabled=true;ctx.drawImage(window.__fogCvs,0,0,gw,gh,wx,wy,gw*ts,gh*ts);ctx.imageSmoothingEnabled=si}
    else if(o===0xF9){const n=u16(v,i);i+=2;prevEnts=Object.assign({},ents);for(let j=0;j<n;j++){const id=u16(v,i),x=i16(v,i+2),y=i16(v,i+4),sp=u16(v,i+6),fl=d[i+8];i+=9;ents[id]={x,y,sp,fl}}}
    else{console.warn('unknown canvas opcode',o,'at',i-1);break}
  }
}

// Scene render loop (60fps) — only active when sceneMode is true
function renderScene(now){
  requestAnimationFrame(renderScene);
  if(!sceneMode||!ready)return;

  ctx=mainCtx;
  ctx.clearRect(0,0,W,H);

  // Replay overlay — includes terrain layer draw, camera transforms, DRAW_ANIM_SPRITES, and all content
  if(overlayBuf)exec(overlayBuf);
}
requestAnimationFrame(renderScene);

// WebSocket
const loc=window.location;
const wsUrl=(loc.protocol==='https:'?'wss:':'ws:')+'//'+loc.host;
let w;
function connect(){
  w=new WebSocket(wsUrl);
  w.binaryType='arraybuffer';
  w.onmessage=e=>{if(e.data instanceof ArrayBuffer){exec(new Uint8Array(e.data))}};
  w.onclose=()=>{sceneMode=false;rSprites={};layers={};setTimeout(connect,1000)};
}
connect();

// Input
document.addEventListener('keydown',e=>{
  const k=e.key.toLowerCase();
  if(k==='w'||k==='z'||k==='arrowup')keys|=0x01;
  if(k==='a'||k==='q'||k==='arrowleft')keys|=0x02;
  if(k==='s'||k==='arrowdown')keys|=0x04;
  if(k==='d'||k==='arrowright')keys|=0x08;
  if(k===' ')keys|=0x10;
  if(k==='h')keys|=0x20;
  if(k==='g')keys|=0x40;
  if(k==='r')keys|=0x80;
  if(k==='f')keys2|=0x01;
  if(k==='+')keys2|=0x02;
  if(k==='-')keys2|=0x04;
  e.preventDefault();
});
document.addEventListener('keyup',e=>{
  const k=e.key.toLowerCase();
  if(k==='w'||k==='z'||k==='arrowup')keys&=~0x01;
  if(k==='a'||k==='q'||k==='arrowleft')keys&=~0x02;
  if(k==='s'||k==='arrowdown')keys&=~0x04;
  if(k==='d'||k==='arrowright')keys&=~0x08;
  if(k===' ')keys&=~0x10;
  if(k==='h')keys&=~0x20;
  if(k==='g')keys&=~0x40;
  if(k==='r')keys&=~0x80;
  if(k==='f')keys2&=~0x01;
  if(k==='+')keys2&=~0x02;
  if(k==='-')keys2&=~0x04;
});

// Touch
cvs.addEventListener('touchstart',e=>{e.preventDefault();const t=e.touches[0];const r=cvs.getBoundingClientRect();touchX=Math.round((t.clientX-r.left)*W/r.width);touchY=Math.round((t.clientY-r.top)*H/r.height);touching=1},{passive:false});
cvs.addEventListener('touchmove',e=>{e.preventDefault();const t=e.touches[0];const r=cvs.getBoundingClientRect();touchX=Math.round((t.clientX-r.left)*W/r.width);touchY=Math.round((t.clientY-r.top)*H/r.height)},{passive:false});
cvs.addEventListener('touchend',e=>{e.preventDefault();touching=0},{passive:false});

// Mouse
cvs.addEventListener('mousedown',e=>{const r=cvs.getBoundingClientRect();touchX=Math.round((e.clientX-r.left)*W/r.width);touchY=Math.round((e.clientY-r.top)*H/r.height);touching=1});
cvs.addEventListener('mousemove',e=>{if(touching){const r=cvs.getBoundingClientRect();touchX=Math.round((e.clientX-r.left)*W/r.width);touchY=Math.round((e.clientY-r.top)*H/r.height)}});
cvs.addEventListener('mouseup',()=>{touching=0});

// Latch keys: once set, stay set until sent
let latchKeys=0,latchKeys2=0;
document.addEventListener('keydown',e2=>{
  const k2=e2.key.toLowerCase();
  if(k2===' ')latchKeys|=0x10;
  if(k2==='h')latchKeys|=0x20;
  if(k2==='g')latchKeys|=0x40;
  if(k2==='r')latchKeys|=0x80;
  if(k2==='f')latchKeys2|=0x01;
});

// Send input at tick rate
function sendInput(){
  resizeCanvas();
  if(w&&w.readyState===1){
    const msg=new Uint8Array(11);
    msg[0]=keys|latchKeys;msg[1]=keys2|latchKeys2;
    msg[2]=(touchX>>8)&0xFF;msg[3]=touchX&0xFF;
    msg[4]=(touchY>>8)&0xFF;msg[5]=touchY&0xFF;
    msg[6]=touching;
    msg[7]=(W>>8)&0xFF;msg[8]=W&0xFF;
    msg[9]=(H>>8)&0xFF;msg[10]=H&0xFF;
    w.send(msg);
    latchKeys=0;latchKeys2=0;
  }
}
window.__inputIv=setInterval(sendInput,tickInterval);
})();
"#;
