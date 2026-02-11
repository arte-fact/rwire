//! Generate minimal capsule HTML with tree-shaken element and event mappings.
//!
//! This module generates a capsule that contains only the element types and
//! event types actually used by the application, reducing the capsule size.
//!
//! For styled capsules, CSS is embedded in a `<style>` tag within `<head>`,
//! tree-shaken to include only the styles for components actually used.

use std::collections::HashSet;

use crate::protocol::opcodes::{ELEMENT_MAPPINGS, EVENT_MAPPINGS, SVG_ELEMENT_CODES};
use crate::protocol::El;
use crate::style_tokens::St;
use crate::theme::Theme;

/// Generate a tree-shaken JS object literal from a mappings array.
///
/// Filters `mappings` to include only entries whose code is in `used`,
/// then formats as `{code:'string', ...}` entries joined by commas.
fn generate_js_map<T: std::fmt::Display + std::hash::Hash + Eq>(
    mappings: &[(T, &str)],
    used: &HashSet<T>,
) -> String {
    let entries: Vec<String> = mappings
        .iter()
        .filter(|(code, _)| used.contains(code))
        .map(|(code, name)| format!("{}:'{}'", code, name))
        .collect();
    entries.join(",")
}

/// Generate a tree-shaken SVG element type set `{code:1, ...}`.
///
/// Only includes SVG element codes that are in `used_elements`.
fn generate_svg_set(used_elements: &HashSet<u8>) -> String {
    let entries: Vec<String> = SVG_ELEMENT_CODES
        .iter()
        .filter(|code| used_elements.contains(code))
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
const RUNTIME_JS: &str = r#"const O={S:0xF0,SE:0xF1,WT:0xF2,G:0x01,C:0x02,CS:0x03,GS:0x05,L:0x10,T:0x11,TW:0x13,D:0x14,TI:0x15,A:0x12,P:0x20,CC:0x25,AE:0x26,AB:0x27,AK:0x28,B:0x30,R:0x31,DB:0x33,RP:0x34,IL:0x40,DH:0x42,IT:0x47,BT:0x48,TG:0x49,IS:0x4A,BS:0x4B,SS2:0x4C,TT:0x4D,AT2:0x4E,RU:0x70,RR:0x71,SS:0x81,SU:0x82,SP:0x83,SM:0x84,SC:0x85,CT:0x86,PD:0x89,BP:0x8A,E:0xFF};
const A={4:'id'};
let s={},wt=[],w,sc=0,K={};
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
function x(d){
let r=[],i=0,_oc=0;
let ae=document.activeElement,ai=ae&&ae.id,ap=ae?ae.selectionStart:0;
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
else if(o===O.CC){let[f,fl]=rv(d,i);i+=fl;r[f].innerHTML=''}
else if(o===O.B){let[f,fl]=rv(d,i);i+=fl;let t=d[i++];let[h,hl]=rv(d,i);i+=hl;BL(f,t,h,r)}
else if(o===O.R){let[f,fl]=rv(d,i);i+=fl;let t=d[i++];let[h,hl]=rv(d,i);i+=hl;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();se(h,t,f,e,r[f])})}
else if(o===O.DB){let[f,fl]=rv(d,i);i+=fl;let t=d[i++];let[h,hl]=rv(d,i);i+=hl;let ms=(d[i++]<<8)|d[i++];let tm;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();clearTimeout(tm);tm=setTimeout(()=>se(h,t,f,e,r[f]),ms)})}
else if(o===O.RP){let[f,fl]=rv(d,i);i+=fl;let t=d[i++];let[h,hl]=rv(d,i);i+=hl;let pl=d[i++],prm=d.slice(i,i+pl);i+=pl;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();sep(h,t,f,prm,e,r[f])})}
else if(o===O.IL||o===O.DH){i=xi(d,i-1)}
else if(o===O.RU){let[k,l]=rv(d,i);i+=l;history.pushState(null,'',s[k])}
else if(o===O.RR){let[k,l]=rv(d,i);i+=l;history.replaceState(null,'',s[k])}
else if(o===O.SS){let[f,fl]=rv(d,i);i+=fl;let[k,l]=rv(d,i);i+=l;r[f].style.cssText=s[k]||''}
else if(o===O.SU){let[f,fl]=rv(d,i);i+=fl;let[u,l]=rv(d,i);i+=l;r[f].classList.add('u'+u)}
else if(o===O.SP){let[f,fl]=rv(d,i);i+=fl;let p=d[i++],v=d[i++];r[f].style[P[p]]=Y[v]}
else if(o===O.SM){let[f,fl]=rv(d,i);i+=fl;let n=d[i++];while(n--){let[u,l]=rv(d,i);i+=l;r[f].classList.add('u'+u)}}
else if(o===O.CT){let[n,l]=rv(d,i);i+=l;while(n--){let[id,il]=rv(d,i);i+=il;let c=d[i++];while(c--){let[u,ul]=rv(d,i);i+=ul}K[id]='c'+id}}
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
else if(o===O.E){if(ai){let ne=document.getElementById(ai);if(ne&&ne!==document.activeElement){ne.focus();try{ne.setSelectionRange(ap,ap)}catch(_){}}}return}
else{console.error('Unknown opcode 0x'+o.toString(16)+' at pos '+_p+' after '+_oc+' ops, r.len='+r.length)}
}}catch(e){console.error('PARSE ERROR at pos='+i+' op#'+_oc+' opcode=0x'+(d[i-1]||0).toString(16)+' r.len='+r.length+': '+e.message);console.error('Context:',Array.from(d.slice(Math.max(0,i-10),i+10)).map(b=>'0x'+b.toString(16).padStart(2,'0')).join(' '))}}
function sh(h){if(!h)return;let id=h.slice(1);if(!id)return;let ts=()=>{let el=document.getElementById(id);if(el){el.scrollIntoView({behavior:'smooth'});return true}return false};if(!ts()){let ob=new MutationObserver(()=>{if(ts())ob.disconnect()});ob.observe(document.body,{childList:true,subtree:true});setTimeout(()=>ob.disconnect(),2000)}}
if('scrollRestoration' in history)history.scrollRestoration='manual';
let rc=0,rn=false;
function connect(){
w=new WebSocket('ws://'+location.host);
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
const BIND_JS: &str = r#"function BL(f,t,h,r){r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();se(h,t,f,e,r[f])})}
function xi(d,i){return i}"#;

/// Generate a minimal capsule HTML with only the used element and event types.
pub fn generate_capsule(
    used_elements: &HashSet<u8>,
    used_events: &HashSet<u8>,
) -> String {
    let elements_js = generate_js_map(ELEMENT_MAPPINGS, used_elements);
    let events_js = generate_js_map(EVENT_MAPPINGS, used_events);
    let svg_js = generate_svg_set(used_elements);

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
    /// Used style utility tokens (for tree-shaking)
    pub(crate) used_style_utils: HashSet<u16>,
    /// Used style property codes (for tree-shaking)
    pub(crate) used_style_props: HashSet<u8>,
    /// Used style value codes (for tree-shaking)
    pub(crate) used_style_values: HashSet<u8>,
    /// Used pseudo-class (Pc, St) pairs (for tree-shaking)
    pub(crate) used_pseudo_pairs: HashSet<(u8, u16)>,
    /// Used breakpoint (Bp, St) pairs (for tree-shaking)
    pub(crate) used_breakpoint_pairs: HashSet<(u8, u16)>,
    /// Used attribute key codes (for tree-shaking)
    pub(crate) used_attr_keys: HashSet<u8>,
    /// Used attribute value codes (for tree-shaking)
    pub(crate) used_attr_values: HashSet<u8>,
    /// Whether any client actions (targets/selectors) are used
    pub(crate) has_client_actions: bool,
    /// Pre-generated composite CSS from style grouping (`.c{id}{declarations}`)
    pub(crate) composite_css: String,
    /// Extra element types to include in the capsule beyond what tree-shaking discovers.
    pub(crate) extra_elements: HashSet<u8>,
    /// Extra style utility tokens to include beyond what tree-shaking discovers.
    pub(crate) extra_style_utils: HashSet<u16>,
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

    /// Set all used style utility tokens (from build context).
    pub(crate) fn with_style_utils(mut self, utils: &HashSet<u16>) -> Self {
        self.used_style_utils = utils.clone();
        self
    }

    /// Set all used style property codes (from build context).
    pub(crate) fn with_style_props(mut self, props: &HashSet<u8>) -> Self {
        self.used_style_props = props.clone();
        self
    }

    /// Set all used style value codes (from build context).
    pub(crate) fn with_style_values(mut self, values: &HashSet<u8>) -> Self {
        self.used_style_values = values.clone();
        self
    }

    /// Set all used pseudo-class (Pc, St) pairs (from build context).
    pub(crate) fn with_pseudo_pairs(mut self, pairs: &HashSet<(u8, u16)>) -> Self {
        self.used_pseudo_pairs = pairs.clone();
        self
    }

    /// Set all used breakpoint (Bp, St) pairs (from build context).
    pub(crate) fn with_breakpoint_pairs(mut self, pairs: &HashSet<(u8, u16)>) -> Self {
        self.used_breakpoint_pairs = pairs.clone();
        self
    }

    /// Set all used attribute key codes (from build context).
    pub(crate) fn with_attr_keys(mut self, keys: &HashSet<u8>) -> Self {
        self.used_attr_keys = keys.clone();
        self
    }

    /// Set all used attribute value codes (from build context).
    pub(crate) fn with_attr_values(mut self, values: &HashSet<u8>) -> Self {
        self.used_attr_values = values.clone();
        self
    }

    /// Set pre-generated composite CSS from style grouping analysis.
    pub(crate) fn with_composite_css(mut self, css: String) -> Self {
        self.composite_css = css;
        self
    }

    /// Declare extra element types that should be included in the capsule
    /// beyond what tree-shaking discovers from the initial render.
    ///
    /// Use this when your app creates element types dynamically (e.g.,
    /// markdown rendering uses `<table>`, `<pre>`, `<code>` etc. that
    /// aren't present on the initial page).
    pub fn extra_elements(mut self, elements: &[El]) -> Self {
        for el in elements {
            self.extra_elements.insert(el.as_u8());
        }
        self
    }

    /// Declare extra style utility tokens that should be included in the capsule
    /// beyond what tree-shaking discovers from the initial render.
    ///
    /// Use this when your app uses St tokens in conditional code paths
    /// (e.g., active sidebar links, markdown styling) that aren't exercised
    /// during the initial render.
    pub fn extra_styles(mut self, styles: &[St]) -> Self {
        for st in styles {
            self.extra_style_utils.insert(st.as_u16());
        }
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

    /// Check if any style tokens are used.
    pub fn has_style_tokens(&self) -> bool {
        !self.used_style_utils.is_empty()
            || !self.used_style_props.is_empty()
    }
}

/// Extract all CSS variable references from a CSS string.
///
/// Scans for `var(--..)` patterns and returns a set of variable names.
/// Tracks all custom property references (short-name primitives, color vars, etc.)
/// but excludes semantic vars (lowercase `--a` through `--z`, uppercase `--A`-`--L`,
/// `--n1`-`--n12`) since those are always emitted by the theme system.
fn extract_used_variables(css: &str) -> HashSet<String> {
    let mut vars = HashSet::new();

    for (idx, _) in css.match_indices("var(") {
        let rest = &css[idx + 4..]; // Skip "var("

        if let Some(end) = rest.find([',', ')']) {
            let var_name = rest[..end].trim();

            // Track short-name primitives and color vars:
            // S=spacing, R=radius, T=text, W=weight, X=leading, Z=shadow
            // N=neutral, U=blue, O=red, P=green, M=amber, Y=special
            // Q=component hooks
            if var_name.starts_with("--") && var_name.len() >= 4 {
                let suffix = &var_name[2..];
                let first = suffix.as_bytes()[0];
                if matches!(first, b'S' | b'R' | b'T' | b'W' | b'X' | b'Z'
                    | b'N' | b'U' | b'O' | b'P' | b'M' | b'Y' | b'Q') {
                    vars.insert(var_name.to_string());
                }
            }
        }
    }

    vars
}

/// Generate complete CSS for the capsule with resolved theme.
///
/// Includes:
/// - Base reset CSS
/// - Non-color primitive CSS (spacing, radius, typography, shadows)
/// - Resolved semantic CSS (light + dark, with direct oklch values)
/// - Color primitive CSS (tree-shaken: only palette colors referenced by St tokens)
/// - Theme style preset overrides
/// - Component CSS (tree-shaken utility, pseudo, breakpoint classes)
pub fn generate_capsule_css(config: &CapsuleConfig) -> String {
    use crate::style_tokens::{generate_utility_css, generate_pseudo_css, generate_breakpoint_css};
    use crate::theme::{generate_base_css, generate_theme_css};
    use crate::tokens::css::{generate_primitive_css_filtered, generate_color_css_filtered};

    let mut css = String::with_capacity(8192);

    // 1. Generate utility + pseudo + breakpoint token CSS (class-based, tree-shaken)
    let mut all_utils = config.used_style_utils.clone();
    all_utils.extend(&config.extra_style_utils);
    let utility_token_css = generate_utility_css(&all_utils);
    let pseudo_token_css = generate_pseudo_css(&config.used_pseudo_pairs);
    let breakpoint_token_css = generate_breakpoint_css(&config.used_breakpoint_pairs);

    // 2. Extract var(--...) references from token CSS rules.
    //    Short-name vars: S=spacing, R=radius, T=text, W=weight, X=leading, Z=shadow
    //    Color vars: N=neutral, U=blue, O=red, P=green, M=amber, Y=special
    //    Component hooks: Q=hooks
    let base_css = generate_base_css();
    let mut used_vars = extract_used_variables(base_css);
    used_vars.extend(extract_used_variables(&utility_token_css));
    used_vars.extend(extract_used_variables(&pseudo_token_css));
    used_vars.extend(extract_used_variables(&breakpoint_token_css));
    // Scan composite CSS for var() references (Concern 3: composite vars)
    used_vars.extend(extract_used_variables(&config.composite_css));

    // 3. Base reset (must come first)
    css.push_str(base_css);

    // 4. Non-color primitives (tree-shaken: S, R, T, W, X, Z prefixes)
    let primitive_vars: HashSet<String> = used_vars
        .iter()
        .filter(|v| {
            v.len() >= 4 && matches!(v.as_bytes()[2], b'S' | b'R' | b'T' | b'W' | b'X' | b'Z')
        })
        .cloned()
        .collect();
    if !primitive_vars.is_empty() {
        css.push_str(&generate_primitive_css_filtered(&primitive_vars));
    }

    // 5. Color primitives (tree-shaken: N, U, O, P, M, Y prefixes)
    let color_vars: HashSet<String> = used_vars
        .iter()
        .filter(|v| {
            v.len() >= 4 && matches!(v.as_bytes()[2], b'N' | b'U' | b'O' | b'P' | b'M' | b'Y')
        })
        .cloned()
        .collect();
    if !color_vars.is_empty() {
        css.push_str(&generate_color_css_filtered(&color_vars));
    }

    // 6. Theme CSS — single :root{...} block with mode-aware colors, style preset Q-vars,
    //    and radius overrides. Replaces the old dual light/dark blocks + [data-style] +
    //    [data-palette] + [data-radius] selectors.
    css.push_str(&generate_theme_css(&config.theme));

    // 7. Utility token CSS classes (.u{code}{declaration})
    css.push_str(&utility_token_css);

    // 8. Pseudo-class token CSS rules (.h{pc}u{st}:hover{...}, etc.)
    css.push_str(&pseudo_token_css);

    // 8b. Breakpoint token CSS rules (@media(min-width:...){.b{bp}u{st}{...}})
    css.push_str(&breakpoint_token_css);

    // 9. Composite style CSS classes (.c{id}{declarations})
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

/// Generate a capsule with styling support.
///
/// This is the recommended way to generate capsules for styled applications.
/// Includes:
/// - Tree-shaken element/event mappings
/// - Theme data attributes on root element
/// - CSS embedded in `<style>` tag (tree-shaken utility + semantic + theme CSS)
///
/// CSS is embedded directly in the capsule HTML `<style>` tag within `<head>`.
/// This ensures styles are available immediately when the page loads,
/// without waiting for the WebSocket connection.
pub fn generate_styled_capsule(
    used_elements: &HashSet<u8>,
    used_events: &HashSet<u8>,
    config: &CapsuleConfig,
    css: &str,
) -> String {
    use crate::attr_tokens::{AT_MAPPINGS, AV_MAPPINGS};
    use crate::style_tokens::{PROP_MAPPINGS, VALUE_MAPPINGS};

    let elements_js = generate_js_map(ELEMENT_MAPPINGS, used_elements);
    let events_js = generate_js_map(EVENT_MAPPINGS, used_events);
    let svg_js = generate_svg_set(used_elements);

    // Generate style token lookup tables (tree-shaken)
    // Note: U (utility) map removed - utilities now use CSS classes generated server-side
    let props_js = generate_js_map(PROP_MAPPINGS, &config.used_style_props);
    let values_js = generate_js_map(VALUE_MAPPINGS, &config.used_style_values);

    // Generate attribute lookup tables (tree-shaken)
    let attr_keys_js = generate_js_map(AT_MAPPINGS, &config.used_attr_keys);
    let attr_values_js = generate_js_map(AV_MAPPINGS, &config.used_attr_values);

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
        // Verify the resolved theme pipeline: semantic vars use short names
        // with direct oklch values, and primitives are tree-shaken.

        let mut used_utils = HashSet::new();
        used_utils.insert(0xC0); // BgApp -> var(--a)
        used_utils.insert(0x8B); // RoundedLg -> var(--R3)
        used_utils.insert(0xD1); // BorderSubtle -> var(--g)
        used_utils.insert(0x53); // PMd -> var(--S4)
        used_utils.insert(0xE9); // ShadowSm -> var(--Z1)

        let config = CapsuleConfig::new()
            .with_style_utils(&used_utils);
        let css = generate_capsule_css(&config);

        // Non-color primitives (tree-shaken: only used ones)
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
        let mut elements = HashSet::new();
        elements.insert(0); // div
        elements.insert(2); // button

        let mut events = HashSet::new();
        events.insert(1); // click

        let config = CapsuleConfig::new()
            .theme(Theme::dark().accent("#00FF00"));

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

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
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let mut events = HashSet::new();
        events.insert(1); // click

        let mut used_utils = HashSet::new();
        used_utils.insert(0xC0); // BgApp -> var(--a)

        let config = CapsuleConfig::new()
            .with_style_utils(&used_utils);

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        // CSS should be in <style> tag within <head>
        assert!(capsule.contains("<style>"));
        assert!(capsule.contains("</style></head>"));

        // Should contain resolved semantic var --a (bg-app)
        assert!(capsule.contains("--a:"), "Missing resolved --a (bg-app)");
    }

    #[test]
    fn test_styled_capsule_size() {
        let mut elements = HashSet::new();
        elements.insert(0); // div
        elements.insert(2); // button

        let mut events = HashSet::new();
        events.insert(1); // click

        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        // Capsule includes CSS in <style> tag - should be reasonable size
        println!("Styled capsule size: {} bytes (CSS embedded)", capsule.len());
    }

    #[test]
    fn test_minimal_element_map() {
        let mut used = HashSet::new();
        used.insert(0); // div
        used.insert(2); // button

        let map = generate_js_map(ELEMENT_MAPPINGS, &used);
        assert!(map.contains("0:'div'"));
        assert!(map.contains("2:'button'"));
        assert!(!map.contains("span"));
    }

    #[test]
    fn test_minimal_event_map() {
        let mut used = HashSet::new();
        used.insert(1); // click

        let map = generate_js_map(EVENT_MAPPINGS, &used);
        assert_eq!(map, "1:'click'");
    }

    #[test]
    fn test_generate_capsule() {
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let mut events = HashSet::new();
        events.insert(1); // click

        let capsule = generate_capsule(&elements, &events);
        assert!(capsule.contains("const E={0:'div'}"));
        assert!(capsule.contains("const V={1:'click'}"));
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
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let events = HashSet::new();

        let config = CapsuleConfig::new()
            .with_composite_css(".c256{display:flex;flex-direction:column;gap:var(--S4)}".to_string());

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        // Composite CSS should be embedded in the <style> tag
        assert!(css.contains(".c256{"), "Composite CSS missing from generated CSS");
        assert!(capsule.contains(".c256{"), "Composite CSS missing from capsule HTML");
    }

    #[test]
    fn test_composite_css_appended_after_utility_css() {
        let mut used_utils = HashSet::new();
        used_utils.insert(0x02); // DisplayFlex

        let config = CapsuleConfig::new()
            .with_style_utils(&used_utils)
            .with_composite_css(".c256{display:flex;gap:1rem}".to_string());

        let css = generate_capsule_css(&config);

        // Utility CSS for .u2 should come before composite CSS .c256
        let utility_pos = css.find(".u2{").expect("utility CSS .u2 not found");
        let composite_pos = css.find(".c256{").expect("composite CSS .c256 not found");
        assert!(utility_pos < composite_pos, "Composite CSS should come after utility CSS");
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
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let mut events = HashSet::new();
        events.insert(1); // click

        let config = CapsuleConfig::new()
            .has_client_actions(true);
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        // Client actions JS should be included
        assert!(capsule.contains("fl2"), "Client actions JS vars should be present");
        assert!(capsule.contains("uf2"), "Target update function should be present");
        assert!(capsule.contains("us2"), "Selector update function should be present");
    }

    #[test]
    fn test_client_actions_js_excluded_when_disabled() {
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let mut events = HashSet::new();
        events.insert(1); // click

        let config = CapsuleConfig::new()
            .has_client_actions(false);
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        // Client actions update functions should NOT be included
        assert!(!capsule.contains("function uf2"), "Target update function should not be present when disabled");
        assert!(!capsule.contains("function us2"), "Selector update function should not be present when disabled");
    }

    #[test]
    fn test_client_action_opcodes_in_runtime() {
        let mut elements = HashSet::new();
        elements.insert(0);
        let mut events = HashSet::new();
        events.insert(1);
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

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
        let mut elements = HashSet::new();
        elements.insert(0);
        let mut events = HashSet::new();
        events.insert(1);
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        assert!(capsule.contains("TT:0x4D"), "BIND_TIMED_TOGGLE opcode should be in O object");
    }

    #[test]
    fn test_auto_toggle_opcode_in_runtime() {
        let mut elements = HashSet::new();
        elements.insert(0);
        let mut events = HashSet::new();
        events.insert(1);
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        assert!(capsule.contains("AT2:0x4E"), "AUTO_TOGGLE opcode should be in O object");
    }
}
