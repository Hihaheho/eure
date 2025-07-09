use crate::error::{Error, Result};
use eure_value::value::{Array, Code, KeyCmpValue, Map, Tuple, Value, Variant};
use serde::Deserialize;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

pub struct Deserializer {
    value: Value,
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    // Parse EURE string to CST
    let tree = eure_parol::parse(s).map_err(|e| Error::ParseError(e.to_string()))?;

    // Extract values using ValueVisitor
    let mut values = eure_tree::value_visitor::Values::default();
    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(s, &mut values);
    tree.visit_from_root(&mut visitor)
        .map_err(|e| Error::ValueVisitorError(e.to_string()))?;

    // Extract the main value from the document
    let value = extract_document_value(&tree, &values, s);

    // Deserialize from Value
    from_value(value)
}

pub fn from_value<'a, T>(value: Value) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::new(value);
    T::deserialize(&mut deserializer)
}

impl Deserializer {
    fn new(value: Value) -> Self {
        Deserializer { value }
    }
}

// Helper function to extract values from the parsed EURE document (same as in eure-cli)
fn extract_document_value(
    tree: &eure_tree::Cst,
    values: &eure_tree::value_visitor::Values,
    input: &str,
) -> Value {
    use eure_tree::prelude::*;
    use eure_value::value::{Map, Value};

    let mut result_map = ahash::AHashMap::new();

    if let Ok(root_view) = tree.root_handle().get_view(tree)
        && let Ok(eure_view) = root_view.eure.get_view(tree)
    {
        if let Ok(Some(bindings_view)) = eure_view.eure_bindings.get_view(tree) {
            collect_bindings(&mut result_map, bindings_view, values, tree, input);
        }

        if let Ok(Some(sections_view)) = eure_view.eure_sections.get_view(tree) {
            process_sections(&mut result_map, sections_view, values, tree, input);
        }
    }

    let transformed = transform_variants(Value::Map(Map(result_map)));

    // Check if this is a synthetic "value" binding (for non-map top-level values)
    if let Value::Map(Map(ref map)) = transformed
        && map.len() == 1
        && map.contains_key(&KeyCmpValue::String("value".to_string()))
    {
        // Unwrap the synthetic binding
        return map
            .get(&KeyCmpValue::String("value".to_string()))
            .cloned()
            .unwrap_or(Value::Null);
    }

    transformed
}

