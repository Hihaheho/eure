# Eure for Zed

The extension starts `eurels` when Zed first needs the Eure language server
(for example, when opening a `.eure` file). Installing the extension alone does
not download the server.

On every launch, the extension first looks for `eurels` on the worktree's `PATH`
using Zed's `worktree.which`. If found (for example, after `cargo install`), that
binary is used without checking its version or accessing GitHub. Ensure its
installation directory, such as `~/.cargo/bin`, is on the `PATH` visible to Zed.
If the local server fails to start, the failure is reported rather than silently
switching to a downloaded server.

`src/lib.rs` pins `EURE_VERSION` to `0.2.0`, the first planned binary release.
When no local `eurels` is found, the extension uses the cached pinned version or
downloads the exact asset from `Hihaheho/eure` release `v0.2.0` for the host OS
and CPU.
macOS, Linux (GNU), and Windows (MSVC) each support x86_64 and ARM64.
Unix assets are `.tar.gz`; Windows assets are `.zip`. Each contains a directory
named `eure-v<VERSION>-<TARGET>` with `eure` and `eurels` (`.exe` on Windows).

Completed installations are cached under Zed's extension working directory in
`eure-ls-<VERSION>-<TARGET>/`. Cached launches make no GitHub API or download
requests. Downloads are extracted into a staging directory and promoted only
after the server file is checked and Unix executable permissions are set.
Installation failures, including blocked download capabilities or missing
release assets, are reported through Zed's language server installation status.
An interrupted download is retried on the next launch.

## Development

Use Zed's **Install Dev Extension** command and select `editors/zed`.
This directory is an independent Cargo workspace targeting `wasm32-wasip2`.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --target wasm32-wasip2 -- -D warnings
cargo test --locked
cargo build --locked --release --target wasm32-wasip2
```

Before distributing the extension, publish all six assets for the pinned tag
using `.github/workflows/release-binaries.yml`. Bump `EURE_VERSION` only after
the corresponding assets are available; bump `extension.toml` and `Cargo.toml`
versions when publishing an extension update. When no local `eurels` is found,
updating the pin causes the next LSP launch to install that version while
keeping older caches intact.
