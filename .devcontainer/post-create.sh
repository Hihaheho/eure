#!/bin/bash
set -e

# Install wrangler
npm install -g wrangler

# Fetch Rust dependencies
cargo fetch

# Install VS Code extension dependencies
(cd editors/vscode && pnpm install)

# Run local setup if exists
if [ -f .devcontainer/post-create-local.sh ]; then
    .devcontainer/post-create-local.sh
fi
