use eure_value::identifier::Identifier;
use eure_value::value::{Array, Code, KeyCmpValue, Map, Tuple, Value, Variant};
use std::fmt::Write;
use std::str::FromStr;

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
        Value::F32(f) => {
            if f.is_nan() {
                output.push_str("NaN");
            } else if f.is_infinite() {
                if f.is_sign_positive() {
                    output.push_str("inf");
                } else {
                    output.push_str("-inf");
                }
            } else {
                write!(output, "{f}").unwrap();
            }
        }
        Value::F64(f) => {
            if f.is_nan() {
                output.push_str("NaN");
            } else if f.is_infinite() {
                if f.is_sign_positive() {
                    output.push_str("inf");
                } else {
                    output.push_str("-inf");
                }
            } else {
                write!(output, "{f}").unwrap();
            }
        }
        Value::String(s) => write!(output, "\"{}\"", escape_string(s)).unwrap(),
        Value::Code(Code { language, content }) => {
            write!(output, "{}`{}`", language, escape_string(content)).unwrap()
        }
        Value::CodeBlock(Code { language, content }) => {
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
        Value::Hole => output.push('!'),
        Value::MetaExtension(meta) => {
            // Format meta-extension with $$ prefix
            write!(output, "$${}", meta).unwrap()
        }
        Value::Path(path) => {
            // Format path with dot notation
            output.push('.');
            let mut path_parts = Vec::new();
            let mut i = 0;

            while i < path.0.len() {
                match &path.0[i] {
                    eure_value::value::PathSegment::Ident(id) => {
                        // Check if next segment is ArrayIndex
                        if i + 1 < path.0.len()
                            && let eure_value::value::PathSegment::ArrayIndex(idx) = &path.0[i + 1]
                        {
                            // Combine identifier with array index
                            if let Some(index) = idx {
                                path_parts.push(format!("{}[{}]", id.as_ref(), index));
                            } else {
                                path_parts.push(format!("{}[]", id.as_ref()));
                            }
                            i += 2; // Skip the ArrayIndex segment
                            continue;
                        }
                        path_parts.push(id.as_ref().to_string());
                    }
                    eure_value::value::PathSegment::Extension(id) => {
                        path_parts.push(format!("${}", id.as_ref()))
                    }
                    eure_value::value::PathSegment::MetaExt(id) => {
                        path_parts.push(format!("$${}", id.as_ref()))
                    }
                    eure_value::value::PathSegment::Value(v) => path_parts.push(format!("{v:?}")),
                    eure_value::value::PathSegment::TupleIndex(idx) => {
                        path_parts.push(idx.to_string())
                    }
                    eure_value::value::PathSegment::ArrayIndex(idx) => {
                        // Standalone array index (shouldn't normally happen after an ident)
                        if let Some(index) = idx {
                            path_parts.push(format!("[{index}]"));
                        } else {
                            path_parts.push("[]".to_string());
                        }
                    }
                }
                i += 1;
            }

            output.push_str(&path_parts.join("."));
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

/// Check if a string can be used as an unquoted identifier in EURE syntax
fn is_valid_identifier(s: &str) -> bool {
    Identifier::from_str(s).is_ok()
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
