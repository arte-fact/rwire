# LLM-to-Binary UI: The Intermediate DSL Approach

## Overview

This guide describes a compact Domain-Specific Language (DSL) designed as an intermediate representation between LLM text output and a binary-serialized web UI format. The DSL is human-readable (so LLMs can learn it from examples), maps deterministically to a binary wire format, and achieves significant token reduction compared to raw HTML/CSS/JS.

**Pipeline:**

```
User Prompt → LLM generates DSL → Deterministic Encoder → Binary Format → WebSocket → Client Decoder → DOM
```

The key insight: the LLM never needs to reason about bytes. It reasons about a minimal, structured text format that a trivial Rust encoder compiles to binary in microseconds.

---

## 1. DSL Design Principles

The DSL must satisfy four constraints simultaneously:

1. **LLM-friendly** — regular syntax, learnable from few examples, no ambiguity
2. **Compact** — 3-5x fewer tokens than equivalent HTML/CSS
3. **1:1 binary mapping** — every DSL construct maps to exactly one binary opcode/structure
4. **Diff-capable** — supports incremental updates, not just full renders

### Token Budget Comparison

A simple login form in HTML/CSS: ~180 tokens.
The same form in this DSL: ~40-50 tokens.
Binary wire format: ~120-200 bytes.

---

## 2. DSL Syntax Specification

### 2.1 Elements

Elements use a shorthand notation inspired by CSS selectors and Emmet, but stripped to the minimum.

```
element_type#id.class1.class2 [props] { children }
```

**Short tags** replace verbose HTML element names:

| DSL Tag | HTML Equivalent     | Binary Opcode |
|---------|---------------------|---------------|
| `v`     | `<div>`             | `0x01`        |
| `t`     | `<span>` / text     | `0x02`        |
| `b`     | `<button>`          | `0x03`        |
| `i`     | `<input>`           | `0x04`        |
| `img`   | `<img>`             | `0x05`        |
| `a`     | `<a>`               | `0x06`        |
| `h1-h6` | `<h1>`–`<h6>`      | `0x10-0x15`   |
| `p`     | `<p>`               | `0x07`        |
| `ls`    | `<ul>` / `<ol>`     | `0x08`        |
| `li`    | `<li>`              | `0x09`        |
| `fm`    | `<form>`            | `0x0A`        |
| `tb`    | `<table>`           | `0x0B`        |
| `tr`    | `<tr>`              | `0x0C`        |
| `td`    | `<td>`              | `0x0D`        |
| `sv`    | `<svg>` (container) | `0x0E`        |
| `cn`    | `<canvas>`          | `0x0F`        |

### 2.2 Styling

Styles are inlined using a compact property notation inside square brackets. No CSS class definitions, no separate stylesheet — everything is co-located.

**Syntax:** `[property:value property:value ...]`

**Shorthand properties:**

| DSL Prop  | CSS Equivalent        | Binary ID |
|-----------|-----------------------|-----------|
| `w`       | `width`               | `0x01`    |
| `h`       | `height`              | `0x02`    |
| `p`       | `padding`             | `0x03`    |
| `m`       | `margin`              | `0x04`    |
| `bg`      | `background`          | `0x05`    |
| `c`       | `color`               | `0x06`    |
| `fs`      | `font-size`           | `0x07`    |
| `fw`      | `font-weight`         | `0x08`    |
| `br`      | `border-radius`       | `0x09`    |
| `bd`      | `border`              | `0x0A`    |
| `d`       | `display`             | `0x0B`    |
| `fx`      | `flex`  (shorthand)   | `0x0C`    |
| `fd`      | `flex-direction`      | `0x0D`    |
| `ai`      | `align-items`         | `0x0E`    |
| `jc`      | `justify-content`     | `0x0F`    |
| `g`       | `gap`                 | `0x10`    |
| `pos`     | `position`            | `0x11`    |
| `top`     | `top`                 | `0x12`    |
| `left`    | `left`                | `0x13`    |
| `right`   | `right`               | `0x14`    |
| `bottom`  | `bottom`              | `0x15`    |
| `z`       | `z-index`             | `0x16`    |
| `ov`      | `overflow`            | `0x17`    |
| `op`      | `opacity`             | `0x18`    |
| `sh`      | `box-shadow`          | `0x19`    |
| `tr`      | `transition`          | `0x1A`    |
| `tf`      | `transform`           | `0x1B`    |
| `gr`      | `grid`  (shorthand)   | `0x1C`    |
| `gtc`     | `grid-template-cols`  | `0x1D`    |
| `gtr`     | `grid-template-rows`  | `0x1E`    |
| `ta`      | `text-align`          | `0x1F`    |
| `ws`      | `white-space`         | `0x20`    |
| `cur`     | `cursor`              | `0x21`    |

