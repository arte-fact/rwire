# Phase 5: Binary Format + CLI

## Goal

Production-ready tooling with binary compilation, CLI commands, hot reload, and helpful validation.

---

## Deliverables

1. Binary encoder/decoder (.ptok format)
2. CLI with validate, compile, serve, run commands
3. Hot reload in development mode
4. Validation with helpful error messages
5. Watch mode

---

## Prerequisites

- Phase 4 complete (full expression system + styling)

---

## Binary Format (.ptok)

### Format Structure

```
┌────────────────────────────────────────┐
│ Header (10 bytes, uncompressed):       │
│   Magic: "RWPT" (4 bytes)              │
│   Version: u16 (big-endian)            │
│   Uncompressed size: u32 (big-endian)  │
├────────────────────────────────────────┤
│ LZ4-Compressed Payload:                │
│   String Table + Token Data            │
└────────────────────────────────────────┘
```

### String Table

Deduplicate strings for compact encoding.

```rust
// src/binary/string_table.rs

pub struct StringTable {
    strings: Vec<String>,
    indices: HashMap<String, u16>,
}

impl StringTable {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            indices: HashMap::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> u16 {
        if let Some(&idx) = self.indices.get(s) {
            return idx;
        }

        let idx = self.strings.len() as u16;
        self.strings.push(s.to_string());
        self.indices.insert(s.to_string(), idx);
        idx
    }

    pub fn get(&self, idx: u16) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // String count
        buf.extend_from_slice(&(self.strings.len() as u16).to_be_bytes());

        // Each string: length (u16) + bytes
        for s in &self.strings {
            let bytes = s.as_bytes();
            buf.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
            buf.extend_from_slice(bytes);
        }

        buf
    }

    pub fn decode(data: &[u8]) -> Result<(Self, usize), DecodeError> {
        let mut pos = 0;

        let count = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;

        let mut table = Self::new();

        for _ in 0..count {
            let len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;

            let s = std::str::from_utf8(&data[pos..pos + len])?;
            table.strings.push(s.to_string());
            pos += len;
        }

        // Rebuild indices
        for (i, s) in table.strings.iter().enumerate() {
            table.indices.insert(s.clone(), i as u16);
        }

        Ok((table, pos))
    }
}
```

### Discriminants

```rust
// src/binary/discriminants.rs

// Token types
pub const NODE_TOKEN: u8 = 0x01;

// Child types
pub const CHILD_TEXT: u8 = 0x02;
pub const CHILD_DYNAMIC_TEXT: u8 = 0x03;
pub const CHILD_NODE: u8 = 0x04;
pub const CHILD_CONDITIONAL: u8 = 0x05;
pub const CHILD_LOOP: u8 = 0x06;
pub const CHILD_SLOT: u8 = 0x07;
pub const CHILD_REF: u8 = 0x08;

// Expression types
pub const EXPR_LITERAL: u8 = 0x10;
pub const EXPR_STATE: u8 = 0x11;
pub const EXPR_LOCAL: u8 = 0x12;
pub const EXPR_EVENT_VALUE: u8 = 0x13;
pub const EXPR_PARAM: u8 = 0x14;
pub const EXPR_FORMAT: u8 = 0x15;
pub const EXPR_COMPARE: u8 = 0x16;
pub const EXPR_LOGICAL: u8 = 0x17;
pub const EXPR_TERNARY: u8 = 0x18;
pub const EXPR_LENGTH: u8 = 0x19;
pub const EXPR_FIELD: u8 = 0x1A;

// Action types
pub const ACTION_UPDATE_STATE: u8 = 0x20;
pub const ACTION_NAVIGATE: u8 = 0x21;
pub const ACTION_PREVENT_DEFAULT: u8 = 0x22;
pub const ACTION_LOG: u8 = 0x23;

// Mutation ops
pub const MUT_SET: u8 = 0x30;
pub const MUT_INCREMENT: u8 = 0x31;
pub const MUT_DECREMENT: u8 = 0x32;
pub const MUT_TOGGLE: u8 = 0x33;
pub const MUT_PUSH: u8 = 0x34;
pub const MUT_POP: u8 = 0x35;
pub const MUT_CLEAR: u8 = 0x36;
pub const MUT_REMOVE_AT: u8 = 0x37;

// Scopes
pub const SCOPE_GLOBAL: u8 = 0x00;
pub const SCOPE_USER: u8 = 0x01;

// Events
pub const EVENT_CLICK: u8 = 0x00;
pub const EVENT_INPUT: u8 = 0x01;
pub const EVENT_CHANGE: u8 = 0x02;
pub const EVENT_SUBMIT: u8 = 0x03;
pub const EVENT_FOCUS: u8 = 0x04;
pub const EVENT_BLUR: u8 = 0x05;

// Compare ops
pub const CMP_EQ: u8 = 0x00;
pub const CMP_NE: u8 = 0x01;
pub const CMP_LT: u8 = 0x02;
pub const CMP_LE: u8 = 0x03;
pub const CMP_GT: u8 = 0x04;
pub const CMP_GE: u8 = 0x05;

// Logical ops
pub const LOG_AND: u8 = 0x00;
pub const LOG_OR: u8 = 0x01;
pub const LOG_NOT: u8 = 0x02;
```

