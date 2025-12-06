# Eure

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Hihaheho/eure)

> [!NOTE]
> Preparing for Alpha Release!

## Name

Eure (not "EURE"), pronounced "your." Not an acronym, but think: Eureka, Extensible Universal Representation, "your" data, "your" way.

## Focus

- Minimalist
- Schema-frieldly
- Algebraic data model
- JSON data model compatible
- JSON schema compatible
- Rich Editor Experience
- Human friendly
- Dedicated templating language
- Programmatically editable

## Example

```eure
$eure.version: https://eure.dev/versions/v0.1.0
title: test
language: en
target_window: Main

@ actions[]
$variant: use-script
script-id: title

@ actions[]
$variant: sleep
seconds = 2.0

@ actions[]
$variant: set-base-background-image
image: images/backgrounds/blank.png

@ actions[]
$variant: set-scene
scene: scenes/room_a.scn.ron

@ actions[] {
  $variant: set-text

  @ pages[]
  text: Hi,

  @ pages[]
  speaker: Ryo
  text: I'm Ryo.
}
```

## TODO

- [ ] eure-parol: Complete the grammar and parser
- [ ] eure-ls: Syntax highlighting
- [ ] eure-schema: Eure Schema specification
- [ ] serde-eure: Serde support
- [ ] eure-dev: Making the landing page on <https://eure.dev>
- [ ] eure-fmt: Make the formatter
- [ ] eure-cli: command to convert Eure to other formats
- [ ] eure-check: Eure files validator
- [ ] eure-lint: Some lint rules
- [ ] eure-template: Templating extension for Eure files
- [ ] eure-editor-support: Editor support for Eure files
- [ ] eure-toml: TOML conversion support
- [ ] eure-json: JSON conversion support
- [ ] eure-yaml: YAML conversion support
- [ ] eure-document: Type-safe data-type of Eure data-model

## Credits

- [Parol](https://github.com/jsinger67/parol) for the parser generator
- [TOML](https://toml.io) for the flattened document structure and its minimalism
- [jq](https://jqlang.org) for the key syntax
- [Serde](https://serde.rs/) for the data model and attributes (especially about enum representation)
- [JSON Schema](https://json-schema.org) for the idea of describing schema in the same language as the data
- [Helm](https://helm.sh) for the idea of templating
- [YAML](https://yaml.org) for easy nesting and the `:` syntax

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
