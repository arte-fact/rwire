//! Generate minimal capsule HTML with tree-shaken element and event mappings.
//!
//! This module generates a capsule that contains only the element types and
//! event types actually used by the application, reducing the capsule size.
//!
//! When local state handlers are used, the capsule includes a mutation
//! interpreter (~150 bytes) that executes mutations on the client without
//! server round-trips.
//!
//! For styled capsules, CSS is embedded in a `<style>` tag within `<head>`,
//! tree-shaken to include only the styles for components actually used.

use std::collections::HashSet;

use crate::protocol::opcodes::{ELEMENT_MAPPINGS, EVENT_MAPPINGS, SVG_ELEMENT_CODES};
use crate::theme::Theme;
use crate::tokens::ColorPalette;

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
/// Does NOT include local state handling - that's added separately when needed.
/// When local state is used, the xi() function must be defined before this.
///
/// The gp() function collects payload data from events:
/// - Form submit: collects all field values as JSON {t:'form',v:{...}}
/// - Input/change on input/textarea: collects value as JSON {t:'text',v:'...'}
/// - Click: collects data-* attributes as JSON {t:'data',v:{...}}
///
/// The sep() function sends events with param bytes (for ItemRef handlers):
/// - Format: [handler_idx | 0x80, event_type, ref, param_len, ...param_bytes, payload_len, ...payload]
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
const RUNTIME_JS: &str = r#"const O={S:0xF0,SE:0xF1,WT:0xF2,G:0x01,C:0x02,CS:0x03,GS:0x05,L:0x10,T:0x11,TW:0x13,D:0x14,TI:0x15,A:0x12,P:0x20,CC:0x25,AE:0x26,AB:0x27,AK:0x28,B:0x30,R:0x31,DB:0x33,RP:0x34,IL:0x40,DH:0x42,RU:0x70,RR:0x71,SS:0x81,SU:0x82,SP:0x83,SM:0x84,SC:0x85,CT:0x86,PD:0x89,E:0xFF};
const A={4:'id'};
let s={},wt=[],w,sc=0,K={};
function rv(d,i){let b=d[i];if(b<0x80)return[b,1];if(b<0xC0)return[0x80+((b&0x3F)<<8)+d[i+1],2];return[0x4080+((b&0x3F)<<16)+(d[i+1]<<8)+d[i+2],3]}
function gp(e,el){
let t=el.tagName.toLowerCase();
if(e.type==='submit'&&t==='form'){e.preventDefault();let fd=new FormData(el),obj={};fd.forEach((v,k)=>obj[k]=v);return JSON.stringify({t:'form',v:obj})}
if((e.type==='input'||e.type==='change')&&(t==='input'||t==='textarea'||t==='select')){return JSON.stringify({t:'text',v:el.value})}
if(e.type==='click'){let tg=e.target.closest('[data-id]')||el,dt={};for(let k in tg.dataset)dt[k]=tg.dataset[k];if(Object.keys(dt).length)return JSON.stringify({t:'data',v:dt})}
return ''}
function se(h,t,f,e,el){let p=gp(e,el),pb=new TextEncoder().encode(p),msg=new Uint8Array(4+pb.length);msg[0]=h;msg[1]=t;msg[2]=f;msg[3]=pb.length;msg.set(pb,4);w.send(msg)}
function sep(h,t,f,prm,e,el){let p=gp(e,el),pb=new TextEncoder().encode(p),msg=new Uint8Array(5+prm.length+pb.length);msg[0]=h|0x80;msg[1]=t;msg[2]=f;msg[3]=prm.length;msg.set(prm,4);msg[4+prm.length]=pb.length;msg.set(pb,5+prm.length);w.send(msg)}
function x(d){
let r=[],i=0,_oc=0;
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
else if(o===O.T){let f=d[i++],[k,l]=rv(d,i);i+=l;r[f].textContent=s[k]||''}
else if(o===O.TW){let f=d[i++],n=d[i++],ws=[];while(n--)ws.push(wt[d[i++]]||'');r[f].textContent=ws.join(' ')}
else if(o===O.TI){let f=d[i++],[v,l]=rv(d,i);i+=l;let n=(v>>>1)^-(v&1);r[f].textContent=n.toString()}
else if(o===O.L){let f=d[i++],[k,l]=rv(d,i);i+=l;r[f].className=s[k]||''}
else if(o===O.A){let f=d[i++],[ak,al]=rv(d,i);i+=al;let[vk,vl]=rv(d,i);i+=vl;let an=A[ak]||s[ak]||'data';r[f].setAttribute(an,s[vk]||'')}
else if(o===O.AE){let f=d[i++],k=d[i++],v=d[i++];r[f].setAttribute(AT[k]||'data',AV[v]||'')}
else if(o===O.AB){let f=d[i++],k=d[i++];r[f].setAttribute(AT[k]||'data','')}
else if(o===O.AK){let f=d[i++],k=d[i++],[v,l]=rv(d,i);i+=l;r[f].setAttribute(AT[k]||'data',s[v]||'')}
else if(o===O.D){let f=d[i++],[kk,kl]=rv(d,i);i+=kl;let[vk,vl]=rv(d,i);i+=vl;r[f].dataset[s[kk]||'']=s[vk]||''}
else if(o===O.P){let p=d[i++],c=d[i++];(p<255?r[p]:document.body).appendChild(r[c])}
else if(o===O.CC){r[d[i++]].innerHTML=''}
else if(o===O.B){BL(d,i,r);i+=3}
else if(o===O.R){let f=d[i++],t=d[i++],h=d[i++];r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();se(h,t,f,e,r[f])})}
else if(o===O.DB){let f=d[i++],t=d[i++],h=d[i++],ms=(d[i++]<<8)|d[i++];let tm;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();clearTimeout(tm);tm=setTimeout(()=>se(h,t,f,e,r[f]),ms)})}
else if(o===O.RP){let f=d[i++],t=d[i++],h=d[i++],pl=d[i++],prm=d.slice(i,i+pl);i+=pl;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();sep(h,t,f,prm,e,r[f])})}
else if(o===O.IL||o===O.DH){i=xi(d,i-1)}
else if(o===O.RU){let[k,l]=rv(d,i);i+=l;history.pushState(null,'',s[k])}
else if(o===O.RR){let[k,l]=rv(d,i);i+=l;history.replaceState(null,'',s[k])}
else if(o===O.SS){let f=d[i++],[k,l]=rv(d,i);i+=l;r[f].style.cssText=s[k]||''}
else if(o===O.SU){let f=d[i++],[u,l]=rv(d,i);i+=l;r[f].classList.add('u'+u)}
else if(o===O.SP){let f=d[i++],p=d[i++],v=d[i++];r[f].style[P[p]]=Y[v]}
else if(o===O.SM){let f=d[i++],n=d[i++];while(n--){let[u,l]=rv(d,i);i+=l;r[f].classList.add('u'+u)}}
else if(o===O.CT){let[n,l]=rv(d,i);i+=l;while(n--){let[id,il]=rv(d,i);i+=il;let c=d[i++];while(c--){let[u,ul]=rv(d,i);i+=ul}K[id]='c'+id}}
else if(o===O.SC){let f=d[i++],[id,l]=rv(d,i);i+=l;r[f].classList.add(K[id]||'c'+id)}
else if(o===O.PD){let f=d[i++],pc=d[i++],n=d[i++];while(n--){let[u,l]=rv(d,i);i+=l;r[f].classList.add('h'+pc+'u'+u)}}
else if(o===O.E){return}
else{console.error('Unknown opcode 0x'+o.toString(16)+' at pos '+_p+' after '+_oc+' ops, r.len='+r.length)}
}}catch(e){console.error('PARSE ERROR at pos='+i+' op#'+_oc+' opcode=0x'+(d[i-1]||0).toString(16)+' r.len='+r.length+': '+e.message);console.error('Context:',Array.from(d.slice(Math.max(0,i-10),i+10)).map(b=>'0x'+b.toString(16).padStart(2,'0')).join(' '))}}
w=new WebSocket('ws://'+location.host);
w.binaryType='arraybuffer';
w.onmessage=e=>x(new Uint8Array(e.data));
document.addEventListener('click',e=>{let a=e.target.closest('a[data-route]');if(a){e.preventDefault();let h=a.getAttribute('href');history.pushState(null,'',h);w.send('R'+h)}});
window.addEventListener('popstate',()=>{w.send('R'+location.pathname)});"#;

