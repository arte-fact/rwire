//! Generate capsule HTML with the element and event mappings.
//!
//! The small u8 token enum maps (elements, events, attribute keys/values, style
//! props/values) are shipped whole. For styled capsules, composite and global
//! CSS is embedded in a `<style>` tag within `<head>`; utility/pseudo/breakpoint
//! rules are delivered lazily over the WebSocket (STYLE_DEF).

use crate::protocol::opcodes::{ELEMENT_MAPPINGS, EVENT_MAPPINGS, SVG_ELEMENT_CODES};
use crate::theme::Theme;

/// Generate a JS object literal `{code:'string', ...}` from a mappings array.
///
/// Emits **every** entry. These maps cover the small `u8` token enums
/// (elements, events, attribute keys/values, style props/values) whose total
/// size is ~1-2 KB; shipping them whole removes the structural-failure risk of a
/// token reached only through a plain helper function being absent from a
/// tree-shaken map. CSS (the part that actually justifies tree-shaking) is
/// handled separately. See `docs/tree-shaking-redesign.md`.
fn generate_js_map<T: std::fmt::Display>(mappings: &[(T, &str)]) -> String {
    let entries: Vec<String> = mappings
        .iter()
        .map(|(code, name)| format!("{}:'{}'", code, name))
        .collect();
    entries.join(",")
}

/// Generate the SVG element type set `{code:1, ...}` for every SVG element.
fn generate_svg_set() -> String {
    let entries: Vec<String> = SVG_ELEMENT_CODES
        .iter()
        .map(|code| format!("{}:1", code))
        .collect();
    entries.join(",")
}

