# syntax=docker/dockerfile:1
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# System dependencies: GTK for Dioxus + fish shell
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        libwebkit2gtk-4.1-dev \
        libgtk-3-dev \
        libasound2-dev \
        libudev-dev \
        libayatana-appindicator3-dev \
        libxdo-dev \
        libglib2.0-dev \
        unzip \
        fish \
        vim \
        gnupg2

# Rust target for WASM
RUN rustup target add wasm32-unknown-unknown

# Install cargo-binstall for fast binary installation
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# CLI tools (pre-built binaries with version pinning)
RUN cargo binstall -y cargo-nextest dioxus-cli@0.7.2

# Fix permissions for vscode user (for named volume mounts)
RUN mkdir -p /usr/local/cargo/registry /usr/local/cargo/git \
    && chown -R vscode:vscode /usr/local/cargo \
    && mkdir -p /home/vscode/.claude \
    && chown -R vscode:vscode /home/vscode/.claude