### Encoder

```rust
// src/binary/encoder.rs

use super::{StringTable, discriminants::*};
use crate::schema::*;
use crate::expr::*;

pub struct Encoder {
    strings: StringTable,
    data: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            strings: StringTable::new(),
            data: Vec::new(),
        }
    }

    pub fn encode_app(app: &AppToken) -> Result<Vec<u8>, EncodeError> {
        let mut encoder = Self::new();

        // Encode version
        encoder.write_string(&app.version);

        // Encode state schema
        encoder.encode_state_schema(&app.state)?;

        // Encode tokens (if present)
        encoder.encode_tokens(&app.tokens)?;

        // Encode UI
        encoder.encode_ui(&app.ui)?;

        // Encode routes
        encoder.write_u16(app.routes.len() as u16);
        for route in &app.routes {
            encoder.write_string(&route.path);
            encoder.write_string(&route.page);
        }

        // Build final output
        let string_table_data = encoder.strings.encode();
        let mut payload = Vec::new();
        payload.extend_from_slice(&string_table_data);
        payload.extend_from_slice(&encoder.data);

        // Compress with LZ4
        let compressed = lz4_flex::compress_prepend_size(&payload);

        // Build header
        let mut output = Vec::new();
        output.extend_from_slice(b"RWPT");  // Magic
        output.extend_from_slice(&1u16.to_be_bytes());  // Version
        output.extend_from_slice(&(payload.len() as u32).to_be_bytes());  // Uncompressed size
        output.extend_from_slice(&compressed);

        Ok(output)
    }

    fn encode_child(&mut self, child: &ChildToken) -> Result<(), EncodeError> {
        match child {
            ChildToken::Text { content } => {
                self.data.push(CHILD_TEXT);
                self.write_string(content);
            }

            ChildToken::DynamicText { expr } => {
                self.data.push(CHILD_DYNAMIC_TEXT);
                self.encode_expr(expr)?;
            }

            ChildToken::Node(node) => {
                self.data.push(CHILD_NODE);
                self.encode_node(node)?;
            }

            ChildToken::Conditional { condition, then, else_ } => {
                self.data.push(CHILD_CONDITIONAL);
                self.encode_expr(condition)?;
                self.encode_child(then)?;
                self.data.push(if else_.is_some() { 1 } else { 0 });
                if let Some(e) = else_ {
                    self.encode_child(e)?;
                }
            }

            ChildToken::Loop { over, as_, index, body } => {
                self.data.push(CHILD_LOOP);
                self.encode_expr(over)?;
                self.write_string(as_);
                self.data.push(if index.is_some() { 1 } else { 0 });
                if let Some(idx) = index {
                    self.write_string(idx);
                }
                self.encode_child(body)?;
            }

            ChildToken::Slot { name, default } => {
                self.data.push(CHILD_SLOT);
                self.write_string(name);
                self.write_u16(default.len() as u16);
                for child in default {
                    self.encode_child(child)?;
                }
            }

            ChildToken::Ref { component_id, slots, variants } => {
                self.data.push(CHILD_REF);
                self.write_string(component_id);

                // Slots
                self.write_u16(slots.len() as u16);
                for (name, children) in slots {
                    self.write_string(name);
                    self.write_u16(children.len() as u16);
                    for child in children {
                        self.encode_child(child)?;
                    }
                }

                // Variants
                self.write_u16(variants.len() as u16);
                for (name, value) in variants {
                    self.write_string(name);
                    self.write_string(value);
                }
            }
        }
        Ok(())
    }

    fn encode_expr(&mut self, expr: &Expr) -> Result<(), EncodeError> {
        match expr {
            Expr::Literal { value } => {
                self.data.push(EXPR_LITERAL);
                self.write_json_value(value);
            }

            Expr::State { scope, state_id, path } => {
                self.data.push(EXPR_STATE);
                self.data.push(scope_to_byte(*scope));
                self.write_string(state_id);
                self.write_string(path);
            }

            Expr::Local { name } => {
                self.data.push(EXPR_LOCAL);
                self.write_string(name);
            }

            Expr::EventValue => {
                self.data.push(EXPR_EVENT_VALUE);
            }

            Expr::Format { template, args } => {
                self.data.push(EXPR_FORMAT);
                self.write_string(template);
                self.write_u16(args.len() as u16);
                for arg in args {
                    self.encode_expr(arg)?;
                }
            }

            Expr::Compare { op, left, right } => {
                self.data.push(EXPR_COMPARE);
                self.data.push(compare_op_to_byte(*op));
                self.encode_expr(left)?;
                self.encode_expr(right)?;
            }

            // ... other expressions
        }
        Ok(())
    }

    fn write_string(&mut self, s: &str) {
        let idx = self.strings.intern(s);
        self.data.extend_from_slice(&idx.to_be_bytes());
    }

    fn write_u16(&mut self, n: u16) {
        self.data.extend_from_slice(&n.to_be_bytes());
    }

    fn write_json_value(&mut self, value: &Value) {
        let json = serde_json::to_string(value).unwrap();
        self.write_string(&json);
    }
}
```