/// The runtime JavaScript code (constant part, not affected by tree shaking).
///
/// The gp() function collects payload data from events:
/// - Form submit: collects all field values as JSON {t:'form',v:{...}}
/// - Input/change on input/textarea: collects value as JSON {t:'text',v:'...'}
/// - Click: collects data-* attributes as JSON {t:'data',v:{...}}
///
/// The sep() function sends events with param bytes (for ItemRef handlers):
/// - Format: [handler_idx | 0x80, event_type, ref, param_len, ...param_bytes, payload_len, ...payload]
///
/// The sh() function scrolls to a hash target (#id) using MutationObserver:
/// - Tries immediate getElementById; if not found, observes DOM mutations
/// - Auto-disconnects after 2s safety timeout
///
/// New opcodes for bandwidth optimization:
/// - CREATE_SYNCED (0x03): Create span with id="__synced_N" using varint ID
/// - GET_SYNCED (0x05): Get synced element by numeric ID using varint
/// - SYMBOLS_EXTEND (0xF1): Add new symbols to existing table
/// - WORD_TABLE (0xF2): Define word table for text compression
/// - SET_TEXT_WORDS (0x13): Set text from word indices
/// - SET_TEXT_INT (0x15): Set text from zigzag-encoded integer
/// - STYLE_UTIL (0x82): Set style from utility token (varint encoded)
/// - STYLE_PROP (0x83): Set style from property+value (4 bytes)
/// - STYLE_MULTI (0x84): Set multiple style utilities (varint encoded)
const RUNTIME_JS: &str = r#"const O={S:0xF0,SE:0xF1,WT:0xF2,G:0x01,C:0x02,CS:0x03,GS:0x05,L:0x10,T:0x11,TW:0x13,D:0x14,TI:0x15,A:0x12,P:0x20,CC:0x25,AE:0x26,AB:0x27,AK:0x28,B:0x30,R:0x31,DB:0x33,RP:0x34,IL:0x40,DH:0x42,IT:0x47,BT:0x48,TG:0x49,IS:0x4A,BS:0x4B,SS2:0x4C,TT:0x4D,AT2:0x4E,RU:0x70,RR:0x71,SS:0x81,SU:0x82,SP:0x83,SM:0x84,SC:0x85,CT:0x86,SD:0x87,PD:0x89,BP:0x8A,E:0xFF};
const A={4:'id'};
let s={},wt=[],w,sc=0,K={},DS,pm=null;
function rv(d,i){let b=d[i];if(b<0x80)return[b,1];if(b<0xC0)return[0x80+((b&0x3F)<<8)+d[i+1],2];return[0x4080+((b&0x3F)<<16)+(d[i+1]<<8)+d[i+2],3]}
function wv(a,v){if(v<128)a.push(v);else if(v<16512){v-=128;a.push(128|(v>>8),v&255)}else{v-=16512;a.push(192|(v>>16),(v>>8)&255,v&255)}}
function gp(e,el){
let t=el.tagName.toLowerCase();
if(e.type==='submit'&&t==='form'){e.preventDefault();let fd=new FormData(el),obj={};fd.forEach((v,k)=>obj[k]=v);return JSON.stringify({t:'form',v:obj})}
if((e.type==='input'||e.type==='change')&&(t==='input'||t==='textarea'||t==='select')){return JSON.stringify({t:'text',v:el.value})}
if(e.type==='click'){let tg=e.target.closest('[data-id]')||el,dt={};for(let k in tg.dataset)dt[k]=tg.dataset[k];if(Object.keys(dt).length)return JSON.stringify({t:'data',v:dt})}
return ''}
function se(h,t,f,e,el){let p=gp(e,el),pb=new TextEncoder().encode(p),a=[0];wv(a,h);a.push(t,f&255,pb.length);let msg=new Uint8Array(a.length+pb.length);for(let j=0;j<a.length;j++)msg[j]=a[j];msg.set(pb,a.length);w.send(msg)}
function sep(h,t,f,prm,e,el){let p=gp(e,el),pb=new TextEncoder().encode(p),a=[0x80];wv(a,h);a.push(t,f&255,prm.length);let msg=new Uint8Array(a.length+prm.length+1+pb.length);let j=0;for(let b of a)msg[j++]=b;msg.set(prm,j);j+=prm.length;msg[j++]=pb.length;msg.set(pb,j);w.send(msg)}
function snd(fn,e,el){if(e.type==='input'){if(el.__t)clearTimeout(el.__t);el.__t=setTimeout(fn,250)}else fn()}
// --- DOM morphing (node reuse on update) ---
// me/mk reconcile a live subtree toward a freshly-built shadow subtree, reusing
// nodes by id (and id-less nodes positionally) so focus/caret/scroll/uncontrolled
// input state survive updates. A bound node is reused only when its binding key
// (__hk) is unchanged — its existing listener stays valid via stable handler ids;
// when the binding changed, the freshly-built node (with its new listener) is
// swapped in instead, so listeners are never stale and never leak.
function me(a,b){
if(a.nodeType===3){if(a.nodeValue!==b.nodeValue)a.nodeValue=b.nodeValue;return}
if(a.nodeType!==1)return;
let ba=b.attributes;for(let k=0;k<ba.length;k++){let n=ba[k].name;if(a.getAttribute(n)!==ba[k].value)a.setAttribute(n,ba[k].value)}
let aa=a.attributes;for(let k=aa.length-1;k>=0;k--){let n=aa[k].name;if(!b.hasAttribute(n))a.removeAttribute(n)}
a.__hk=b.__hk;
if(a.id&&a.id.indexOf('__synced_')===0)return; // nested region: its own update owns it
mk(a,b)}
function mk(a,b){
let byId={};for(let c=a.firstChild;c;c=c.nextSibling)if(c.nodeType===1&&c.id)byId[c.id]=c;
let cur=a.firstChild;
for(let bc=b.firstChild;bc;){
let nb=bc.nextSibling,m=null;
if(bc.nodeType===1&&bc.id){if(byId[bc.id])m=byId[bc.id]}
else if(cur&&!(cur.nodeType===1&&cur.id)&&cur.nodeType===bc.nodeType&&(cur.nodeType!==1||cur.tagName===bc.tagName))m=cur;
if(m&&m.nodeType===1&&(m.tagName!==bc.tagName||(m.__hk||'')!==(bc.__hk||'')))m=null;
if(m){if(m!==cur)a.insertBefore(m,cur);else cur=cur.nextSibling;me(m,bc)}
else a.insertBefore(bc,cur);
bc=nb}
while(cur){let n=cur.nextSibling;a.removeChild(cur);cur=n}}
function fm(){if(pm){mk(pm.live,pm.shadow);pm=null}}
function x(d){
let r=[],i=0,_oc=0;
let ae=document.activeElement,ai=ae&&ae.id,ap=ae?ae.selectionStart:0,aq=ae?ae.selectionEnd:0,ax=ae&&(ae.tagName==='INPUT'||ae.tagName==='TEXTAREA'),av=ax?ae.value:null;
try{
while(i<d.length){
let _p=i,o=d[i++];_oc++;
if(o===O.S){let[n,l]=rv(d,i);i+=l;sc=0x80;while(n--){let[sl,ll]=rv(d,i);i+=ll;s[sc++]=new TextDecoder().decode(d.slice(i,i+sl));i+=sl}}
else if(o===O.SE){let[n,l]=rv(d,i);i+=l;let[si,sl]=rv(d,i);i+=sl;sc=si;while(n--){let[sl2,ll]=rv(d,i);i+=ll;s[sc++]=new TextDecoder().decode(d.slice(i,i+sl2));i+=sl2}}
else if(o===O.WT){let n=d[i++];wt=[];while(n--){let[l,ll]=rv(d,i);i+=ll;wt.push(new TextDecoder().decode(d.slice(i,i+l)));i+=l}}
else if(o===O.G){let[k,l]=rv(d,i);i+=l;let el=document.getElementById(s[k]);r.push(el)}
else if(o===O.C){let t=d[i++];r.push(SE[t]?document.createElementNS('http://www.w3.org/2000/svg',E[t]||'svg'):document.createElement(E[t]||'div'))}
else if(o===O.CS){let[id,l]=rv(d,i);i+=l;let e=document.createElement('span');e.id='__synced_'+id;r.push(e)}
else if(o===O.GS){let[id,l]=rv(d,i);i+=l;r.push(document.getElementById('__synced_'+id))}
else if(o===O.T){let[f,fl]=rv(d,i);i+=fl;let[k,l]=rv(d,i);i+=l;r[f].textContent=s[k]||''}
else if(o===O.TW){let[f,fl]=rv(d,i);i+=fl;let n=d[i++],ws=[];while(n--)ws.push(wt[d[i++]]||'');r[f].textContent=ws.join(' ')}
else if(o===O.TI){let[f,fl]=rv(d,i);i+=fl;let[v,l]=rv(d,i);i+=l;let n=(v>>>1)^-(v&1);r[f].textContent=n.toString()}
else if(o===O.L){let[f,fl]=rv(d,i);i+=fl;let[k,l]=rv(d,i);i+=l;r[f].className=s[k]||''}
else if(o===O.A){let[f,fl]=rv(d,i);i+=fl;let[ak,al]=rv(d,i);i+=al;let[vk,vl]=rv(d,i);i+=vl;let an=A[ak]||s[ak]||'data';r[f].setAttribute(an,s[vk]||'')}
else if(o===O.AE){let[f,fl]=rv(d,i);i+=fl;let k=d[i++],v=d[i++];r[f].setAttribute(AT[k]||'data',AV[v]||'')}
else if(o===O.AB){let[f,fl]=rv(d,i);i+=fl;let k=d[i++];r[f].setAttribute(AT[k]||'data','')}
else if(o===O.AK){let[f,fl]=rv(d,i);i+=fl;let k=d[i++],[v,l]=rv(d,i);i+=l;r[f].setAttribute(AT[k]||'data',s[v]||'')}
else if(o===O.D){let[f,fl]=rv(d,i);i+=fl;let[kk,kl]=rv(d,i);i+=kl;let[vk,vl]=rv(d,i);i+=vl;r[f].dataset[s[kk]||'']=s[vk]||''}
else if(o===O.P){let[p,pl]=rv(d,i);i+=pl;let[c,cl]=rv(d,i);i+=cl;(p<0xFFFF?r[p]:document.body).appendChild(r[c])}
else if(o===O.CC){let[f,fl]=rv(d,i);i+=fl;fm();let lv=r[f];let sh=document.createElement(lv.tagName||'DIV');pm={live:lv,shadow:sh};r[f]=sh}
else if(o===O.B){let[f,fl]=rv(d,i);i+=fl;let t=d[i++];let[h,hl]=rv(d,i);i+=hl;BL(f,t,h,r)}
else if(o===O.R){let[f,fl]=rv(d,i);i+=fl;let t=d[i++];let[h,hl]=rv(d,i);i+=hl;r[f].__hk='r'+t+'_'+h;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();snd(()=>se(h,t,f,e,r[f]),e,r[f])})}
else if(o===O.DB){let[f,fl]=rv(d,i);i+=fl;let t=d[i++];let[h,hl]=rv(d,i);i+=hl;let ms=(d[i++]<<8)|d[i++];let tm;r[f].__hk='d'+t+'_'+h;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();clearTimeout(tm);tm=setTimeout(()=>se(h,t,f,e,r[f]),ms)})}
else if(o===O.RP){let[f,fl]=rv(d,i);i+=fl;let t=d[i++];let[h,hl]=rv(d,i);i+=hl;let pl=d[i++],prm=d.slice(i,i+pl);i+=pl;r[f].__hk='p'+t+'_'+h+'_'+prm.join(',');r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();snd(()=>sep(h,t,f,prm,e,r[f]),e,r[f])})}
else if(o===O.IL||o===O.DH){i=xi(d,i-1)}
else if(o===O.RU){let[k,l]=rv(d,i);i+=l;history.pushState(null,'',s[k])}
else if(o===O.RR){let[k,l]=rv(d,i);i+=l;history.replaceState(null,'',s[k])}
else if(o===O.SS){let[f,fl]=rv(d,i);i+=fl;let[k,l]=rv(d,i);i+=l;r[f].style.cssText=s[k]||''}
else if(o===O.SU){let[f,fl]=rv(d,i);i+=fl;let[u,l]=rv(d,i);i+=l;r[f].classList.add('u'+u)}
else if(o===O.SP){let[f,fl]=rv(d,i);i+=fl;let p=d[i++],v=d[i++];r[f].style[P[p]]=Y[v]}
else if(o===O.SM){let[f,fl]=rv(d,i);i+=fl;let n=d[i++];while(n--){let[u,l]=rv(d,i);i+=l;r[f].classList.add('u'+u)}}
else if(o===O.CT){let[n,l]=rv(d,i);i+=l;while(n--){let[id,il]=rv(d,i);i+=il;let c=d[i++];while(c--){let[u,ul]=rv(d,i);i+=ul}K[id]='c'+id}}
else if(o===O.SD){let[n,l]=rv(d,i);i+=l;if(!DS){DS=document.createElement('style');document.head.appendChild(DS)}while(n--){let[rl,ll]=rv(d,i);i+=ll;let rule=new TextDecoder().decode(d.slice(i,i+rl));i+=rl;try{DS.sheet.insertRule(rule,DS.sheet.cssRules.length)}catch(_e){DS.textContent+=rule}}}
else if(o===O.SC){let[f,fl]=rv(d,i);i+=fl;let[id,l]=rv(d,i);i+=l;r[f].classList.add(K[id]||'c'+id)}
else if(o===O.PD){let[f,fl]=rv(d,i);i+=fl;let pc=d[i++],n=d[i++];while(n--){let[u,l]=rv(d,i);i+=l;r[f].classList.add('h'+pc+'u'+u)}}
else if(o===O.BP){let[f,fl]=rv(d,i);i+=fl;let bp=d[i++],n=d[i++];while(n--){let[u,l]=rv(d,i);i+=l;r[f].classList.add('b'+bp+'u'+u)}}
else if(o===O.IT){if(typeof fl2!=='undefined'){fl2[d[i]]=!!d[i+1]}i+=2}
else if(o===O.BT){let[f,l]=rv(d,i);i+=l;let ti=d[i++];let[st,sl]=rv(d,i);i+=sl;let inv=d[i++];if(typeof fb2!=='undefined'){(fb2[ti]||(fb2[ti]=[])).push({e:r[f],s:st,n:!!inv});uf2(ti)}}
else if(o===O.TG){let[f,l]=rv(d,i);i+=l;let t=d[i++],ti=d[i++];if(typeof fl2!=='undefined'){r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();fl2[ti]=!fl2[ti];uf2(ti)})}}
else if(o===O.IS){if(typeof sl2!=='undefined'){sl2[d[i]]=d[i+1]}i+=2}
else if(o===O.BS){let[f,l]=rv(d,i);i+=l;let si=d[i++],mv=d[i++];let[st,sl]=rv(d,i);i+=sl;if(typeof sb2!=='undefined'){(sb2[si]||(sb2[si]=[])).push({e:r[f],v:mv,s:st});us2(si)}}
else if(o===O.SS2){let[f,l]=rv(d,i);i+=l;let t=d[i++],si=d[i++],sv=d[i++];if(typeof sl2!=='undefined'){r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();sl2[si]=sv;us2(si)})}}
else if(o===O.TT){let[f,l]=rv(d,i);i+=l;let t=d[i++],ti=d[i++],ms=(d[i++]<<8)|d[i++];if(typeof fl2!=='undefined'){let tm;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();clearTimeout(tm);fl2[ti]=true;uf2(ti);tm=setTimeout(()=>{fl2[ti]=false;uf2(ti)},ms)})}}
else if(o===O.AT2){let ti=d[i++],ms=(d[i++]<<8)|d[i++];if(typeof fl2!=='undefined'){setTimeout(()=>{fl2[ti]=!fl2[ti];uf2(ti)},ms)}}
else if(o===O.E){fm();if(ai){let ne=document.getElementById(ai);if(ne){if(av!==null&&(ne.tagName==='INPUT'||ne.tagName==='TEXTAREA')&&ne.value!==av)ne.value=av;if(ne!==document.activeElement)ne.focus();try{ne.setSelectionRange(ap,aq)}catch(_){}}}return}
else{console.error('Unknown opcode 0x'+o.toString(16)+' at pos '+_p+' after '+_oc+' ops, r.len='+r.length)}
}}catch(e){console.error('PARSE ERROR at pos='+i+' op#'+_oc+' opcode=0x'+(d[i-1]||0).toString(16)+' r.len='+r.length+': '+e.message);console.error('Context:',Array.from(d.slice(Math.max(0,i-10),i+10)).map(b=>'0x'+b.toString(16).padStart(2,'0')).join(' '))}}
function sh(h){if(!h)return;let id=h.slice(1);if(!id)return;let ts=()=>{let el=document.getElementById(id);if(el){el.scrollIntoView({behavior:'smooth'});return true}return false};if(!ts()){let ob=new MutationObserver(()=>{if(ts())ob.disconnect()});ob.observe(document.body,{childList:true,subtree:true});setTimeout(()=>ob.disconnect(),2000)}}
if('scrollRestoration' in history)history.scrollRestoration='manual';
let rc=0,rn=false;
function connect(){
w=new WebSocket((location.protocol==='https:'?'wss://':'ws://')+location.host);
w.binaryType='arraybuffer';
w.onopen=()=>{
if(rn){document.body.querySelectorAll(':scope>:not(script):not(style)').forEach(c=>c.remove());s={};wt=[];K={};sc=0;if(typeof ls!=='undefined'){ls={};lh={}}if(typeof fl2!=='undefined'){fl2={};fb2={};sl2={};sb2={}}}
rn=false;rc=0;
if(location.pathname!=='/')w.send('R'+location.pathname);
if(location.hash)sh(location.hash)};
w.onmessage=e=>x(new Uint8Array(e.data));
w.onclose=()=>{rn=true;setTimeout(connect,Math.min(1000*Math.pow(2,rc++),30000))};
w.onerror=()=>{}}
connect();
document.addEventListener('visibilitychange',()=>{if(!document.hidden&&w.readyState>1){rc=0;connect()}});
document.addEventListener('click',e=>{let a=e.target.closest('a[data-route]');if(a){e.preventDefault();let h=a.getAttribute('href');history.pushState(null,'',h);w.send('R'+h);let hs=h.indexOf('#');if(hs>=0)sh(h.slice(hs));else scrollTo(0,0)}let b=e.target.closest('[data-copy]');if(b){navigator.clipboard.writeText(b.dataset.copy);b.classList.add('copied');setTimeout(()=>b.classList.remove('copied'),2000)}});
window.addEventListener('popstate',()=>{w.send('R'+location.pathname);if(location.hash)sh(location.hash);else scrollTo(0,0)});"#;