**Value shorthands:**

| DSL Value   | Meaning            |
|-------------|--------------------|
| `cen`       | `center`           |
| `col`       | `column`           |
| `row`       | `row`              |
| `bet`       | `space-between`    |
| `aro`       | `space-around`     |
| `str`       | `stretch`          |
| `abs`       | `absolute`         |
| `rel`       | `relative`         |
| `fix`       | `fixed`            |
| `hid`       | `hidden`           |
| `aut`       | `auto`             |
| `non`       | `none`             |
| `ptr`       | `pointer`          |
| `bol`       | `bold`             |
| `100%`      | `100%`             |

**Colors** use 3 or 6 hex digits without `#`: `bg:1a1a2e c:fff`

**Units** default to `px` when bare numbers are used. Append `%`, `em`, `rem`, `vw`, `vh` explicitly when needed.

### 2.3 Text Content

Text is quoted inline with backticks:

```
t `Hello World`
```

For dynamic/interpolated text:

```
t `Welcome, ${user.name}`
```

### 2.4 Attributes and Props

Non-style attributes go inside parentheses:

```
i(type:password placeholder:`Enter password` name:pwd)
a(href:/dashboard target:_blank)
img(src:/logo.png alt:`Company Logo`)
```

### 2.5 Events (Limited JS)

Events use the `@` prefix and reference named actions:

```
b @click:submit `Send`
b @click:toggle:sidebar `☰`
i @input:validate:email(type:email)
```

The action namespace (`submit`, `toggle`, `validate`) maps to predefined client-side handlers registered in the decoder. The LLM doesn't generate JavaScript — it references action identifiers that the client runtime resolves.

### 2.6 Conditionals and Loops

For dynamic rendering, the DSL supports minimal control flow:

```
?visible {
  v `This is conditionally shown`
}

*items -> item {
  li `${item.name}`
}
```

- `?` — conditional block, bound to a state key
- `*` — iteration block with binding

---

## 3. Complete Example

### Login Page

**DSL (≈45 tokens):**

```
v#root [d:fx fd:col ai:cen jc:cen h:100vh bg:0f0f23] {
  v#card [w:400 p:40 br:12 bg:1a1a2e sh:0,4,20,0003] {
    h2 [c:e0e0e0 ta:cen m:0,0,24,0] `Sign In`
    v [d:fx fd:col g:16] {
      i#email(type:email placeholder:`Email`) [w:100% p:12 br:8 bg:16163a c:fff bd:1,solid,2a2a4a]
      i#pass(type:password placeholder:`Password`) [w:100% p:12 br:8 bg:16163a c:fff bd:1,solid,2a2a4a]
      b#submit @click:auth:login [w:100% p:12 br:8 bg:6c63ff c:fff fw:bol cur:ptr] `Log In`
    }
    t.link [c:888 fs:13 ta:cen m:16,0,0,0] `Forgot password?`
  }
}
```

**Equivalent HTML/CSS would be ≈150-200 tokens of markup + separate style block.**

### Dashboard Card Grid

```
v#dash [p:24 bg:0a0a1a] {
  h1 [c:fff m:0,0,24,0] `Dashboard`
  v [d:gr gtc:repeat(3,1fr) g:20] {
    *cards -> card {
      v.card [p:20 br:12 bg:1a1a2e] {
        t.label [c:888 fs:13] `${card.label}`
        t.value [c:fff fs:32 fw:bol] `${card.value}`
        t.delta [c:${card.color} fs:14] `${card.delta}`
      }
    }
  }
}
```