### Decoder

```rust
// src/binary/decoder.rs

pub struct Decoder<'a> {
    data: &'a [u8],
    pos: usize,
    strings: StringTable,
}

impl<'a> Decoder<'a> {
    pub fn decode(data: &'a [u8]) -> Result<AppToken, DecodeError> {
        // Verify header
        if &data[0..4] != b"RWPT" {
            return Err(DecodeError::InvalidMagic);
        }

        let version = u16::from_be_bytes([data[4], data[5]]);
        if version != 1 {
            return Err(DecodeError::UnsupportedVersion(version));
        }

        let uncompressed_size = u32::from_be_bytes([data[6], data[7], data[8], data[9]]) as usize;

        // Decompress
        let compressed = &data[10..];
        let decompressed = lz4_flex::decompress_size_prepended(compressed)?;

        if decompressed.len() != uncompressed_size {
            return Err(DecodeError::SizeMismatch);
        }

        // Decode string table
        let (strings, table_size) = StringTable::decode(&decompressed)?;

        let mut decoder = Self {
            data: &decompressed[table_size..],
            pos: 0,
            strings,
        };

        decoder.decode_app()
    }

    fn decode_app(&mut self) -> Result<AppToken, DecodeError> {
        let version = self.read_string()?;
        let state = self.decode_state_schema()?;
        let tokens = self.decode_tokens()?;
        let ui = self.decode_ui()?;

        let route_count = self.read_u16()? as usize;
        let mut routes = Vec::with_capacity(route_count);
        for _ in 0..route_count {
            routes.push(Route {
                path: self.read_string()?,
                page: self.read_string()?,
            });
        }

        Ok(AppToken {
            version,
            state,
            tokens,
            ui,
            routes,
        })
    }

    fn read_string(&mut self) -> Result<String, DecodeError> {
        let idx = self.read_u16()?;
        self.strings.get(idx)
            .map(|s| s.to_string())
            .ok_or(DecodeError::InvalidStringIndex(idx))
    }

    fn read_u16(&mut self) -> Result<u16, DecodeError> {
        if self.pos + 2 > self.data.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let value = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        if self.pos >= self.data.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }

    // ... decode methods for each type
}
```

---

## CLI Tool

### Main Entry Point

