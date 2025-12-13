# Types

## Formatting

Eure recommends **2-space indentation** for nested structures.

```eure
@ config {
  server = {
    host = "localhost"
    port = 8080
  }
}
```

## Primitive Types

- `string`
- `typed string`
- `code`
- `integer`
- `float`
- `decimal`
- `boolean`
- `array`
- `object`
- `enum`
- `variant`
- `tuple`
- `unit`
- `null`

## Officially Provided Types

- `uri`
- `url`
- `email`
- `uuid`
- `datetime`
- `second-wise duration`
- `calendar-wise duration`

## String

Notation as value: `"value"`
Notation as type: `"string"`

## Typed String

Notation as value: `url"https://example.com"`
Notation as type: `url"string"`

## Code Block

Notation as value:

````eure
key = ```rust
fn main() {
    println!("Hello, world!");
}
```
````

- Newline character is **not inserted at the head** of the code unless you manually insert a blank line.
- Newline character is **always inserted at the tail** of the last line of the code.

## Inline Code

Notation as type: `rust"code"` or `"code"`

## Integer

Notation as value: `1`
Notation as type: `"integer"`

## Float

Notation as value: `1.1`
Notation as type: `"float"`

## Decimal

Notation as value: `1.1`
Notation as type: `"decimal"`

## Boolean

Notation as value: `= true`
Notation as type: `"= boolean"`

## Array

Notation as value: `= [1, 2, 3]`
Notation as type: `"array"`

## Object

Notation as value: `= { a = 1, b = 2}`
Notation as type: `"object"`

## Variant

Variant types (also known as tagged unions or sum types) allow a value to be one of several possible variants. Each variant is identified by a name and can contain associated data.

Notation as value:

```eure
@ result {
  $variant = .ok
  value = 42
}

@ error {
  $variant = .err
  message = "Something went wrong"
}
```

Notation as type: Defined using `$variants` in schema (see [schema-extensions.md](./schema-extensions.md#variants))

### Simple Variants

Use a string or path to specify the variant:

```eure
$variant = "success"
# or equivalently
$variant = .success
```

### Nested Variants

For nested variant structures (similar to Rust's `Result<Result<T, E>, E>` or `Option<Option<T>>`), use dot notation to specify the full variant path:

```eure
@ response {
  $variant = .ok.ok.err
  error_code = 404
}
```

This represents a three-level nested structure:

| Eure | Rust Equivalent |
|------|----------------|
| `$variant = .ok` | `Ok(value)` |
| `$variant = .ok.ok` | `Ok(Ok(value))` |
| `$variant = .ok.ok.err` | `Ok(Ok(Err(value)))` |
| `$variant = .err` | `Err(value)` |

Each segment in the path represents one level of variant selection:

```eure
# Two-level nesting
@ outer {
  $variant = .some.none
}
# Rust: Some(None)

# Three-level nesting
@ deep {
  $variant = .some.some.some
  value = "deeply nested"
}
# Rust: Some(Some(Some("deeply nested")))
```

The dot notation follows Eure's standard path syntax, making it consistent with other path-based features in the language.

## Unit

## Null

## Datetime