---

## 4. Binary Encoding Specification

The Rust encoder walks the DSL AST and emits a byte stream. Each node follows a TLV (Type-Length-Value) inspired structure.

### 4.1 Frame Structure

```
┌─────────┬─────────┬──────────┬─────────────┐
│ Version │ Flags   │ Node Cnt │ Nodes...    │
│ 1 byte  │ 1 byte  │ 2 bytes  │ variable    │
└─────────┴─────────┴──────────┴─────────────┘
```

**Flags byte:**
- Bit 0: Full render (0) or diff/patch (1)
- Bit 1: Compressed payload (0=raw, 1=zstd)
- Bit 2-7: Reserved

### 4.2 Node Encoding

```
┌──────────┬──────────┬────────────┬────────────┬──────────┬──────────┐
│ Tag      │ Flags    │ ID (opt)   │ Styles     │ Content  │ Children │
│ 1 byte   │ 1 byte   │ varint+str │ style blob │ text blob│ count+..│
└──────────┴──────────┴────────────┴────────────┴──────────┴──────────┘
```

**Node Flags byte:**
- Bit 0: Has ID
- Bit 1: Has classes
- Bit 2: Has styles
- Bit 3: Has attributes
- Bit 4: Has text content
- Bit 5: Has children
- Bit 6: Has events
- Bit 7: Reserved

This means a bare `v {}` with only children encodes as just 3 bytes: `0x01 0x20 0x__` (tag + flags + child count).

### 4.3 Style Encoding

Styles are a sequence of (property_id, value) pairs terminated by `0x00`:

```
┌───────────┬────────────┬───────────┬────────────┬──────┐
│ Prop ID   │ Value      │ Prop ID   │ Value      │ 0x00 │
│ 1 byte    │ variable   │ 1 byte    │ variable   │ term │
└───────────┴────────────┴───────────┴────────────┴──────┘
```

**Value encoding by type:**

| Type         | Encoding                                    | Bytes  |
|--------------|---------------------------------------------|--------|
| Pixel value  | `0x01` + uint16                             | 3      |
| Percentage   | `0x02` + uint8                              | 2      |
| Color (hex)  | `0x03` + 3 bytes (RGB)                      | 4      |
| Color (rgba) | `0x04` + 4 bytes (RGBA)                     | 5      |
| Enum value   | `0x05` + uint8 (maps to `cen`, `col`, etc.) | 2      |
| Rem value    | `0x06` + uint8 (in 0.25rem increments)      | 2      |
| Composite    | `0x07` + length + raw bytes                 | 2 + n  |
| Keyword      | `0x08` + uint8 (auto, none, inherit, etc.)  | 2      |

### 4.4 Text Encoding

```
┌─────────┬──────────────┐
│ Length   │ UTF-8 bytes  │
│ varint   │ ...          │
└─────────┴──────────────┘
```

Dynamic text segments (`${...}`) are encoded as:

```
┌──────┬──────────┬──────┬───────────┬──────┬──────────┐
│ 0xFE │ static   │ 0xFF │ binding   │ 0xFE │ static   │ ...
│ flag │ text     │ flag │ key path  │ flag │ text     │
└──────┴──────────┴──────┴───────────┴──────┴──────────┘
```

### 4.5 Diff/Patch Encoding

When the LLM outputs an incremental update, the encoder produces a patch frame instead of a full frame.

**Patch operations:**

| Opcode | Operation         | Payload                     |
|--------|-------------------|-----------------------------|
| `0x80` | Replace node      | target_id + new node        |
| `0x81` | Update styles     | target_id + style pairs     |
| `0x82` | Update text       | target_id + new text        |
| `0x83` | Append child      | parent_id + new node        |
| `0x84` | Remove node       | target_id                   |
| `0x85` | Insert before     | sibling_id + new node       |
| `0x86` | Update attribute  | target_id + key + value     |
| `0x87` | Batch             | count + operations[]        |