// Helper functions from eure-cli
fn collect_bindings<F: eure_tree::prelude::CstFacade>(
    map: &mut ahash::AHashMap<KeyCmpValue, Value>,
    bindings_view: eure_tree::nodes::EureBindingsView,
    values: &eure_tree::value_visitor::Values,
    tree: &F,
    input: &str,
) {
    use eure_tree::prelude::*;

    if let Ok(binding_view) = bindings_view.binding.get_view(tree)
        && let Some(key_handles) = values.get_keys(&binding_view.keys)
        && !key_handles.is_empty()
    {
        let binding_value = match binding_view.binding_rhs.get_view(tree) {
            Ok(BindingRhsView::ValueBinding(value_binding_handle)) => {
                if let Ok(value_binding_view) = value_binding_handle.get_view(tree) {
                    values.get_value(&value_binding_view.value).cloned()
                } else {
                    None
                }
            }
            Ok(BindingRhsView::TextBinding(text_binding_handle)) => {
                if let Ok(text_binding_view) = text_binding_handle.get_view(tree) {
                    if let Ok(text_view) = text_binding_view.text.get_view(tree) {
                        if let Ok(data) = text_view.text.get_data(tree) {
                            let text = tree.get_str(data, input).unwrap_or("").trim();
                            Some(Value::String(text.to_string()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Ok(BindingRhsView::SectionBinding(section_binding_handle)) => {
                if let Ok(section_binding_view) = section_binding_handle.get_view(tree) {
                    if let Ok(eure_view) = section_binding_view.eure.get_view(tree) {
                        let mut section_map = ahash::AHashMap::new();

                        if let Ok(Some(bindings_view)) = eure_view.eure_bindings.get_view(tree) {
                            collect_bindings(&mut section_map, bindings_view, values, tree, input);
                        }

                        if let Ok(Some(sections_view)) = eure_view.eure_sections.get_view(tree) {
                            process_sections(&mut section_map, sections_view, values, tree, input);
                        }

                        Some(Value::Map(Map(section_map)))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(value) = binding_value {
            process_path_recursive(map, key_handles, value, values);
        }
    }

    if let Ok(more_bindings) = bindings_view.eure_bindings.get_view(tree)
        && let Some(more) = more_bindings
    {
        collect_bindings(map, more, values, tree, input);
    }
}

fn process_sections<F: eure_tree::prelude::CstFacade>(
    map: &mut ahash::AHashMap<KeyCmpValue, Value>,
    sections_view: eure_tree::nodes::EureSectionsView,
    values: &eure_tree::value_visitor::Values,
    tree: &F,
    input: &str,
) {
    use eure_tree::prelude::*;

    if let Ok(section_view) = sections_view.section.get_view(tree)
        && let Some(path_handles) = values.get_keys(&section_view.keys)
        && !path_handles.is_empty()
    {
        let mut section_map = ahash::AHashMap::new();

        if let Ok(section_body) = section_view.section_body.get_view(tree) {
            match section_body {
                SectionBodyView::SectionBinding(binding_handle) => {
                    if let Ok(binding_view) = binding_handle.get_view(tree)
                        && let Ok(eure_view) = binding_view.eure.get_view(tree)
                        && let Ok(Some(bindings_view)) = eure_view.eure_bindings.get_view(tree)
                    {
                        collect_bindings(&mut section_map, bindings_view, values, tree, input);
                    }
                }
                SectionBodyView::SectionBodyList(body_list_handle) => {
                    process_section_body_list(
                        &mut section_map,
                        body_list_handle,
                        values,
                        tree,
                        input,
                    );
                }
                SectionBodyView::Bind(bind_handle) => {
                    // Handle "Bind Value" case - this is a value assignment to the section
                    if let Ok(_bind_view) = bind_handle.get_view(tree) {
                        // The section itself has a value, treat it as a special key
                        // This would be something like: @ section = value
                        // For now, we'll skip this case as it's not commonly used
                    }
                }
            }
        }

        process_section_path(map, path_handles, Value::Map(Map(section_map)), values);
    }

    if let Ok(Some(more_sections)) = sections_view.eure_sections.get_view(tree) {
        process_sections(map, more_sections, values, tree, input);
    }
}

fn process_section_body_list<F: eure_tree::prelude::CstFacade>(
    map: &mut ahash::AHashMap<KeyCmpValue, Value>,
    body_list_handle: eure_tree::nodes::SectionBodyListHandle,
    values: &eure_tree::value_visitor::Values,
    tree: &F,
    input: &str,
) {
    use eure_tree::prelude::*;
    use eure_value::value::PathSegment;

    if let Ok(Some(body_list_view)) = body_list_handle.get_view(tree) {
        if let Ok(binding_view) = body_list_view.binding.get_view(tree)
            && let Some(key_handles) = values.get_keys(&binding_view.keys)
            && let Some(first_key) = key_handles.first()
            && let Some(path_seg) = values.get_path_segment(first_key)
        {
            let key = match path_seg {
                PathSegment::Ident(ident) => KeyCmpValue::String(ident.to_string()),
                PathSegment::Extension(ident) => KeyCmpValue::String(format!("${ident}")),
                PathSegment::Value(val) => val.clone(),
                _ => KeyCmpValue::String("unknown".to_string()),
            };

            if let Ok(binding_rhs_view) = binding_view.binding_rhs.get_view(tree) {
                match binding_rhs_view {
                    BindingRhsView::ValueBinding(value_binding_handle) => {
                        if let Ok(value_binding_view) = value_binding_handle.get_view(tree)
                            && let Some(value) = values.get_value(&value_binding_view.value)
                        {
                            map.insert(key, value.clone());
                        }
                    }
                    BindingRhsView::TextBinding(text_binding_handle) => {
                        if let Ok(text_binding_view) = text_binding_handle.get_view(tree)
                            && let Ok(text_view) = text_binding_view.text.get_view(tree)
                            && let Ok(data) = text_view.text.get_data(tree)
                        {
                            let text = tree.get_str(data, input).unwrap_or("").trim();
                            map.insert(key, Value::String(text.to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }

        if body_list_view.section_body_list.node_id() != body_list_handle.node_id() {
            process_section_body_list(map, body_list_view.section_body_list, values, tree, input);
        }
    }
}

fn process_section_path(
    map: &mut ahash::AHashMap<KeyCmpValue, Value>,
    path_handles: &[eure_tree::nodes::KeyHandle],
    section_value: Value,
    values: &eure_tree::value_visitor::Values,
) {
    if path_handles.is_empty() {
        return;
    }

    process_path_recursive(map, path_handles, section_value, values);
}

fn process_path_recursive(
    current_map: &mut ahash::AHashMap<KeyCmpValue, Value>,
    path_handles: &[eure_tree::nodes::KeyHandle],
    value: Value,
    values: &eure_tree::value_visitor::Values,
) {
    use eure_value::value::{Array, PathSegment};

    if path_handles.is_empty() {
        return;
    }

    let current_handle = &path_handles[0];
    let remaining_path = &path_handles[1..];

    if let Some(path_seg) = values.get_path_segment(current_handle) {
        match path_seg {
            PathSegment::Ident(ident) if !remaining_path.is_empty() => {
                // Check if the next segment is an ArrayIndex
                if let Some(next_handle) = remaining_path.first() {
                    if let Some(PathSegment::ArrayIndex(idx)) = values.get_path_segment(next_handle) {
                        // This is an array field
                        let key_cmp = KeyCmpValue::String(ident.to_string());
                        
                        // Check if we have more array indices in the remaining path
                        let has_more_arrays = remaining_path[1..].iter().any(|h| {
                            if let Some(seg) = values.get_path_segment(h) {
                                matches!(seg, PathSegment::ArrayIndex(_))
                            } else {
                                false
                            }
                        });
                        
                        // Skip the ArrayIndex segment since we're handling it here
                        let remaining_after_array = &remaining_path[1..];

                        if has_more_arrays && !remaining_after_array.is_empty() {
                            match current_map.entry(key_cmp) {
                                std::collections::hash_map::Entry::Occupied(mut entry) => {
                                    match entry.get_mut() {
                                        Value::Array(Array(arr)) => {
                                            if arr.is_empty() {
                                                arr.push(Value::Map(Map(ahash::AHashMap::new())));
                                            }
                                            if let Some(Value::Map(Map(last_element))) = arr.last_mut() {
                                                process_path_recursive(
                                                    last_element,
                                                    remaining_after_array,
                                                    value,
                                                    values,
                                                );
                                            }
                                        }
                                        _ => {
                                            let mut nested_map = ahash::AHashMap::new();
                                            process_path_recursive(
                                                &mut nested_map,
                                                remaining_after_array,
                                                value,
                                                values,
                                            );
                                            entry.insert(Value::Array(Array(vec![Value::Map(Map(
                                                nested_map,
                                            ))])));
                                        }
                                    }
                                }
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    let mut nested_map = ahash::AHashMap::new();
                                    process_path_recursive(&mut nested_map, remaining_after_array, value, values);
                                    entry.insert(Value::Array(Array(vec![Value::Map(Map(nested_map))])));
                                }
                            }
                        } else {
                            let element_value = if remaining_after_array.is_empty() {
                                value
                            } else {
                                let mut nested_map = ahash::AHashMap::new();
                                process_path_recursive(&mut nested_map, remaining_after_array, value, values);
                                Value::Map(Map(nested_map))
                            };

                            match current_map.entry(key_cmp) {
                                std::collections::hash_map::Entry::Occupied(mut entry) => {
                                    match entry.get_mut() {
                                        Value::Array(Array(arr)) => {
                                            arr.push(element_value);
                                        }
                                        _ => {
                                            let existing = entry.get().clone();
                                            entry
                                                .insert(Value::Array(Array(vec![existing, element_value])));
                                        }
                                    }
                                }
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    entry.insert(Value::Array(Array(vec![element_value])));
                                }
                            }
                        }
                        return; // We've handled the array case, return early
                    }
                }
            }
            _ => {
                let key = match path_seg {
                    PathSegment::Ident(ident) => KeyCmpValue::String(ident.to_string()),
                    PathSegment::Extension(ident) => KeyCmpValue::String(format!("${ident}")),
                    PathSegment::Value(val) => val.clone(),
                    _ => KeyCmpValue::String("key".to_string()),
                };

                if remaining_path.is_empty() {
                    current_map.insert(key, value);
                } else {
                    match current_map.entry(key) {
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            match entry.get_mut() {
                                Value::Map(Map(nested_map)) => {
                                    process_path_recursive(
                                        nested_map,
                                        remaining_path,
                                        value,
                                        values,
                                    );
                                }
                                _ => {
                                    let mut nested_map = ahash::AHashMap::new();
                                    process_path_recursive(
                                        &mut nested_map,
                                        remaining_path,
                                        value,
                                        values,
                                    );
                                    entry.insert(Value::Map(Map(nested_map)));
                                }
                            }
                        }
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            let mut nested_map = ahash::AHashMap::new();
                            process_path_recursive(&mut nested_map, remaining_path, value, values);
                            entry.insert(Value::Map(Map(nested_map)));
                        }
                    }
                }
            }
        }
    }
}

fn transform_variants(value: Value) -> Value {
    match value {
        Value::Map(Map(mut map)) => {
            let variant_name = map
                .get(&KeyCmpValue::String("$variant".to_string()))
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                });

            if let Some(name) = variant_name {
                map.remove(&KeyCmpValue::String("$variant".to_string()));
                map.remove(&KeyCmpValue::String("$variant.repr".to_string()));

                let mut transformed_map = ahash::AHashMap::new();
                for (key, val) in map {
                    transformed_map.insert(key, transform_variants(val));
                }

                Value::Variant(Variant {
                    tag: name,
                    content: Box::new(Value::Map(Map(transformed_map))),
                })
            } else {
                let mut transformed_map = ahash::AHashMap::new();
                for (key, val) in map {
                    transformed_map.insert(key, transform_variants(val));
                }
                Value::Map(Map(transformed_map))
            }
        }
        Value::Array(Array(items)) => {
            let transformed_items = items.into_iter().map(transform_variants).collect();
            Value::Array(Array(transformed_items))
        }
        other => other,
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::I64(i) => visitor.visit_i64(*i),
            Value::U64(u) => visitor.visit_u64(*u),
            Value::F32(f) => visitor.visit_f32(*f),
            Value::F64(f) => visitor.visit_f64(*f),
            Value::String(s) => visitor.visit_string(s.clone()),
            Value::Code(Code { content, .. }) => visitor.visit_string(content.clone()),
            Value::CodeBlock(Code { content, .. }) => visitor.visit_string(content.clone()),
            Value::Array(_) => self.deserialize_seq(visitor),
            Value::Tuple(_) => self.deserialize_tuple(0, visitor),
            Value::Map(_) => self.deserialize_map(visitor),
            Value::Variant(_) => self.deserialize_enum("", &[], visitor),
            Value::Unit => visitor.visit_unit(),
            Value::Hole => {
                // Holes should be caught and reported during validation
                // For now, return an error during deserialization
                Err(Error::Message(
                    "Cannot deserialize hole value (!) - holes must be filled with actual values"
                        .to_string(),
                ))
            }
            Value::Path(path) => {
                // Convert path to string representation, skipping extensions
                let mut path_parts = Vec::new();
                let mut i = 0;
                
                while i < path.0.len() {
                    match &path.0[i] {
                        eure_value::value::PathSegment::Ident(id) => {
                            // Check if next segment is ArrayIndex
                            if i + 1 < path.0.len() {
                                if let eure_value::value::PathSegment::ArrayIndex(idx) = &path.0[i + 1] {
                                    // Combine identifier with array index
                                    if let Some(index) = idx {
                                        path_parts.push(format!("{}[{}]", id.as_ref(), index));
                                    } else {
                                        path_parts.push(format!("{}[]", id.as_ref()));
                                    }
                                    i += 2; // Skip the ArrayIndex segment
                                    continue;
                                }
                            }
                            path_parts.push(id.as_ref().to_string());
                        }
                        eure_value::value::PathSegment::Extension(_) => {
                            // Extensions are metadata, not data - skip in serialization
                            i += 1;
                            continue;
                        }
                        eure_value::value::PathSegment::MetaExt(_) => {
                            // Meta-extensions are metadata, not data - skip in serialization
                            i += 1;
                            continue;
                        }
                        eure_value::value::PathSegment::Value(v) => path_parts.push(format!("{v:?}")),
                        eure_value::value::PathSegment::TupleIndex(idx) => path_parts.push(idx.to_string()),
                        eure_value::value::PathSegment::ArrayIndex(idx) => {
                            // Standalone array index (shouldn't normally happen after an ident)
                            if let Some(index) = idx {
                                path_parts.push(format!("[{}]", index));
                            } else {
                                path_parts.push("[]".to_string());
                            }
                        }
                    }
                    i += 1;
                }
                
                let path_str = path_parts.join(".");
                visitor.visit_string(format!(".{path_str}"))
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Bool(b) => visitor.visit_bool(*b),
            _ => Err(Error::InvalidType(format!(
                "expected bool, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::I64(i) => visitor.visit_i8(*i as i8),
            _ => Err(Error::InvalidType(format!(
                "expected i8, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::I64(i) => visitor.visit_i16(*i as i16),
            _ => Err(Error::InvalidType(format!(
                "expected i16, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::I64(i) => visitor.visit_i32(*i as i32),
            _ => Err(Error::InvalidType(format!(
                "expected i32, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::I64(i) => visitor.visit_i64(*i),
            _ => Err(Error::InvalidType(format!(
                "expected i64, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::U64(u) => visitor.visit_u8(*u as u8),
            Value::I64(i) if *i >= 0 && *i <= u8::MAX as i64 => visitor.visit_u8(*i as u8),
            _ => Err(Error::InvalidType(format!(
                "expected u8, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::U64(u) => visitor.visit_u16(*u as u16),
            Value::I64(i) if *i >= 0 && *i <= u16::MAX as i64 => visitor.visit_u16(*i as u16),
            _ => Err(Error::InvalidType(format!(
                "expected u16, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::U64(u) => visitor.visit_u32(*u as u32),
            Value::I64(i) if *i >= 0 && *i <= u32::MAX as i64 => visitor.visit_u32(*i as u32),
            _ => Err(Error::InvalidType(format!(
                "expected u32, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::U64(u) => visitor.visit_u64(*u),
            Value::I64(i) if *i >= 0 => visitor.visit_u64(*i as u64),
            _ => Err(Error::InvalidType(format!(
                "expected u64, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::F32(f) => visitor.visit_f32(*f),
            Value::F64(f) => visitor.visit_f32(*f as f32),
            _ => Err(Error::InvalidType(format!(
                "expected f32, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::F64(f) => visitor.visit_f64(*f),
            Value::F32(f) => visitor.visit_f64(*f as f64),
            _ => Err(Error::InvalidType(format!(
                "expected f64, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::String(s) if s.len() == 1 => visitor.visit_char(s.chars().next().unwrap()),
            _ => Err(Error::InvalidType(format!(
                "expected char, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::String(s) => visitor.visit_str(s),
            Value::Code(Code { content, .. }) => visitor.visit_str(content),
            Value::CodeBlock(Code { content, .. }) => visitor.visit_str(content),
            _ => Err(Error::InvalidType(format!(
                "expected string, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::String(s) => visitor.visit_string(s.clone()),
            Value::Code(Code { content, .. }) => visitor.visit_string(content.clone()),
            Value::CodeBlock(Code { content, .. }) => visitor.visit_string(content.clone()),
            _ => Err(Error::InvalidType(format!(
                "expected string, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Array(Array(values)) => {
                let bytes: Result<Vec<u8>> = values
                    .iter()
                    .map(|v| match v {
                        Value::U64(u) if *u <= 255 => Ok(*u as u8),
                        _ => Err(Error::InvalidType("expected array of bytes".to_string())),
                    })
                    .collect();
                visitor.visit_bytes(&bytes?)
            }
            _ => Err(Error::InvalidType(format!(
                "expected bytes, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Unit | Value::Null => visitor.visit_unit(),
            _ => Err(Error::InvalidType(format!(
                "expected unit, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match std::mem::replace(&mut self.value, Value::Null) {
            Value::Array(Array(values)) => visitor.visit_seq(SeqDeserializer::new(values)),
            _ => Err(Error::InvalidType("expected array".to_string())),
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match std::mem::replace(&mut self.value, Value::Null) {
            Value::Tuple(Tuple(values)) => {
                if values.len() != len {
                    return Err(Error::InvalidType(format!(
                        "expected tuple of length {}, found {}",
                        len,
                        values.len()
                    )));
                }
                visitor.visit_seq(SeqDeserializer::new(values))
            }
            Value::Array(Array(values)) => {
                if values.len() != len {
                    return Err(Error::InvalidType(format!(
                        "expected tuple of length {}, found array of length {}",
                        len,
                        values.len()
                    )));
                }
                visitor.visit_seq(SeqDeserializer::new(values))
            }
            _ => Err(Error::InvalidType("expected tuple".to_string())),
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match std::mem::replace(&mut self.value, Value::Null) {
            Value::Map(Map(map)) => visitor.visit_map(MapDeserializer::new(map)),
            _ => Err(Error::InvalidType("expected map".to_string())),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // For internally tagged enums, serde handles the tag extraction
        // Just treat it as a regular map
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match std::mem::replace(&mut self.value, Value::Null) {
            Value::Variant(variant) => visitor.visit_enum(EnumDeserializer::new(variant)),
            Value::Map(Map(map))
                if map.contains_key(&KeyCmpValue::String("$variant".to_string())) =>
            {
                // Handle map-based enum representation (external tagging)
                // Put the value back for the enum access to use
                self.value = Value::Map(Map(map));
                visitor.visit_enum(self)
            }
            value => {
                // For untagged enums, pass the value directly
                self.value = value;
                visitor.visit_enum(self)
            }
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<Value>,
}

impl SeqDeserializer {
    fn new(values: Vec<Value>) -> Self {
        SeqDeserializer {
            iter: values.into_iter(),
        }
    }
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let mut deserializer = Deserializer::new(value);
                seed.deserialize(&mut deserializer).map(Some)
            }
            None => Ok(None),
        }
    }
}

struct MapDeserializer {
    iter: std::vec::IntoIter<(KeyCmpValue, Value)>,
    value: Option<Value>,
}

impl MapDeserializer {
    fn new(map: ahash::AHashMap<KeyCmpValue, Value>) -> Self {
        MapDeserializer {
            iter: map.into_iter().collect::<Vec<_>>().into_iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let key_value = key_cmp_to_value(key);
                let mut deserializer = Deserializer::new(key_value);
                seed.deserialize(&mut deserializer).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                let mut deserializer = Deserializer::new(value);
                seed.deserialize(&mut deserializer)
            }
            None => Err(Error::Message("value called before key".to_string())),
        }
    }
}

struct EnumDeserializer {
    variant: Variant,
}

impl EnumDeserializer {
    fn new(variant: Variant) -> Self {
        EnumDeserializer { variant }
    }
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let variant_name = Value::String(self.variant.tag.clone());
        let mut deserializer = Deserializer::new(variant_name);
        let variant_index = seed.deserialize(&mut deserializer)?;
        Ok((
            variant_index,
            VariantDeserializer {
                content: *self.variant.content,
            },
        ))
    }
}

// Also implement EnumAccess for Deserializer (for untagged enums)
impl<'de> de::EnumAccess<'de> for &mut Deserializer {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // For map-based enums with $variant, extract the tag for variant matching
        if let Value::Map(Map(map)) = &self.value
            && let Some(Value::String(tag)) = map.get(&KeyCmpValue::String("$variant".to_string()))
        {
            let tag_value = Value::String(tag.clone());
            let mut tag_deserializer = Deserializer::new(tag_value);
            let variant_index = seed.deserialize(&mut tag_deserializer)?;
            return Ok((variant_index, self));
        }

        let value = seed.deserialize(&mut *self)?;
        Ok((value, self))
    }
}

struct VariantDeserializer {
    content: Value,
}

impl<'de> de::VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.content {
            Value::Unit | Value::Null => Ok(()),
            _ => Err(Error::InvalidType("expected unit variant".to_string())),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let mut deserializer = Deserializer::new(self.content);
        seed.deserialize(&mut deserializer)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut deserializer = Deserializer::new(self.content);
        de::Deserializer::deserialize_tuple(&mut deserializer, 0, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut deserializer = Deserializer::new(self.content);
        de::Deserializer::deserialize_map(&mut deserializer, visitor)
    }
}

// Also implement VariantAccess for Deserializer (for untagged enums)
impl<'de> de::VariantAccess<'de> for &mut Deserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        // For map-based enums, the map should only contain $variant for unit variants
        if let Value::Map(Map(map)) = &self.value
            && map.len() == 1
            && map.contains_key(&KeyCmpValue::String("$variant".to_string()))
        {
            return Ok(());
        }
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        // For map-based enums with $content, extract it
        if let Value::Map(Map(map)) = &self.value
            && let Some(content) = map.get(&KeyCmpValue::String("$content".to_string()))
        {
            let content_value = content.clone();
            self.value = content_value;
        }
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // For map-based enums, we need to remove the $variant field and deserialize the rest
        if let Value::Map(Map(map)) = &mut self.value
            && map.contains_key(&KeyCmpValue::String("$variant".to_string()))
        {
            let mut content_map = map.clone();
            content_map.remove(&KeyCmpValue::String("$variant".to_string()));
            self.value = Value::Map(Map(content_map));
        }
        de::Deserializer::deserialize_struct(self, "", fields, visitor)
    }
}

fn key_cmp_to_value(key: KeyCmpValue) -> Value {
    match key {
        KeyCmpValue::Null => Value::Null,
        KeyCmpValue::Bool(b) => Value::Bool(b),
        KeyCmpValue::I64(i) => Value::I64(i),
        KeyCmpValue::U64(u) => Value::U64(u),
        KeyCmpValue::String(s) => Value::String(s),
        KeyCmpValue::Tuple(Tuple(keys)) => {
            let values = keys.into_iter().map(key_cmp_to_value).collect();
            Value::Tuple(eure_value::value::Tuple(values))
        }
        KeyCmpValue::Unit => Value::Unit,
        KeyCmpValue::MetaExtension(meta) => todo!("This function must return Option"),
        KeyCmpValue::Hole => Value::Hole,
    }
}
