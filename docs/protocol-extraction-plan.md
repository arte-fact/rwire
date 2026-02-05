# Protocol Extraction & DSL Experimentation Plan

## Overview

This document outlines the plan to extract rwire's binary protocol into a standalone `rwire-protocol` crate and create a `rwire-dsl` crate for DSL-based UI generation experiments.

**Goals:**
1. Clean separation of protocol (opcodes, encoding) from framework (builder, state, server)
2. Enable DSL experiments that share the same binary wire format
3. Single JS decoder works with both rwire and DSL-generated UIs

---

## Current Architecture

### Protocol Module (~1,083 LOC)

```
rwire/src/protocol/
├── mod.rs        (13 LOC)   - Re-exports
├── opcodes.rs    (308 LOC)  - El, Ev enums + opcode constants
├── encoder.rs    (466 LOC)  - OpcodeBuffer (symbol tables, DOM ops)
├── decoder.rs    (120 LOC)  - ClientEvent parsing
└── varint.rs     (176 LOC)  - Variable-length integer encoding
```

### Dependencies

| Type | Dependencies |
|------|--------------|
| **External** | `bytes` crate only (BufMut, BytesMut) |
| **Internal** | Zero - fully self-contained |

### Consumers of Protocol

| Module | Usage | Impact |
|--------|-------|--------|
| `builder.rs` | OpcodeBuffer, El, Ev | Heavy |
| `item_ref.rs` | read_varint, write_varint | Medium |
| `server.rs` | ClientEvent, OpcodeBuffer | Medium |
| `state.rs` | MUT_* opcode constants | Low |
| `router.rs` | El enum | Very low |
| `form.rs` | El, Ev enums | Low |
| `capsule_gen.rs` | Duplicates element/event mappings | Medium |

---

## Phase 1: Extract `rwire-protocol`

### Target Structure

```
rwire/
├── rwire-protocol/          # NEW CRATE
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # Public re-exports
│       ├── opcodes.rs       # El, Ev enums, opcode constants
│       ├── encoder.rs       # OpcodeBuffer
│       ├── decoder.rs       # ClientEvent, DecodeError
│       ├── varint.rs        # Varint encoding/decoding
│       └── mappings.rs      # Element/event name mappings (NEW)
├── rwire/                   # UPDATED - depends on rwire-protocol
├── rwire-macros/
└── examples/
```

### Tasks

#### 1.1 Create `rwire-protocol/Cargo.toml`

```toml
[package]
name = "rwire-protocol"
version = "0.1.0"
edition = "2021"
description = "Binary protocol for rwire WebSocket UI framework"

[dependencies]
bytes = "1"
```

#### 1.2 Create `rwire-protocol/src/lib.rs`

```rust
//! Binary protocol for rwire WebSocket UI framework.
//!
//! This crate provides the encoding/decoding primitives shared between
//! the rwire framework and DSL experiments.

mod opcodes;
mod encoder;
mod decoder;
mod varint;
mod mappings;

pub use opcodes::*;
pub use encoder::OpcodeBuffer;
pub use decoder::{ClientEvent, DecodeError};
pub use varint::{read_varint, write_varint};
pub use mappings::{ELEMENT_MAPPINGS, EVENT_MAPPINGS};
```

#### 1.3 Move Protocol Files

| Source | Destination |
|--------|-------------|
| `rwire/src/protocol/opcodes.rs` | `rwire-protocol/src/opcodes.rs` |
| `rwire/src/protocol/encoder.rs` | `rwire-protocol/src/encoder.rs` |
| `rwire/src/protocol/decoder.rs` | `rwire-protocol/src/decoder.rs` |
| `rwire/src/protocol/varint.rs` | `rwire-protocol/src/varint.rs` |

Update internal imports from `super::` to `crate::`.

#### 1.4 Create `rwire-protocol/src/mappings.rs`

Consolidate from `capsule_gen.rs`:

