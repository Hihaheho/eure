use eure_value::value::{
    Array, Code, KeyCmpValue, Map, Path, PathSegment, Tuple, Value, Variant,
};

/// Format a Value as EURE syntax
pub fn format_eure(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::I64(i) => i.to_string(),
        Value::U64(u) => u.to_string(),
        Value::F32(f) => {
            let s = f.to_string();
            if s.contains('.') || s.contains('e') || s.contains('E') {
                s
            } else {
                format!("{s}.0")
            }
        }
        Value::F64(f) => f.to_string(),
        Value::String(s) => format_string(s),
        Value::Code(Code { language, content }) => {
            format!("{language}`{content}`")
        }
        Value::CodeBlock(Code { language, content }) => {
            if language.is_empty() {
                format!("`{content}`")
            } else {
                format!("```{language}\n{content}\n```")
            }
        }
        Value::Array(Array(items)) => {
            let inner = items.iter().map(format_eure).collect::<Vec<_>>().join(", ");
            format!("[{inner}]")
        }
        Value::Tuple(Tuple(items)) => {
            let inner = items.iter().map(format_eure).collect::<Vec<_>>().join(", ");
            format!("({inner})")
        }
        Value::Map(Map(map)) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let entries = map
                .iter()
                .map(|(k, v)| {
                    let key_str = format_key(k);
                    format!("{}: {}", key_str, format_eure(v))
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{entries}}}")
        }
        Value::Variant(Variant { tag, content }) => {
            format!("$variant: {}\n{}", format_string(tag), format_eure(content))
        }
        Value::Unit => "()".to_string(),
        Value::Path(Path(segments)) => {
            // Format path as dot-separated string
            let path_str = segments
                .iter()
                .map(|seg| match seg {
                    PathSegment::Ident(id) => id.as_ref().to_string(),
                    PathSegment::Extension(id) => format!("${}", id.as_ref()),
                    PathSegment::MetaExt(id) => format!("$̄{}", id.as_ref()),
                    PathSegment::Value(v) => format!("[{}]", format_key(v)),
                    PathSegment::TupleIndex(idx) => idx.to_string(),
                    PathSegment::Array { key, index } => {
                        if let Some(idx) = index {
                            format!("{}[{}]", format_eure(key), format_eure(idx))
                        } else {
                            format!("{}[]", format_eure(key))
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(".");
            format!(".{path_str}")
        }
        Value::Hole => "!".to_string(),
    }
}

/// Format a Value as EURE bindings (for root-level objects)
pub fn format_eure_bindings(value: &Value) -> String {
    match value {
        Value::Map(Map(map)) => map
            .iter()
            .map(|(k, v)| {
                let key_str = format_key(k);
                match v {
                    Value::Map(_) | Value::Array(_) => {
                        let value_str = format_eure_value_multiline(v, 0);
                        format!("{key_str} = {value_str}")
                    }
                    _ => format!("{} = {}", key_str, format_eure(v)),
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => format_eure(value),
    }
}

fn format_string(s: &str) -> String {
    // Check if string needs escaping
    let needs_escape = s
        .chars()
        .any(|c| matches!(c, '"' | '\\' | '\n' | '\r' | '\t'));

    if needs_escape {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!("\"{escaped}\"")
    } else {
        format!("\"{s}\"")
    }
}

fn format_key(key: &KeyCmpValue) -> String {
    match key {
        KeyCmpValue::String(s) => {
            if is_valid_identifier(s) {
                s.clone()
            } else {
                format_string(s)
            }
        }
        KeyCmpValue::I64(i) => i.to_string(),
        KeyCmpValue::U64(u) => u.to_string(),
        KeyCmpValue::Bool(b) => b.to_string(),
        KeyCmpValue::Null => "null".to_string(),
        KeyCmpValue::Tuple(Tuple(items)) => {
            let inner = items.iter().map(format_key).collect::<Vec<_>>().join(", ");
            format!("({inner})")
        }
        KeyCmpValue::Unit => "()".to_string(),
        KeyCmpValue::MetaExtension(meta) => format!("$${meta}"),
        KeyCmpValue::Hole => "!".to_string(),
    }
}

fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    if let Some(first) = chars.next()
        && !first.is_alphabetic()
        && first != '_'
        && first != '$'
    {
        return false;
    }

    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn format_eure_value_multiline(value: &Value, indent: usize) -> String {
    let indent_str = "  ".repeat(indent);
    let next_indent = "  ".repeat(indent + 1);

    match value {
        Value::Map(Map(map)) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let entries = map
                .iter()
                .map(|(k, v)| {
                    let key_str = format_key(k);
                    let value_str = match v {
                        Value::Map(_) | Value::Array(_) => {
                            format_eure_value_multiline(v, indent + 1)
                        }
                        _ => format_eure(v),
                    };
                    format!("{next_indent}{key_str}: {value_str}")
                })
                .collect::<Vec<_>>()
                .join(",\n");
            format!("{{\n{entries}\n{indent_str}}}")
        }
        Value::Array(Array(items)) => {
            if items.is_empty() {
                return "[]".to_string();
            }
            let entries = items
                .iter()
                .map(|v| {
                    let value_str = match v {
                        Value::Map(_) | Value::Array(_) => {
                            format_eure_value_multiline(v, indent + 1)
                        }
                        _ => format_eure(v),
                    };
                    format!("{next_indent}{value_str}")
                })
                .collect::<Vec<_>>()
                .join(",\n");
            format!("[\n{entries}\n{indent_str}]")
        }
        _ => format_eure(value),
    }
}