```rust
// rwire-token-cli/src/main.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rwire")]
#[command(about = "rwire token CLI - declarative apps from YAML")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Token-related commands
    Token {
        #[command(subcommand)]
        action: TokenCommands,
    },
}

#[derive(Subcommand)]
enum TokenCommands {
    /// Validate YAML syntax and references
    Validate {
        /// Path to YAML file
        file: PathBuf,
        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Watch file and validate on changes
    Watch {
        /// Path to YAML file
        file: PathBuf,
        /// Clear screen between validations
        #[arg(long, default_value = "true")]
        clear: bool,
    },

    /// Compile YAML to binary
    Compile {
        /// Path to YAML file
        file: PathBuf,
        /// Output path
        #[arg(short, long)]
        output: PathBuf,
        /// Skip validation
        #[arg(long)]
        no_validate: bool,
    },

    /// Development server with hot reload
    Serve {
        /// Path to YAML file
        file: PathBuf,
        /// Port to bind
        #[arg(short, long, default_value = "9000")]
        port: u16,
        /// Open browser automatically
        #[arg(long)]
        open: bool,
        /// Disable hot reload
        #[arg(long)]
        no_reload: bool,
    },

    /// Production server from compiled binary
    Run {
        /// Path to .ptok file
        file: PathBuf,
        /// Address to bind
        #[arg(short, long, default_value = "127.0.0.1")]
        address: String,
        /// Port to bind
        #[arg(short, long, default_value = "9000")]
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Token { action } => match action {
            TokenCommands::Validate { file, strict, json } => {
                commands::validate::run(&file, strict, json);
            }
            TokenCommands::Watch { file, clear } => {
                commands::watch::run(&file, clear);
            }
            TokenCommands::Compile { file, output, no_validate } => {
                commands::compile::run(&file, &output, no_validate);
            }
            TokenCommands::Serve { file, port, open, no_reload } => {
                commands::serve::run(&file, port, open, no_reload);
            }
            TokenCommands::Run { file, address, port } => {
                commands::run::run(&file, &address, port);
            }
        },
    }
}
```

### Validate Command

```rust
// rwire-token-cli/src/commands/validate.rs

use rwire_token::{AppToken, validation::validate};
use std::path::Path;

pub fn run(file: &Path, strict: bool, json: bool) {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    let app: AppToken = match serde_yaml::from_str(&content) {
        Ok(a) => a,
        Err(e) => {
            if json {
                println!(r#"{{"error": "parse_error", "message": "{}"}}"#, e);
            } else {
                eprintln!("Parse error: {}", e);
            }
            std::process::exit(1);
        }
    };

    let result = validate(&app);

    if json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        for error in &result.errors {
            eprintln!("error: {} at {}", error.message, error.path);
            if let Some(hint) = &error.hint {
                eprintln!("  help: {}", hint);
            }
        }

        for warning in &result.warnings {
            eprintln!("warning: {} at {}", warning.message, warning.path);
        }

        if result.errors.is_empty() && (result.warnings.is_empty() || !strict) {
            println!("✓ Valid");
        }
    }

    if !result.errors.is_empty() || (strict && !result.warnings.is_empty()) {
        std::process::exit(1);
    }
}
```

### Watch Command

```rust
// rwire-token-cli/src/commands/watch.rs

use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::Path;

pub fn run(file: &Path, clear: bool) {
    let (tx, rx) = channel();

    let mut watcher = watcher(tx, Duration::from_millis(100)).unwrap();
    watcher.watch(file, RecursiveMode::NonRecursive).unwrap();

    println!("Watching {} for changes...", file.display());

    // Initial validation
    super::validate::run(file, false, false);

    loop {
        match rx.recv() {
            Ok(_) => {
                if clear {
                    print!("\x1B[2J\x1B[1;1H");  // Clear screen
                }
                println!("\n--- File changed ---\n");
                super::validate::run(file, false, false);
            }
            Err(e) => {
                eprintln!("Watch error: {}", e);
                break;
            }
        }
    }
}
```

### Serve Command (Hot Reload)