```rust
//! Element and event type mappings to HTML/JS names.
//!
//! Used by capsule generation and DSL parsing.

/// Maps element type byte to HTML tag name.
pub const ELEMENT_MAPPINGS: &[(u8, &str)] = &[
    (0, "div"),
    (1, "span"),
    (2, "button"),
    (3, "input"),
    (4, "textarea"),
    (5, "select"),
    (6, "option"),
    (7, "label"),
    (8, "form"),
    (9, "a"),
    (10, "img"),
    (11, "ul"),
    (12, "ol"),
    (13, "li"),
    (14, "table"),
    (15, "thead"),
    (16, "tbody"),
    (17, "tr"),
    (18, "th"),
    (19, "td"),
    (20, "h1"),
    (21, "h2"),
    (22, "h3"),
    (23, "p"),
    (24, "br"),
];

/// Maps event type byte to JavaScript event name.
pub const EVENT_MAPPINGS: &[(u8, &str)] = &[
    (1, "click"),
    (2, "dblclick"),
    (3, "input"),
    (4, "change"),
    (5, "submit"),
    (6, "keydown"),
    (7, "keyup"),
    (8, "focus"),
    (9, "blur"),
    (10, "mouseenter"),
    (11, "mouseleave"),
];

/// Get HTML tag name for element type.
pub fn element_name(el_type: u8) -> Option<&'static str> {
    ELEMENT_MAPPINGS.iter()
        .find(|(code, _)| *code == el_type)
        .map(|(_, name)| *name)
}

/// Get JS event name for event type.
pub fn event_name(ev_type: u8) -> Option<&'static str> {
    EVENT_MAPPINGS.iter()
        .find(|(code, _)| *code == ev_type)
        .map(|(_, name)| *name)
}
```

#### 1.5 Update `rwire/Cargo.toml`

```toml
[dependencies]
rwire-protocol = { path = "../rwire-protocol" }
# ... existing dependencies
```

#### 1.6 Update rwire Imports

**`rwire/src/lib.rs`:**
```rust
// Re-export protocol types for backward compatibility
pub use rwire_protocol::{
    El, Ev, OpcodeBuffer, ClientEvent, DecodeError,
    read_varint, write_varint,
    ELEMENT_MAPPINGS, EVENT_MAPPINGS,
};
```

**All other files** - replace:
```rust
// Before
use crate::protocol::{...};

// After
use rwire_protocol::{...};
```

#### 1.7 Update `capsule_gen.rs`

Remove duplicated constants, import from rwire-protocol:

```rust
use rwire_protocol::{ELEMENT_MAPPINGS, EVENT_MAPPINGS};

// Remove local ELEMENT_MAPPINGS and EVENT_MAPPINGS definitions
```

#### 1.8 Delete Old Protocol Module

Remove `rwire/src/protocol/` directory after migration is complete.

---

## Phase 2: Create `rwire-dsl`

### Target Structure

```
rwire/
├── rwire-dsl/               # NEW CRATE
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # Public API
│       ├── lexer.rs         # Tokenize DSL text
│       ├── parser.rs        # Build AST from tokens
│       ├── ast.rs           # DslNode enum and types
│       └── emit.rs          # AST → OpcodeBuffer
```

### Tasks

#### 2.1 Create `rwire-dsl/Cargo.toml`

```toml
[package]
name = "rwire-dsl"
version = "0.1.0"
edition = "2021"
description = "DSL parser for rwire binary UI protocol"

[dependencies]
rwire-protocol = { path = "../rwire-protocol" }

[dev-dependencies]
```

#### 2.2 Create `rwire-dsl/src/ast.rs`

