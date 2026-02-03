# Phase 1: Normalized Schema Generation

**Goal**: Generate SQL schema at compile time from Rust struct definitions.

## Overview

Instead of storing state as JSON blobs, generate normalized tables:

```rust
#[derive(State)]
#[storage(persisted, table = "todos")]
struct TodoState {
    #[key]
    session_id: String,
    filter: Filter,
    items: Vec<TodoItem>,
}

struct TodoItem {
    text: String,
    done: bool,
}

enum Filter { All, Active, Completed }
```

**Generated SQL:**

```sql
CREATE TABLE todos (
    session_id TEXT PRIMARY KEY,
    filter INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE todos__items (
    _parent TEXT NOT NULL,
    _idx INTEGER NOT NULL,
    text TEXT NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (_parent, _idx),
    FOREIGN KEY (_parent) REFERENCES todos(session_id) ON DELETE CASCADE
);
```

## Type Mapping

| Rust Type | SQL Type | Notes |
|-----------|----------|-------|
| `i8, i16, i32, i64` | `INTEGER` | |
| `u8, u16, u32` | `INTEGER` | |
| `u64` | `INTEGER` | SQLite stores as signed i64 |
| `f32, f64` | `REAL` | |
| `bool` | `INTEGER` | 0 = false, 1 = true |
| `String` | `TEXT` | |
| `Option<T>` | `T` nullable | `NULL` when None |
| `Vec<T>` | Child table | `{table}__{field}` with FK |
| Unit enum | `INTEGER` | Variant index (0, 1, 2...) |
| Nested struct | Flattened | `parent_child` column names |

## New File: `rwire-macros/src/schema_gen.rs`

```rust
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Type, Ident};

/// SQL column definition
pub struct SqlColumn {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub is_primary_key: bool,
}

/// SQL table definition
pub struct SqlTable {
    pub name: String,
    pub columns: Vec<SqlColumn>,
    pub foreign_keys: Vec<ForeignKey>,
}

pub struct ForeignKey {
    pub column: String,
    pub references_table: String,
    pub references_column: String,
}

/// Map Rust type to SQL type
pub fn rust_type_to_sql(ty: &Type) -> SqlTypeMapping {
    match ty {
        // Primitives
        Type::Path(p) => {
            let ident = &p.path.segments.last().unwrap().ident;
            match ident.to_string().as_str() {
                "i8" | "i16" | "i32" | "i64" |
                "u8" | "u16" | "u32" | "u64" => SqlTypeMapping::Column("INTEGER".into()),
                "f32" | "f64" => SqlTypeMapping::Column("REAL".into()),
                "bool" => SqlTypeMapping::Column("INTEGER".into()),
                "String" => SqlTypeMapping::Column("TEXT".into()),
                "Option" => {
                    // Extract inner type, mark as nullable
                    let inner = extract_generic_arg(p);
                    SqlTypeMapping::Nullable(Box::new(rust_type_to_sql(&inner)))
                }
                "Vec" => {
                    // Child table needed
                    let inner = extract_generic_arg(p);
                    SqlTypeMapping::ChildTable(Box::new(rust_type_to_sql(&inner)))
                }
                _ => {
                    // Assume it's an enum or struct - need more context
                    SqlTypeMapping::Unknown(ident.to_string())
                }
            }
        }
        _ => SqlTypeMapping::Unknown("complex".into()),
    }
}

pub enum SqlTypeMapping {
    Column(String),           // Direct column: INTEGER, TEXT, REAL
    Nullable(Box<SqlTypeMapping>),  // Optional<T>
    ChildTable(Box<SqlTypeMapping>), // Vec<T>
    Unknown(String),          // Needs struct/enum analysis
}

/// Generate CREATE TABLE statements
pub fn generate_schema(
    table_name: &str,
    key_field: &str,
    fields: &Fields,
) -> Vec<SqlTable> {
    let mut tables = Vec::new();
    let mut main_columns = Vec::new();

    // Add primary key
    main_columns.push(SqlColumn {
        name: key_field.to_string(),
        sql_type: "TEXT".into(),
        nullable: false,
        default: None,
        is_primary_key: true,
    });

    if let Fields::Named(named) = fields {
        for field in &named.named {
            let field_name = field.ident.as_ref().unwrap().to_string();

            // Skip key field (already added)
            if field_name == key_field {
                continue;
            }

            match rust_type_to_sql(&field.ty) {
                SqlTypeMapping::Column(sql_type) => {
                    main_columns.push(SqlColumn {
                        name: field_name,
                        sql_type,
                        nullable: false,
                        default: Some(default_for_type(&field.ty)),
                        is_primary_key: false,
                    });
                }
                SqlTypeMapping::Nullable(inner) => {
                    if let SqlTypeMapping::Column(sql_type) = *inner {
                        main_columns.push(SqlColumn {
                            name: field_name,
                            sql_type,
                            nullable: true,
                            default: None,
                            is_primary_key: false,
                        });
                    }
                }
                SqlTypeMapping::ChildTable(_inner) => {
                    // Create child table
                    let child_table = generate_child_table(
                        table_name,
                        &field_name,
                        key_field,
                        &field.ty,
                    );
                    tables.push(child_table);
                }
                SqlTypeMapping::Unknown(_) => {
                    // Flatten struct or treat enum as INTEGER
                    main_columns.push(SqlColumn {
                        name: field_name,
                        sql_type: "INTEGER".into(), // Assume enum
                        nullable: false,
                        default: Some("0".into()),
                        is_primary_key: false,
                    });
                }
            }
        }
    }

    // Main table first
    tables.insert(0, SqlTable {
        name: table_name.to_string(),
        columns: main_columns,
        foreign_keys: vec![],
    });

    tables
}

fn generate_child_table(
    parent_table: &str,
    field_name: &str,
    parent_key: &str,
    vec_type: &Type,
) -> SqlTable {
    let table_name = format!("{}__{}", parent_table, field_name);
    let mut columns = vec![
        SqlColumn {
            name: "_parent".into(),
            sql_type: "TEXT".into(),
            nullable: false,
            default: None,
            is_primary_key: false,
        },
        SqlColumn {
            name: "_idx".into(),
            sql_type: "INTEGER".into(),
            nullable: false,
            default: None,
            is_primary_key: false,
        },
    ];

    // Extract Vec<T> inner type and add its fields
    // For now, assume simple struct with named fields
    // TODO: Handle nested types recursively

    SqlTable {
        name: table_name,
        columns,
        foreign_keys: vec![ForeignKey {
            column: "_parent".into(),
            references_table: parent_table.into(),
            references_column: parent_key.into(),
        }],
    }
}

fn default_for_type(ty: &Type) -> String {
    match ty {
        Type::Path(p) => {
            let ident = &p.path.segments.last().unwrap().ident;
            match ident.to_string().as_str() {
                "bool" => "0".into(),
                "i8" | "i16" | "i32" | "i64" |
                "u8" | "u16" | "u32" | "u64" => "0".into(),
                "f32" | "f64" => "0.0".into(),
                "String" => "''".into(),
                _ => "0".into(),
            }
        }
        _ => "NULL".into(),
    }
}

/// Generate SQL CREATE TABLE statement
pub fn table_to_sql(table: &SqlTable) -> String {
    let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table.name);

    let mut column_defs = Vec::new();
    let mut primary_keys = Vec::new();

    for col in &table.columns {
        let mut def = format!("    {} {}", col.name, col.sql_type);

        if !col.nullable {
            def.push_str(" NOT NULL");
        }

        if let Some(default) = &col.default {
            def.push_str(&format!(" DEFAULT {}", default));
        }

        if col.is_primary_key {
            primary_keys.push(col.name.clone());
        }

        column_defs.push(def);
    }

    // Primary key constraint
    if !primary_keys.is_empty() {
        column_defs.push(format!("    PRIMARY KEY ({})", primary_keys.join(", ")));
    }

    // Foreign keys
    for fk in &table.foreign_keys {
        column_defs.push(format!(
            "    FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE CASCADE",
            fk.column, fk.references_table, fk.references_column
        ));
    }

    sql.push_str(&column_defs.join(",\n"));
    sql.push_str("\n)");

    sql
}
```