```rust
// rwire-token-cli/src/commands/serve.rs

use rwire_token::{AppToken, TokenRuntime};
use std::path::Path;
use std::sync::{Arc, RwLock};
use notify::{Watcher, RecursiveMode, watcher};

pub fn run(file: &Path, port: u16, open: bool, no_reload: bool) {
    let content = std::fs::read_to_string(file).expect("Failed to read file");
    let app: AppToken = serde_yaml::from_str(&content).expect("Failed to parse YAML");

    let runtime = Arc::new(RwLock::new(TokenRuntime::new(app)));

    // Start file watcher for hot reload
    if !no_reload {
        let runtime_clone = Arc::clone(&runtime);
        let file_path = file.to_path_buf();

        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = watcher(tx, std::time::Duration::from_millis(100)).unwrap();
            watcher.watch(&file_path, RecursiveMode::NonRecursive).unwrap();

            loop {
                if rx.recv().is_ok() {
                    println!("File changed, reloading...");

                    match std::fs::read_to_string(&file_path) {
                        Ok(content) => {
                            match serde_yaml::from_str::<AppToken>(&content) {
                                Ok(app) => {
                                    let mut rt = runtime_clone.write().unwrap();
                                    *rt = TokenRuntime::new(app);
                                    println!("✓ Reloaded");
                                }
                                Err(e) => eprintln!("Parse error: {}", e),
                            }
                        }
                        Err(e) => eprintln!("Read error: {}", e),
                    }
                }
            }
        });
    }

    let addr = format!("127.0.0.1:{}", port);
    println!("Server running at http://{}", addr);

    if open {
        let _ = open::that(format!("http://{}", addr));
    }

    rwire_token::server::run(runtime, &addr).expect("Server failed");
}
```

### Run Command (Production)

```rust
// rwire-token-cli/src/commands/run.rs

use rwire_token::{AppToken, TokenRuntime, binary::Decoder};
use std::path::Path;
use std::sync::{Arc, RwLock};

pub fn run(file: &Path, address: &str, port: u16) {
    let data = std::fs::read(file).expect("Failed to read file");
    let app = Decoder::decode(&data).expect("Failed to decode binary");

    let runtime = Arc::new(RwLock::new(TokenRuntime::new(app)));

    let addr = format!("{}:{}", address, port);
    println!("Server running at http://{}", addr);

    rwire_token::server::run(runtime, &addr).expect("Server failed");
}
```

---

## Validation

