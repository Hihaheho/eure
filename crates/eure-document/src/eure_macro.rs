/// A declarative macro for building Eure documents, inspired by serde_json's `json!` macro.
///
/// # Syntax
///
/// The macro uses a TT muncher pattern to support arbitrary path combinations:
/// - Idents: `a.b.c`
/// - Extensions: `a.%ext` (use `%` instead of `$` since `$` is reserved in macros)
/// - Tuple index: `a.#0`, `a.#1`
/// - Array markers: `a[]` (push), `a[0]` (index)
/// - Tuple keys: `a.(1, "key")` (composite map keys)
/// - Mixed paths: `a.%ext[].b`, `a[].%ext.#0`, `a.(1, 2).name`
///
/// # Special Values
///
/// - `null`: Creates a null value
/// - `!`: Creates an unbound hole (explicit placeholder)
/// - `@code("content")`: Creates inline code with implicit language
/// - `@code("lang", "content")`: Creates inline code with explicit language
/// - `@block("content")`: Creates block code with implicit language
/// - `@block("lang", "content")`: Creates block code with explicit language
///
/// # Examples
///
/// ```
/// use eure_document::eure;
///
/// // Simple assignment (commas are optional)
/// let doc = eure!({
///     name = "Alice"
///     age = 30
/// });
///
/// // Null and hole values
/// let doc = eure!({
///     optional = null
///     placeholder = !
/// });
///
/// // Code values
/// let doc = eure!({
///     snippet = @code("let x = 1")
///     sql = @code("sql", "SELECT * FROM users")
///     script = @block("fn main() {}")
///     rust_code = @block("rust", "fn main() {\n    println!(\"Hello\");\n}")
/// });
///
/// // Nested paths
/// let doc = eure!({
///     user.name = "Bob"
///     user.active = true
/// });
///
/// // Blocks (for grouping)
/// let doc = eure!({
///     user {
///         name = "Charlie"
///         role = "admin"
///     }
/// });
///
/// // Extensions
/// let doc = eure!({
///     field.%variant = @code("text")
/// });
///
/// // Tuple index
/// let doc = eure!({
///     point.#0 = 1.0f64
///     point.#1 = 2.0f64
/// });
///
/// // Array markers
/// let doc = eure!({
///     items[] = 1
///     items[] = 2
/// });
///
/// // Tuple keys (composite map keys)
/// let doc = eure!({
///     map.(1, "key") = "value"
///     map.(true, 2) = "another"
/// });
///
/// // Arrays (literal)
/// let doc = eure!({
///     tags = ["a", "b", "c"]
/// });
///
/// // Tuples (literal)
/// let doc = eure!({
///     point = (1.0f64, 2.0f64)
/// });
///
/// // Sections (like TOML)
/// let doc = eure!({
///     @user
///     name = "Alice"
///     age = 30
///
///     @settings
///     theme = "dark"
/// });
/// ```
#[macro_export]
macro_rules! eure {
    // ========================================================================
    // Entry points
    //
    // The macro entry points handle the top-level document structure.
    // ========================================================================

    // Empty document: `eure!({})` creates an empty map document
    ({}) => {{
        $crate::document::EureDocument::new_empty()
    }};

    // Document with body: `eure!({ ... })` creates a document and processes the body
    ({ $($body:tt)* }) => {{
        #[allow(unused_mut)]
        let mut c = $crate::document::constructor::DocumentConstructor::new();
        $crate::eure!(@stmt c; $($body)*);
        c.finish()
    }};

    // ========================================================================
    // Value conversion helper (@value_tt)
    //
    // Converts a single token tree to a primitive value. This allows comma-free
    // syntax by matching exactly one tt at a time.
    //
    // Note: Arrays, tuples, and object literals are NOT handled here.
    // They require explicit patterns in @terminal/@stmt because they need
    // the DocumentConstructor's navigation system.
    // ========================================================================

    // null literal
    (@value_tt null) => { $crate::value::PrimitiveValue::Null };

    // Boolean identifiers
    (@value_tt true) => { true };
    (@value_tt false) => { false };

    // General literal fallback (string, int, float)
    (@value_tt $v:literal) => { $v };

    // General expression fallback (variables, expressions)
    (@value_tt $v:expr) => { $v };

    // ========================================================================
    // Array items helper (@array_items)
    //
    // Processes array items using the DocumentConstructor. Each item is added
    // by navigating to ArrayIndex(None) which appends to the array.
    // ========================================================================

    // Array items: empty (terminal)
    (@array_items $c:ident;) => {};

    // Array items: skip comma
    (@array_items $c:ident; , $($rest:tt)*) => {{
        $crate::eure!(@array_items $c; $($rest)*);
    }};

    // Array items: @code literal
    (@array_items $c:ident; @ code ($content:literal) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
        $c.bind_from($crate::text::Text::inline_implicit($content)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@array_items $c; $($rest)*);
    }};

    // Array items: @code with language
    (@array_items $c:ident; @ code ($lang:literal, $content:literal) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
        $c.bind_from($crate::text::Text::inline($content, $lang)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@array_items $c; $($rest)*);
    }};

    // Array items: nested array
    (@array_items $c:ident; [$($inner:tt)*] $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
        $c.bind_empty_array().unwrap();
        $crate::eure!(@array_items $c; $($inner)*);
        $c.end_scope(scope).unwrap();
        $crate::eure!(@array_items $c; $($rest)*);
    }};

    // Array items: nested tuple
    (@array_items $c:ident; ($($inner:tt)*) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
        $c.bind_empty_tuple().unwrap();
        $crate::eure!(@tuple_items $c 0; $($inner)*);
        $c.end_scope(scope).unwrap();
        $crate::eure!(@array_items $c; $($rest)*);
    }};

    // Array items: single item (primitive)
    (@array_items $c:ident; $item:tt $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
        $c.bind_from($crate::eure!(@value_tt $item)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@array_items $c; $($rest)*);
    }};

    // ========================================================================
    // Tuple items helper (@tuple_items)
    //
    // Processes tuple items using the DocumentConstructor. Each item is added
    // by navigating to TupleIndex(idx).
    // ========================================================================

    // Tuple items: empty (terminal)
    (@tuple_items $c:ident $idx:expr;) => {};

    // Tuple items: skip comma
    (@tuple_items $c:ident $idx:expr; , $($rest:tt)*) => {{
        $crate::eure!(@tuple_items $c $idx; $($rest)*);
    }};

    // Tuple items: @code literal
    (@tuple_items $c:ident $idx:expr; @ code ($content:literal) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::TupleIndex($idx)).unwrap();
        $c.bind_from($crate::text::Text::inline_implicit($content)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@tuple_items $c ($idx + 1); $($rest)*);
    }};

    // Tuple items: @code with language
    (@tuple_items $c:ident $idx:expr; @ code ($lang:literal, $content:literal) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::TupleIndex($idx)).unwrap();
        $c.bind_from($crate::text::Text::inline($content, $lang)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@tuple_items $c ($idx + 1); $($rest)*);
    }};

    // Tuple items: nested array
    (@tuple_items $c:ident $idx:expr; [$($inner:tt)*] $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::TupleIndex($idx)).unwrap();
        $c.bind_empty_array().unwrap();
        $crate::eure!(@array_items $c; $($inner)*);
        $c.end_scope(scope).unwrap();
        $crate::eure!(@tuple_items $c ($idx + 1); $($rest)*);
    }};

    // Tuple items: nested tuple
    (@tuple_items $c:ident $idx:expr; ($($inner:tt)*) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::TupleIndex($idx)).unwrap();
        $c.bind_empty_tuple().unwrap();
        $crate::eure!(@tuple_items $c 0; $($inner)*);
        $c.end_scope(scope).unwrap();
        $crate::eure!(@tuple_items $c ($idx + 1); $($rest)*);
    }};

    // Tuple items: single item (primitive)
    (@tuple_items $c:ident $idx:expr; $item:tt $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::TupleIndex($idx)).unwrap();
        $c.bind_from($crate::eure!(@value_tt $item)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@tuple_items $c ($idx + 1); $($rest)*);
    }};

    // ========================================================================
    // Object items helper (@object_items)
    //
    // Processes object literal items (k => v syntax) using the DocumentConstructor.
    // ========================================================================

    // Object items: empty (terminal)
    (@object_items $c:ident;) => {};

    // Object items: skip comma
    (@object_items $c:ident; , $($rest:tt)*) => {{
        $crate::eure!(@object_items $c; $($rest)*);
    }};

    // Object items: k => @code(content)
    (@object_items $c:ident; $key:tt => @ code ($content:literal) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
        $c.bind_from($crate::text::Text::inline_implicit($content)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@object_items $c; $($rest)*);
    }};

    // Object items: k => @code(lang, content)
    (@object_items $c:ident; $key:tt => @ code ($lang:literal, $content:literal) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
        $c.bind_from($crate::text::Text::inline($content, $lang)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@object_items $c; $($rest)*);
    }};

    // Object items: k => [array]
    (@object_items $c:ident; $key:tt => [$($inner:tt)*] $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
        $c.bind_empty_array().unwrap();
        $crate::eure!(@array_items $c; $($inner)*);
        $c.end_scope(scope).unwrap();
        $crate::eure!(@object_items $c; $($rest)*);
    }};

    // Object items: k => (tuple)
    (@object_items $c:ident; $key:tt => ($($inner:tt)*) $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
        $c.bind_empty_tuple().unwrap();
        $crate::eure!(@tuple_items $c 0; $($inner)*);
        $c.end_scope(scope).unwrap();
        $crate::eure!(@object_items $c; $($rest)*);
    }};

    // Object items: k => primitive
    (@object_items $c:ident; $key:tt => $val:tt $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
        $c.bind_from($crate::eure!(@value_tt $val)).unwrap();
        $c.end_scope(scope).unwrap();
        $crate::eure!(@object_items $c; $($rest)*);
    }};

    // ========================================================================
    // Statement handlers (@stmt)
    //
    // Process statements within a document or block. Each statement is either:
    // - A root value binding: `= value`
    // - A root special value: `= null`, `= !`, `= @code(...)`
    // - A path-based statement: `path = value` or `path { block }`
    // - A section: `@path` followed by bindings
    //
    // The statement handler delegates path parsing to @path.
    // ========================================================================

    // Empty body - nothing to process
    (@stmt $c:ident;) => {};

    // Skip optional comma at statement start (commas are optional separators)
    (@stmt $c:ident; , $($rest:tt)*) => {{
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: hole (!) - explicit unbound placeholder
    // Note: This must come before general `= $v:tt` to match `!` specifically
    (@stmt $c:ident; = ! $($rest:tt)*) => {{
        // Hole is the default state, so we don't need to bind anything
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: negative literal (e.g., = -42)
    // Negative numbers are two tokens: `-` and the number, so they need special handling
    (@stmt $c:ident; = - $v:literal $($rest:tt)*) => {{
        $c.bind_from(-$v).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: inline code with implicit language - @code("content")
    (@stmt $c:ident; = @ code ($content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::inline_implicit($content)).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: inline code with explicit language - @code("lang", "content")
    (@stmt $c:ident; = @ code ($lang:literal, $content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::inline($content, $lang)).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: block code with implicit language - @block("content")
    (@stmt $c:ident; = @ block ($content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::block_implicit($content)).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: block code with explicit language - @block("lang", "content")
    (@stmt $c:ident; = @ block ($lang:literal, $content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::block($content, $lang)).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: empty array
    (@stmt $c:ident; = [] $($rest:tt)*) => {{
        $c.bind_empty_array().unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: array with items
    (@stmt $c:ident; = [$($items:tt)+] $($rest:tt)*) => {{
        $c.bind_empty_array().unwrap();
        $crate::eure!(@array_items $c; $($items)+);
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: empty tuple
    (@stmt $c:ident; = () $($rest:tt)*) => {{
        $c.bind_empty_tuple().unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: tuple with items
    (@stmt $c:ident; = ($($items:tt)+) $($rest:tt)*) => {{
        $c.bind_empty_tuple().unwrap();
        $crate::eure!(@tuple_items $c 0; $($items)+);
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: object literal with map syntax { k => v, ... }
    (@stmt $c:ident; = { $key:tt => $($inner:tt)+ } $($rest:tt)*) => {{
        $c.bind_empty_map().unwrap();
        $crate::eure!(@object_items $c; $key => $($inner)+);
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Root binding: general value (single token tree for primitives)
    (@stmt $c:ident; = $v:tt $($rest:tt)*) => {{
        $c.bind_from($crate::eure!(@value_tt $v)).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Section: @path followed by bindings
    (@stmt $c:ident; @ $seg:ident $($rest:tt)*) => {{
        let scope = $c.begin_scope();
        $c.navigate($crate::path::PathSegment::Ident(
            $crate::identifier::Identifier::new_unchecked(stringify!($seg))
        )).unwrap();
        $crate::eure!(@section_after_seg $c scope; $($rest)*);
    }};

    // Start parsing a path-based statement - delegate to path parser
    // Creates a scope that will be closed when the statement ends
    (@stmt $c:ident; $($tokens:tt)+) => {{
        let scope = $c.begin_scope();
        $crate::eure!(@path $c scope; $($tokens)+);
    }};

    // ========================================================================
    // Section handlers (@section_*)
    //
    // Parse section syntax: @path followed by bindings until next section or end.
    // ========================================================================

    // After parsing a segment, check for more path or bindings
    (@section_after_seg $c:ident $scope:ident; . $seg:ident $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Ident(
            $crate::identifier::Identifier::new_unchecked(stringify!($seg))
        )).unwrap();
        $crate::eure!(@section_after_seg $c $scope; $($rest)*);
    }};

    // Section with value binding: @path = value
    (@section_after_seg $c:ident $scope:ident; = $v:tt $($rest:tt)*) => {{
        $c.bind_from($crate::eure!(@value_tt $v)).unwrap();
        // Continue parsing bindings within this section
        $crate::eure!(@section_bindings $c $scope; $($rest)*);
    }};

    // Section with empty block: @path {}
    (@section_after_seg $c:ident $scope:ident; {} $($rest:tt)*) => {{
        $c.bind_empty_map().unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Section with non-empty block: @path { ... }
    (@section_after_seg $c:ident $scope:ident; { $($inner:tt)+ } $($rest:tt)*) => {{
        $crate::eure!(@stmt $c; $($inner)+);
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Section body starts (no value binding, no block)
    (@section_after_seg $c:ident $scope:ident; $($rest:tt)*) => {{
        $crate::eure!(@section_bindings $c $scope; $($rest)*);
    }};

    // Section bindings: empty - close scope
    (@section_bindings $c:ident $scope:ident;) => {{
        $c.end_scope($scope).unwrap();
    }};

    // Section bindings: skip optional comma
    (@section_bindings $c:ident $scope:ident; , $($rest:tt)*) => {{
        $crate::eure!(@section_bindings $c $scope; $($rest)*);
    }};

    // Section bindings: new section starts - close current and start new
    (@section_bindings $c:ident $scope:ident; @ $seg:ident $($rest:tt)*) => {{
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; @ $seg $($rest)*);
    }};

    // Section bindings: regular binding (path-based)
    (@section_bindings $c:ident $scope:ident; $($tokens:tt)+) => {{
        let inner_scope = $c.begin_scope();
        $crate::eure!(@section_path $c $scope inner_scope; $($tokens)+);
    }};

    // Section path parsing - similar to @path but returns to @section_bindings
    (@section_path $c:ident $section_scope:ident $scope:ident; $seg:ident $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Ident(
            $crate::identifier::Identifier::new_unchecked(stringify!($seg))
        )).unwrap();
        $crate::eure!(@section_after_path $c $section_scope $scope; $($rest)*);
    }};

    // Section path: extension segment
    (@section_path $c:ident $section_scope:ident $scope:ident; % $ext:ident $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Extension(
            $crate::identifier::Identifier::new_unchecked(stringify!($ext))
        )).unwrap();
        $crate::eure!(@section_after_path $c $section_scope $scope; $($rest)*);
    }};

    // Section path: extension with string literal
    (@section_path $c:ident $section_scope:ident $scope:ident; % $ext:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Extension(
            $ext.parse().unwrap()
        )).unwrap();
        $crate::eure!(@section_after_path $c $section_scope $scope; $($rest)*);
    }};

    // Section path: tuple index
    (@section_path $c:ident $section_scope:ident $scope:ident; # $idx:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::TupleIndex($idx)).unwrap();
        $crate::eure!(@section_after_path $c $section_scope $scope; $($rest)*);
    }};

    // Section path: tuple key
    (@section_path $c:ident $section_scope:ident $scope:ident; ($($tuple:tt)*) $($rest:tt)*) => {{
        let key = $crate::eure!(@build_tuple_key; $($tuple)*);
        $c.navigate($crate::path::PathSegment::Value(key)).unwrap();
        $crate::eure!(@section_after_path $c $section_scope $scope; $($rest)*);
    }};

    // Section path: string literal key
    (@section_path $c:ident $section_scope:ident $scope:ident; $key:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
        $crate::eure!(@section_after_path $c $section_scope $scope; $($rest)*);
    }};

    // After section path segment: array marker
    (@section_after_path $c:ident $section_scope:ident $scope:ident; [$($arr:tt)*] $($rest:tt)*) => {{
        $crate::eure!(@section_array_marker $c $section_scope $scope [$($arr)*]; $($rest)*);
    }};

    // After section path segment: continue to terminal
    (@section_after_path $c:ident $section_scope:ident $scope:ident; $($rest:tt)*) => {{
        $crate::eure!(@section_terminal $c $section_scope $scope; $($rest)*);
    }};

    // Section array marker: empty (push)
    (@section_array_marker $c:ident $section_scope:ident $scope:ident []; $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
        $crate::eure!(@section_terminal $c $section_scope $scope; $($rest)*);
    }};

    // Section array marker: with index
    (@section_array_marker $c:ident $section_scope:ident $scope:ident [$idx:literal]; $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::ArrayIndex(Some($idx))).unwrap();
        $crate::eure!(@section_terminal $c $section_scope $scope; $($rest)*);
    }};

    // Section terminal: more path
    (@section_terminal $c:ident $section_scope:ident $scope:ident; . $($rest:tt)+) => {{
        $crate::eure!(@section_path $c $section_scope $scope; $($rest)+);
    }};

    // Section terminal: hole
    (@section_terminal $c:ident $section_scope:ident $scope:ident; = ! $($rest:tt)*) => {{
        $c.end_scope($scope).unwrap();
        $crate::eure!(@section_bindings $c $section_scope; $($rest)*);
    }};

    // Section terminal: inline code with implicit language
    (@section_terminal $c:ident $section_scope:ident $scope:ident; = @ code ($content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::inline_implicit($content)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@section_bindings $c $section_scope; $($rest)*);
    }};

    // Section terminal: inline code with explicit language
    (@section_terminal $c:ident $section_scope:ident $scope:ident; = @ code ($lang:literal, $content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::inline($content, $lang)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@section_bindings $c $section_scope; $($rest)*);
    }};

    // Section terminal: block code with implicit language
    (@section_terminal $c:ident $section_scope:ident $scope:ident; = @ block ($content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::block_implicit($content)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@section_bindings $c $section_scope; $($rest)*);
    }};

    // Section terminal: block code with explicit language
    (@section_terminal $c:ident $section_scope:ident $scope:ident; = @ block ($lang:literal, $content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::block($content, $lang)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@section_bindings $c $section_scope; $($rest)*);
    }};

    // Section terminal: value assignment
    (@section_terminal $c:ident $section_scope:ident $scope:ident; = $v:tt $($rest:tt)*) => {{
        $c.bind_from($crate::eure!(@value_tt $v)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@section_bindings $c $section_scope; $($rest)*);
    }};

    // Section terminal: empty block
    (@section_terminal $c:ident $section_scope:ident $scope:ident; {} $($rest:tt)*) => {{
        $c.bind_empty_map().unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@section_bindings $c $section_scope; $($rest)*);
    }};

    // Section terminal: non-empty block
    (@section_terminal $c:ident $section_scope:ident $scope:ident; { $($inner:tt)+ } $($rest:tt)*) => {{
        $crate::eure!(@stmt $c; $($inner)+);
        $c.end_scope($scope).unwrap();
        $crate::eure!(@section_bindings $c $section_scope; $($rest)*);
    }};

    // ========================================================================
    // Path segment parsing (@path)
    //
    // Parse one path segment at a time using TT muncher pattern.
    // Each segment type navigates to a child node and then delegates to @after_path.
    //
    // Supported segment types:
    // - `ident`: Regular identifier (a, user, field_name)
    // - `%ext`: Extension namespace ($variant becomes %variant in macro)
    // - `#N`: Tuple index (#0, #1, #2)
    // - `(a, b)`: Tuple key for composite map keys
    // - `"key"`: String literal for non-identifier keys (e.g., "min-length")
    // ========================================================================

    // Segment: identifier (e.g., `field`, `user`, `name`)
    (@path $c:ident $scope:ident; $seg:ident $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Ident(
            $crate::identifier::Identifier::new_unchecked(stringify!($seg))
        )).unwrap();
        $crate::eure!(@after_path $c $scope; $($rest)*);
    }};

    // Segment: extension with identifier (e.g., `%variant`, `%schema`)
    // Note: Uses % instead of $ because $ is reserved in macros
    (@path $c:ident $scope:ident; % $ext:ident $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Extension(
            $crate::identifier::Identifier::new_unchecked(stringify!($ext))
        )).unwrap();
        $crate::eure!(@after_path $c $scope; $($rest)*);
    }};

    // Segment: extension with string literal (e.g., `%"variant-repr"`)
    // Used for hyphenated extension names that aren't valid Rust identifiers
    (@path $c:ident $scope:ident; % $ext:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Extension(
            $ext.parse().unwrap()
        )).unwrap();
        $crate::eure!(@after_path $c $scope; $($rest)*);
    }};

    // Segment: tuple index (e.g., `#0`, `#1`, `#255`)
    (@path $c:ident $scope:ident; # $idx:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::TupleIndex($idx)).unwrap();
        $crate::eure!(@after_path $c $scope; $($rest)*);
    }};

    // Segment: tuple key (e.g., `(1, "key")`, `(true, 2)`)
    // Used as composite map keys
    (@path $c:ident $scope:ident; ($($tuple:tt)*) $($rest:tt)*) => {{
        let key = $crate::eure!(@build_tuple_key; $($tuple)*);
        $c.navigate($crate::path::PathSegment::Value(key)).unwrap();
        $crate::eure!(@after_path $c $scope; $($rest)*);
    }};

    // Segment: string literal key (e.g., `"min-length"`, `"Content-Type"`)
    // Used for keys that aren't valid identifiers
    (@path $c:ident $scope:ident; $key:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
        $crate::eure!(@after_path $c $scope; $($rest)*);
    }};

    // ========================================================================
    // Build tuple key (@build_tuple_key)
    //
    // Constructs an ObjectKey::Tuple from comma-separated values.
    // Used for composite map keys like (1, "key").
    // ========================================================================

    // Empty tuple key: ()
    (@build_tuple_key;) => {{
        $crate::value::ObjectKey::Tuple($crate::value::Tuple(Default::default()))
    }};

    // Non-empty tuple key: (a, b, c) - each item converted via Into<ObjectKey>
    (@build_tuple_key; $($item:expr),+ $(,)?) => {{
        $crate::value::ObjectKey::Tuple($crate::value::Tuple::from_iter(
            [$(<_ as Into<$crate::value::ObjectKey>>::into($item)),+]
        ))
    }};

    // ========================================================================
    // After path segment (@after_path)
    //
    // After parsing a segment, check if there's an optional array marker [].
    // If found, handle it; otherwise proceed to terminal handling.
    // ========================================================================

    // Has array marker - delegate to @array_marker
    (@after_path $c:ident $scope:ident; [$($arr:tt)*] $($rest:tt)*) => {{
        $crate::eure!(@array_marker $c $scope [$($arr)*]; $($rest)*);
    }};

    // No array marker - proceed to terminal handling
    (@after_path $c:ident $scope:ident; $($rest:tt)*) => {{
        $crate::eure!(@terminal $c $scope; $($rest)*);
    }};

    // ========================================================================
    // Array marker handling (@array_marker)
    //
    // Process the content of array markers:
    // - `[]`: Push to array (creates new element)
    // - `[N]`: Access array at index N
    // ========================================================================

    // Empty array marker: push operation (creates new element at end)
    (@array_marker $c:ident $scope:ident []; $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
        $crate::eure!(@terminal $c $scope; $($rest)*);
    }};

    // Array marker with index: access at specific position
    (@array_marker $c:ident $scope:ident [$idx:literal]; $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::ArrayIndex(Some($idx))).unwrap();
        $crate::eure!(@terminal $c $scope; $($rest)*);
    }};

    // ========================================================================
    // Terminal handling (@terminal)
    //
    // Handle what comes after the path:
    // - `.more.path`: Continue parsing more segments
    // - `= value`: Bind a value (single token tree)
    // - `= !`: Leave as hole (explicit placeholder)
    // - `= @code(...)`: Bind inline code
    // - `= @block(...)`: Bind block code
    // - `{ ... }`: Block syntax (grouped bindings)
    // - `{}`: Empty block (creates empty map)
    //
    // Uses $v:tt for values to enable comma-free syntax.
    // Note: @code and @block must be handled specially as they span multiple tokens.
    // ========================================================================

    // Continuation: more path segments after dot
    (@terminal $c:ident $scope:ident; . $($rest:tt)+) => {{
        $crate::eure!(@path $c $scope; $($rest)+);
    }};

    // Terminal: hole (!) - explicit unbound placeholder
    (@terminal $c:ident $scope:ident; = ! $($rest:tt)*) => {{
        // Hole is the default state, so we just close the scope
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: negative literal (e.g., path = -42)
    (@terminal $c:ident $scope:ident; = - $v:literal $($rest:tt)*) => {{
        $c.bind_from(-$v).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: inline code with implicit language - @code("content")
    (@terminal $c:ident $scope:ident; = @ code ($content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::inline_implicit($content)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: inline code with explicit language - @code("lang", "content")
    (@terminal $c:ident $scope:ident; = @ code ($lang:literal, $content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::inline($content, $lang)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: block code with implicit language - @block("content")
    (@terminal $c:ident $scope:ident; = @ block ($content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::block_implicit($content)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: block code with explicit language - @block("lang", "content")
    (@terminal $c:ident $scope:ident; = @ block ($lang:literal, $content:literal) $($rest:tt)*) => {{
        $c.bind_from($crate::text::Text::block($content, $lang)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: empty array
    (@terminal $c:ident $scope:ident; = [] $($rest:tt)*) => {{
        $c.bind_empty_array().unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: array with items
    (@terminal $c:ident $scope:ident; = [$($items:tt)+] $($rest:tt)*) => {{
        $c.bind_empty_array().unwrap();
        $crate::eure!(@array_items $c; $($items)+);
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: empty tuple
    (@terminal $c:ident $scope:ident; = () $($rest:tt)*) => {{
        $c.bind_empty_tuple().unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: tuple with items
    (@terminal $c:ident $scope:ident; = ($($items:tt)+) $($rest:tt)*) => {{
        $c.bind_empty_tuple().unwrap();
        $crate::eure!(@tuple_items $c 0; $($items)+);
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: object literal with map syntax { k => v, ... }
    (@terminal $c:ident $scope:ident; = { $key:tt => $($inner:tt)+ } $($rest:tt)*) => {{
        $c.bind_empty_map().unwrap();
        $crate::eure!(@object_items $c; $key => $($inner)+);
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: value assignment (single token tree for primitives)
    (@terminal $c:ident $scope:ident; = $v:tt $($rest:tt)*) => {{
        $c.bind_from($crate::eure!(@value_tt $v)).unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: empty block -> empty map
    (@terminal $c:ident $scope:ident; {} $($rest:tt)*) => {{
        $c.bind_empty_map().unwrap();
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};

    // Terminal: non-empty block
    (@terminal $c:ident $scope:ident; { $($inner:tt)+ } $($rest:tt)*) => {{
        $crate::eure!(@stmt $c; $($inner)+);
        $c.end_scope($scope).unwrap();
        $crate::eure!(@stmt $c; $($rest)*);
    }};
}

#[cfg(test)]
mod tests {
    use crate::document::EureDocument;
    use alloc::vec;

    #[test]
    fn test_eure_empty() {
        let doc = eure!({});
        assert_eq!(doc, EureDocument::new_empty());
    }

    #[test]
    fn test_eure_simple_assignment() {
        let doc = eure!({ name = "Alice" });

        // Verify the structure
        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let name_node_id = root.as_map().unwrap().get(&"name".into()).unwrap();
        let name_node = doc.node(name_node_id);
        let prim = name_node.as_primitive().unwrap();
        assert_eq!(prim.as_str(), Some("Alice"));
    }

    #[test]
    fn test_eure_nested_path() {
        let doc = eure!({
            user.name = "Bob"
            user.age = 30
        });

        // Verify structure: root.user.name = "Bob", root.user.age = 30
        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        let name_id = user.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("Bob"));

        let age_id = user.as_map().unwrap().get(&"age".into()).unwrap();
        let age = doc.node(age_id);
        assert!(matches!(
            age.as_primitive(),
            Some(crate::value::PrimitiveValue::Integer(_))
        ));
    }

    #[test]
    fn test_eure_block() {
        let doc = eure!({
            user {
                name = "Charlie"
                active = true
            }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        let name_id = user.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("Charlie"));
    }

    #[test]
    fn test_eure_extension() {
        let doc = eure!({
            field.%variant = @code("text")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);

        // Check extension
        let variant_id = field.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);
        let text = variant.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "text");
    }

    #[test]
    fn test_eure_extension_with_child() {
        // Test pattern: a.%ext.b = value
        let doc = eure!({
            field.%variant.name = @code("text")
            field.%variant.min_length = 3
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);

        // Check extension
        let variant_id = field.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);

        // Check child of extension
        let name_id = variant.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        let text = name.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "text");

        let min_length_id = variant.as_map().unwrap().get(&"min_length".into()).unwrap();
        let min_length = doc.node(min_length_id);
        assert!(matches!(
            min_length.as_primitive(),
            Some(crate::value::PrimitiveValue::Integer(_))
        ));
    }

    #[test]
    fn test_eure_array() {
        let doc = eure!({ tags = ["a", "b", "c"] });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let tags_id = root.as_map().unwrap().get(&"tags".into()).unwrap();
        let tags = doc.node(tags_id);
        let array = tags.as_array().unwrap();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_eure_tuple() {
        let doc = eure!({ point = (1.5, 2.5) });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let point_id = root.as_map().unwrap().get(&"point".into()).unwrap();
        let point = doc.node(point_id);
        let tuple = point.as_tuple().unwrap();
        assert_eq!(tuple.len(), 2);
    }

    #[test]
    fn test_eure_multiple_assignments() {
        let doc = eure!({
            a = 1
            b = 2
            c = 3
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map = root.as_map().unwrap();
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_eure_complex() {
        // A more complex example combining features
        let doc = eure!({
            schema {
                field.%variant = @code("text")
                field.min_length = 3
                field.max_length = 20
            }
            tags = ["required"]
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);

        // Check schema exists
        let schema_id = root.as_map().unwrap().get(&"schema".into()).unwrap();
        let schema = doc.node(schema_id);

        // Check field exists with extension
        let field_id = schema.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);
        assert!(field.get_extension(&"variant".parse().unwrap()).is_some());

        // Check tags array
        let tags_id = root.as_map().unwrap().get(&"tags".into()).unwrap();
        let tags = doc.node(tags_id);
        assert_eq!(tags.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_eure_array_push() {
        // Test array push syntax: items[] = value
        let doc = eure!({
            items[] = 1
            items[] = 2
            items[] = 3
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_eure_array_push_with_child() {
        // Test: items[].name = value (array push then navigate to child)
        let doc = eure!({
            items[].name = "first"
            items[].name = "second"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Check first element has name = "first"
        let first_id = array.get(0).unwrap();
        let first = doc.node(first_id);
        let name_id = first.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("first"));
    }

    #[test]
    fn test_eure_tuple_index() {
        // Test tuple index syntax: point.#0, point.#1
        let doc = eure!({
            point.#0 = 1.5
            point.#1 = 2.5
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let point_id = root.as_map().unwrap().get(&"point".into()).unwrap();
        let point = doc.node(point_id);
        let tuple = point.as_tuple().unwrap();
        assert_eq!(tuple.len(), 2);
    }

    #[test]
    fn test_eure_mixed_path_extension_array() {
        // Test: a.%ext[].b = value
        let doc = eure!({
            field.%items[].name = "item1"
            field.%items[].name = "item2"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);

        // Get extension
        let items_id = field.get_extension(&"items".parse().unwrap()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_eure_mixed_path_array_extension() {
        // Test: items[].%variant = value
        let doc = eure!({
            items[].%variant = @code("text")
            items[].%variant = @code("number")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Check first element has extension
        let first_id = array.get(0).unwrap();
        let first = doc.node(first_id);
        let variant_id = first.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);
        assert_eq!(
            variant.as_primitive().unwrap().as_text().unwrap().as_str(),
            "text"
        );
    }

    #[test]
    fn test_eure_tuple_key() {
        use crate::value::{ObjectKey, Tuple};

        // Test tuple key: map.(1, "a") = value
        let doc = eure!({
            map.(1, "key") = "value1"
            map.(2, "key") = "value2"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map_id = root.as_map().unwrap().get(&"map".into()).unwrap();
        let map_node = doc.node(map_id);
        let map = map_node.as_map().unwrap();
        assert_eq!(map.len(), 2);

        // Check key (1, "key") exists
        let tuple_key = ObjectKey::Tuple(Tuple(alloc::vec![1.into(), "key".into()]));
        let value_id = map.get(&tuple_key).unwrap();
        let value = doc.node(value_id);
        assert_eq!(value.as_primitive().unwrap().as_str(), Some("value1"));
    }

    #[test]
    fn test_eure_tuple_key_with_bool() {
        use crate::value::{ObjectKey, Tuple};

        // Test tuple key with bool: map.(true, 1) = value
        let doc = eure!({
            map.(true, 1) = "yes"
            map.(false, 1) = "no"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map_id = root.as_map().unwrap().get(&"map".into()).unwrap();
        let map_node = doc.node(map_id);
        let map = map_node.as_map().unwrap();
        assert_eq!(map.len(), 2);

        // Check key (true, 1) exists
        let tuple_key = ObjectKey::Tuple(Tuple(alloc::vec![true.into(), 1.into()]));
        let value_id = map.get(&tuple_key).unwrap();
        let value = doc.node(value_id);
        assert_eq!(value.as_primitive().unwrap().as_str(), Some("yes"));
    }

    #[test]
    fn test_eure_tuple_key_with_child() {
        use crate::value::{ObjectKey, Tuple};

        // Test tuple key with child path: map.(1, 2).name = value
        let doc = eure!({
            map.(1, 2).name = "point_a"
            map.(1, 2).value = 42
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map_id = root.as_map().unwrap().get(&"map".into()).unwrap();
        let map_node = doc.node(map_id);
        let map = map_node.as_map().unwrap();

        // Check key (1, 2) has children
        let tuple_key = ObjectKey::Tuple(Tuple(alloc::vec![1.into(), 2.into()]));
        let entry_id = map.get(&tuple_key).unwrap();
        let entry = doc.node(entry_id);
        let entry_map = entry.as_map().unwrap();

        let name_id = entry_map.get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("point_a"));
    }

    #[test]
    fn test_eure_string_key() {
        // Test string literal key for hyphenated identifiers: "min-length" = 3
        let doc = eure!({
            field."min-length" = 3
            field."max-length" = 20
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);
        let field_map = field.as_map().unwrap();

        // Check "min-length" key exists
        let min_id = field_map.get(&"min-length".into()).unwrap();
        let min_node = doc.node(min_id);
        assert!(matches!(
            min_node.as_primitive(),
            Some(crate::value::PrimitiveValue::Integer(_))
        ));
    }

    #[test]
    fn test_eure_object_literal() {
        // Test object literal with => syntax
        let doc = eure!({
            variants.click = { "x" => 1.0, "y" => 2.0 }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let variants_id = root.as_map().unwrap().get(&"variants".into()).unwrap();
        let variants = doc.node(variants_id);
        let click_id = variants.as_map().unwrap().get(&"click".into()).unwrap();
        let click = doc.node(click_id);
        let click_map = click.as_map().unwrap();

        assert_eq!(click_map.len(), 2);
        assert!(click_map.get(&"x".into()).is_some());
        assert!(click_map.get(&"y".into()).is_some());
    }

    #[test]
    fn test_eure_object_literal_with_string() {
        // Test object literal for schema-like patterns
        let doc = eure!({
            schema.variants.success = { "data" => "any" }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let schema_id = root.as_map().unwrap().get(&"schema".into()).unwrap();
        let schema = doc.node(schema_id);
        let variants_id = schema.as_map().unwrap().get(&"variants".into()).unwrap();
        let variants = doc.node(variants_id);
        let success_id = variants.as_map().unwrap().get(&"success".into()).unwrap();
        let success = doc.node(success_id);
        let success_map = success.as_map().unwrap();

        let data_id = success_map.get(&"data".into()).unwrap();
        let data = doc.node(data_id);
        assert_eq!(data.as_primitive().unwrap().as_str(), Some("any"));
    }

    #[test]
    fn test_eure_value_binding() {
        // Test value binding at root: = value
        let doc = eure!({
            = @code("hello")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let text = root.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "hello");
    }

    #[test]
    fn test_eure_value_binding_with_extension() {
        // Test value binding with extension: = value, %ext = value
        let doc = eure!({
            = @code("any")
            %variant = "literal"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);

        // Check value
        let text = root.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "any");

        // Check extension
        let variant_id = root.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);
        assert_eq!(variant.as_primitive().unwrap().as_str(), Some("literal"));
    }

    #[test]
    fn test_eure_empty_block() {
        // Empty block should create an empty map, not a Hole
        let doc = eure!({ config {} });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let config_id = root.as_map().unwrap().get(&"config".into()).unwrap();
        let config = doc.node(config_id);

        // Should be an empty map, not Hole
        let map = config
            .as_map()
            .expect("Empty block should create an empty map");
        assert!(map.is_empty());
    }

    // ========================================================================
    // Tests for new features: null, !, @code, @block
    // ========================================================================

    #[test]
    fn test_eure_null_literal() {
        // Test null literal at field level
        let doc = eure!({ optional = null });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let opt_id = root.as_map().unwrap().get(&"optional".into()).unwrap();
        let opt = doc.node(opt_id);
        assert!(matches!(
            opt.as_primitive(),
            Some(crate::value::PrimitiveValue::Null)
        ));
    }

    #[test]
    fn test_eure_null_root() {
        // Test null literal at root level
        let doc = eure!({
            = null
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        assert!(matches!(
            root.as_primitive(),
            Some(crate::value::PrimitiveValue::Null)
        ));
    }

    #[test]
    fn test_eure_hole_literal() {
        use crate::document::node::NodeValue;

        // Test hole (!) literal at field level
        let doc = eure!({
            placeholder = !
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let placeholder_id = root.as_map().unwrap().get(&"placeholder".into()).unwrap();
        let placeholder = doc.node(placeholder_id);
        assert_eq!(placeholder.content, NodeValue::Hole(None));
    }

    #[test]
    fn test_eure_hole_root() {
        use crate::document::node::NodeValue;

        // Test hole at root level - root should remain unbound (Hole), but
        // finish() converts unbound root to empty map
        let doc = eure!({
            = !
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        // finish() converts Hole root to Map
        assert!(matches!(root.content, NodeValue::Map(_)));
    }

    #[test]
    fn test_eure_code_inline_implicit() {
        // Test @code("content") - inline code with implicit language
        let doc = eure!({
            snippet = @code("let x = 1")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let snippet_id = root.as_map().unwrap().get(&"snippet".into()).unwrap();
        let snippet = doc.node(snippet_id);
        let text = snippet.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "let x = 1");
        assert!(text.language.is_implicit());
    }

    #[test]
    fn test_eure_code_inline_with_language() {
        // Test @code("lang", "content") - inline code with explicit language
        let doc = eure!({
            query = @code("sql", "SELECT * FROM users")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let query_id = root.as_map().unwrap().get(&"query".into()).unwrap();
        let query = doc.node(query_id);
        let text = query.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "SELECT * FROM users");
        assert_eq!(text.language.as_str(), Some("sql"));
    }

    #[test]
    fn test_eure_block_implicit() {
        // Test @block("content") - block code with implicit language
        let doc = eure!({
            script = @block("fn main() {}")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let script_id = root.as_map().unwrap().get(&"script".into()).unwrap();
        let script = doc.node(script_id);
        let text = script.as_primitive().unwrap().as_text().unwrap();
        // block adds trailing newline
        assert_eq!(text.as_str(), "fn main() {}\n");
        assert!(text.language.is_implicit());
    }

    #[test]
    fn test_eure_block_with_language() {
        // Test @block("lang", "content") - block code with explicit language
        let doc = eure!({
            code = @block("rust", "fn main() {\n    println!(\"Hello\");\n}")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let code_id = root.as_map().unwrap().get(&"code".into()).unwrap();
        let code = doc.node(code_id);
        let text = code.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.language.as_str(), Some("rust"));
        assert!(text.as_str().contains("println!"));
    }

    #[test]
    fn test_eure_code_at_root() {
        // Test @code at root level
        let doc = eure!({
            = @code("hello")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let text = root.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "hello");
    }

    #[test]
    fn test_eure_code_with_language_at_root() {
        // Test @code("lang", "content") at root level
        let doc = eure!({
            = @code("sql", "SELECT 1")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let text = root.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "SELECT 1");
        assert_eq!(text.language.as_str(), Some("sql"));
    }

    #[test]
    fn test_eure_block_at_root() {
        // Test @block("content") at root level
        let doc = eure!({
            = @block("fn main() {}")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let text = root.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "fn main() {}\n");
        assert!(text.language.is_implicit());
    }

    #[test]
    fn test_eure_block_with_language_at_root() {
        // Test @block("lang", "content") at root level
        let doc = eure!({
            = @block("rust", "fn main() {}")
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let text = root.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "fn main() {}\n");
        assert_eq!(text.language.as_str(), Some("rust"));
    }

    // ========================================================================
    // Tests for edge cases and missing coverage
    // ========================================================================

    #[test]
    fn test_eure_array_specific_index() {
        // Test array with specific index: items[0] = value, items[1] = value
        let doc = eure!({
            items[0] = "first"
            items[1] = "second"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Check values
        let first_id = array.get(0).unwrap();
        let first = doc.node(first_id);
        assert_eq!(first.as_primitive().unwrap().as_str(), Some("first"));

        let second_id = array.get(1).unwrap();
        let second = doc.node(second_id);
        assert_eq!(second.as_primitive().unwrap().as_str(), Some("second"));
    }

    #[test]
    fn test_eure_array_index_with_child() {
        // Test array index with child path: items[0].name = value
        let doc = eure!({
            items[0].name = "first"
            items[0].value = 1
            items[1].name = "second"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Check first element
        let first_id = array.get(0).unwrap();
        let first = doc.node(first_id);
        let name_id = first.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("first"));
    }

    #[test]
    fn test_eure_nested_empty_blocks() {
        // Test nested empty blocks: a { b { c {} } }
        let doc = eure!({
            a {
                b {
                    c {}
                }
            }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);

        let a_id = root.as_map().unwrap().get(&"a".into()).unwrap();
        let a = doc.node(a_id);

        let b_id = a.as_map().unwrap().get(&"b".into()).unwrap();
        let b = doc.node(b_id);

        let c_id = b.as_map().unwrap().get(&"c".into()).unwrap();
        let c = doc.node(c_id);

        // c should be an empty map
        let map = c.as_map().expect("c should be an empty map");
        assert!(map.is_empty());
    }

    #[test]
    fn test_eure_multiple_extensions() {
        // Test multiple extensions on same node
        let doc = eure!({
            field.%variant = @code("text")
            field.%"variant-repr" = "internal"
            field.%schema = "custom"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);

        // Check all extensions exist
        assert!(field.get_extension(&"variant".parse().unwrap()).is_some());
        assert!(
            field
                .get_extension(&"variant-repr".parse().unwrap())
                .is_some()
        );
        assert!(field.get_extension(&"schema".parse().unwrap()).is_some());
    }

    #[test]
    fn test_eure_extension_on_array_element() {
        // Test extension on array element using indexed access
        // Note: items[] creates a new element each time, so we use items[0], items[1] etc.
        let doc = eure!({
            items[0].%variant = @code("text")
            items[0].value = "first"
            items[1].%variant = @code("number")
            items[1].value = 42
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Check first element has extension and value
        let first_id = array.get(0).unwrap();
        let first = doc.node(first_id);
        let variant_id = first.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);
        assert_eq!(
            variant.as_primitive().unwrap().as_text().unwrap().as_str(),
            "text"
        );
        let value_id = first.as_map().unwrap().get(&"value".into()).unwrap();
        let value = doc.node(value_id);
        assert_eq!(value.as_primitive().unwrap().as_str(), Some("first"));

        // Check second element
        let second_id = array.get(1).unwrap();
        let second = doc.node(second_id);
        let variant_id = second.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);
        assert_eq!(
            variant.as_primitive().unwrap().as_text().unwrap().as_str(),
            "number"
        );
    }

    #[test]
    fn test_eure_deep_nesting() {
        // Test deeply nested paths (5+ levels)
        let doc = eure!({ a.b.c.d.e.f = "deep" });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);

        let a_id = root.as_map().unwrap().get(&"a".into()).unwrap();
        let a = doc.node(a_id);
        let b_id = a.as_map().unwrap().get(&"b".into()).unwrap();
        let b = doc.node(b_id);
        let c_id = b.as_map().unwrap().get(&"c".into()).unwrap();
        let c = doc.node(c_id);
        let d_id = c.as_map().unwrap().get(&"d".into()).unwrap();
        let d = doc.node(d_id);
        let e_id = d.as_map().unwrap().get(&"e".into()).unwrap();
        let e = doc.node(e_id);
        let f_id = e.as_map().unwrap().get(&"f".into()).unwrap();
        let f = doc.node(f_id);

        assert_eq!(f.as_primitive().unwrap().as_str(), Some("deep"));
    }

    #[test]
    fn test_eure_empty_array_literal() {
        // Test empty array literal: items = []
        let doc = eure!({ items = [] });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert!(array.is_empty());
    }

    #[test]
    fn test_eure_empty_tuple_literal() {
        // Test empty tuple literal: point = ()
        let doc = eure!({ point = () });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let point_id = root.as_map().unwrap().get(&"point".into()).unwrap();
        let point = doc.node(point_id);
        let tuple = point.as_tuple().unwrap();
        assert!(tuple.is_empty());
    }

    #[test]
    fn test_eure_empty_map_literal() {
        // Test empty map literal: data = {}
        // Note: This uses block syntax which creates empty map
        let doc = eure!({ data {} });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let data_id = root.as_map().unwrap().get(&"data".into()).unwrap();
        let data = doc.node(data_id);
        let map = data.as_map().unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn test_eure_mixed_null_and_values() {
        // Test mixing null with other values
        let doc = eure!({
            name = "Alice"
            age = null
            active = true
            score = null
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map = root.as_map().unwrap();
        assert_eq!(map.len(), 4);

        let age_id = map.get(&"age".into()).unwrap();
        let age = doc.node(age_id);
        assert!(matches!(
            age.as_primitive(),
            Some(crate::value::PrimitiveValue::Null)
        ));
    }

    // ========================================================================
    // Tests for section syntax
    // ========================================================================

    #[test]
    fn test_eure_section_basic() {
        // Test basic section syntax: @section followed by bindings
        let doc = eure!({
            @user
            name = "Alice"
            age = 30
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        let name_id = user.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("Alice"));
    }

    #[test]
    fn test_eure_section_multiple() {
        // Test multiple sections
        let doc = eure!({
            @user
            name = "Alice"

            @settings
            theme = "dark"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);

        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        assert!(user.as_map().unwrap().get(&"name".into()).is_some());

        let settings_id = root.as_map().unwrap().get(&"settings".into()).unwrap();
        let settings = doc.node(settings_id);
        assert!(settings.as_map().unwrap().get(&"theme".into()).is_some());
    }

    #[test]
    fn test_eure_section_dotted_path() {
        // Test section with dotted path
        let doc = eure!({
            @user.profile
            name = "Alice"
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        let profile_id = user.as_map().unwrap().get(&"profile".into()).unwrap();
        let profile = doc.node(profile_id);
        assert!(profile.as_map().unwrap().get(&"name".into()).is_some());
    }

    #[test]
    fn test_eure_section_with_block() {
        // Test section with block syntax: @path { ... }
        let doc = eure!({
            @user {
                name = "Alice"
                age = 30
            }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        assert!(user.as_map().unwrap().get(&"name".into()).is_some());
        assert!(user.as_map().unwrap().get(&"age".into()).is_some());
    }

    #[test]
    fn test_eure_section_block_with_nested() {
        // Test section block with nested structure
        let doc = eure!({
            @config {
                server {
                    host = "localhost"
                    port = 8080
                }
                debug = true
            }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let config_id = root.as_map().unwrap().get(&"config".into()).unwrap();
        let config = doc.node(config_id);
        assert!(config.as_map().unwrap().get(&"server".into()).is_some());
        assert!(config.as_map().unwrap().get(&"debug".into()).is_some());

        let server_id = config.as_map().unwrap().get(&"server".into()).unwrap();
        let server = doc.node(server_id);
        assert!(server.as_map().unwrap().get(&"host".into()).is_some());
        assert!(server.as_map().unwrap().get(&"port".into()).is_some());
    }

    #[test]
    fn test_eure_section_block_multiple() {
        // Test multiple sections with block syntax
        let doc = eure!({
            @user {
                name = "Alice"
            }
            @settings {
                theme = "dark"
            }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        assert!(root.as_map().unwrap().get(&"user".into()).is_some());
        assert!(root.as_map().unwrap().get(&"settings".into()).is_some());
    }

    #[test]
    fn test_eure_section_block_dotted_path() {
        // Test section block with dotted path: @a.b { ... }
        let doc = eure!({
            @server.config {
                host = "localhost"
                port = 8080
            }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let server_id = root.as_map().unwrap().get(&"server".into()).unwrap();
        let server = doc.node(server_id);
        let config_id = server.as_map().unwrap().get(&"config".into()).unwrap();
        let config = doc.node(config_id);
        assert!(config.as_map().unwrap().get(&"host".into()).is_some());
        assert!(config.as_map().unwrap().get(&"port".into()).is_some());
    }

    #[test]
    fn test_eure_section_block_empty() {
        // Test section with empty block
        let doc = eure!({
            @empty {}
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let empty_id = root.as_map().unwrap().get(&"empty".into()).unwrap();
        let empty = doc.node(empty_id);
        // Empty block creates empty map
        assert!(empty.as_map().unwrap().is_empty());
    }

    #[test]
    fn test_eure_section_mixed_styles() {
        // Test mixing section styles: some with blocks, some without
        let doc = eure!({
            @user {
                name = "Alice"
            }

            @settings
            theme = "dark"
            debug = true

            @logging {
                level = "info"
            }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);

        // user should have name
        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        assert!(user.as_map().unwrap().get(&"name".into()).is_some());

        // settings should have theme and debug
        let settings_id = root.as_map().unwrap().get(&"settings".into()).unwrap();
        let settings = doc.node(settings_id);
        assert!(settings.as_map().unwrap().get(&"theme".into()).is_some());
        assert!(settings.as_map().unwrap().get(&"debug".into()).is_some());

        // logging should have level
        let logging_id = root.as_map().unwrap().get(&"logging".into()).unwrap();
        let logging = doc.node(logging_id);
        assert!(logging.as_map().unwrap().get(&"level".into()).is_some());
    }

    #[test]
    fn test_eure_section_in_section_block() {
        // Test section in section block
        let doc = eure!({
            @ settings {
                theme = "dark"
                @ logging
                level = "info"
            }
        });

        let mut settings = doc
            .parse_context(doc.get_root_id())
            .parse_record()
            .expect("Failed to parse record")
            .field_record("settings")
            .expect("Failed to parse settings");
        let theme = settings
            .parse_field::<&str>("theme")
            .expect("Failed to parse theme");
        let logging = settings
            .field_record("logging")
            .expect("Failed to parse logging")
            .parse_field::<&str>("level")
            .expect("Failed to parse level");
        settings
            .deny_unknown_fields()
            .expect("Failed to deny unknown fields");
        assert_eq!(theme, "dark");
        assert_eq!(logging, "info");
    }

    #[test]
    fn test_eure_variable_text() {
        // Test using a variable for Text value
        use crate::text::Text;
        let code = Text::inline_implicit("fn main() {}");
        let doc = eure!({ snippet = code });

        let mut root = doc.parse_context(doc.get_root_id()).parse_record().unwrap();
        let snippet_ctx = root.field("snippet").unwrap();
        let snippet_node = doc.node(snippet_ctx.node_id());
        let text = snippet_node.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "fn main() {}");
    }

    #[test]
    fn test_eure_variable_in_array() {
        // Test using variables in array literals
        use alloc::vec::Vec;
        let first = "one";
        let second = "two";
        let third = "three";
        let doc = eure!({ items = [first, second, third] });

        let mut root = doc.parse_context(doc.get_root_id()).parse_record().unwrap();
        let items = root.parse_field::<Vec<&str>>("items").unwrap();
        assert_eq!(items, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_eure_variable_in_tuple() {
        // Test using variables in tuple literals
        let x = 1.5;
        let y = 2.5;
        let doc = eure!({ point = (x, y) });

        let mut root = doc.parse_context(doc.get_root_id()).parse_record().unwrap();
        let point = root.parse_field::<(f64, f64)>("point").unwrap();
        assert_eq!(point, (1.5, 2.5));
    }

    #[test]
    fn test_eure_variable_in_object_literal() {
        // Test using variables in object literal values
        let x_val = 10.0;
        let y_val = 20.0;
        let doc = eure!({
            coords = {
                "x" => x_val
                "y" => y_val
            }
        });

        let mut root = doc.parse_context(doc.get_root_id()).parse_record().unwrap();
        let mut coords = root.field_record("coords").unwrap();
        let x = coords.parse_field::<f64>("x").unwrap();
        let y = coords.parse_field::<f64>("y").unwrap();
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)] // Testing parse_field deserialization returns correct value
    fn test_eure_variable_mixed_with_literals() {
        // Test mixing variables and literals
        let username = "bob";
        let is_active = true;
        let doc = eure!({
            user.name = username
            user.active = is_active
            user.role = "admin"
            user.level = 5
        });

        let mut root = doc.parse_context(doc.get_root_id()).parse_record().unwrap();
        let mut user = root.field_record("user").unwrap();
        assert_eq!(user.parse_field::<&str>("name").unwrap(), "bob");
        assert_eq!(user.parse_field::<bool>("active").unwrap(), true);
        assert_eq!(user.parse_field::<&str>("role").unwrap(), "admin");
        assert_eq!(user.parse_field::<i32>("level").unwrap(), 5);
    }

    #[test]
    fn test_eure_variable_in_nested_array() {
        // Test using variables in nested array structures
        use alloc::vec::Vec;
        let tag1 = "rust";
        let tag2 = "macro";
        let doc = eure!({
            tags[] = tag1
            tags[] = tag2
        });

        let mut root = doc.parse_context(doc.get_root_id()).parse_record().unwrap();
        let tags = root.parse_field::<Vec<&str>>("tags").unwrap();
        assert_eq!(tags, vec!["rust", "macro"]);
    }

    #[test]
    fn test_eure_variable_at_root() {
        // Test using a variable for root value binding
        let value = 42;
        let doc = eure!({
            = value
        });

        let ctx = doc.parse_context(doc.get_root_id());
        let root_value = ctx.parse::<i32>().unwrap();
        assert_eq!(root_value, 42);
    }

    #[test]
    fn test_eure_variable_in_section() {
        // Test using variables in section syntax
        let theme_value = "dark";
        let lang_value = "en";
        let doc = eure!({
            @settings
            theme = theme_value
            language = lang_value
        });

        let mut root = doc.parse_context(doc.get_root_id()).parse_record().unwrap();
        let mut settings = root.field_record("settings").unwrap();
        assert_eq!(settings.parse_field::<&str>("theme").unwrap(), "dark");
        assert_eq!(settings.parse_field::<&str>("language").unwrap(), "en");
    }

    #[test]
    fn test_eure_variable_null_and_primitive() {
        // Test using PrimitiveValue::Null from a variable
        use crate::value::PrimitiveValue;
        let null_value = PrimitiveValue::Null;
        let doc = eure!({ optional = null_value });

        let mut root = doc.parse_context(doc.get_root_id()).parse_record().unwrap();
        let optional_ctx = root.field("optional").unwrap();
        let optional_node = doc.node(optional_ctx.node_id());
        assert!(matches!(
            optional_node.as_primitive().unwrap(),
            PrimitiveValue::Null
        ));
    }
}
