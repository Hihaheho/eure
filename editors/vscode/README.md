# Eure for Visual Studio Code

Official Visual Studio Code extension for [Eure](https://eure.dev) files (`.eure`).

## Features

### Syntax Highlighting

Color-codes Eure syntax elements:

- Keywords (`true`, `false`, `null`)
- Numbers (integers, floats, `Inf`, `NaN`)
- Strings
- Comments (`//` and `/* */`)
- Property names
- Section markers (`@`)
- Extension namespaces (`$variant`, `$eure`, etc.)

### Embedded Code Block Highlighting

Syntax highlighting is applied to code blocks based on their language tag.

```
code = ```rust
fn main() {
    println!("Hello!");
}
```
```

Supported languages: Rust, JSON, YAML, TOML, JavaScript, TypeScript, Python, HTML, CSS, SQL, Markdown, Shell

### Error Diagnostics

Parse errors are detected in real-time and displayed in the editor.

### Editing Support

- **Bracket completion**: Typing `{`, `[`, `(`, `"`, `` ` ``, or ` ``` ` automatically inserts the closing bracket
- **Bracket matching**: Matching brackets are highlighted at cursor position
- **Comment shortcuts**: Toggle line comments with `Ctrl+/` (`Cmd+/` on Mac), block comments with `Shift+Alt+A`

## Platforms

- VS Code Desktop
- VS Code Web (vscode.dev)

On the web, a WebAssembly-based Language Server is automatically used.

## Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `eure.useWasm` | Use WASM-based Language Server | `true` |
| `eure.path` | Path to native eurels binary (searches PATH if empty) | `""` |

## About Eure

Eure is a minimalist data format designed for configuration and data exchange. It maintains JSON compatibility while achieving TOML-like simplicity.

Learn more at [eure.dev](https://eure.dev).

## License

MIT OR Apache-2.0
