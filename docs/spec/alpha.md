# Eure Language Specification

**Version:** 0.1.0-alpha
**Status:** Draft

## Table of Contents

1. [Introduction](#1-introduction)
2. [Lexical Structure](#2-lexical-structure)
3. [Primitive Values](#3-primitive-values)
4. [Text Values](#4-text-values)
5. [Composite Values](#5-composite-values)
6. [Keys and Paths](#6-keys-and-paths)
7. [Document Structure](#7-document-structure)
8. [Document Interpretation](#8-document-interpretation)
9. [Extensions](#9-extensions)
10. [Data Model](#10-data-model)
11. [Formal Grammar](#11-formal-grammar)

---

## 1. Introduction

### 1.1 Purpose

This document specifies the Eure data format language. Eure is a minimalist, schema-friendly data format designed for configuration and data exchange. It combines JSON compatibility with TOML-like minimalism, featuring algebraic data models and rich text support.

### 1.2 Conformance

A conforming Eure implementation MUST:
- Parse all syntactically valid Eure documents as defined in this specification
- Reject documents that violate the syntax rules
- Preserve the semantic meaning of values during parsing and serialization

A conforming implementation MAY:
- Provide additional features beyond this specification
- Support format conversion to and from other data formats

### 1.3 Notation Conventions

This specification uses the following conventions:

- **EBNF notation** for grammar rules (see [Section 11](#11-formal-grammar))
- **Regular expressions** enclosed in `/slashes/` using Unicode-aware regex syntax
- `monospace` for literal syntax elements
- *italics* for terms being defined
- **bold** for emphasis

Examples are presented in fenced code blocks:

```eure
key = "value"
```

---

## 2. Lexical Structure

### 2.1 Character Encoding

Eure documents MUST be encoded in UTF-8. No byte order mark (BOM) is required or recommended.

### 2.2 Whitespace

*Whitespace* consists of all Unicode whitespace characters (characters with the Unicode `White_Space` property). Whitespace is generally insignificant except:
- Within quoted strings
- Within text bindings
- For indentation in code blocks

### 2.3 Line Terminators

*Line terminators* are:
- Line feed (LF, U+000A)
- Carriage return (CR, U+000D)
- Carriage return followed by line feed (CRLF)

### 2.4 Comments

Eure supports two forms of comments:

**Line comments** begin with `//` and extend to the end of the line:

```eure
key = "value"  // This is a comment
```

**Block comments** are enclosed in `/*` and `*/`:

```eure
/* This is a
  multi-line comment */
key = "value"
```

Block comments do not nest.

### 2.5 Identifiers

An *identifier* is a sequence of characters matching the pattern:

```
/[\p{XID_Start}_][\p{XID_Continue}-]*/
```

Where:
- `\p{XID_Start}` matches Unicode characters with the XID_Start property
- `\p{XID_Continue}` matches Unicode characters with the XID_Continue property
- Hyphens (`-`) are allowed after the first character

Examples of valid identifiers:
- `name`
- `_private`
- `camelCase`
- `kebab-case`
- `日本語`

### 2.6 Reserved Words

The following identifiers have special meaning and are reserved:
- `true` - boolean true value
- `false` - boolean false value
- `null` - null value

When these appear in key position, they are treated as string identifiers rather than their keyword meaning.

---

## 3. Primitive Values

### 3.1 Null

The *null* value represents the absence of a value. It is written as the keyword `null`:

```eure
value = null
```

### 3.2 Boolean

*Boolean* values represent truth values. They are written as the keywords `true` or `false`:

```eure
enabled = true
disabled = false
```

### 3.3 Integer

*Integer* values represent whole numbers with arbitrary precision. The syntax is:

```
/\d[\d_]*/
```

Underscores may be used as visual separators but have no semantic meaning:

```eure
count = 42
large = 1_000_000
binary_looking = 1010_1010
```

Implementations MUST support arbitrary precision integers. The exact range depends on the implementation, but SHOULD support at least 64-bit signed integers.

### 3.4 Float

*Float* values represent IEEE 754 floating-point numbers. Eure supports both 32-bit and 64-bit precision.

The syntax is:

```
/[-+]?(\d+\.\d*|\d+\.\d+)([eE][-+]?\d+)?|[-+]?\d+[eE][-+]?\d+|[-+]?[Ii]nf|[Nn]a[Nn]/
```

Examples:

```eure
pi = 3.14159
negative = -2.5
scientific = 6.022e23
positive_inf = Inf
negative_inf = -Inf
not_a_number = NaN
```

Special values:
- `Inf` or `inf` - positive infinity
- `-Inf` or `-inf` - negative infinity
- `NaN` or `nan` - not a number

---

## 4. Text Values

Eure provides a unified *text* type that represents both strings and code. Text values have an associated language classification.

### 4.1 Language Classification

Text values are classified into three categories:

1. **Plaintext**: Explicitly plain text, produced by quoted string syntax. Indicates the content is data, not code.

2. **Implicit**: No language specified, produced by backtick syntax without a language tag. The language can be inferred from schema context.

3. **Tagged**: Explicit language identifier, produced by backtick syntax with a language tag. Specifies exactly what language the content is.

### 4.2 Quoted Strings

Quoted strings produce *plaintext* values. They are enclosed in double quotes:

```eure
message = "Hello, World!"
```

#### Escape Sequences

The following escape sequences are recognized within quoted strings:

| Sequence | Meaning |
|----------|---------|
| `\\` | Backslash |
| `\"` | Double quote |
| `\'` | Single quote |
| `\n` | Line feed (LF) |
| `\r` | Carriage return (CR) |
| `\t` | Horizontal tab |
| `\0` | Null character |
| `\u{XXXX}` | Unicode code point (1-6 hex digits) |

Examples:

```eure
escaped = "line1\nline2"
unicode = "Hello \u{4e16}\u{754c}"  // Hello 世界
```

#### String Continuation

Long strings can be split across multiple lines using the backslash continuation:

```eure
long = "This is a very long string " \
  "that spans multiple lines"
```

The strings are concatenated without any separator.

### 4.3 Text Bindings

*Text bindings* provide unquoted single-line text using the colon syntax:

```eure
title: Hello World
description: This is unquoted text
```

Text bindings:
- Begin after `:` and optional whitespace
- End at the line terminator
- Have leading and trailing whitespace trimmed
- Produce *plaintext* values
- Support escape sequences

### 4.4 Inline Code

Inline code is enclosed in backticks and produces *implicit* or *tagged* text:

**Single backtick** (`` ` ``):

```eure
command = `ls -la`
```

**Double backtick** (``` `` ```):

```eure
template = ``Hello, {{name}}!``
```

Double backticks allow single backticks within the content.

**With language tag**:

```eure
sql_query = sql`SELECT * FROM users`
regex = regex`\d{3}-\d{4}`
```

The language tag must immediately precede the opening backtick(s) with no whitespace.

### 4.5 Code Blocks

Code blocks use 3 to 6 backticks and support multi-line content:

````eure
code = ```
line 1
line 2
```
````

**With language tag**:

````eure
script = ```python
def hello():
  print("Hello!")
```
````

#### Indentation Handling

Code blocks automatically strip common indentation based on the closing delimiter's position:

````eure
@ config {
  script = ```python
    def main():
      pass
    ```
}
````

The indentation of the closing ``` determines how much whitespace to strip from each line.

#### Nesting Code Blocks

Use more backticks to nest code blocks:

`````eure
doc = ````markdown
Here is some code:
```python
print("hello")
```
````
`````

---

## 5. Composite Values

### 5.1 Arrays

*Arrays* are ordered sequences of values enclosed in square brackets:

```eure
numbers = [1, 2, 3]
mixed = [1, "two", true]
nested = [[1, 2], [3, 4]]
empty = []
```

Arrays may contain values of any type, including other arrays. Trailing commas are permitted.

### 5.2 Tuples

*Tuples* are fixed-length sequences enclosed in parentheses:

```eure
point = (10, 20)
rgb = (255, 128, 0)
nested = ((1, 2), (3, 4))
empty = ()
```

Tuples differ from arrays in that they represent a fixed structure where each position has semantic meaning. Trailing commas are permitted.

### 5.3 Maps

*Maps* (also called objects) are collections of key-value pairs enclosed in braces.

**Map literal syntax** uses `=>` to associate keys with values:

```eure
lookup = {
  "key one" => 1
  "key two" => 2
}
```

```eure
person = {
  "name" => "Alice"
  "age" => 30
}
```

A map can also have an optional *value binding* using `=`:

```eure
data = {
  = "default value"
  "key" => "value"
}
```

The value binding (using `=`) provides the map's "own value" and is distinct from key-value entries (using `=>`). This is useful for nodes that need both a direct value and associated properties.

**Note**: The `=` binding syntax is used in document structure (bindings and sections), not inside map literals. Inside map literals, use `=>` for key-value pairs.

### 5.4 Holes

*Holes* mark positions in a document where a value has not yet been provided:

```eure
template = !
named_hole = !placeholder
```

A hole consists of `!` optionally followed by a label (an identifier).

**Important**: Holes are NOT valid final values. A document containing any holes is considered *incomplete*. Holes exist to support:
- Document editing (marking positions to fill in)
- Templates (placeholders to be substituted)
- Partial validation (checking structure before all values are known)

A complete, valid document MUST NOT contain any holes.

---

## 6. Keys and Paths

### 6.1 Valid Key Types

Map keys are restricted to types that support deterministic equality and hashing:

1. **String keys**: Identifiers or quoted strings
2. **Integer keys**: Arbitrary precision integers
3. **Tuple keys**: Tuples containing only valid key types

Invalid key types: floats, booleans, null, arrays, maps, holes.

### 6.2 Path Components

Paths navigate through document structure using dot notation:

**Identifier**: Regular field access
```eure
user.name = "Alice"
```

**Extension**: Extension namespace (see [Section 9](#9-extensions))
```eure
$eure.version = "1.0"
```

**Quoted string key**: Keys with special characters
```eure
"key with spaces".value = 1
```

**Integer key**: Numeric keys
```eure
0.value = "first"
```

**Tuple key**: Composite keys
```eure
(1, 2).label = "point"
```

**Tuple index**: Positional access (0-255)
```eure
point.#0 = 10
point.#1 = 20
```

**Array index**: Specific index or append
```eure
items[0] = "first"
items[1] = "second"
items[] = "appended"  // Appends to array
```

### 6.3 Keywords in Key Position

When `true`, `false`, or `null` appear in key position, they are treated as string identifiers:

```eure
true.value = 1      // Key is the string "true"
false.value = 2     // Key is the string "false"
null.value = 3      // Key is the string "null"
```

---

## 7. Document Structure

### 7.1 Top-Level Structure

An Eure document consists of:
1. An optional value binding
2. Zero or more bindings
3. Zero or more sections

```eure
key1 = "value1"
key2 = "value2"

@ section1
field = "data"
```

**Value binding constraint**: If a value binding is present, only extension bindings (keys starting with `$`) may follow—not regular key bindings. A node cannot simultaneously be a primitive value and a map.

```eure
// Valid: value binding with extension only
= "root value"
$metadata = "info"

// Invalid: value binding followed by regular key
= "root value"
key = "value"    // ERROR: node cannot be both string and map
```

### 7.2 Bindings

A *binding* associates a path with a value:

```eure
simple = "value"
nested.path = "deep value"
array[0] = "first element"
```

### 7.3 Sections

*Sections* provide a way to group related bindings under a common path prefix:

```eure
@ server
host = "localhost"
port = 8080

@ database
host = "db.example.com"
port = 5432
```

Sections can be nested using dot notation:

```eure
@ server.http
port = 80

@ server.https
port = 443
```

### 7.4 Block Sections

Sections can use block syntax `{ }` to create a **sub-document**:

```eure
@ server {
  host = "localhost"
  port = 8080

  @ tls {
    enabled = true
    cert = "/path/to/cert"
  }
}
```

Block sections are powerful because they create a complete sub-document context. Unlike regular sections (which only allow bindings), block sections support:
- **Value bindings**: The section can have its own direct value
- **Nested sections**: Full `@ path` section syntax
- **Complete document structure**: All features available at the top level

```eure
@ item {
  = "item value"        // Value binding for this node
  $variant: tagged      // Extension binding (allowed with value binding)
}

@ item2 {
  $variant: map-type    // Extension binding
  meta = "data"         // Regular binding

  @ nested              // Nested section
  field = "value"
}
```

### 7.5 Array Sections

Sections can append to arrays:

```eure
@ users[]
name = "Alice"
role = "admin"

@ users[]
name = "Bob"
role = "user"
```

This creates an array with two objects.

---

## 8. Document Interpretation

This section defines the operational semantics of Eure documents using abstract interpretation actions.

### 8.1 Interpretation State

Document interpretation maintains:

1. **Document tree**: A tree of nodes, each either unbound (a *hole*) or bound to a value
2. **Cursor**: The current position within the tree, starting at the root
3. **Scope stack**: A stack of saved cursor positions

### 8.2 Interpretation Actions

The following primitive actions define document interpretation:

| Action | Description |
|--------|-------------|
| `begin_scope()` | Save the current cursor position onto the scope stack |
| `end_scope()` | Restore the cursor to the most recently saved position (pop from stack) |
| `navigate(segment)` | Move cursor to a child node identified by segment; create the node if absent |
| `assert_unbound()` | Verify the current position is a hole; error if already bound |
| `bind(value)` | Assign a value to the current position |

**Constraints:**

- `bind(value)` MUST only succeed when the current position is unbound
- `end_scope()` MUST be called in LIFO order (most recent scope first)
- `navigate(segment)` implicitly creates intermediate nodes as needed

### 8.3 Navigate Segments

The `navigate(segment)` action accepts different segment types:

| Segment | Syntax | Description |
|---------|--------|-------------|
| Identifier | `navigate("key")` | Navigate to a string key (from identifier or quoted string) |
| Extension | `navigate($ext)` | Navigate to an extension namespace |
| Integer key | `navigate(42)` | Navigate to a numeric key |
| Tuple key | `navigate((1, "a"))` | Navigate to a composite tuple key |
| Tuple index | `navigate(#0)` | Navigate to a tuple element by position (0-255) |
| Array append | `navigate([])` | Append a new element to an array |
| Array index | `navigate([0])` | Navigate to a specific array index |

### 8.4 Interpretation Examples

#### Simple Binding

```eure
key = 1
```

```
begin_scope()
  navigate("key") assert_unbound() bind(1)
end_scope()
```

#### Nested Path Binding

```eure
key1.key2 = 1
```

```
begin_scope()
  navigate("key1") navigate("key2") assert_unbound() bind(1)
end_scope()
```

#### Map Literal

```eure
x = {
  "a" => 1
}
```

```
begin_scope()
  navigate("x") assert_unbound()
  // Map construction begins at current position
  begin_scope()
    navigate("a") assert_unbound() bind(1)
  end_scope()
end_scope()
```

#### Section

```eure
@ server
host = "localhost"
port = 8080
```

```
begin_scope()
  navigate("server") assert_unbound()
  begin_scope()
    navigate("host") assert_unbound() bind("localhost")
  end_scope()
  begin_scope()
    navigate("port") assert_unbound() bind(8080)
  end_scope()
end_scope()
```

#### Array Append

```eure
@ items[]
value = 1

@ items[]
value = 2
```

```
// First array element
begin_scope()
  navigate("items") navigate([]) assert_unbound()  // [] appends new element
  begin_scope()
    navigate("value") assert_unbound() bind(1)
  end_scope()
end_scope()

// Second array element
begin_scope()
  navigate("items") navigate([]) assert_unbound()  // [] appends another element
  begin_scope()
    navigate("value") assert_unbound() bind(2)
  end_scope()
end_scope()
```

#### Block Section

```eure
@ config {
  = "default"
  $tag: meta
}
```

```
begin_scope()
  navigate("config") assert_unbound()
  bind("default")  // Value binding
  begin_scope()
    navigate($tag) assert_unbound() bind("meta")  // Extension binding
  end_scope()
end_scope()
```

#### Extension Binding

```eure
$variant: success
field = 42
```

```
begin_scope()
  navigate($variant) assert_unbound() bind("success")
end_scope()
begin_scope()
  navigate("field") assert_unbound() bind(42)
end_scope()
```

#### Text Binding

```eure
title: Hello World
```

```
begin_scope()
  navigate("title") assert_unbound() bind("Hello World")
end_scope()
```

#### Section Binding (Inline Block)

```eure
config = {
  name = "app"
  version = "1.0"
}
```

```
begin_scope()
  navigate("config") assert_unbound()  // SectionBinding creates sub-document context
  begin_scope()
    navigate("name") assert_unbound() bind("app")
  end_scope()
  begin_scope()
    navigate("version") assert_unbound() bind("1.0")
  end_scope()
end_scope()
```

#### Array Literal

```eure
numbers = [1, 2, 3]
```

```
begin_scope()
  navigate("numbers") assert_unbound()  // Array construction
  begin_scope()
    navigate([0]) assert_unbound() bind(1)
  end_scope()
  begin_scope()
    navigate([1]) assert_unbound() bind(2)
  end_scope()
  begin_scope()
    navigate([2]) assert_unbound() bind(3)
  end_scope()
end_scope()
```

#### Tuple Literal

```eure
point = (10, 20)
```

```
begin_scope()
  navigate("point") assert_unbound()  // Tuple construction
  begin_scope()
    navigate(#0) assert_unbound() bind(10)
  end_scope()
  begin_scope()
    navigate(#1) assert_unbound() bind(20)
  end_scope()
end_scope()
```

#### Special Key Types

```eure
"key with space" = 1
0 = "first"
(1, 2) = "point"
point.#0 = 10
items[0] = "specific"
```

```
// String key
begin_scope()
  navigate("key with space") assert_unbound() bind(1)
end_scope()
// Integer key
begin_scope()
  navigate(0) assert_unbound() bind("first")
end_scope()
// Tuple key
begin_scope()
  navigate((1, 2)) assert_unbound() bind("point")
end_scope()
// Tuple index
begin_scope()
  navigate("point") navigate(#0) assert_unbound() bind(10)
end_scope()
// Specific array index
begin_scope()
  navigate("items") navigate([0]) assert_unbound() bind("specific")
end_scope()
```

#### Hole

```eure
placeholder = !
named = !todo
```

```
begin_scope()
  navigate("placeholder") assert_unbound() bind(!)
end_scope()
begin_scope()
  navigate("named") assert_unbound() bind(!todo)
end_scope()
```

---

## 9. Extensions

### 9.1 Extension Syntax

*Extensions* provide metadata separate from data values. They use the `$` prefix:

```eure
$variant: success
field.$annotation: metadata
```

The `$` is a grammar token, not part of the identifier string.

**Important distinction**: `$ext` and `"$ext"` are completely different:
- `$ext` — Extension namespace access (metadata channel)
- `"$ext"` — A string key literally containing the characters "$ext"

```eure
$variant: ok           // Extension: sets variant metadata
"$variant" => "text"   // Map entry: key is the string "$variant"
```

### 9.2 Extension Bindings

Extensions can be bound like regular values:

```eure
$variant = "success"
$metadata {
  author = "system"
  timestamp = "2024-01-01"
}
```

### 9.3 Storage Model

Extensions are stored separately from data values. They do not appear in the serialized output unless explicitly handled by the implementation.

### 9.4 Language-Defined Extension

The following extension is defined by this specification:

#### `$variant`

Indicates variant selection in sum/union types. This is the only extension with language-level semantics.

```eure
@ result {
  $variant: success
  value = 42
}
```

For nested variant types, use a path string:

```eure
$variant: ok              // Single level
$variant: ok.ok.err       // Nested variants (three levels)
```

### 9.5 Application-Level Extensions

Other extensions like `$eure`, `$local`, `$license`, etc. are **application-level conventions**, not part of this language specification. Applications and tools may define their own extension semantics.

---

## 10. Data Model

### 10.1 JSON Compatibility

Eure's data model is a superset of JSON. The following JSON types map directly:

| JSON | Eure |
|------|------|
| `null` | `null` |
| `true`/`false` | `true`/`false` |
| number (integer) | integer |
| number (float) | float |
| string | text (plaintext) |
| array | array |
| object | map |

### 10.2 Extensions Beyond JSON

Eure extends the JSON data model with:

1. **Arbitrary precision integers**: No fixed size limit

2. **Tuples**: Fixed-length sequences distinct from arrays

3. **Holes**: Markers for incomplete documents (see [Section 5.4](#54-holes))

4. **Language-tagged text**: Text with associated language metadata

5. **Extension metadata**: Separate metadata channel

### 10.3 Type Coercion

When converting to formats with fewer types (like JSON):

- Tuples MAY be converted to arrays
- Integers MAY be converted to floating-point if they exceed the target range
- Language tags on text MAY be discarded
- Extensions MAY be discarded or stored in a separate structure

**Note**: Documents containing holes are incomplete and SHOULD NOT be converted to other formats. Implementations should report an error when attempting to serialize incomplete documents.

---

## 11. Formal Grammar

This section presents the complete grammar in EBNF notation.

### 11.1 Notation

```
rule = definition ;          // Rule definition
"literal"                    // Literal string
/regex/                      // Regular expression
[ optional ]                 // Optional element
{ repeated }                 // Zero or more repetitions
( group )                    // Grouping
a | b                        // Alternative
```

### 11.2 Document Structure

```ebnf
Document = [ ValueBinding ] { Binding } { Section } ;

Binding = Keys BindingRhs ;
BindingRhs = ValueBinding | SectionBinding | TextBinding ;
ValueBinding = "=" Value ;
SectionBinding = "{" Document "}" ;
TextBinding = ":" Text ;

Section = "@" Keys SectionBody ;
SectionBody = [ ValueBinding ] { Binding } | "{" Document "}" ;
```

### 11.3 Keys and Paths

```ebnf
Keys = Key { "." Key } ;
Key = KeyBase [ ArrayMarker ] ;
KeyBase = Identifier | ExtensionName | String | Integer | KeyTuple | TupleIndex ;
ArrayMarker = "[" [ Integer ] "]" ;
TupleIndex = "#" Integer ;
ExtensionName = "$" Identifier ;
KeyTuple = "(" [ KeyValue { "," KeyValue } [ "," ] ] ")" ;
KeyValue = Integer | Boolean | String | KeyTuple ;
```

### 11.4 Values

```ebnf
Value = Object | Array | Tuple | Float | Integer | Boolean | Null | Strings | Hole | CodeBlock | InlineCode ;

Object = "{" [ ValueBinding [ "," ] ] { Keys "=>" Value [ "," ] } "}" ;
Array = "[" [ Value { "," Value } [ "," ] ] "]" ;
Tuple = "(" [ Value { "," Value } [ "," ] ] ")" ;

Boolean = "true" | "false" ;
Null = "null" ;
Hole = "!" [ Identifier ] ;

Strings = String { "\\" String } ;
```

### 11.5 Lexical Elements

```ebnf
Identifier = /[\p{XID_Start}_][\p{XID_Continue}-]*/ ;
Integer = /\d[\d_]*/ ;
Float = /[-+]?(\d+\.\d*|\d+\.\d+)([eE][-+]?\d+)?|[-+]?\d+[eE][-+]?\d+|[-+]?[Ii]nf|[Nn]a[Nn]/ ;
String = /"([^"\\]|\\.)*"/ ;

InlineCode = InlineCode1 | InlineCode2 ;
InlineCode1 = /[a-zA-Z0-9_-]*`[^`\r\n]*`/ ;
InlineCode2 = /[a-zA-Z0-9_-]*``/ { /[^`]+/ | /`/ } "``" ;

CodeBlock = CodeBlock3 | CodeBlock4 | CodeBlock5 | CodeBlock6 ;
CodeBlock3 = /```[a-zA-Z0-9_-]*\s*\n/ { /[^`]+/ | /`{1,2}/ } "```" ;
CodeBlock4 = /````[a-zA-Z0-9_-]*\s*\n/ { /[^`]+/ | /`{1,3}/ } "````" ;
CodeBlock5 = /`````[a-zA-Z0-9_-]*\s*\n/ { /[^`]+/ | /`{1,4}/ } "`````" ;
CodeBlock6 = /``````[a-zA-Z0-9_-]*\s*\n/ { /[^`]+/ | /`{1,5}/ } "``````" ;

Text = /[^\r\n]*/ ;

LineComment = "//" /[^\r\n]*/ ;
BlockComment = "/*" { /[^*]/ | "*" /[^/]/ } "*/" ;
```

### 11.6 Lexical Contexts

The lexer operates in different contexts where whitespace, newlines, and comments are handled differently:

**Default context**: Normal tokenization with whitespace and comments ignored between tokens.

**Text binding context**: After `:` in a binding, the lexer captures all characters until the line terminator as raw text content. Whitespace and comments are NOT recognized in this context.

**Inline code context**: Inside backtick-delimited inline code, only the content and closing delimiter are recognized. Whitespace and comments are part of the content.

**Code block context**: Inside multi-line code blocks (3-6 backticks), only the content and the matching closing delimiter are recognized. All characters including newlines become part of the content.

Context transitions:
- `:` in binding position → text binding context (until line end)
- Opening backticks → inline code or code block context
- Closing delimiters → return to default context

---

## Appendix A: Examples

### A.1 Configuration File

```eure
@ server
host = "0.0.0.0"
port = 8080

@ database {
  host = "localhost"
  port = 5432
  name = "myapp"

  @ pool
  min = 5
  max = 20
}

@ logging
level = "info"
format = "json"
```

### A.2 Data with Variants

```eure
@ responses[]
$variant: success
data = { "id" => 1, "name" => "Alice" }

@ responses[]
$variant: error
code = 404
message = "Not found"
```

### A.3 Code Documentation

````eure
@ example
title: Fibonacci Function
description: A simple recursive implementation

code = ```python
def fib(n):
  if n <= 1:
    return n
  return fib(n-1) + fib(n-2)
```

@ tests[]
input = 10
expected = 55
````

---

## Appendix B: Change Log

### Version 0.1.0-alpha

- Initial draft specification
