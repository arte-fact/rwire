//! Generate minimal capsule HTML with tree-shaken element and event mappings.
//!
//! This module generates a capsule that contains only the element types and
//! event types actually used by the application, reducing the capsule size.

use std::collections::HashSet;

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
    (16, "form"),
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
const RUNTIME_JS: &str = r#"const O={S:0xF0,G:0x01,C:0x02,L:0x10,T:0x11,A:0x12,D:0x14,P:0x20,B:0x30,R:0x31,O:0x32,DB:0x33,E:0xFF};
const A={4:'id'};
let s={},w;
function x(d){
let r=[],i=0;
while(i<d.length){
let o=d[i++];
if(o===O.S){let n=d[i++],j=0x80;while(n--){let l=d[i++];s[j++]=new TextDecoder().decode(d.slice(i,i+l));i+=l}}
else if(o===O.G){let k=s[d[i++]];let el=document.getElementById(k);r.push(el)}
else if(o===O.C){r.push(document.createElement(E[d[i++]]||'div'))}
else if(o===O.T){r[d[i++]].textContent=s[d[i++]]||''}
else if(o===O.L){r[d[i++]].className=s[d[i++]]||''}
else if(o===O.A){let f=d[i++];r[f].setAttribute(A[d[i]]||s[d[i]]||'data',s[d[i+1]]||'');i+=2}
else if(o===O.D){let f=d[i++];r[f].dataset[s[d[i++]]||'']=s[d[i++]]||''}
else if(o===O.P){let p=d[i++],c=d[i++];(p<255?r[p]:document.body).appendChild(r[c])}
else if(o===O.B||o===O.R||o===O.O){let f=d[i++],t=d[i++],h=d[i++];r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();w.send(new Uint8Array([h,t,f,0]))})}
else if(o===O.DB){let f=d[i++],t=d[i++],h=d[i++],ms=(d[i++]<<8)|d[i++];let tm;r[f].addEventListener(V[t]||'click',e=>{e.preventDefault();clearTimeout(tm);tm=setTimeout(()=>w.send(new Uint8Array([h,t,f,0])),ms)})}
else if(o===O.E){return}
}}
w=new WebSocket('ws://'+location.host);
w.binaryType='arraybuffer';
w.onmessage=e=>x(new Uint8Array(e.data));"#;

/// Generate a minimal capsule HTML with only the used element and event types.
pub fn generate_capsule(used_elements: &HashSet<u8>, used_events: &HashSet<u8>) -> String {
    let elements_js = generate_element_map(used_elements);
    let events_js = generate_event_map(used_events);

    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head><body>
<script>
const E={{{elements_js}}};
const V={{{events_js}}};
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_generate_capsule() {
        let mut elements = HashSet::new();
        elements.insert(0); // div

        let mut events = HashSet::new();
        events.insert(1); // click

        let capsule = generate_capsule(&elements, &events);
        assert!(capsule.contains("const E={0:'div'}"));
        assert!(capsule.contains("const V={1:'click'}"));
        assert!(capsule.contains("<!DOCTYPE html>"));
    }
}