/// Bind handler without local state support (sends to server).
/// Also includes a stub xi() since the main runtime references it.
const BIND_LOCAL_REMOTE_JS: &str = r#"function BL(d,i,r){let f=d[i],t=d[i+1],h=d[i+2];r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();se(h,t,f,e,r[f])})}
function xi(d,i){return i}"#;

/// Local state mutation interpreter (~200 bytes).
/// Adds support for:
/// - INIT_LOCAL_STATE (0x40): Initialize local state from JSON
/// - DEF_LOCAL_HANDLER (0x42): Define a local handler with mutations
/// - Local event binding that executes mutations without server round-trip
///
/// Mutation opcodes:
/// - 0x50 TOGGLE: Toggle boolean field
/// - 0x51 ADD_I8: Add signed byte to number field
/// - 0x52 ADD_I32: Add signed 32-bit int to number field
/// - 0x53 SET_BOOL: Set boolean field
/// - 0x54 SET_I32: Set 32-bit int field
/// - 0x55 SET_STR: Set string field
const LOCAL_STATE_JS: &str = r#"let ls={},lh={};
function mut(st,m){let i=0;while(i<m.length){let o=m[i++],f=m[i++],k=Object.keys(st)[f];
if(o===0x50)st[k]=!st[k];
else if(o===0x51)st[k]+=(m[i++]<<24>>24);
else if(o===0x52)st[k]+=(m[i++]<<24|m[i++]<<16|m[i++]<<8|m[i++]);
else if(o===0x53)st[k]=!!m[i++];
else if(o===0x54)st[k]=(m[i++]<<24|m[i++]<<16|m[i++]<<8|m[i++]);
else if(o===0x55){let l=m[i++];st[k]=new TextDecoder().decode(new Uint8Array(m.slice(i,i+l)));i+=l}}}
function BL(d,i,r){let f=d[i],t=d[i+1],h=d[i+2];
r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();let hd=lh[h];if(hd){mut(ls[hd.si],hd.ms)}else{se(h,t,f,e,r[f])}})}
function xi(d,i){
if(d[i]===0x40){let si=d[i+1],l=(d[i+2]<<8)|d[i+3];ls[si]=JSON.parse(new TextDecoder().decode(d.slice(i+4,i+4+l)));return i+4+l}
if(d[i]===0x42){let hi=d[i+1],si=d[i+2],mc=d[i+3],ms=[],j=i+4;
for(let c=0;c<mc;c++){let o=d[j++],f=d[j++];ms.push(o,f);
if(o===0x51)ms.push(d[j++]);
else if(o>=0x52&&o<=0x54){for(let k=0;k<4;k++)ms.push(d[j++])}
else if(o===0x55){let l=d[j++];ms.push(l);for(let k=0;k<l;k++)ms.push(d[j++])}}
lh[hi]={si,ms};return j}return i}"#;

