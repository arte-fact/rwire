//! Generate minimal capsule HTML with tree-shaken element and event mappings.
//!
//! This module generates a capsule that contains only the element types and
//! event types actually used by the application, reducing the capsule size.
//!
//! When local state handlers are used, the capsule includes a mutation
//! interpreter (~150 bytes) that executes mutations on the client without
//! server round-trips.
//!
//! The capsule also includes design token CSS and component CSS, tree-shaken
//! to include only the styles for components actually used.

use std::collections::HashSet;

use crate::components::ComponentRegistry;
use crate::theme::Theme;

/// All supported element types with their byte codes and tag names.
const ELEMENT_MAPPINGS: &[(u8, &str)] = &[
    (0, "div"),
    (1, "span"),
    (2, "button"),
    (3, "input"),
    (4, "p"),
    (5, "h1"),
    (6, "h2"),
    (7, "a"),
    (8, "textarea"),
    (9, "select"),
    (10, "option"),
    (11, "label"),
    (12, "fieldset"),
    (13, "legend"),
    (16, "form"),
    (17, "ul"),
    (18, "li"),
    (19, "nav"),
    (20, "header"),
    (21, "footer"),
    (22, "section"),
    (23, "article"),
    (24, "svg"),
    (25, "path"),
];

/// All supported event types with their byte codes and event names.
const EVENT_MAPPINGS: &[(u8, &str)] = &[
    (1, "click"),
    (2, "dblclick"),
    (3, "mousedown"),
    (4, "mouseup"),
    (5, "mousemove"),
    (6, "submit"),
    (7, "input"),
    (8, "change"),
    (9, "keydown"),
    (10, "keyup"),
    (11, "focus"),
    (12, "blur"),
];

/// Generate the JavaScript object literal for element mappings.
fn generate_element_map(used: &HashSet<u8>) -> String {
    let entries: Vec<String> = ELEMENT_MAPPINGS
        .iter()
        .filter(|(code, _)| used.contains(code))
        .map(|(code, name)| format!("{}:'{}'", code, name))
        .collect();
    entries.join(",")
}

/// Generate the JavaScript object literal for event mappings.
fn generate_event_map(used: &HashSet<u8>) -> String {
    let entries: Vec<String> = EVENT_MAPPINGS
        .iter()
        .filter(|(code, _)| used.contains(code))
        .map(|(code, name)| format!("{}:'{}'", code, name))
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
const RUNTIME_JS: &str = r#"const O={S:0xF0,SE:0xF1,WT:0xF2,G:0x01,C:0x02,CS:0x03,GS:0x05,L:0x10,T:0x11,TW:0x13,D:0x14,TI:0x15,A:0x12,P:0x20,CC:0x25,B:0x30,R:0x31,O:0x32,DB:0x33,RP:0x34,IL:0x40,DH:0x42,RU:0x70,RR:0x71,SI:0x80,SS:0x81,E:0xFF};
const A={4:'id'};
let s={},wt=[],w,sc=0;
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
let r=[],i=0;
while(i<d.length){
let o=d[i++];
if(o===O.S){let[n,l]=rv(d,i);i+=l;sc=0x80;while(n--){let sl=d[i++];s[sc++]=new TextDecoder().decode(d.slice(i,i+sl));i+=sl}}
else if(o===O.SE){let[n,l]=rv(d,i);i+=l;let[si,sl]=rv(d,i);i+=sl;sc=si;while(n--){let sl=d[i++];s[sc++]=new TextDecoder().decode(d.slice(i,i+sl));i+=sl}}
else if(o===O.WT){let n=d[i++];wt=[];while(n--){let l=d[i++];wt.push(new TextDecoder().decode(d.slice(i,i+l)));i+=l}}
else if(o===O.G){let[k,l]=rv(d,i);i+=l;let el=document.getElementById(s[k]);r.push(el)}
else if(o===O.C){r.push(document.createElement(E[d[i++]]||'div'))}
else if(o===O.CS){let[id,l]=rv(d,i);i+=l;let e=document.createElement('span');e.id='__synced_'+id;r.push(e)}
else if(o===O.GS){let[id,l]=rv(d,i);i+=l;r.push(document.getElementById('__synced_'+id))}
else if(o===O.T){let f=d[i++],[k,l]=rv(d,i);i+=l;r[f].textContent=s[k]||''}
else if(o===O.TW){let f=d[i++],n=d[i++],ws=[];while(n--)ws.push(wt[d[i++]]||'');r[f].textContent=ws.join(' ')}
else if(o===O.TI){let f=d[i++],[v,l]=rv(d,i);i+=l;let n=(v>>>1)^-(v&1);r[f].textContent=n.toString()}
else if(o===O.L){let f=d[i++],[k,l]=rv(d,i);i+=l;r[f].className=s[k]||''}
else if(o===O.A){let f=d[i++],[ak,al]=rv(d,i);i+=al;let[vk,vl]=rv(d,i);i+=vl;let an=A[ak]||s[ak]||'data';console.log('SET_ATTR: ak='+ak+' vk='+vk+' attr='+an+' val='+s[vk]);r[f].setAttribute(an,s[vk]||'')}
else if(o===O.D){let f=d[i++],[kk,kl]=rv(d,i);i+=kl;let[vk,vl]=rv(d,i);i+=vl;r[f].dataset[s[kk]||'']=s[vk]||''}
else if(o===O.P){let p=d[i++],c=d[i++];(p<255?r[p]:document.body).appendChild(r[c])}
else if(o===O.CC){r[d[i++]].innerHTML=''}
else if(o===O.B){BL(d,i,r);i+=3}
else if(o===O.R||o===O.O){let f=d[i++],t=d[i++],h=d[i++];r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();se(h,t,f,e,r[f])})}
else if(o===O.DB){let f=d[i++],t=d[i++],h=d[i++],ms=(d[i++]<<8)|d[i++];let tm;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();clearTimeout(tm);tm=setTimeout(()=>se(h,t,f,e,r[f]),ms)})}
else if(o===O.RP){let f=d[i++],t=d[i++],h=d[i++],pl=d[i++],prm=d.slice(i,i+pl);i+=pl;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();sep(h,t,f,prm,e,r[f])})}
else if(o===O.IL||o===O.DH){i=xi(d,i-1)}
else if(o===O.RU){let[k,l]=rv(d,i);i+=l;history.pushState(null,'',s[k])}
else if(o===O.RR){let[k,l]=rv(d,i);i+=l;history.replaceState(null,'',s[k])}
else if(o===O.SI){let l=(d[i++]<<8)|d[i++];let css=new TextDecoder().decode(d.slice(i,i+l));let st=document.createElement('style');st.textContent=css;document.head.appendChild(st);i+=l}
else if(o===O.SS){let f=d[i++],[k,l]=rv(d,i);i+=l;r[f].style.cssText=s[k]||''}
else if(o===O.E){return}
}}
w=new WebSocket('ws://'+location.host);
w.binaryType='arraybuffer';
w.onmessage=e=>x(new Uint8Array(e.data));
document.addEventListener('click',e=>{let a=e.target.closest('a[data-route]');if(a){e.preventDefault();let h=a.getAttribute('href');history.pushState(null,'',h);w.send(new TextEncoder().encode('R'+h))}});
window.addEventListener('popstate',()=>{w.send(new TextEncoder().encode('R'+location.pathname))});"#;

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
    let elements_js = generate_element_map(used_elements);
    let events_js = generate_event_map(used_events);

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
{bind_and_local_js}
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

