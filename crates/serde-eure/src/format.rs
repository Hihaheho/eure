use eure_value::value::{Array, Code, KeyCmpValue, Map, Tuple, TypedString, Value, Variant};
use std::fmt::Write;

/// Format a Value as EURE syntax
pub fn format_eure(value: &Value) -> String {
    let mut output = String::new();
    format_value(&mut output, value, 0);
    output
}

fn format_value(output: &mut String, value: &Value, _indent: usize) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        Value::I64(i) => write!(output, "{i}").unwrap(),
        Value::U64(u) => write!(output, "{u}").unwrap(),
        Value::F32(f) => write!(output, "{f}").unwrap(),
        Value::F64(f) => write!(output, "{f}").unwrap(),
        Value::String(s) => write!(output, "\"{}\"", escape_string(s)).unwrap(),
        Value::TypedString(TypedString { type_name, value }) => {
            write!(output, "{}\"{}\"", type_name, escape_string(value)).unwrap()
        }
        Value::Code(Code { language, content }) => {
            write!(output, "```{language}\n{content}\n```").unwrap()
        }
        Value::Array(Array(values)) => {
            output.push('[');
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                format_value(output, v, 0);
            }
            if !values.is_empty() {
                output.push(',');
            }
            output.push(']');
        }
        Value::Tuple(Tuple(values)) => {
            output.push('(');
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                format_value(output, v, 0);
            }
            if !values.is_empty() {
                output.push(',');
            }
            output.push(')');
        }
        Value::Map(Map(map)) => {
            output.push('{');
            let mut first = true;
            for (k, v) in map.iter() {
                if !first {
                    output.push_str(", ");
                }
                first = false;
                format_key(output, k);
                output.push_str(" = ");
                format_value(output, v, 0);
            }
            if !map.is_empty() {
                output.push(',');
            }
            output.push('}');
        }
        Value::Variant(Variant { tag, content }) => {
            output.push_str("$variant: ");
            output.push_str(tag);
            output.push('\n');
            format_value(output, content, 0);
        }
        Value::Unit => output.push_str("()"),
        Value::Path(path) => {
            // Format path with dot notation
            output.push('.');
            let segments: Vec<String> = path.0.iter()
                .map(|seg| match seg {
                    eure_value::value::PathSegment::Ident(id) => id.as_ref().to_string(),
                    eure_value::value::PathSegment::Extension(id) => format!("${}", id.as_ref()),
                    eure_value::value::PathSegment::MetaExt(id) => format!("$̄{}", id.as_ref()),
                    eure_value::value::PathSegment::Value(v) => format!("{:?}", v),
                    eure_value::value::PathSegment::Array { key, index } => {
                        if let Some(idx) = index {
                            format!("{:?}[{:?}]", key, idx)
                        } else {
                            format!("{:?}[]", key)
                        }
                    }
                })
                .collect();
            output.push_str(&segments.join("."));
        }
    }
}

fn format_key(output: &mut String, key: &KeyCmpValue) {
    match key {
        KeyCmpValue::String(s) => {
            if is_valid_identifier(s) {
                output.push_str(s);
            } else {
                write!(output, "\"{}\"", escape_string(s)).unwrap();
            }
        }
        KeyCmpValue::I64(i) => write!(output, "{i}").unwrap(),
        KeyCmpValue::U64(u) => write!(output, "{u}").unwrap(),
        // Handle other key types that might not be valid in EURE
        _ => write!(output, "\"<unsupported-key>\"").unwrap(),
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
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '$')
}

fn escape_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// Format a Value as EURE bindings (for root-level)
///
/// EURE format requires all values to be in bindings at the top level.
/// Non-map values are wrapped in a synthetic "value" binding.
pub fn format_eure_bindings(value: &Value) -> String {
    match value {
        Value::Map(Map(map)) => {
            // Check if this looks like an internally tagged enum
            // (has a single $tag field, or $tag plus other fields)
            let has_tag = map.contains_key(&KeyCmpValue::String("$tag".to_string()));

            if has_tag {
                // This is likely an internally tagged enum, format as object
                format!("value = {}\n", format_eure(value))
            } else {
                // Regular map, format as root-level bindings
                let mut output = String::new();
                for (k, v) in map.iter() {
                    format_key(&mut output, k);
                    output.push_str(" = ");
                    format_value(&mut output, v, 0);
                    output.push('\n');
                }
                output
            }
        }
        _ => {
            // Wrap non-map values in a synthetic binding for valid EURE
            format!("value = {}\n", format_eure(value))
        }
    }
}
