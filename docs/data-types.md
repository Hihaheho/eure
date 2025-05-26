# Types

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
- `path`

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

## Unit

## Null

## Datetime

## Path

Same syntax as section path and binding path.

Notation as value: `path = .[0].a.b.c`
Notation as type: `"path"`