```rust
// src/validation.rs

use crate::schema::*;
use std::collections::{HashSet, HashMap};

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Serialize)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationWarning {
    pub path: String,
    pub message: String,
}

pub fn validate(app: &AppToken) -> ValidationResult {
    let mut ctx = ValidationContext::new(app);
    ctx.validate();
    ctx.result
}

struct ValidationContext<'a> {
    app: &'a AppToken,
    result: ValidationResult,
    defined_states: HashMap<(Scope, String), &'a StateDefinition>,
    defined_components: HashSet<String>,
    defined_pages: HashSet<String>,
    used_components: HashSet<String>,
}

impl<'a> ValidationContext<'a> {
    fn new(app: &'a AppToken) -> Self {
        // Build lookup tables
        let mut defined_states = HashMap::new();
        for state in &app.state.global {
            defined_states.insert((Scope::Global, state.id.clone()), state);
        }
        for state in &app.state.user {
            defined_states.insert((Scope::User, state.id.clone()), state);
        }

        let defined_components: HashSet<_> = app.ui.components.keys().cloned().collect();
        let defined_pages: HashSet<_> = app.ui.pages.keys().cloned().collect();

        Self {
            app,
            result: ValidationResult {
                errors: Vec::new(),
                warnings: Vec::new(),
            },
            defined_states,
            defined_components,
            defined_pages,
            used_components: HashSet::new(),
        }
    }

    fn validate(&mut self) {
        // Validate routes reference existing pages
        for route in &self.app.routes {
            if !self.defined_pages.contains(&route.page) {
                self.error(
                    format!("routes[path={}]", route.path),
                    format!("Page '{}' not found", route.page),
                    self.suggest_page(&route.page),
                );
            }
        }

        // Validate pages
        for (name, page) in &self.app.ui.pages {
            self.validate_node(&page.node, &format!("ui.pages.{}", name));
        }

        // Validate components
        for (name, component) in &self.app.ui.components {
            self.validate_node(component, &format!("ui.components.{}", name));
        }

        // Warn about unused components
        for name in &self.defined_components {
            if !self.used_components.contains(name) {
                self.warning(
                    format!("ui.components.{}", name),
                    format!("Component '{}' is defined but never used", name),
                );
            }
        }
    }

    fn validate_node(&mut self, node: &NodeToken, path: &str) {
        // Validate children
        for (i, child) in node.children.iter().enumerate() {
            self.validate_child(child, &format!("{}.children[{}]", path, i));
        }

        // Validate handlers
        for (i, handler) in node.handlers.iter().enumerate() {
            self.validate_handler(handler, &format!("{}.handlers[{}]", path, i));
        }
    }

    fn validate_child(&mut self, child: &ChildToken, path: &str) {
        match child {
            ChildToken::DynamicText { expr } => {
                self.validate_expr(expr, path);
            }

            ChildToken::Node(node) => {
                self.validate_node(node, path);
            }

            ChildToken::Conditional { condition, then, else_ } => {
                self.validate_expr(condition, &format!("{}.condition", path));
                self.validate_child(then, &format!("{}.then", path));
                if let Some(e) = else_ {
                    self.validate_child(e, &format!("{}.else", path));
                }
            }

            ChildToken::Loop { over, body, .. } => {
                self.validate_expr(over, &format!("{}.over", path));
                self.validate_child(body, &format!("{}.body", path));
            }

            ChildToken::Ref { component_id, slots, .. } => {
                if !self.defined_components.contains(component_id) {
                    self.error(
                        path.to_string(),
                        format!("Component '{}' not found", component_id),
                        self.suggest_component(component_id),
                    );
                } else {
                    self.used_components.insert(component_id.clone());
                }

                for (slot_name, children) in slots {
                    for (i, child) in children.iter().enumerate() {
                        self.validate_child(
                            child,
                            &format!("{}.slots.{}[{}]", path, slot_name, i),
                        );
                    }
                }
            }

            _ => {}
        }
    }

    fn validate_expr(&mut self, expr: &Expr, path: &str) {
        match expr {
            Expr::State { scope, state_id, path: field_path } => {
                let key = (*scope, state_id.clone());
                if let Some(state_def) = self.defined_states.get(&key) {
                    // Check field exists
                    let field_name = field_path.split('.').next().unwrap_or(field_path);
                    if !state_def.fields.iter().any(|f| f.name == field_name) {
                        self.error(
                            path.to_string(),
                            format!("Field '{}' not found in state '{}'", field_name, state_id),
                            None,
                        );
                    }
                } else {
                    self.error(
                        path.to_string(),
                        format!("State '{}' not found", state_id),
                        self.suggest_state(state_id),
                    );
                }
            }

            Expr::Format { args, .. } => {
                for (i, arg) in args.iter().enumerate() {
                    self.validate_expr(arg, &format!("{}.args[{}]", path, i));
                }
            }

            Expr::Compare { left, right, .. } => {
                self.validate_expr(left, &format!("{}.left", path));
                self.validate_expr(right, &format!("{}.right", path));
            }

            _ => {}
        }
    }

    fn error(&mut self, path: String, message: String, hint: Option<String>) {
        self.result.errors.push(ValidationError { path, message, hint });
    }

    fn warning(&mut self, path: String, message: String) {
        self.result.warnings.push(ValidationWarning { path, message });
    }

    fn suggest_state(&self, name: &str) -> Option<String> {
        self.find_similar(name, self.defined_states.keys().map(|(_, n)| n.as_str()))
    }

    fn suggest_component(&self, name: &str) -> Option<String> {
        self.find_similar(name, self.defined_components.iter().map(|s| s.as_str()))
    }

    fn suggest_page(&self, name: &str) -> Option<String> {
        self.find_similar(name, self.defined_pages.iter().map(|s| s.as_str()))
    }

    fn find_similar<'b>(&self, target: &str, candidates: impl Iterator<Item = &'b str>) -> Option<String> {
        candidates
            .filter(|c| levenshtein_distance(target, c) <= 2)
            .min_by_key(|c| levenshtein_distance(target, c))
            .map(|s| format!("Did you mean '{}'?", s))
    }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    // Simple implementation
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();

    let mut dp = vec![vec![0; b.len() + 1]; a.len() + 1];

    for i in 0..=a.len() {
        dp[i][0] = i;
    }
    for j in 0..=b.len() {
        dp[0][j] = j;
    }

    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[a.len()][b.len()]
}
```

---

## Success Criteria

- [ ] Binary encoder produces valid .ptok files
- [ ] Binary decoder reconstructs AppToken
- [ ] LZ4 compression reduces file size
- [ ] `rwire token validate` reports errors and warnings
- [ ] `rwire token watch` re-validates on changes
- [ ] `rwire token compile` produces .ptok
- [ ] `rwire token serve` starts dev server with hot reload
- [ ] `rwire token run` loads from binary
- [ ] Validation catches undefined states
- [ ] Validation catches undefined components
- [ ] Validation suggests similar names
- [ ] Validation warns about unused components

---

## Next Phase

Phase 6 adds:
- URL routing with parameters
- State persistence
- Rust handler escape hatch
- Task system
