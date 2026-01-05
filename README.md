<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Hihaheho/eure/main/assets/eure-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Hihaheho/eure/main/assets/eure-logo-light.svg">
  <img alt="Eure" src="https://raw.githubusercontent.com/Hihaheho/eure/main/assets/eure-logo.svg" height="100px" >
</picture>

> [!NOTE]
> Preparing for Alpha Release!

## Name

Eure (not "EURE"), pronounced "your." Not an acronym, but think: Eureka, Extensible Universal Representation, "your" data, "your" way.

## Focus

- Minimalist
- Schema-frieldly
- Algebraic data model
- JSON data model support
- JSON schema support
- Rich Editor Experience
- Human friendly
- Dedicated templating extension
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

- [x] eure-parol: Complete the grammar and parser
- [x] eure-ls: Syntax highlighting and schema verification
- [x] eure-schema: Eure Schema specification
- [ ] serde-eure: Serde support
- [x] eure-dev: Making the landing page on <https://eure.dev>
- [x] eure-fmt: Make the formatter
- [x] eure-cli: command to convert Eure to other formats
- [ ] eure-lint: Some lint rules
- [ ] eure-template: Templating extension for Eure files
- [x] eure-toml: TOML conversion support
- [x] eure-json: JSON conversion support
- [ ] eure-yaml: YAML conversion support
- [x] eure-document: Type-safe data-type of Eure data-model

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