**DSL diff syntax:**

```
@patch {
  #email [bd:1,solid,ff4444]
  #error > t `Invalid email address`
  #submit [op:0.5] (disabled:true)
}
```

The `@patch` block tells the encoder to emit patch opcodes targeting existing nodes by ID, rather than a full re-render.

---

## 5. Rust Encoder Architecture

### 5.1 Pipeline

```
DSL Text
  │
  ▼
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Lexer   │───▶│  Parser  │───▶│ Encoder  │───▶│  zstd    │
│ (tokenize│    │ (build   │    │ (emit    │    │ compress │
│  stream) │    │  AST)    │    │  bytes)  │    │ (optional│
└──────────┘    └──────────┘    └──────────┘    └──────────┘
                                                      │
                                                      ▼
                                              Binary Frame
                                              (WebSocket-ready)
```

### 5.2 Core Data Structures

```rust
#[derive(Debug, Clone)]
pub enum DslNode {
    Element {
        tag: Tag,
        id: Option<String>,
        classes: Vec<String>,
        styles: Vec<(StyleProp, StyleValue)>,
        attrs: Vec<(String, String)>,
        events: Vec<Event>,
        children: Vec<DslNode>,
    },
    Text {
        content: TextContent,
    },
    Conditional {
        binding: String,
        children: Vec<DslNode>,
    },
    Loop {
        source: String,
        binding: String,
        children: Vec<DslNode>,
    },
    Patch {
        operations: Vec<PatchOp>,
    },
}

#[repr(u8)]
pub enum Tag {
    Div = 0x01,
    Span = 0x02,
    Button = 0x03,
    Input = 0x04,
    Img = 0x05,
    Anchor = 0x06,
    Paragraph = 0x07,
    List = 0x08,
    ListItem = 0x09,
    Form = 0x0A,
    Table = 0x0B,
    TableRow = 0x0C,
    TableCell = 0x0D,
    Svg = 0x0E,
    Canvas = 0x0F,
    H1 = 0x10,
    H2 = 0x11,
    H3 = 0x12,
    H4 = 0x13,
    H5 = 0x14,
    H6 = 0x15,
}

#[repr(u8)]
pub enum StyleProp {
    Width = 0x01,
    Height = 0x02,
    Padding = 0x03,
    Margin = 0x04,
    Background = 0x05,
    Color = 0x06,
    FontSize = 0x07,
    FontWeight = 0x08,
    BorderRadius = 0x09,
    Border = 0x0A,
    Display = 0x0B,
    Flex = 0x0C,
    FlexDirection = 0x0D,
    AlignItems = 0x0E,
    JustifyContent = 0x0F,
    Gap = 0x10,
    // ... remaining props
}

pub enum StyleValue {
    Px(u16),
    Percent(u8),
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
    Enum(EnumValue),
    Rem(u8),       // 0.25rem increments
    Composite(Vec<u8>),
    Keyword(Keyword),
}

pub struct Event {
    pub trigger: EventTrigger,  // click, input, hover, etc.
    pub action: String,         // namespace:action
    pub params: Vec<String>,    // optional parameters
}
```

### 5.3 Encoder Core