```rust
//! DSL Abstract Syntax Tree types.

/// A node in the DSL tree.
#[derive(Debug, Clone)]
pub enum DslNode {
    /// Element node with tag, attributes, styles, and children.
    Element {
        tag: String,           // "v", "b", "i", "h1", etc.
        id: Option<String>,
        classes: Vec<String>,
        styles: Vec<StylePair>,
        attrs: Vec<AttrPair>,
        events: Vec<DslEvent>,
        text: Option<String>,
        children: Vec<DslNode>,
    },
    /// Raw text node.
    Text(String),
}

/// A style property-value pair.
#[derive(Debug, Clone)]
pub struct StylePair {
    pub prop: String,   // "d", "w", "h", "bg", etc.
    pub value: String,  // "fx", "100", "fff", etc.
}

/// An attribute name-value pair.
#[derive(Debug, Clone)]
pub struct AttrPair {
    pub name: String,
    pub value: String,
}

/// An event binding.
#[derive(Debug, Clone)]
pub struct DslEvent {
    pub event_type: String,  // "click", "input", etc.
    pub action: String,      // "submit", "toggle:sidebar", etc.
}

impl DslNode {
    /// Create a new element node.
    pub fn element(tag: impl Into<String>) -> Self {
        DslNode::Element {
            tag: tag.into(),
            id: None,
            classes: Vec::new(),
            styles: Vec::new(),
            attrs: Vec::new(),
            events: Vec::new(),
            text: None,
            children: Vec::new(),
        }
    }

    /// Create a text node.
    pub fn text(content: impl Into<String>) -> Self {
        DslNode::Text(content.into())
    }
}
```

#### 2.3 Create `rwire-dsl/src/lexer.rs`

```rust
//! DSL lexer - tokenizes input text.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Tags
    Tag(String),         // v, b, i, h1, etc.

    // Identifiers
    Id(String),          // #id
    Class(String),       // .class

    // Brackets
    LBracket,            // [
    RBracket,            // ]
    LParen,              // (
    RParen,              // )
    LBrace,              // {
    RBrace,              // }

    // Punctuation
    Colon,               // :
    Comma,               // ,
    At,                  // @

    // Values
    Text(String),        // `text content`
    Ident(String),       // bare identifier
    Number(String),      // numeric value

    // End
    Eof,
}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Token::Eof;
        }

        let ch = self.current_char();

        match ch {
            '#' => self.read_id(),
            '.' => self.read_class(),
            '[' => { self.pos += 1; Token::LBracket }
            ']' => { self.pos += 1; Token::RBracket }
            '(' => { self.pos += 1; Token::LParen }
            ')' => { self.pos += 1; Token::RParen }
            '{' => { self.pos += 1; Token::LBrace }
            '}' => { self.pos += 1; Token::RBrace }
            ':' => { self.pos += 1; Token::Colon }
            ',' => { self.pos += 1; Token::Comma }
            '@' => { self.pos += 1; Token::At }
            '`' => self.read_text(),
            _ if ch.is_ascii_alphabetic() => self.read_tag_or_ident(),
            _ if ch.is_ascii_digit() || ch == '-' => self.read_number(),
            _ => { self.pos += 1; self.next_token() }
        }
    }

    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap_or('\0')
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn read_id(&mut self) -> Token {
        self.pos += 1; // skip #
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        Token::Id(self.input[start..self.pos].to_string())
    }

    fn read_class(&mut self) -> Token {
        self.pos += 1; // skip .
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        Token::Class(self.input[start..self.pos].to_string())
    }

    fn read_text(&mut self) -> Token {
        self.pos += 1; // skip opening `
        let start = self.pos;
        while self.pos < self.input.len() && self.current_char() != '`' {
            self.pos += 1;
        }
        let text = self.input[start..self.pos].to_string();
        if self.pos < self.input.len() {
            self.pos += 1; // skip closing `
        }
        Token::Text(text)
    }

    fn read_tag_or_ident(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let s = self.input[start..self.pos].to_string();

        // Check if it's a known tag
        match s.as_str() {
            "v" | "t" | "b" | "i" | "img" | "a" | "p" | "ls" | "li" |
            "fm" | "tb" | "tr" | "td" | "sv" | "cn" |
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Token::Tag(s),
            _ => Token::Ident(s),
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        if self.current_char() == '-' {
            self.pos += 1;
        }
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_ascii_digit() || ch == '.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        // Include unit suffix
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch == '%' || ch.is_ascii_alphabetic() {
                self.pos += 1;
            } else {
                break;
            }
        }
        Token::Number(self.input[start..self.pos].to_string())
    }
}
```

#### 2.4 Create `rwire-dsl/src/parser.rs`

```rust
//! DSL parser - builds AST from tokens.