/// Client actions JS (~250 bytes): targets (bool toggle) and selectors (exclusive enum).
///
/// - `fl2[idx]` = bool state, `fb2[idx]` = [{e: element, s: st_code, n: invert}]
/// - `sl2[idx]` = u8 state, `sb2[idx]` = [{e: element, v: match_value, s: st_code}]
/// - `uf2(idx)` = update all target bindings for index
/// - `us2(idx)` = update all selector bindings for index
///
/// Conditionally included when `has_client_actions` is true.
const CLIENT_ACTIONS_JS: &str = r#"let fl2={},fb2={},sl2={},sb2={};
function uf2(i){let v=fl2[i];for(let b of fb2[i]||[]){if(v!==b.n)b.e.classList.add('u'+b.s);else b.e.classList.remove('u'+b.s)}}
function us2(i){let v=sl2[i];for(let b of sb2[i]||[]){if(v===b.v)b.e.classList.add('u'+b.s);else b.e.classList.remove('u'+b.s)}}"#;

/// Bind handler (sends to server via WebSocket).
/// Also includes a stub xi() since the main runtime references it.
const BIND_JS: &str = r#"function BL(f,t,h,r){r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();snd(()=>se(h,t,f,e,r[f]),e,r[f])})}
function xi(d,i){return i}"#;