/// Generate a minimal capsule HTML with only the used element and event types.
///
/// If `has_local_handlers` is true, includes the local state mutation interpreter
/// which adds ~200 bytes but enables client-side state mutations without server round-trips.
pub fn generate_capsule(
    used_elements: &HashSet<u8>,
    used_events: &HashSet<u8>,
    has_local_handlers: bool,
) -> String {
    let elements_js = generate_js_map(ELEMENT_MAPPINGS, used_elements);
    let events_js = generate_js_map(EVENT_MAPPINGS, used_events);
    let svg_js = generate_svg_set(used_elements);

    // Choose the appropriate bind handler based on whether we have local state
    let bind_and_local_js = if has_local_handlers {
        LOCAL_STATE_JS
    } else {
        BIND_LOCAL_REMOTE_JS
    };

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
{bind_and_local_js}
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

/// Configuration for capsule generation with styling.
#[derive(Clone, Debug, Default)]
pub struct CapsuleConfig {
    /// Theme configuration for CSS variables
    pub theme: Theme,
    /// Optional custom color palette (Nord, custom, etc.)
    pub palette: Option<ColorPalette>,
    /// Whether to include local state mutation interpreter
    pub has_local_handlers: bool,
    /// Used style utility tokens (for tree-shaking)
    pub used_style_utils: HashSet<u16>,
    /// Used style property codes (for tree-shaking)
    pub used_style_props: HashSet<u8>,
    /// Used style value codes (for tree-shaking)
    pub used_style_values: HashSet<u8>,
    /// Used pseudo-class (Pc, St) pairs (for tree-shaking)
    pub used_pseudo_pairs: HashSet<(u8, u16)>,
    /// Used attribute key codes (for tree-shaking)
    pub used_attr_keys: HashSet<u8>,
    /// Used attribute value codes (for tree-shaking)
    pub used_attr_values: HashSet<u8>,
    /// Pre-generated composite CSS from style grouping (`.c{id}{declarations}`)
    pub composite_css: String,
}

impl CapsuleConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a dark theme with Nord color palette.
    pub fn dark_nord() -> Self {
        Self::default()
            .theme(Theme::dark())
            .palette(ColorPalette::nord())
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

    /// Set a custom color palette.
    ///
    /// When set, the palette's colors will be used instead of the default
    /// Oklch-based colors. Use presets like `ColorPalette::nord()` or
    /// create custom palettes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rwire::tokens::ColorPalette;
    /// use rwire::capsule_gen::CapsuleConfig;
    ///
    /// let config = CapsuleConfig::new()
    ///     .palette(ColorPalette::nord());
    /// ```
    pub fn palette(mut self, palette: ColorPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// Set whether local handlers are used.
    pub fn has_local_handlers(mut self, has: bool) -> Self {
        self.has_local_handlers = has;
        self
    }

    /// Add a used style utility token.
    pub fn use_style_util(mut self, util: u16) -> Self {
        self.used_style_utils.insert(util);
        self
    }

    /// Add used style property and value codes.
    pub fn use_style_prop(mut self, prop: u8, value: u8) -> Self {
        self.used_style_props.insert(prop);
        self.used_style_values.insert(value);
        self
    }

    /// Set all used style utility tokens (from build context).
    pub fn with_style_utils(mut self, utils: &HashSet<u16>) -> Self {
        self.used_style_utils = utils.clone();
        self
    }

    /// Set all used style property codes (from build context).
    pub fn with_style_props(mut self, props: &HashSet<u8>) -> Self {
        self.used_style_props = props.clone();
        self
    }

    /// Set all used style value codes (from build context).
    pub fn with_style_values(mut self, values: &HashSet<u8>) -> Self {
        self.used_style_values = values.clone();
        self
    }

    /// Set all used pseudo-class (Pc, St) pairs (from build context).
    pub fn with_pseudo_pairs(mut self, pairs: &HashSet<(u8, u16)>) -> Self {
        self.used_pseudo_pairs = pairs.clone();
        self
    }

    /// Set all used attribute key codes (from build context).
    pub fn with_attr_keys(mut self, keys: &HashSet<u8>) -> Self {
        self.used_attr_keys = keys.clone();
        self
    }

    /// Set all used attribute value codes (from build context).
    pub fn with_attr_values(mut self, values: &HashSet<u8>) -> Self {
        self.used_attr_values = values.clone();
        self
    }

    /// Set pre-generated composite CSS from style grouping analysis.
    pub fn with_composite_css(mut self, css: String) -> Self {
        self.composite_css = css;
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
/// Scans for `var(--rw-*)` patterns and returns a set of variable names.
fn extract_used_variables(css: &str) -> HashSet<String> {
    let mut vars = HashSet::new();

    // Find all occurrences of "var(" and extract the variable name
    for (idx, _) in css.match_indices("var(") {
        let rest = &css[idx + 4..]; // Skip "var("

        // Find the end of the variable name (either ',' or ')')
        if let Some(end) = rest.find([',', ')']) {
            let var_name = rest[..end].trim();

            // Only track --rw-* variables
            if var_name.starts_with("--rw-") {
                vars.insert(var_name.to_string());
            }
        }
    }

    vars
}

/// Generate complete CSS for the capsule with tree-shaken variables.
///
/// Includes:
/// - Base reset CSS
/// - Primitive token CSS variables (tree-shaken, or from custom palette)
/// - Semantic token CSS variables (tree-shaken)
/// - Theme overrides (accent, radius)
/// - Component CSS (tree-shaken)
pub fn generate_capsule_css(config: &CapsuleConfig) -> String {
    use crate::style_tokens::{generate_utility_css, generate_pseudo_css};
    use crate::theme::{generate_base_css, generate_semantic_css, generate_accent_css, generate_radius_css};
    use crate::tokens::css::{generate_primitive_css_filtered, generate_primitive_css_with_palette};

    let mut css = String::with_capacity(12288);

    // 1. Get base CSS and extract variables used in it
    let base_css = generate_base_css();
    let mut used_vars = extract_used_variables(base_css);

    // 2. Generate utility + pseudo token CSS (class-based, tree-shaken)
    let utility_token_css = generate_utility_css(&config.used_style_utils);
    let pseudo_token_css = generate_pseudo_css(&config.used_pseudo_pairs);

    // Extract variables used in token CSS rules
    used_vars.extend(extract_used_variables(&utility_token_css));
    used_vars.extend(extract_used_variables(&pseudo_token_css));

    // 3. Generate semantic CSS to extract primitive variables it references
    let semantic_css = generate_semantic_css();
    used_vars.extend(extract_used_variables(&semantic_css));

    // 4. Also check theme overrides for additional variables
    if let Some(accent_css) = generate_accent_css(config.theme.accent) {
        used_vars.extend(extract_used_variables(&accent_css));
    }
    if let Some(radius_css) = generate_radius_css(config.theme.radius) {
        used_vars.extend(extract_used_variables(radius_css));
    }

    // 5. Base reset (must come first)
    css.push_str(base_css);

    // 6. Primitive tokens (tree-shaken or from custom palette)
    match &config.palette {
        Some(palette) => css.push_str(&generate_primitive_css_with_palette(palette)),
        None => css.push_str(&generate_primitive_css_filtered(&used_vars)),
    }

    // 7. Semantic tokens
    css.push_str(&semantic_css);

    // 8. Theme overrides
    if let Some(accent_css) = generate_accent_css(config.theme.accent) {
        css.push_str(&accent_css);
    }
    if let Some(radius_css) = generate_radius_css(config.theme.radius) {
        css.push_str(radius_css);
    }

    // 9. Utility token CSS classes (.u{code}{declaration})
    css.push_str(&utility_token_css);

    // 10. Pseudo-class token CSS rules (.h{pc}u{st}:hover{...}, etc.)
    css.push_str(&pseudo_token_css);

    // 11. Composite style CSS classes (.c{id}{declarations})
    if !config.composite_css.is_empty() {
        css.push_str(&config.composite_css);
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
    let theme_attrs = config.theme.data_attrs();

    // Generate style token lookup tables (tree-shaken)
    // Note: U (utility) map removed - utilities now use CSS classes generated server-side
    let props_js = generate_js_map(PROP_MAPPINGS, &config.used_style_props);
    let values_js = generate_js_map(VALUE_MAPPINGS, &config.used_style_values);

    // Generate attribute lookup tables (tree-shaken)
    let attr_keys_js = generate_js_map(AT_MAPPINGS, &config.used_attr_keys);
    let attr_values_js = generate_js_map(AV_MAPPINGS, &config.used_attr_values);

    // Choose the appropriate bind handler based on whether we have local state
    let bind_and_local_js = if config.has_local_handlers {
        LOCAL_STATE_JS
    } else {
        BIND_LOCAL_REMOTE_JS
    };

    // Theme attrs go on <html> so CSS variables cascade properly to body
    // CSS is embedded in <style> tag so styles are available before WS connects
    format!(
        r#"<!DOCTYPE html><html {theme_attrs}><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
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
{bind_and_local_js}
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{AccentColor, ThemeMode};

    #[test]
    fn test_capsule_config_defaults() {
        let config = CapsuleConfig::new();
        assert_eq!(config.theme.mode, ThemeMode::Light);
        assert_eq!(config.theme.accent, AccentColor::Blue);
        assert!(!config.has_local_handlers);
    }

    #[test]
    fn test_capsule_css_generation() {
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);

        // Should contain base reset
        assert!(css.contains("box-sizing"));
        // Should contain primitive tokens
        assert!(css.contains("--rw-neutral-1"));
        // Should contain semantic tokens
        assert!(css.contains("--rw-bg-app"));
    }

    #[test]
    fn test_css_variables_not_empty() {
        // This test verifies the fix for the bug where CSS variables were empty
        // because primitive tokens referenced by semantic CSS were being tree-shaken out

        // Simulate St tokens that components use. These tokens reference CSS
        // variables that must be included in the primitive CSS output.
        let mut used_utils = HashSet::new();
        used_utils.insert(0xC0); // BgApp -> --rw-bg-app
        used_utils.insert(0x8B); // RoundedLg -> --rw-radius-lg
        used_utils.insert(0xD1); // BorderSubtle -> --rw-border-subtle
        used_utils.insert(0x53); // PMd -> --rw-space-4
        used_utils.insert(0xE9); // ShadowSm -> --rw-shadow-sm

        let config = CapsuleConfig::new()
            .with_style_utils(&used_utils);
        let css = generate_capsule_css(&config);

        // Semantic tokens that reference primitives should all be present
        assert!(css.contains("--rw-neutral-1:"), "Missing --rw-neutral-1");
        assert!(css.contains("--rw-neutral-2:"), "Missing --rw-neutral-2");
        assert!(css.contains("--rw-neutral-11:"), "Missing --rw-neutral-11");
        assert!(css.contains("--rw-neutral-12:"), "Missing --rw-neutral-12");

        // Blue scale (used by default accent)
        assert!(css.contains("--rw-blue-9:"), "Missing --rw-blue-9");

        // White (used by text-on-accent)
        assert!(css.contains("--rw-white:"), "Missing --rw-white");

        // Spacing tokens used by St utility classes
        assert!(css.contains("--rw-space-4:"), "Missing --rw-space-4");

        // Radius tokens used by St utility classes
        assert!(css.contains("--rw-radius-lg:"), "Missing --rw-radius-lg");

        // Typography tokens used in base CSS
        assert!(css.contains("--rw-leading-normal:"), "Missing --rw-leading-normal");

        // Shadow tokens used by St utility classes
        assert!(css.contains("--rw-shadow-sm:"), "Missing --rw-shadow-sm");

        // Verify semantic tokens are defined and reference primitives correctly
        assert!(css.contains("--rw-bg-app:var(--rw-neutral-1)"), "Semantic token not referencing primitive");
        assert!(css.contains("--rw-text-default:var(--rw-neutral-11)"), "Semantic token not referencing primitive");
    }

    #[test]
    fn test_styled_capsule_structure() {
        let mut elements = HashSet::new();
        elements.insert(0); // div
        elements.insert(2); // button

        let mut events = HashSet::new();
        events.insert(1); // click

        let config = CapsuleConfig::new()
            .theme(Theme::dark().with_accent(AccentColor::Green))
            .has_local_handlers(false);

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        // Should have HTML structure
        assert!(capsule.contains("<!DOCTYPE html>"));

        // CSS should be embedded in <style> tag
        assert!(capsule.contains("<style>"));
        assert!(capsule.contains("box-sizing"));

        // Should have theme data attribute
        assert!(capsule.contains("data-theme=\"dark\""));
        assert!(capsule.contains("data-accent=\"green\""));

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
        used_utils.insert(0xC0); // BgApp

        let config = CapsuleConfig::new()
            .with_style_utils(&used_utils);

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&elements, &events, &config, &css);

        // CSS should be in <style> tag within <head>
        assert!(capsule.contains("<style>"));
        assert!(capsule.contains("</style></head>"));

        // Should contain generated CSS content
        assert!(capsule.contains("--rw-bg-app"));
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
    fn test_generate_capsule_without_local() {
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let mut events = HashSet::new();
        events.insert(1); // click

        let capsule = generate_capsule(&elements, &events, false);
        assert!(capsule.contains("const E={0:'div'}"));
        assert!(capsule.contains("const V={1:'click'}"));
        assert!(capsule.contains("<!DOCTYPE html>"));
        // Should NOT contain local state code
        assert!(!capsule.contains("let ls={},lh={}"));
    }

    #[test]
    fn test_generate_capsule_with_local() {
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let mut events = HashSet::new();
        events.insert(1); // click

        let capsule = generate_capsule(&elements, &events, true);
        assert!(capsule.contains("const E={0:'div'}"));
        assert!(capsule.contains("const V={1:'click'}"));
        assert!(capsule.contains("<!DOCTYPE html>"));
        // Should contain local state code
        assert!(capsule.contains("let ls={},lh={}"));
        assert!(capsule.contains("function mut"));
    }

    #[test]
    fn test_capsule_config_dark_nord() {
        let config = CapsuleConfig::dark_nord();
        assert_eq!(config.theme.mode, ThemeMode::Dark);
        assert!(config.palette.is_some());
    }

    #[test]
    fn test_capsule_config_light() {
        let config = CapsuleConfig::light();
        assert_eq!(config.theme.mode, ThemeMode::Light);
        assert!(config.palette.is_none());
    }

    #[test]
    fn test_capsule_config_dark() {
        let config = CapsuleConfig::dark();
        assert_eq!(config.theme.mode, ThemeMode::Dark);
        assert!(config.palette.is_none());
    }

    #[test]
    fn test_composite_css_included_in_capsule() {
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let events = HashSet::new();

        let config = CapsuleConfig::new()
            .with_composite_css(".c256{display:flex;flex-direction:column;gap:var(--rw-space-4)}".to_string());

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
}