## Update: `rwire-macros/src/lib.rs`

```rust
mod schema_gen;

use schema_gen::{generate_schema, table_to_sql};

#[proc_macro_derive(State, attributes(storage, key))]
pub fn derive_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // ... existing parsing ...

    let (storage_type, table_name, key_field) = parse_storage_attr(&input.attrs);

    // Generate schema for persisted types
    let schema_impl = if storage_type == StorageType::Persisted {
        let tables = generate_schema(&table_name, &key_field, &input.data.fields());
        let schema_sql: Vec<String> = tables.iter().map(table_to_sql).collect();

        quote! {
            impl #name {
                /// SQL statements to create tables for this state.
                pub const SCHEMA: &'static [&'static str] = &[
                    #(#schema_sql),*
                ];

                /// Table name for this state.
                pub const TABLE_NAME: &'static str = #table_name;

                /// Primary key field name.
                pub const KEY_FIELD: &'static str = #key_field;
            }
        }
    } else {
        quote! {}
    };

    // ... rest of macro ...
}
```

## Generated Code Example

For the TodoState example:

```rust
impl TodoState {
    pub const SCHEMA: &'static [&'static str] = &[
        "CREATE TABLE IF NOT EXISTS todos (
            session_id TEXT NOT NULL,
            filter INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (session_id)
        )",
        "CREATE TABLE IF NOT EXISTS todos__items (
            _parent TEXT NOT NULL,
            _idx INTEGER NOT NULL,
            text TEXT NOT NULL DEFAULT '',
            done INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (_parent, _idx),
            FOREIGN KEY (_parent) REFERENCES todos(session_id) ON DELETE CASCADE
        )",
    ];

    pub const TABLE_NAME: &'static str = "todos";
    pub const KEY_FIELD: &'static str = "session_id";
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_type_mapping() {
        assert_eq!(rust_type_to_sql_str("i32"), "INTEGER");
        assert_eq!(rust_type_to_sql_str("String"), "TEXT");
        assert_eq!(rust_type_to_sql_str("bool"), "INTEGER");
        assert_eq!(rust_type_to_sql_str("f64"), "REAL");
    }

    #[test]
    fn test_table_generation() {
        // Test with actual struct parsing
        let sql = TodoState::SCHEMA[0];
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("session_id TEXT"));
        assert!(sql.contains("filter INTEGER"));
    }

    #[test]
    fn test_child_table_generation() {
        let sql = TodoState::SCHEMA[1];
        assert!(sql.contains("todos__items"));
        assert!(sql.contains("_parent TEXT"));
        assert!(sql.contains("_idx INTEGER"));
        assert!(sql.contains("FOREIGN KEY"));
    }
}
```

## Checklist

- [ ] Create `rwire-macros/src/schema_gen.rs`
- [ ] Implement `rust_type_to_sql()` for primitives
- [ ] Implement `generate_schema()` for structs
- [ ] Implement `generate_child_table()` for Vec fields
- [ ] Generate `SCHEMA` constant in `#[derive(State)]`
- [ ] Handle `Option<T>` as nullable columns
- [ ] Handle unit enums as INTEGER
- [ ] Add unit tests for type mapping
- [ ] Add integration tests with actual structs