```rust
pub struct BinaryEncoder {
    buffer: Vec<u8>,
    string_table: HashMap<String, u16>,  // dedup repeated strings
}

impl BinaryEncoder {
    pub fn encode_frame(&mut self, root: &DslNode, is_patch: bool) -> Vec<u8> {
        self.buffer.clear();

        // Frame header
        self.buffer.push(0x01);  // version
        let flags = if is_patch { 0x01 } else { 0x00 };
        self.buffer.push(flags);

        // Node count placeholder (backfill)
        let count_pos = self.buffer.len();
        self.buffer.extend_from_slice(&[0x00, 0x00]);

        let count = self.encode_node(root);

        // Backfill count
        let count_bytes = (count as u16).to_le_bytes();
        self.buffer[count_pos] = count_bytes[0];
        self.buffer[count_pos + 1] = count_bytes[1];

        // Optional: zstd compress if payload > threshold
        if self.buffer.len() > 256 {
            self.buffer[1] |= 0x02;  // set compression flag
            let compressed = zstd::encode_all(
                &self.buffer[4..],  // compress after header
                3                    // compression level
            ).unwrap();
            self.buffer.truncate(4);
            self.buffer.extend(compressed);
        }

        self.buffer.clone()
    }

    fn encode_node(&mut self, node: &DslNode) -> u32 {
        match node {
            DslNode::Element { tag, id, classes, styles, attrs, events, children } => {
                // Tag byte
                self.buffer.push(*tag as u8);

                // Build flags
                let mut flags: u8 = 0;
                if id.is_some()       { flags |= 0x01; }
                if !classes.is_empty() { flags |= 0x02; }
                if !styles.is_empty()  { flags |= 0x04; }
                if !attrs.is_empty()   { flags |= 0x08; }
                // text flag set separately for Text variant
                if !children.is_empty(){ flags |= 0x20; }
                if !events.is_empty()  { flags |= 0x40; }
                self.buffer.push(flags);

                // Encode each present section
                if let Some(id) = id {
                    self.encode_string(id);
                }
                if !classes.is_empty() {
                    self.buffer.push(classes.len() as u8);
                    for class in classes {
                        self.encode_string(class);
                    }
                }
                if !styles.is_empty() {
                    self.encode_styles(styles);
                }
                if !attrs.is_empty() {
                    self.encode_attrs(attrs);
                }
                if !events.is_empty() {
                    self.encode_events(events);
                }
                if !children.is_empty() {
                    self.encode_varint(children.len() as u64);
                    let mut count = 1;
                    for child in children {
                        count += self.encode_node(child);
                    }
                    return count;
                }
                1
            }
            DslNode::Text { content } => {
                self.buffer.push(0x02);  // span/text tag
                self.buffer.push(0x10);  // has-text flag
                self.encode_text_content(content);
                1
            }
            // ... Conditional, Loop, Patch variants
            _ => { 1 }
        }
    }

    fn encode_styles(&mut self, styles: &[(StyleProp, StyleValue)]) {
        for (prop, value) in styles {
            self.buffer.push(*prop as u8);
            self.encode_style_value(value);
        }
        self.buffer.push(0x00);  // terminator
    }

    fn encode_style_value(&mut self, value: &StyleValue) {
        match value {
            StyleValue::Px(v) => {
                self.buffer.push(0x01);
                self.buffer.extend_from_slice(&v.to_le_bytes());
            }
            StyleValue::Percent(v) => {
                self.buffer.push(0x02);
                self.buffer.push(*v);
            }
            StyleValue::Rgb(r, g, b) => {
                self.buffer.push(0x03);
                self.buffer.extend_from_slice(&[*r, *g, *b]);
            }
            StyleValue::Enum(e) => {
                self.buffer.push(0x05);
                self.buffer.push(*e as u8);
            }
            // ... remaining variants
            _ => {}
        }
    }

    fn encode_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.encode_varint(bytes.len() as u64);
        self.buffer.extend_from_slice(bytes);
    }

    fn encode_varint(&mut self, mut value: u64) {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 { byte |= 0x80; }
            self.buffer.push(byte);
            if value == 0 { break; }
        }
    }
}
```

---

## 6. Client Decoder (JavaScript)

The client receives binary frames over WebSocket and materializes DOM nodes.

