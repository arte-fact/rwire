//! SQL schema generation from Rust struct definitions.
//!
//! This module generates normalized database schemas at compile time from
//! `#[storage(persisted)]` structs. It maps Rust types to SQL types and
//! creates child tables for Vec fields.

use syn::{Fields, GenericArgument, PathArguments, Type};

/// SQL column definition.
#[derive(Debug, Clone)]
pub struct SqlColumn {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub is_primary_key: bool,
}

/// SQL table definition.
#[derive(Debug, Clone)]
pub struct SqlTable {
    pub name: String,
    pub columns: Vec<SqlColumn>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
}

/// Foreign key constraint.
#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub column: String,
    pub references_table: String,
    pub references_column: String,
}

/// Result of type mapping.
#[derive(Clone)]
pub enum SqlTypeMapping {
    /// Direct column mapping (INTEGER, TEXT, REAL)
    Column(String),
    /// Nullable column (Option<T>)
    Nullable(Box<SqlTypeMapping>),
    /// Child table needed (Vec<T>)
    ChildTable { inner_type: Box<Type> },
    /// Unknown type - treated as INTEGER (for enums)
    Unknown,
}

/// Map Rust type to SQL type.
pub fn rust_type_to_sql(ty: &Type) -> SqlTypeMapping {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let ident = &segment.ident;
            let type_name = ident.to_string();

            match type_name.as_str() {
                // Integers
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" => {
                    SqlTypeMapping::Column("INTEGER".to_string())
                }
                // Floats
                "f32" | "f64" => SqlTypeMapping::Column("REAL".to_string()),
                // Boolean
                "bool" => SqlTypeMapping::Column("INTEGER".to_string()),
                // String
                "String" => SqlTypeMapping::Column("TEXT".to_string()),
                // Option<T>
                "Option" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner)) = args.args.first() {
                            let inner_mapping = rust_type_to_sql(inner);
                            return SqlTypeMapping::Nullable(Box::new(inner_mapping));
                        }
                    }
                    SqlTypeMapping::Unknown
                }
                // Vec<T>
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner)) = args.args.first() {
                            return SqlTypeMapping::ChildTable {
                                inner_type: Box::new(inner.clone()),
                            };
                        }
                    }
                    SqlTypeMapping::Unknown
                }
                // Unknown - assume it's an enum or struct
                _ => SqlTypeMapping::Unknown,
            }
        }
        _ => SqlTypeMapping::Unknown,
    }
}

/// Get SQL default value for a type.
pub fn default_for_sql_type(sql_type: &str) -> String {
    match sql_type {
        "INTEGER" => "0".to_string(),
        "REAL" => "0.0".to_string(),
        "TEXT" => "''".to_string(),
        _ => "NULL".to_string(),
    }
}

