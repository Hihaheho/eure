# (Architecture) Decision Memos

## Abandon cool, but dangerous `$ref-*` extensions -- 2024-08-27

- A EURE document should be self-contained as a sane data serialization format. It's always surprising that desirialization requires network access or file system access.
- It will be big security holes, especially with third-party implementations.

## No implicit tuple key like array's -- 2025-05-10

- Tuple's heart is what index a value is stored, not as like array only cares the order of values.
- For tuple always explicit index is required like `@ tuple.#0` or `tuple.#5 = 1`.

## Newline character inserted on the tail not on the head in code block -- 2025-05-10

- A text file tends to need the last line to have a newline character.
- For rare case you want omit the newline charactor, we may create a `$no-final-newline = true` extension.