```javascript
class BinaryUIDecoder {
  constructor(container) {
    this.container = container;
    this.nodeMap = new Map();  // id -> DOM node
    this.actionHandlers = new Map();
  }

  handleFrame(arrayBuffer) {
    const view = new DataView(arrayBuffer);
    let offset = 0;

    const version = view.getUint8(offset++);
    const flags = view.getUint8(offset++);
    const nodeCount = view.getUint16(offset, true); offset += 2;

    const isPatch = (flags & 0x01) !== 0;
    const isCompressed = (flags & 0x02) !== 0;

    let payload = new Uint8Array(arrayBuffer, 4);
    if (isCompressed) {
      payload = this.decompress(payload);  // zstd-wasm
    }

    if (isPatch) {
      this.applyPatch(payload);
    } else {
      const root = this.decodeNode(payload, { offset: 0 });
      this.container.innerHTML = '';
      this.container.appendChild(root);
    }
  }

  decodeNode(data, cursor) {
    const tag = data[cursor.offset++];
    const flags = data[cursor.offset++];

    const el = document.createElement(TAG_MAP[tag]);

    if (flags & 0x01) {  // has ID
      const id = this.decodeString(data, cursor);
      el.id = id;
      this.nodeMap.set(id, el);
    }

    if (flags & 0x02) {  // has classes
      const count = data[cursor.offset++];
      for (let i = 0; i < count; i++) {
        el.classList.add(this.decodeString(data, cursor));
      }
    }

    if (flags & 0x04) {  // has styles
      this.decodeStyles(data, cursor, el);
    }

    if (flags & 0x08) {  // has attributes
      this.decodeAttrs(data, cursor, el);
    }

    if (flags & 0x10) {  // has text
      el.textContent = this.decodeTextContent(data, cursor);
    }

    if (flags & 0x20) {  // has children
      const childCount = this.decodeVarint(data, cursor);
      for (let i = 0; i < childCount; i++) {
        el.appendChild(this.decodeNode(data, cursor));
      }
    }

    if (flags & 0x40) {  // has events
      this.decodeEvents(data, cursor, el);
    }

    return el;
  }

  decodeStyles(data, cursor, el) {
    while (true) {
      const propId = data[cursor.offset++];
      if (propId === 0x00) break;  // terminator

      const prop = STYLE_PROP_MAP[propId];
      const value = this.decodeStyleValue(data, cursor);
      el.style[prop] = value;
    }
  }

  decodeStyleValue(data, cursor) {
    const type = data[cursor.offset++];
    switch (type) {
      case 0x01: {  // px
        const v = data[cursor.offset] | (data[cursor.offset + 1] << 8);
        cursor.offset += 2;
        return `${v}px`;
      }
      case 0x02:  // percent
        return `${data[cursor.offset++]}%`;
      case 0x03: {  // rgb
        const r = data[cursor.offset++];
        const g = data[cursor.offset++];
        const b = data[cursor.offset++];
        return `#${r.toString(16).padStart(2,'0')}${g.toString(16).padStart(2,'0')}${b.toString(16).padStart(2,'0')}`;
      }
      case 0x05:  // enum
        return ENUM_VALUE_MAP[data[cursor.offset++]];
      // ... remaining types
    }
  }

  applyPatch(data) {
    const cursor = { offset: 0 };
    while (cursor.offset < data.length) {
      const op = data[cursor.offset++];
      const targetId = this.decodeString(data, cursor);
      const target = this.nodeMap.get(targetId);
      if (!target) continue;

      switch (op) {
        case 0x81:  // update styles
          this.decodeStyles(data, cursor, target);
          break;
        case 0x82:  // update text
          target.textContent = this.decodeTextContent(data, cursor);
          break;
        case 0x83:  // append child
          target.appendChild(this.decodeNode(data, cursor));
          break;
        case 0x84:  // remove node
          target.remove();
          this.nodeMap.delete(targetId);
          break;
        // ... remaining ops
      }
    }
  }

  decodeString(data, cursor) {
    const len = this.decodeVarint(data, cursor);
    const bytes = data.slice(cursor.offset, cursor.offset + len);
    cursor.offset += len;
    return new TextDecoder().decode(bytes);
  }

  decodeVarint(data, cursor) {
    let value = 0, shift = 0;
    while (true) {
      const byte = data[cursor.offset++];
      value |= (byte & 0x7F) << shift;
      if ((byte & 0x80) === 0) break;
      shift += 7;
    }
    return value;
  }
}

// Lookup tables
const TAG_MAP = {
  0x01: 'div', 0x02: 'span', 0x03: 'button', 0x04: 'input',
  0x05: 'img', 0x06: 'a', 0x07: 'p', 0x08: 'ul', 0x09: 'li',
  0x0A: 'form', 0x0B: 'table', 0x0C: 'tr', 0x0D: 'td',
  0x0E: 'svg', 0x0F: 'canvas',
  0x10: 'h1', 0x11: 'h2', 0x12: 'h3', 0x13: 'h4', 0x14: 'h5', 0x15: 'h6',
};