/// Generate a minimal (unstyled) capsule HTML with the full element/event maps.
///
/// The small u8 enum maps are shipped whole (see `generate_js_map`).
pub fn generate_capsule() -> String {
    let elements_js = generate_js_map(ELEMENT_MAPPINGS);
    let events_js = generate_js_map(EVENT_MAPPINGS);
    let svg_js = generate_svg_set();

    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head><body>
<script>
const E={{{elements_js}}};
const V={{{events_js}}};
const P={{}};
const Y={{}};
const AT={{}};
const AV={{}};
const SE={{{svg_js}}};
{BIND_JS}
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

/// How the browser should display text while the font loads.
#[derive(Clone, Debug, Default)]
pub enum FontDisplay {
    /// Show fallback immediately, swap when loaded (recommended).
    #[default]
    Swap,
    /// Brief invisible period, then fallback.
    Fallback,
    /// Use font only if already cached.
    Optional,
}

/// A font face definition that generates CSS at capsule build time.
///
/// Supports Google Fonts (via `@import`) and self-hosted fonts (via `@font-face`).
///
/// # Example
///
/// ```ignore
/// // Google Fonts
/// FontFace::google("Inter", &[400, 600])
/// FontFace::google("Fira Code", &[400])
///
/// // Self-hosted
/// FontFace::custom("MyFont")
///     .src("/fonts/myfont.woff2", "woff2")
///     .weight(400)
/// ```
#[derive(Clone, Debug)]
pub struct FontFace {
    family: String,
    source: FontSource,
    display: FontDisplay,
}

#[derive(Clone, Debug)]
enum FontSource {
    /// Google Fonts CDN import.
    Google { weights: Vec<u16> },
    /// Self-hosted font file.
    Custom { url: String, format: String, weight: u16 },
}

impl FontFace {
    /// Create a Google Fonts import.
    ///
    /// Generates `@import url('https://fonts.googleapis.com/css2?family=...')`.
    pub fn google(family: &str, weights: &[u16]) -> Self {
        Self {
            family: family.to_string(),
            source: FontSource::Google { weights: weights.to_vec() },
            display: FontDisplay::Swap,
        }
    }

    /// Create a self-hosted font face definition.
    pub fn custom(family: &str) -> Self {
        Self {
            family: family.to_string(),
            source: FontSource::Custom {
                url: String::new(),
                format: "woff2".to_string(),
                weight: 400,
            },
            display: FontDisplay::Swap,
        }
    }

    /// Set the font source URL and format (for self-hosted fonts).
    pub fn src(mut self, url: &str, format: &str) -> Self {
        if let FontSource::Custom { url: ref mut u, format: ref mut f, .. } = self.source {
            *u = url.to_string();
            *f = format.to_string();
        }
        self
    }

    /// Set the font weight (for self-hosted fonts).
    pub fn weight(mut self, weight: u16) -> Self {
        if let FontSource::Custom { weight: ref mut w, .. } = self.source {
            *w = weight;
        }
        self
    }

    /// Set the font-display strategy.
    pub fn display(mut self, display: FontDisplay) -> Self {
        self.display = display;
        self
    }

    /// Get the font family name.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Generate CSS for this font face.
    pub fn to_css(&self) -> String {
        let display = match self.display {
            FontDisplay::Swap => "swap",
            FontDisplay::Fallback => "fallback",
            FontDisplay::Optional => "optional",
        };

        match &self.source {
            FontSource::Google { weights } => {
                let encoded_family = self.family.replace(' ', "+");
                let wght = weights
                    .iter()
                    .map(|w| w.to_string())
                    .collect::<Vec<_>>()
                    .join(";");
                format!(
                    "@import url('https://fonts.googleapis.com/css2?family={}:wght@{}&display={}');\n",
                    encoded_family, wght, display
                )
            }
            FontSource::Custom { url, format, weight } => {
                format!(
                    "@font-face{{font-family:'{}';src:url('{}') format('{}');font-weight:{};font-display:{}}}\n",
                    self.family, url, format, weight, display
                )
            }
        }
    }
}

/// Configuration for capsule generation with styling.
#[derive(Clone, Debug, Default)]
pub struct CapsuleConfig {
    /// Theme configuration for CSS variables
    pub theme: Theme,
    /// Whether any client actions (targets/selectors) are used
    pub(crate) has_client_actions: bool,
    /// Pre-generated composite CSS from style grouping (`.c{id}{declarations}`)
    pub(crate) composite_css: String,
    /// Registered font faces for the capsule.
    pub fonts: Vec<FontFace>,
}

impl CapsuleConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a light theme with default colors.
    pub fn light() -> Self {
        Self::default().theme(Theme::light())
    }

    /// Create a dark theme with default colors.
    pub fn dark() -> Self {
        Self::default().theme(Theme::dark())
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set whether client actions (targets/selectors) are used.
    pub(crate) fn has_client_actions(mut self, has: bool) -> Self {
        self.has_client_actions = has;
        self
    }

    /// Set pre-generated composite CSS from style grouping analysis.
    pub(crate) fn with_composite_css(mut self, css: String) -> Self {
        self.composite_css = css;
        self
    }

    /// Register a font face for the capsule.
    ///
    /// The font's CSS (`@import` or `@font-face`) is generated at capsule
    /// build time. Reference the font via inline `style` attributes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// CapsuleConfig::new()
    ///     .font(FontFace::google("Inter", &[400, 600]))
    ///     .font(FontFace::google("Fira Code", &[400]))
    /// ```
    pub fn font(mut self, face: FontFace) -> Self {
        self.fonts.push(face);
        self
    }
}

/// Generate complete CSS for the capsule with resolved theme.
///
/// This is the **static** CSS embedded in the capsule `<style>` tag: the parts
/// that must exist before any class is applied. Class-referenced utility, pseudo
/// and breakpoint rules (`.u`/`.h`/`.b`) are NOT here — they are delivered lazily
/// over the wire via `STYLE_DEF` the first time a connection uses them. See
/// `docs/tree-shaking-redesign.md` (Phase 2).
///
/// Includes:
/// - Base reset CSS
/// - All non-color + color CSS variables (shipped whole, since lazy rules
///   reference them and the set is small and bounded)
/// - Theme `:root` semantic vars
/// - Keyframes referenced by animation utilities
/// - Composite classes (`.c{id}`), whose id set is fixed by startup analysis
pub fn generate_capsule_css(config: &CapsuleConfig) -> String {
    use crate::style_tokens::PSEUDO_GLOBAL_CSS;
    use crate::theme::{generate_base_css, generate_theme_css};
    use crate::tokens::css::{generate_color_css_all, generate_noncolor_primitive_css};

    let mut css = String::with_capacity(8192);

    // 1. Base reset (must come first).
    css.push_str(generate_base_css());

    // 2. All CSS variables (no tree-shaking): lazy rules arrive after load and
    //    reference these, so every var must already be defined. The full set is
    //    small and bounded (spacing/radius/type/shadow scales + ~62 colors).
    css.push_str(&generate_noncolor_primitive_css());
    css.push_str(&generate_color_css_all());

    // 3. Theme :root semantic vars (mode-aware colors, style preset Q-vars, radius).
    css.push_str(&generate_theme_css(&config.theme));

    // 4. Keyframes referenced by animation utilities — global, so always shipped.
    css.push_str(PSEUDO_GLOBAL_CSS);

    // 5. Composite classes (.c{id}). The emittable id set is fixed by the startup
    //    pattern analysis, so these never have the lazy/plain-helper gap.
    if !config.composite_css.is_empty() {
        css.push_str(&config.composite_css);
    }

    // 10. Font face CSS (@import and @font-face rules)
    if !config.fonts.is_empty() {
        // @import rules must appear before all other rules in CSS.
        // Collect them separately and prepend to the output.
        let mut imports = String::new();
        let mut font_faces = String::new();
        for font in &config.fonts {
            let font_css = font.to_css();
            if font_css.starts_with("@import") {
                imports.push_str(&font_css);
            } else {
                font_faces.push_str(&font_css);
            }
        }
        if !font_faces.is_empty() {
            css.push_str(&font_faces);
        }
        if !imports.is_empty() {
            css.insert_str(0, &imports);
        }
    }

    css
}

/// Render a self-contained HTML page for a tree built with the `el()` builder, for
/// content shown *before* the WebSocket capsule exists (e.g. an auth login).
///
/// Unlike the live capsule — which embeds only base/composite CSS and delivers
/// utility rules lazily over the wire — this inlines everything the page needs:
/// the capsule's static CSS (reset, variables, theme `:root`, composite classes)
/// plus the utility/pseudo/breakpoint rules for exactly the tokens `body` uses. So
/// `.st(..)`, `.hover`/`.focus`, and theme colors all resolve with no runtime and
/// no hand-written CSS.
pub fn render_static_page(
    config: &CapsuleConfig,
    title: &str,
    body: &crate::builder::ElementBuilder,
) -> String {
    let mut css = generate_capsule_css(config);
    let mut keys = std::collections::BTreeSet::new();
    body.collect_style_keys(&mut keys);
    for key in keys {
        if let Some(rule) = key.to_css_rule() {
            css.push_str(&rule);
        }
    }
    let title = title.replace('&', "&amp;").replace('<', "&lt;");
    let body_html = body.to_static_html();
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title}</title>\
<style>{css}</style></head><body>{body_html}</body></html>"
    )
}

