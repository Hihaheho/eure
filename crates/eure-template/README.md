# eure-template

Templating extension for Eure files.

A structured and type-safe templating tool for eure.

**This crate is still under development and published for name reservation purpose.**

Part of the [Eure](https://eure.dev) project - a minimalist, schema-friendly format with an algebraic data model that's compatible with JSON.

## Design

A template is a normal eure file which using `$template` extensions.

```eure
name.$template.path = .name
name.$type: string

childs.$template.for {
  path = .childs
  map {
    name.$template.if = .childs[].active
    name.$template.path = .childs[].name
    name.$type: string
  }
}
```