const STYLE_PROP_MAP = {
  0x01: 'width', 0x02: 'height', 0x03: 'padding', 0x04: 'margin',
  0x05: 'background', 0x06: 'color', 0x07: 'fontSize', 0x08: 'fontWeight',
  0x09: 'borderRadius', 0x0A: 'border', 0x0B: 'display', 0x0C: 'flex',
  0x0D: 'flexDirection', 0x0E: 'alignItems', 0x0F: 'justifyContent',
  0x10: 'gap', 0x11: 'position',
  // ... remaining props
};
```

---

## 7. LLM Integration

### 7.1 System Prompt Template

```
You are a UI generator. You output interfaces in BinUI DSL format.

SYNTAX:
- Elements: v (div), t (text), b (button), i (input), h1-h6, p, ls (list), li, fm (form)
- Styles: [property:value ...] — w:width h:height p:padding m:margin bg:color c:color fs:font-size fw:font-weight br:border-radius bd:border d:display fx:flex fd:flex-direction ai:align-items jc:justify-content g:gap
- Values: bare numbers = px. Colors = hex without #. Shortcuts: cen=center col=column bet=space-between bol=bold ptr=pointer
- Text: backtick-wrapped `like this`
- IDs: #id, Classes: .class
- Events: @click:action_name
- Patches: @patch { #id [style:changes] }

EXAMPLE:
v#root [d:fx fd:col ai:cen jc:cen h:100vh bg:0f0f23] {
  h1 [c:fff fs:48] `Hello World`
  b#cta @click:start [p:12,24 br:8 bg:6c63ff c:fff cur:ptr] `Get Started`
}

Respond with ONLY DSL code. No markdown. No explanation.
```

### 7.2 Few-Shot Examples in Context

Include 5-10 DSL examples covering common patterns:

1. **Navigation bar** — horizontal flex layout, links, logo
2. **Card component** — rounded box with image, text, button
3. **Form** — labeled inputs, validation states, submit
4. **Data table** — headers, rows, alternating colors
5. **Modal** — overlay, centered card, close button
6. **Dashboard** — grid layout, stat cards, charts placeholder
7. **Chat interface** — message list, input bar, avatars
8. **Landing hero** — full viewport, gradient, CTA buttons

With these examples in context, even a base model (no fine-tuning) should produce valid DSL for most standard UI patterns.

### 7.3 GBNF Grammar for llama.cpp

For constrained decoding, use a grammar to ensure the LLM only outputs valid DSL tokens:

```gbnf
root        ::= node+
node        ::= element | text-node | patch-block | conditional | loop
element     ::= tag id? classes? attrs? styles? events? text-inline? block?
tag         ::= "v" | "t" | "b" | "i" | "img" | "a" | "p" | "ls" | "li"
              | "fm" | "tb" | "tr" | "td" | "sv" | "cn"
              | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
id          ::= "#" identifier
classes     ::= ("." identifier)+
identifier  ::= [a-zA-Z_] [a-zA-Z0-9_-]*
attrs       ::= "(" attr-pair ("," ws attr-pair)* ")"
attr-pair   ::= identifier ":" attr-value
attr-value  ::= identifier | number | backtick-string
styles      ::= "[" style-pair (ws style-pair)* "]"
style-pair  ::= style-prop ":" style-value
style-prop  ::= "w"|"h"|"p"|"m"|"bg"|"c"|"fs"|"fw"|"br"|"bd"|"d"|"fx"|"fd"
              | "ai"|"jc"|"g"|"pos"|"top"|"left"|"right"|"bottom"|"z"|"ov"
              | "op"|"sh"|"tr"|"tf"|"gr"|"gtc"|"gtr"|"ta"|"ws"|"cur"