use crate::ast::{DslNode, StylePair, AttrPair, DslEvent};
use crate::lexer::{Lexer, Token};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token();
        Self { lexer, current }
    }

    pub fn parse(&mut self) -> Result<Vec<DslNode>, String> {
        let mut nodes = Vec::new();
        while self.current != Token::Eof {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn advance(&mut self) {
        self.current = self.lexer.next_token();
    }

    fn parse_node(&mut self) -> Result<DslNode, String> {
        match &self.current {
            Token::Tag(tag) => self.parse_element(tag.clone()),
            Token::Text(text) => {
                let node = DslNode::text(text.clone());
                self.advance();
                Ok(node)
            }
            _ => Err(format!("Expected tag or text, got {:?}", self.current)),
        }
    }

    fn parse_element(&mut self, tag: String) -> Result<DslNode, String> {
        self.advance(); // consume tag

        let mut id = None;
        let mut classes = Vec::new();
        let mut styles = Vec::new();
        let mut attrs = Vec::new();
        let mut events = Vec::new();
        let mut text = None;
        let mut children = Vec::new();

        // Parse id and classes
        loop {
            match &self.current {
                Token::Id(i) => {
                    id = Some(i.clone());
                    self.advance();
                }
                Token::Class(c) => {
                    classes.push(c.clone());
                    self.advance();
                }
                _ => break,
            }
        }

        // Parse attributes (...)
        if self.current == Token::LParen {
            attrs = self.parse_attrs()?;
        }

        // Parse styles [...]
        if self.current == Token::LBracket {
            styles = self.parse_styles()?;
        }

        // Parse events @event:action
        while self.current == Token::At {
            events.push(self.parse_event()?);
        }

        // Parse inline text `...`
        if let Token::Text(t) = &self.current {
            text = Some(t.clone());
            self.advance();
        }

        // Parse children {...}
        if self.current == Token::LBrace {
            self.advance(); // consume {
            while self.current != Token::RBrace && self.current != Token::Eof {
                children.push(self.parse_node()?);
            }
            if self.current == Token::RBrace {
                self.advance(); // consume }
            }
        }

        Ok(DslNode::Element {
            tag,
            id,
            classes,
            styles,
            attrs,
            events,
            text,
            children,
        })
    }

    fn parse_styles(&mut self) -> Result<Vec<StylePair>, String> {
        self.advance(); // consume [
        let mut styles = Vec::new();

        while self.current != Token::RBracket && self.current != Token::Eof {
            let prop = match &self.current {
                Token::Ident(i) => i.clone(),
                Token::Tag(t) => t.clone(), // some props like "p" are also tags
                _ => return Err(format!("Expected style property, got {:?}", self.current)),
            };
            self.advance();

            if self.current != Token::Colon {
                return Err("Expected ':' after style property".to_string());
            }
            self.advance();

            let value = self.parse_style_value()?;
            styles.push(StylePair { prop, value });
        }

        if self.current == Token::RBracket {
            self.advance();
        }
        Ok(styles)
    }

    fn parse_style_value(&mut self) -> Result<String, String> {
        let mut parts = Vec::new();

        loop {
            match &self.current {
                Token::Number(n) => {
                    parts.push(n.clone());
                    self.advance();
                }
                Token::Ident(i) => {
                    parts.push(i.clone());
                    self.advance();
                }
                Token::Comma => {
                    parts.push(",".to_string());
                    self.advance();
                }
                _ => break,
            }
        }

        if parts.is_empty() {
            return Err("Expected style value".to_string());
        }
        Ok(parts.join(""))
    }

    fn parse_attrs(&mut self) -> Result<Vec<AttrPair>, String> {
        self.advance(); // consume (
        let mut attrs = Vec::new();

        while self.current != Token::RParen && self.current != Token::Eof {
            let name = match &self.current {
                Token::Ident(i) => i.clone(),
                _ => return Err(format!("Expected attribute name, got {:?}", self.current)),
            };
            self.advance();

            if self.current != Token::Colon {
                return Err("Expected ':' after attribute name".to_string());
            }
            self.advance();

            let value = match &self.current {
                Token::Ident(i) => i.clone(),
                Token::Text(t) => t.clone(),
                Token::Number(n) => n.clone(),
                _ => return Err(format!("Expected attribute value, got {:?}", self.current)),
            };
            self.advance();

            attrs.push(AttrPair { name, value });
        }

        if self.current == Token::RParen {
            self.advance();
        }
        Ok(attrs)
    }

    fn parse_event(&mut self) -> Result<DslEvent, String> {
        self.advance(); // consume @

        let event_type = match &self.current {
            Token::Ident(i) => i.clone(),
            _ => return Err(format!("Expected event type, got {:?}", self.current)),
        };
        self.advance();

        if self.current != Token::Colon {
            return Err("Expected ':' after event type".to_string());
        }
        self.advance();

        let mut action_parts = Vec::new();
        loop {
            match &self.current {
                Token::Ident(i) => {
                    action_parts.push(i.clone());
                    self.advance();
                }
                Token::Colon => {
                    action_parts.push(":".to_string());
                    self.advance();
                }
                _ => break,
            }
        }

        Ok(DslEvent {
            event_type,
            action: action_parts.join(""),
        })
    }
}
```

#### 2.5 Create `rwire-dsl/src/emit.rs`

```rust
//! Emit DSL AST to binary opcodes.

use rwire_protocol::{OpcodeBuffer, El, Ev, ELEMENT_MAPPINGS, EVENT_MAPPINGS};
use crate::ast::{DslNode, StylePair};

/// Maps DSL tag to element type byte.
fn tag_to_el(tag: &str) -> Option<u8> {
    match tag {
        "v" => Some(El::Div as u8),
        "t" => Some(El::Span as u8),
        "b" => Some(El::Button as u8),
        "i" => Some(El::Input as u8),
        "img" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "img").map(|(c, _)| *c),
        "a" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "a").map(|(c, _)| *c),
        "p" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "p").map(|(c, _)| *c),
        "h1" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "h1").map(|(c, _)| *c),
        "h2" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "h2").map(|(c, _)| *c),
        "h3" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "h3").map(|(c, _)| *c),
        "ls" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "ul").map(|(c, _)| *c),
        "li" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "li").map(|(c, _)| *c),
        "fm" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "form").map(|(c, _)| *c),
        "tb" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "table").map(|(c, _)| *c),
        "tr" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "tr").map(|(c, _)| *c),
        "td" => ELEMENT_MAPPINGS.iter().find(|(_, n)| *n == "td").map(|(c, _)| *c),
        _ => None,
    }
}

