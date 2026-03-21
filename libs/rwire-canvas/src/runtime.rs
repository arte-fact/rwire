//! Canvas JS runtime — browser-side opcode dispatcher + input + interpolation.
//!
//! This is the equivalent of rwire's RUNTIME_JS but for Canvas 2D.
//! It dispatches binary canvas opcodes, manages entity interpolation,
//! and collects input to send back to the server.

/// The complete canvas runtime JS, minified.
///
/// Injected into the capsule HTML. Handles:
/// - WebSocket connection to server
/// - Binary canvas opcode dispatcher
/// - Texture/sprite/font/color table management
/// - Entity interpolation at 60fps
/// - Keyboard and touch input collection at 20Hz
pub const CANVAS_RUNTIME_JS: &str = r#"
(function(){
'use strict';
const cvs=document.getElementById('game-canvas');
const ctx=cvs.getContext('2d');
ctx.imageSmoothingEnabled=false;
const W=cvs.width,H=cvs.height;

// Tables
let tex={},spr=[],fonts=[],colors=[],ents={};
let prevEnts={},lastTick=0,frameTime=0,tickInterval=50;
let loaded=0,totalTex=0,ready=false;

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
    else if(o===0x01){/* FRAME_END — draw entities with interpolation on next rAF */}
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
    // Tables
    else if(o===0xF0){const n=d[i++];totalTex=n;loaded=0;for(let j=0;j<n;j++){const id=d[i++],pl=d[i++];const p=new TextDecoder().decode(d.subarray(i,i+pl));i+=pl;const img=new Image();img.onload=()=>{tex[id]=img;loaded++;if(loaded>=totalTex)ready=true};img.src=p}}
    else if(o===0xF1){const n=d[i++];colors=[];for(let j=0;j<n;j++){colors.push(`rgba(${d[i]},${d[i+1]},${d[i+2]},${d[i+3]/255})`);i+=4}}
    else if(o===0xF2){const n=d[i++];fonts=[];for(let j=0;j<n;j++){const fl=d[i++];fonts.push(new TextDecoder().decode(d.subarray(i,i+fl)));i+=fl}}
    else if(o===0xF3){const n=u16(v,i);i+=2;spr=[];for(let j=0;j<n;j++){spr.push([d[i],u16(v,i+1),u16(v,i+3),u16(v,i+5),u16(v,i+7)]);i+=9}}
    else if(o===0xF8){const wx=i16(v,i),wy=i16(v,i+2),gw=u16(v,i+4),gh=u16(v,i+6),ts=d[i+8];i+=9;if(!window.__fogCvs||window.__fogCvs.width!==gw||window.__fogCvs.height!==gh){window.__fogCvs=document.createElement('canvas');window.__fogCvs.width=gw;window.__fogCvs.height=gh}const fc=window.__fogCvs.getContext('2d');const id=fc.createImageData(gw,gh);for(let j=0;j<gw*gh;j++){const a=d[i++];id.data[j*4]=12;id.data[j*4+1]=12;id.data[j*4+2]=22;id.data[j*4+3]=a}fc.putImageData(id,0,0);const si=ctx.imageSmoothingEnabled;ctx.imageSmoothingEnabled=true;ctx.drawImage(window.__fogCvs,0,0,gw,gh,wx,wy,gw*ts,gh*ts);ctx.imageSmoothingEnabled=si}
    else if(o===0xF9){const n=u16(v,i);i+=2;prevEnts=Object.assign({},ents);for(let j=0;j<n;j++){const id=u16(v,i),x=i16(v,i+2),y=i16(v,i+4),sp=u16(v,i+6),fl=d[i+8];i+=9;ents[id]={x,y,sp,fl}}}
    else{console.warn('unknown canvas opcode',o,'at',i-1);break}
  }
}

// WebSocket
const loc=window.location;
const wsUrl=(loc.protocol==='https:'?'wss:':'ws:')+'//'+loc.host;
let w;
function connect(){
  w=new WebSocket(wsUrl);
  w.binaryType='arraybuffer';
  w.onmessage=e=>{if(e.data instanceof ArrayBuffer){exec(new Uint8Array(e.data))}};
  w.onclose=()=>setTimeout(connect,1000);
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
  // Duplicate listener that latches — ensures short presses are captured
  const k2=e2.key.toLowerCase();
  if(k2===' ')latchKeys|=0x10;
  if(k2==='h')latchKeys|=0x20;
  if(k2==='g')latchKeys|=0x40;
  if(k2==='r')latchKeys|=0x80;
  if(k2==='f')latchKeys2|=0x01;
});

// Send input at 20Hz
setInterval(()=>{
  if(w&&w.readyState===1){
    const msg=new Uint8Array(7);
    msg[0]=keys|latchKeys;msg[1]=keys2|latchKeys2;
    msg[2]=(touchX>>8)&0xFF;msg[3]=touchX&0xFF;
    msg[4]=(touchY>>8)&0xFF;msg[5]=touchY&0xFF;
    msg[6]=touching;
    w.send(msg);
    latchKeys=0;latchKeys2=0;
  }
},50);

// Render loop placeholder (entities drawn by server frames,
// interpolation can be added later)
})();
"#;