/// Generate a minimal capsule HTML with only the used element and event types (legacy API).
///
/// This is for backwards compatibility. New code should use `generate_capsule` with
/// the `has_local_handlers` parameter.
#[deprecated(note = "Use generate_capsule with has_local_handlers parameter")]
pub fn generate_capsule_legacy(used_elements: &HashSet<u8>, used_events: &HashSet<u8>) -> String {
    generate_capsule(used_elements, used_events, false)
}

/// Configuration for capsule generation with styling.
#[derive(Clone, Debug, Default)]
pub struct CapsuleConfig {
    /// Theme configuration for CSS variables
    pub theme: Theme,
    /// Registry of used components for tree-shaking CSS
    pub components: ComponentRegistry,
    /// Whether to include local state mutation interpreter
    pub has_local_handlers: bool,
}

impl CapsuleConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the component registry.
    pub fn components(mut self, registry: ComponentRegistry) -> Self {
        self.components = registry;
        self
    }

    /// Set whether local handlers are used.
    pub fn has_local_handlers(mut self, has: bool) -> Self {
        self.has_local_handlers = has;
        self
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
/// - Primitive token CSS variables (tree-shaken)
/// - Semantic token CSS variables (tree-shaken)
/// - Theme overrides (accent, radius)
/// - Component CSS (tree-shaken)
pub fn generate_capsule_css(config: &CapsuleConfig) -> String {
    use crate::theme::{generate_base_css, generate_semantic_css, generate_accent_css, generate_radius_css};
    use crate::tokens::css::generate_primitive_css_filtered;

    let mut css = String::with_capacity(12288);

    // 1. Get base CSS and extract variables used in it
    let base_css = generate_base_css();
    let mut used_vars = extract_used_variables(base_css);

    // 2. Get component CSS and extract used variables
    let component_css = config.components.generate_css();
    used_vars.extend(extract_used_variables(&component_css));

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

    // 6. Generate tree-shaken primitive tokens (only used variables)
    css.push_str(&generate_primitive_css_filtered(&used_vars));

    // 7. Semantic tokens
    css.push_str(&semantic_css);

    // 8. Theme overrides
    if let Some(accent_css) = generate_accent_css(config.theme.accent) {
        css.push_str(&accent_css);
    }
    if let Some(radius_css) = generate_radius_css(config.theme.radius) {
        css.push_str(radius_css);
    }

    // 9. Component CSS (already tree-shaken)
    css.push_str(&component_css);

    css
}

/// Generate a capsule with styling support.
///
/// This is the recommended way to generate capsules for styled applications.
/// Includes:
/// - Tree-shaken element/event mappings
/// - Theme data attributes on root element
/// - Component CSS (delivered via WebSocket STYLE_INJECT opcode)
///
/// CSS is delivered via the binary WebSocket protocol (STYLE_INJECT opcode)
/// as the first message, before the DOM. This eliminates the extra HTTP request
/// and aligns with rwire's philosophy of all rendering data flowing through
/// the WebSocket.
pub fn generate_styled_capsule(
    used_elements: &HashSet<u8>,
    used_events: &HashSet<u8>,
    config: &CapsuleConfig,
) -> String {
    let elements_js = generate_element_map(used_elements);
    let events_js = generate_event_map(used_events);
    let theme_attrs = config.theme.data_attrs();

    // Choose the appropriate bind handler based on whether we have local state
    let bind_and_local_js = if config.has_local_handlers {
        LOCAL_STATE_JS
    } else {
        BIND_LOCAL_REMOTE_JS
    };

    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head><body>
<div id="rw" {theme_attrs}></div>
<script>
const E={{{elements_js}}};
const V={{{events_js}}};
{bind_and_local_js}
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::registry::ComponentType;
    use crate::theme::{AccentColor, ThemeMode};

    #[test]
    fn test_capsule_config_defaults() {
        let config = CapsuleConfig::new();
        assert_eq!(config.theme.mode, ThemeMode::Light);
        assert_eq!(config.theme.accent, AccentColor::Blue);
        assert!(!config.has_local_handlers);
        assert!(config.components.is_empty());
    }

    #[test]
    fn test_capsule_css_generation() {
        let mut registry = ComponentRegistry::new();
        registry.mark_used(ComponentType::Button);

        let config = CapsuleConfig::new().components(registry);
        let css = generate_capsule_css(&config);

        // Should contain base reset
        assert!(css.contains("box-sizing"));
        // Should contain primitive tokens
        assert!(css.contains("--rw-neutral-1"));
        // Should contain semantic tokens
        assert!(css.contains("--rw-bg-app"));
        // Should contain button CSS
        assert!(css.contains(".rw-btn"));
        // Should NOT contain input CSS (not used)
        assert!(!css.contains(".rw-input"));
    }

    #[test]
    fn test_css_variables_not_empty() {
        // This test verifies the fix for the bug where CSS variables were empty
        // because primitive tokens referenced by semantic CSS were being tree-shaken out
        let mut registry = ComponentRegistry::new();
        registry.mark_used(ComponentType::Button);
        registry.mark_used(ComponentType::Card);

        let config = CapsuleConfig::new().components(registry);
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

        // Spacing tokens used by components
        assert!(css.contains("--rw-space-2:"), "Missing --rw-space-2");
        assert!(css.contains("--rw-space-4:"), "Missing --rw-space-4");

        // Radius tokens used by components
        assert!(css.contains("--rw-radius-md:"), "Missing --rw-radius-md");
        assert!(css.contains("--rw-radius-lg:"), "Missing --rw-radius-lg");

        // Typography tokens used in base CSS
        assert!(css.contains("--rw-leading-normal:"), "Missing --rw-leading-normal");
        assert!(css.contains("--rw-font-medium:"), "Missing --rw-font-medium");

        // Shadow tokens used by Card
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

        let mut registry = ComponentRegistry::new();
        registry.mark_used(ComponentType::Button);

        let config = CapsuleConfig::new()
            .theme(Theme::dark().with_accent(AccentColor::Green))
            .components(registry)
            .has_local_handlers(false);

        let capsule = generate_styled_capsule(&elements, &events, &config);

        // Should have HTML structure
        assert!(capsule.contains("<!DOCTYPE html>"));

        // CSS is now delivered via WebSocket STYLE_INJECT, not in HTML
        assert!(!capsule.contains("<style>"));
        assert!(!capsule.contains("<link rel=\"stylesheet\""));

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
    fn test_styled_capsule_size() {
        let mut elements = HashSet::new();
        elements.insert(0); // div
        elements.insert(2); // button

        let mut events = HashSet::new();
        events.insert(1); // click

        let mut registry = ComponentRegistry::new();
        registry.mark_used(ComponentType::Button);

        let config = CapsuleConfig::new().components(registry);
        let capsule = generate_styled_capsule(&elements, &events, &config);

        // HTML capsule should be small (CSS is delivered via WebSocket)
        // Should be well under 5KB without inline CSS
        assert!(
            capsule.len() < 5120,
            "Styled capsule too large: {} bytes (CSS should be delivered via WebSocket)",
            capsule.len()
        );
        println!("Styled capsule size: {} bytes", capsule.len());
    }

    #[test]
    fn test_minimal_element_map() {
        let mut used = HashSet::new();
        used.insert(0); // div
        used.insert(2); // button

        let map = generate_element_map(&used);
        assert!(map.contains("0:'div'"));
        assert!(map.contains("2:'button'"));
        assert!(!map.contains("span"));
    }

    #[test]
    fn test_minimal_event_map() {
        let mut used = HashSet::new();
        used.insert(1); // click

        let map = generate_event_map(&used);
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
}