/// Generate a capsule with styling support.
///
/// This is the recommended way to generate capsules for styled applications.
/// Includes:
/// - Full element/event mappings (small u8 enum maps shipped whole)
/// - Theme data attributes on root element
/// - CSS embedded in `<style>` tag (composite + global CSS; utility/pseudo/
///   breakpoint rules are delivered lazily over the wire)
///
/// CSS is embedded directly in the capsule HTML `<style>` tag within `<head>`.
/// This ensures styles are available immediately when the page loads,
/// without waiting for the WebSocket connection.
pub fn generate_styled_capsule(config: &CapsuleConfig, css: &str) -> String {
    use crate::attr_tokens::{AT_MAPPINGS, AV_MAPPINGS};
    use crate::style_tokens::{PROP_MAPPINGS, VALUE_MAPPINGS};

    // Small u8 token enums are shipped whole (not tree-shaken) — see generate_js_map.
    let elements_js = generate_js_map(ELEMENT_MAPPINGS);
    let events_js = generate_js_map(EVENT_MAPPINGS);
    let svg_js = generate_svg_set();

    // Style token lookup tables for inline STYLE_PROP (property/value name maps).
    // Note: U (utility) map removed - utilities now use CSS classes generated server-side.
    let props_js = generate_js_map(PROP_MAPPINGS);
    let values_js = generate_js_map(VALUE_MAPPINGS);

    // Attribute lookup tables.
    let attr_keys_js = generate_js_map(AT_MAPPINGS);
    let attr_values_js = generate_js_map(AV_MAPPINGS);

    // Include client actions JS (targets & selectors) when used
    let client_actions_js = if config.has_client_actions {
        CLIENT_ACTIONS_JS
    } else {
        ""
    };

    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<style>{css}</style></head><body>
<div id="rw"></div>
<script>
const E={{{elements_js}}};
const V={{{events_js}}};
const P={{{props_js}}};
const Y={{{values_js}}};
const AT={{{attr_keys_js}}};
const AV={{{attr_values_js}}};
const SE={{{svg_js}}};
{client_actions_js}
{BIND_JS}
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    #[test]
    fn test_capsule_config_defaults() {
        let config = CapsuleConfig::new();
        assert_eq!(config.theme.mode, ThemeMode::Light);
        assert!(config.theme.palette_ref().is_none());
    }

    #[test]
    fn test_capsule_css_generation() {
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);

        // Should contain base reset
        assert!(css.contains("box-sizing"));
        // Should contain resolved semantic tokens with short names
        assert!(css.contains("--a:"), "Missing --a (bg-app)");
        assert!(css.contains("--k:"), "Missing --k (text-default)");
    }

    #[test]
    fn test_css_variables_resolved() {
        // Verify the resolved theme pipeline: semantic vars use short names with
        // direct oklch values, and all primitive vars are shipped (lazy CSS needs
        // every var present up-front).

        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);

        // Non-color primitives are all shipped now.
        assert!(css.contains("--S4:"), "Missing --S4 (space-4)");
        assert!(css.contains("--R3:"), "Missing --R3 (radius-lg)");
        assert!(css.contains("--X3:"), "Missing --X3 (leading-normal, from base CSS)");
        assert!(css.contains("--Z1:"), "Missing --Z1 (shadow-sm)");

        // Resolved semantic tokens use short names with direct oklch values
        assert!(css.contains("--a:oklch("), "Semantic --a should have resolved oklch value");
        assert!(css.contains("--k:oklch("), "Semantic --k should have resolved oklch value");
    }

    #[test]
    fn test_styled_capsule_structure() {
        let config = CapsuleConfig::new()
            .theme(Theme::dark().accent("#00FF00"));

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Should have HTML structure
        assert!(capsule.contains("<!DOCTYPE html>"));

        // CSS should be embedded in <style> tag
        assert!(capsule.contains("<style>"));
        assert!(capsule.contains("box-sizing"));

        // No data-theme/data-style/data-palette attributes (theme-as-state handles CSS vars)
        assert!(!capsule.contains("data-theme"), "data-theme should not be emitted");
        assert!(!capsule.contains("data-accent"), "data-accent should not be emitted");

        // Should have div#rw for app root
        assert!(capsule.contains("<div id=\"rw\""));

        // Should have element/event mappings
        assert!(capsule.contains("const E="));
        assert!(capsule.contains("const V="));
    }

    #[test]
    fn test_styled_capsule_contains_css() {
        let config = CapsuleConfig::new();

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // CSS should be in <style> tag within <head>
        assert!(capsule.contains("<style>"));
        assert!(capsule.contains("</style></head>"));

        // Should contain resolved semantic var --a (bg-app)
        assert!(capsule.contains("--a:"), "Missing resolved --a (bg-app)");
    }

    #[test]
    fn test_styled_capsule_size() {
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Capsule includes CSS in <style> tag - should be reasonable size
        println!("Styled capsule size: {} bytes (CSS embedded)", capsule.len());
    }

    #[test]
    fn test_element_map_ships_all() {
        // The element map is now shipped whole (no tree-shaking) so a token
        // reached only through a plain helper can never be missing.
        let map = generate_js_map(ELEMENT_MAPPINGS);
        assert!(map.contains("0:'div'"));
        assert!(map.contains("2:'button'"));
        assert!(map.contains("1:'span'"), "full map must include span");
    }

    #[test]
    fn test_event_map_ships_all() {
        let map = generate_js_map(EVENT_MAPPINGS);
        assert!(map.contains("1:'click'"));
        // More than one event is present now that the map is full.
        assert!(map.contains("7:'input'"));
    }

    #[test]
    fn test_generate_capsule() {
        let capsule = generate_capsule();
        // Maps are shipped whole now, so they contain every entry (not just div/click).
        assert!(capsule.contains("0:'div'"));
        assert!(capsule.contains("1:'span'"));
        assert!(capsule.contains("1:'click'"));
        assert!(capsule.contains("<!DOCTYPE html>"));
        assert!(!capsule.contains("let ls={},lh={}"));
    }

    #[test]
    fn test_capsule_config_light() {
        let config = CapsuleConfig::light();
        assert_eq!(config.theme.mode, ThemeMode::Light);
        assert!(config.theme.palette_ref().is_none());
    }

    #[test]
    fn test_capsule_config_dark() {
        let config = CapsuleConfig::dark();
        assert_eq!(config.theme.mode, ThemeMode::Dark);
        assert!(config.theme.palette_ref().is_none());
    }

    #[test]
    fn test_composite_css_included_in_capsule() {
        let config = CapsuleConfig::new()
            .with_composite_css(".c256{display:flex;flex-direction:column;gap:var(--S4)}".to_string());

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Composite CSS should be embedded in the <style> tag
        assert!(css.contains(".c256{"), "Composite CSS missing from generated CSS");
        assert!(capsule.contains(".c256{"), "Composite CSS missing from capsule HTML");
    }

    #[test]
    fn test_utility_css_is_lazy_not_static() {
        // Utility rules are delivered lazily over the wire (STYLE_DEF), so they
        // must NOT appear in the static capsule CSS. Composites stay static.
        let config = CapsuleConfig::new()
            .with_composite_css(".c256{display:flex;gap:1rem}".to_string());

        let css = generate_capsule_css(&config);

        assert!(!css.contains(".u2{"), "utility rule must be lazy, not in static CSS");
        assert!(css.contains(".c256{"), "composite CSS should be static");
    }

    #[test]
    fn test_empty_composite_css_not_appended() {
        let config = CapsuleConfig::new();
        assert!(config.composite_css.is_empty());

        let css = generate_capsule_css(&config);
        // Should not contain any composite class patterns
        assert!(!css.contains(".c256"), "Empty composite CSS should not produce .c256");
    }

    #[test]
    fn test_capsule_css_uses_theme_css() {
        let config = CapsuleConfig::new()
            .theme(Theme::dark().accent("#5E81AC"));
        let css = generate_capsule_css(&config);

        // Should contain a single :root{} block with resolved theme CSS
        assert!(css.contains(":root{"), "Missing :root{{ block");
        assert!(css.contains("--a:"), "Missing --a (bg-app)");
        assert!(css.contains("--k:"), "Missing --k (text-default)");
        // Should NOT contain old dual-mode or data-attribute selectors
        assert!(!css.contains("[data-theme"), "Should not contain data-theme selectors");
        assert!(!css.contains("[data-style"), "Should not contain data-style selectors");
        assert!(!css.contains("[data-palette"), "Should not contain data-palette selectors");
    }

    #[test]
    fn test_composite_css_vars_scanned() {
        // Composite CSS references var(--S6) which is not used by any utility token.
        // After the fix, generate_capsule_css should include --S6 in the :root block.
        let config = CapsuleConfig::new()
            .with_composite_css(".c1{gap:var(--S6)}".to_string());
        let css = generate_capsule_css(&config);

        assert!(
            css.contains("--S6:"),
            "Composite CSS var(--S6) should cause --S6 to be included in :root"
        );
    }

    #[test]
    fn test_client_actions_js_included_when_enabled() {
        let config = CapsuleConfig::new()
            .has_client_actions(true);
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Client actions JS should be included
        assert!(capsule.contains("fl2"), "Client actions JS vars should be present");
        assert!(capsule.contains("uf2"), "Target update function should be present");
        assert!(capsule.contains("us2"), "Selector update function should be present");
    }

    #[test]
    fn test_client_actions_js_excluded_when_disabled() {
        let config = CapsuleConfig::new()
            .has_client_actions(false);
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Client actions update functions should NOT be included
        assert!(!capsule.contains("function uf2"), "Target update function should not be present when disabled");
        assert!(!capsule.contains("function us2"), "Selector update function should not be present when disabled");
    }

    #[test]
    fn test_client_action_opcodes_in_runtime() {
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Opcode constants should always be in the runtime
        assert!(capsule.contains("IT:0x47"), "INIT_TARGET opcode should be in O object");
        assert!(capsule.contains("BT:0x48"), "BIND_TARGET opcode should be in O object");
        assert!(capsule.contains("TG:0x49"), "BIND_TOGGLE opcode should be in O object");
        assert!(capsule.contains("IS:0x4A"), "INIT_SELECTOR opcode should be in O object");
        assert!(capsule.contains("BS:0x4B"), "BIND_SELECTOR opcode should be in O object");
        assert!(capsule.contains("SS2:0x4C"), "BIND_SELECT opcode should be in O object");
    }

    #[test]
    fn test_timed_toggle_opcode_in_runtime() {
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        assert!(capsule.contains("TT:0x4D"), "BIND_TIMED_TOGGLE opcode should be in O object");
    }

    #[test]
    fn test_auto_toggle_opcode_in_runtime() {
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        assert!(capsule.contains("AT2:0x4E"), "AUTO_TOGGLE opcode should be in O object");
    }
}
