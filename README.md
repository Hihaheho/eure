# EURE

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Hihaheho/eure)

> [!WARNING]
> Under Construction!

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
- [ ] eure-schema: EURE Schema specification
- [ ] serde-eure: Serde support
- [ ] eure-dev: Making the landing page on <https://eure.dev>
- [ ] eure-fmt: Make the formatter
- [ ] eure-cli: command to convert EURE to other formats
- [ ] eure-check: EURE files validator
- [ ] eure-lint: Some lint rules
- [ ] eure-template: Templating extension for EURE files
- [ ] eure-editor-support: Editor support for EURE files
- [ ] eure-toml: TOML conversion support
- [ ] eure-json: JSON conversion support
- [ ] eure-yaml: YAML conversion support
- [ ] eure-value: Type-safe data-type of EURE data-model

## Credits

- [Parol](https://github.com/jsinger67/parol) for the parser generator
- [TOML](https://toml.io) for the document structure and its minimalisity
- [jq](https://jqlang.github.io/jq/) for the key syntax
- [Serde](https://serde.rs/) for the data model and attributes (especially about enum representation)
- [JSON Schema](https://json-schema.org) for the idea of describing schema in the same language as the data
- [Helm](https://helm.sh) for the idea of templating
- [YAML](https://yaml.org) for easy nesting and the `:` syntax