/// Generate schema for a persisted struct.
pub fn generate_schema(table_name: &str, key_field: &str, fields: &Fields) -> Vec<SqlTable> {
    let mut tables = Vec::new();
    let mut main_columns = Vec::new();
    let mut child_tables = Vec::new();

    // Process fields
    if let Fields::Named(named) = fields {
        for field in &named.named {
            let field_name = field.ident.as_ref().unwrap().to_string();
            let mapping = rust_type_to_sql(&field.ty);

            match mapping {
                SqlTypeMapping::Column(sql_type) => {
                    let is_key = field_name == key_field;
                    main_columns.push(SqlColumn {
                        name: field_name,
                        sql_type: sql_type.clone(),
                        nullable: false,
                        default: if is_key {
                            None
                        } else {
                            Some(default_for_sql_type(&sql_type))
                        },
                        is_primary_key: is_key,
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
                SqlTypeMapping::ChildTable { inner_type } => {
                    // Generate child table
                    let child_table_name = format!("{}__{}", table_name, field_name);
                    let child =
                        generate_child_table(&child_table_name, table_name, key_field, &inner_type);
                    child_tables.push(child);
                }
                SqlTypeMapping::Unknown => {
                    // Treat as INTEGER (enum)
                    main_columns.push(SqlColumn {
                        name: field_name,
                        sql_type: "INTEGER".to_string(),
                        nullable: false,
                        default: Some("0".to_string()),
                        is_primary_key: false,
                    });
                }
            }
        }
    }

    // Ensure key field exists
    if !main_columns.iter().any(|c| c.is_primary_key) && !key_field.is_empty() {
        main_columns.insert(
            0,
            SqlColumn {
                name: key_field.to_string(),
                sql_type: "TEXT".to_string(),
                nullable: false,
                default: None,
                is_primary_key: true,
            },
        );
    }

    // Build main table
    let primary_key: Vec<String> = main_columns
        .iter()
        .filter(|c| c.is_primary_key)
        .map(|c| c.name.clone())
        .collect();

    tables.push(SqlTable {
        name: table_name.to_string(),
        columns: main_columns,
        primary_key,
        foreign_keys: vec![],
    });

    // Add child tables
    tables.extend(child_tables);

    tables
}

/// Generate a child table for Vec<T> field.
fn generate_child_table(
    table_name: &str,
    parent_table: &str,
    parent_key: &str,
    inner_type: &Type,
) -> SqlTable {
    let mut columns = vec![
        SqlColumn {
            name: "_parent".to_string(),
            sql_type: "TEXT".to_string(),
            nullable: false,
            default: None,
            is_primary_key: false,
        },
        SqlColumn {
            name: "_idx".to_string(),
            sql_type: "INTEGER".to_string(),
            nullable: false,
            default: None,
            is_primary_key: false,
        },
    ];

    // Try to extract fields from inner type if it's a struct
    // For now, we'll handle simple cases and treat complex types as JSON
    match rust_type_to_sql(inner_type) {
        SqlTypeMapping::Column(sql_type) => {
            // Simple type like Vec<i32>
            columns.push(SqlColumn {
                name: "value".to_string(),
                sql_type,
                nullable: false,
                default: None,
                is_primary_key: false,
            });
        }
        SqlTypeMapping::Unknown => {
            // Likely a struct - for now, store as JSON
            columns.push(SqlColumn {
                name: "data".to_string(),
                sql_type: "TEXT".to_string(), // JSON
                nullable: false,
                default: None,
                is_primary_key: false,
            });
        }
        _ => {
            // Complex nested type - store as JSON
            columns.push(SqlColumn {
                name: "data".to_string(),
                sql_type: "TEXT".to_string(),
                nullable: false,
                default: None,
                is_primary_key: false,
            });
        }
    }

    SqlTable {
        name: table_name.to_string(),
        columns,
        primary_key: vec!["_parent".to_string(), "_idx".to_string()],
        foreign_keys: vec![ForeignKey {
            column: "_parent".to_string(),
            references_table: parent_table.to_string(),
            references_column: parent_key.to_string(),
        }],
    }
}

/// Convert SqlTable to CREATE TABLE SQL statement.
pub fn table_to_sql(table: &SqlTable) -> String {
    let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table.name);
    let mut parts = Vec::new();

    // Columns
    for col in &table.columns {
        let mut def = format!("    {} {}", col.name, col.sql_type);

        if !col.nullable {
            def.push_str(" NOT NULL");
        }

        if let Some(default) = &col.default {
            def.push_str(&format!(" DEFAULT {}", default));
        }

        parts.push(def);
    }

    // Primary key
    if !table.primary_key.is_empty() {
        parts.push(format!(
            "    PRIMARY KEY ({})",
            table.primary_key.join(", ")
        ));
    }

    // Foreign keys
    for fk in &table.foreign_keys {
        parts.push(format!(
            "    FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE CASCADE",
            fk.column, fk.references_table, fk.references_column
        ));
    }

    sql.push_str(&parts.join(",\n"));
    sql.push_str("\n)");

    sql
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_type_mapping_integers() {
        let ty: Type = parse_quote!(i32);
        assert!(matches!(
            rust_type_to_sql(&ty),
            SqlTypeMapping::Column(s) if s == "INTEGER"
        ));

        let ty: Type = parse_quote!(u64);
        assert!(matches!(
            rust_type_to_sql(&ty),
            SqlTypeMapping::Column(s) if s == "INTEGER"
        ));
    }

    #[test]
    fn test_type_mapping_string() {
        let ty: Type = parse_quote!(String);
        assert!(matches!(
            rust_type_to_sql(&ty),
            SqlTypeMapping::Column(s) if s == "TEXT"
        ));
    }

    #[test]
    fn test_type_mapping_bool() {
        let ty: Type = parse_quote!(bool);
        assert!(matches!(
            rust_type_to_sql(&ty),
            SqlTypeMapping::Column(s) if s == "INTEGER"
        ));
    }

    #[test]
    fn test_type_mapping_float() {
        let ty: Type = parse_quote!(f64);
        assert!(matches!(
            rust_type_to_sql(&ty),
            SqlTypeMapping::Column(s) if s == "REAL"
        ));
    }

    #[test]
    fn test_type_mapping_option() {
        let ty: Type = parse_quote!(Option<i32>);
        match rust_type_to_sql(&ty) {
            SqlTypeMapping::Nullable(inner) => {
                assert!(matches!(*inner, SqlTypeMapping::Column(s) if s == "INTEGER"));
            }
            _ => panic!("Expected Nullable"),
        }
    }

    #[test]
    fn test_type_mapping_vec() {
        let ty: Type = parse_quote!(Vec<String>);
        assert!(matches!(
            rust_type_to_sql(&ty),
            SqlTypeMapping::ChildTable { .. }
        ));
    }

    #[test]
    fn test_type_mapping_unknown() {
        let ty: Type = parse_quote!(MyEnum);
        assert!(matches!(rust_type_to_sql(&ty), SqlTypeMapping::Unknown));
    }

    #[test]
    fn test_table_to_sql_simple() {
        let table = SqlTable {
            name: "todos".to_string(),
            columns: vec![
                SqlColumn {
                    name: "session_id".to_string(),
                    sql_type: "TEXT".to_string(),
                    nullable: false,
                    default: None,
                    is_primary_key: true,
                },
                SqlColumn {
                    name: "count".to_string(),
                    sql_type: "INTEGER".to_string(),
                    nullable: false,
                    default: Some("0".to_string()),
                    is_primary_key: false,
                },
            ],
            primary_key: vec!["session_id".to_string()],
            foreign_keys: vec![],
        };

        let sql = table_to_sql(&table);
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS todos"));
        assert!(sql.contains("session_id TEXT NOT NULL"));
        assert!(sql.contains("count INTEGER NOT NULL DEFAULT 0"));
        assert!(sql.contains("PRIMARY KEY (session_id)"));
    }

    #[test]
    fn test_table_to_sql_with_fk() {
        let table = SqlTable {
            name: "todos__items".to_string(),
            columns: vec![
                SqlColumn {
                    name: "_parent".to_string(),
                    sql_type: "TEXT".to_string(),
                    nullable: false,
                    default: None,
                    is_primary_key: false,
                },
                SqlColumn {
                    name: "_idx".to_string(),
                    sql_type: "INTEGER".to_string(),
                    nullable: false,
                    default: None,
                    is_primary_key: false,
                },
            ],
            primary_key: vec!["_parent".to_string(), "_idx".to_string()],
            foreign_keys: vec![ForeignKey {
                column: "_parent".to_string(),
                references_table: "todos".to_string(),
                references_column: "session_id".to_string(),
            }],
        };

        let sql = table_to_sql(&table);
        assert!(sql.contains("FOREIGN KEY (_parent) REFERENCES todos(session_id)"));
        assert!(sql.contains("PRIMARY KEY (_parent, _idx)"));
    }
}