style-value ::= number unit? | color-hex | enum-val | composite-val
number      ::= "-"? [0-9]+ ("." [0-9]+)?
unit        ::= "%" | "em" | "rem" | "vw" | "vh"
color-hex   ::= [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]
              ( [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] )?
enum-val    ::= "cen"|"col"|"row"|"bet"|"aro"|"str"|"abs"|"rel"|"fix"
              | "hid"|"aut"|"non"|"ptr"|"bol"
composite-val ::= style-value ("," style-value)*
events      ::= (ws event)+
event       ::= "@" event-type ":" identifier (":" identifier)*
event-type  ::= "click"|"input"|"hover"|"focus"|"blur"|"submit"|"change"
text-inline ::= ws backtick-string
backtick-string ::= "`" [^`]* "`"
block       ::= ws "{" ws node* ws "}"
patch-block ::= "@patch" ws "{" ws patch-op* ws "}"
patch-op    ::= "#" identifier ws (styles | ">" ws node | attrs)
conditional ::= "?" identifier ws block
loop        ::= "*" identifier ws "->" ws identifier ws block
ws          ::= [ \t\n\r]*
```

### 7.4 Streaming Integration

The LLM streams DSL tokens as they're generated. The encoder can operate in streaming mode:

```
LLM token stream → DSL parser (incremental) → partial AST → encode complete subtrees → WebSocket
```

When a `}` closes a subtree, that subtree is fully parseable and can be encoded + sent immediately. The client renders progressively as frames arrive, giving instant visual feedback while the LLM is still generating.

---

## 8. Performance Characteristics

### Token Efficiency

| UI Component    | HTML Tokens | DSL Tokens | Reduction |
|-----------------|-------------|------------|-----------|
| Login form      | ~180        | ~45        | 75%       |
| Nav bar         | ~120        | ~35        | 71%       |
| Card component  | ~90         | ~25        | 72%       |
| Data table row  | ~60         | ~18        | 70%       |
| Dashboard page  | ~800        | ~200       | 75%       |
| Full SPA layout | ~2000       | ~500       | 75%       |

### Wire Size

| UI Component    | Raw HTML+CSS | DSL Binary | Binary+zstd | Reduction |
|-----------------|--------------|------------|-------------|-----------|
| Login form      | ~1.8 KB      | ~280 B     | ~180 B      | 90%       |
| Dashboard page  | ~12 KB       | ~1.8 KB    | ~900 B      | 92%       |
| Full SPA        | ~35 KB       | ~5 KB      | ~2.5 KB     | 93%       |

### Encoding Speed (Rust)

Target: < 100μs for a full page encode on a single core. The encoder is a simple tree walk with byte emission — no allocations needed beyond the output buffer when using an arena allocator.

---

## 9. Migration Path

### Phase 1: DSL + Encoder (Week 1-2)
- Implement DSL lexer/parser in Rust
- Implement binary encoder
- Unit test with 20+ UI component examples
- Validate round-trip: DSL → binary → DOM matches expected output

### Phase 2: Client Decoder (Week 2-3)
- Implement JavaScript decoder
- WebSocket integration
- Patch/diff support
- Basic action handler registry

### Phase 3: LLM Integration (Week 3-4)
- System prompt engineering with few-shot examples
- GBNF grammar for llama.cpp constrained decoding
- Streaming pipeline: LLM → parser → encoder → WebSocket
- Benchmark token throughput on MI50 setup

### Phase 4: Training Data (Week 4-6)
- Build programmatic UI generator (random valid DSL + screenshots)
- Generate 10k+ training pairs
- Fine-tune 7B VLM with LoRA on (screenshot + prompt → DSL)
- Evaluate against few-shot baseline

---

## 10. Future: Direct Binary Output (Option 1 Upgrade)

Once the DSL is stable and training data exists, the DSL intermediary can eventually be eliminated:

1. Generate (screenshot → DSL) pairs at scale
2. Compile DSL → binary for each pair
3. Encode binary as base256 tokens with a custom tokenizer
4. Fine-tune directly on (screenshot → binary tokens)

The DSL phase de-risks this by establishing the format, building training data, and validating the binary encoding — all before committing to the harder problem of training a model to think in bytes.