/// Maps DSL style shorthand to CSS property.
fn style_prop_to_css(prop: &str) -> &'static str {
    match prop {
        "w" => "width",
        "h" => "height",
        "p" => "padding",
        "m" => "margin",
        "bg" => "background",
        "c" => "color",
        "fs" => "font-size",
        "fw" => "font-weight",
        "br" => "border-radius",
        "bd" => "border",
        "d" => "display",
        "fx" => "flex",
        "fd" => "flex-direction",
        "ai" => "align-items",
        "jc" => "justify-content",
        "g" => "gap",
        "pos" => "position",
        "top" => "top",
        "left" => "left",
        "right" => "right",
        "bottom" => "bottom",
        "z" => "z-index",
        "ov" => "overflow",
        "op" => "opacity",
        "ta" => "text-align",
        "cur" => "cursor",
        _ => prop, // passthrough unknown
    }
}

/// Maps DSL style value shorthand to CSS value.
fn style_value_to_css(value: &str) -> String {
    match value {
        "cen" => "center".to_string(),
        "col" => "column".to_string(),
        "row" => "row".to_string(),
        "bet" => "space-between".to_string(),
        "aro" => "space-around".to_string(),
        "str" => "stretch".to_string(),
        "abs" => "absolute".to_string(),
        "rel" => "relative".to_string(),
        "fix" => "fixed".to_string(),
        "hid" => "hidden".to_string(),
        "aut" => "auto".to_string(),
        "non" => "none".to_string(),
        "ptr" => "pointer".to_string(),
        "bol" => "bold".to_string(),
        // Check if it's a bare number (add px)
        _ if value.chars().all(|c| c.is_ascii_digit()) => format!("{}px", value),
        // Check if it's a hex color (add #)
        _ if value.len() == 3 || value.len() == 6 => {
            if value.chars().all(|c| c.is_ascii_hexdigit()) {
                format!("#{}", value)
            } else {
                value.to_string()
            }
        }
        _ => value.to_string(),
    }
}

/// Convert style pairs to CSS string.
pub fn styles_to_css(styles: &[StylePair]) -> String {
    styles
        .iter()
        .map(|s| {
            let prop = style_prop_to_css(&s.prop);
            let value = style_value_to_css(&s.value);
            format!("{}:{}", prop, value)
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Emit context for tracking symbols and refs.
pub struct EmitContext {
    symbols: Vec<String>,
    next_ref: u8,
}

impl EmitContext {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            next_ref: 0,
        }
    }

    /// Intern a string, return its symbol index.
    pub fn intern(&mut self, s: &str) -> u16 {
        if let Some(idx) = self.symbols.iter().position(|x| x == s) {
            return 0x80 + idx as u16;
        }
        let idx = self.symbols.len();
        self.symbols.push(s.to_string());
        0x80 + idx as u16
    }

    /// Emit the DSL tree to an OpcodeBuffer.
    pub fn emit(&mut self, nodes: &[DslNode], buf: &mut OpcodeBuffer) {
        // First pass: collect all symbols
        for node in nodes {
            self.collect_symbols(node);
        }

        // Emit symbol table
        if !self.symbols.is_empty() {
            buf.begin_symbols(self.symbols.len());
            for sym in &self.symbols {
                buf.add_symbol(sym);
            }
        }

        // Emit nodes
        for node in nodes {
            self.emit_node(node, buf, 0xFF); // 0xFF = append to body
        }

        buf.batch_end();
    }

    fn collect_symbols(&mut self, node: &DslNode) {
        match node {
            DslNode::Element { id, classes, styles, text, children, .. } => {
                if let Some(id) = id {
                    self.intern(id);
                }
                for class in classes {
                    self.intern(class);
                }
                if !styles.is_empty() {
                    let css = styles_to_css(styles);
                    self.intern(&css);
                }
                if let Some(text) = text {
                    self.intern(text);
                }
                for child in children {
                    self.collect_symbols(child);
                }
            }
            DslNode::Text(text) => {
                self.intern(text);
            }
        }
    }

    fn emit_node(&mut self, node: &DslNode, buf: &mut OpcodeBuffer, parent_ref: u8) {
        match node {
            DslNode::Element { tag, id, classes, styles, text, children, .. } => {
                let el_type = tag_to_el(tag).unwrap_or(El::Div as u8);
                let my_ref = buf.create(el_type);

                if let Some(id) = id {
                    let sym = self.intern(id);
                    buf.set_attr(my_ref, 0x04, sym); // 0x04 = "id" attribute
                }

                if !classes.is_empty() {
                    let class_str = classes.join(" ");
                    let sym = self.intern(&class_str);
                    buf.set_class(my_ref, sym);
                }

                if !styles.is_empty() {
                    let css = styles_to_css(styles);
                    let sym = self.intern(&css);
                    buf.set_attr(my_ref, 0x06, sym); // 0x06 = "style" attribute
                }

                if let Some(text) = text {
                    let sym = self.intern(text);
                    buf.set_text(my_ref, sym);
                }

                for child in children {
                    self.emit_node(child, buf, my_ref);
                }

                buf.append(parent_ref, my_ref);
            }
            DslNode::Text(text) => {
                let my_ref = buf.create(El::Span as u8);
                let sym = self.intern(text);
                buf.set_text(my_ref, sym);
                buf.append(parent_ref, my_ref);
            }
        }
    }
}
```

#### 2.6 Create `rwire-dsl/src/lib.rs`

```rust
//! DSL parser for rwire binary UI protocol.
//!
//! This crate provides a text-based DSL that compiles to rwire's binary
//! wire format, enabling LLM-generated UIs and rapid prototyping.
//!
//! # Example
//!
//! ```
//! use rwire_dsl::{Parser, EmitContext};
//! use rwire_protocol::OpcodeBuffer;
//!
//! let dsl = r#"
//!     v#root [d:fx fd:col ai:cen] {
//!         h1 [c:fff] `Hello World`
//!         b @click:submit [bg:6c63ff] `Click Me`
//!     }
//! "#;
//!
//! let mut parser = Parser::new(dsl);
//! let ast = parser.parse().unwrap();
//!
//! let mut buf = OpcodeBuffer::new();
//! let mut ctx = EmitContext::new();
//! ctx.emit(&ast, &mut buf);
//!
//! let bytes = buf.take();
//! // bytes can now be sent over WebSocket to rwire client
//! ```

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod emit;

pub use ast::DslNode;
pub use lexer::{Lexer, Token};
pub use parser::Parser;
pub use emit::{EmitContext, styles_to_css};
```

---

## Phase 3: Update Workspace

### Update Root `Cargo.toml`

```toml
[workspace]
members = [
    "rwire",
    "rwire-macros",
    "rwire-protocol",  # NEW
    "rwire-dsl",       # NEW
    "examples/counter",
    "examples/todolist",
    "examples/todo-combined",
    "examples/design-system",
]
```

---

## Verification

### After Phase 1 (Protocol Extraction)

```bash
# All tests pass
cargo test --workspace

# Examples still work
cargo run -p counter
cargo run -p todo-combined

# No warnings
cargo clippy --workspace
```

### After Phase 2 (DSL Crate)

```bash
# DSL tests pass
cargo test -p rwire-dsl

# Can parse and emit sample DSL
cargo run --example dsl-demo -p rwire-dsl
```

### Integration Test

Create `rwire-dsl/examples/dsl-demo.rs`:

```rust
use rwire_dsl::{Parser, EmitContext};
use rwire_protocol::OpcodeBuffer;

fn main() {
    let dsl = r#"
        v#root [d:fx fd:col ai:cen jc:cen h:100vh bg:0f0f23] {
            v#card [w:400 p:40 br:12 bg:1a1a2e] {
                h2 [c:e0e0e0 ta:cen] `Sign In`
                i(type:email placeholder:`Email`) [w:100% p:12 br:8]
                i(type:password placeholder:`Password`) [w:100% p:12 br:8]
                b @click:auth:login [w:100% p:12 br:8 bg:6c63ff c:fff] `Log In`
            }
        }
    "#;

    let mut parser = Parser::new(dsl);
    let ast = parser.parse().expect("Parse failed");

    println!("Parsed {} nodes", ast.len());
    println!("{:#?}", ast);

    let mut buf = OpcodeBuffer::new();
    let mut ctx = EmitContext::new();
    ctx.emit(&ast, &mut buf);

    let bytes = buf.take();
    println!("\nEmitted {} bytes", bytes.len());
    println!("Hex: {:02x?}", &bytes[..bytes.len().min(64)]);
}
```

---

## Future Enhancements

Once the foundation is in place:

1. **Style shorthand in rwire builder**: `el(El::Div).css("[d:fx fd:col]")`
2. **GBNF grammar export**: For LLM constrained decoding
3. **Streaming parser**: Emit binary as DSL tokens arrive
4. **Action registry**: Map `@click:name` to rwire handler indexes
5. **`dsl!` macro**: Embed DSL in Rust code with compile-time parsing
