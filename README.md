# NPM Manager for Zed

A [Zed](https://zed.dev) extension that displays npm/pnpm package version status inline in `package.json` and `pnpm-workspace.yaml` files.

## Features

- **Inline version status** — see at a glance whether each dependency is up-to-date, outdated, invalid, or missing
- **Hover details** — hover over any dependency for full version info, installed version, and a link to npmjs.com
- **Diagnostics** — outdated and invalid packages are reported as editor diagnostics
- **Lock file awareness** — shows the actually installed version from your lock file (`package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`)

### Supported files

| File | Sections |
|------|----------|
| `package.json` | `dependencies`, `devDependencies`, `peerDependencies`, `optionalDependencies`, `bundledDependencies`, bun `catalog` / `catalogs` |
| `pnpm-workspace.yaml` | `catalog`, `catalogs` (named catalogs) |

### Inline indicators

| Icon | Meaning |
|------|---------|
| `✓` | Up to date |
| `↑ x.y.z` | Update available (shows latest version) |
| `⚠ x.y.z` | Version not found in registry |
| `✗` | Package not found |

## Installation

### From Zed Extensions (once published)

1. Open Zed
2. `Cmd+Shift+P` → **zed: Extensions**
3. Search for **NPM Manager** and install

### As a dev extension

```bash
git clone https://github.com/cth-latest/zed-npm-manager.git
```

1. Open Zed
2. `Cmd+Shift+P` → **zed: Install Dev Extension**
3. Select the cloned `zed-npm-manager` directory

The LSP binary will be downloaded automatically from GitHub releases on first use.

## Setup

### Enable inlay hints

Inlay hints must be enabled in Zed for the inline version indicators to appear. Add this to your Zed `settings.json` (`Cmd+,`):

```json
{
  "inlay_hints": {
    "enabled": true
  }
}
```

If you only want inlay hints for JSON/YAML files:

```json
{
  "languages": {
    "JSON": {
      "inlay_hints": {
        "enabled": true
      }
    },
    "YAML": {
      "inlay_hints": {
        "enabled": true
      }
    }
  }
}
```

### Configuration

The LSP server accepts configuration via Zed's `settings.json`:

```json
{
  "lsp": {
    "npm-manager-lsp": {
      "initialization_options": {
        "stable_only": false,
        "show_installed_version": true,
        "cache_ttl_seconds": 300
      }
    }
  }
}
```

| Option | Default | Description |
|--------|---------|-------------|
| `stable_only` | `false` | Ignore pre-release versions (alpha, beta, rc, canary, etc.) when determining the latest version |
| `show_installed_version` | `true` | Show the actually installed version from the lock file when it differs from the specified version |
| `cache_ttl_seconds` | `300` | How long (in seconds) to cache npm registry responses before re-fetching |

## Architecture

The extension consists of two components:

- **Zed extension** (`src/lib.rs`) — a thin WASM module that downloads and launches the LSP binary
- **LSP server** (`crates/npm-manager-lsp/`) — a standalone Rust binary using [tower-lsp](https://github.com/ebkalderon/tower-lsp) that handles parsing, registry fetching, and LSP responses

## Building from source

### Prerequisites

- Rust toolchain with `wasm32-wasip2` target
- `cargo`

```bash
# Add the WASM target (one time)
rustup target add wasm32-wasip2

# Build the LSP server
cargo build --release -p npm-manager-lsp

# Build the WASM extension
cargo build --release --target wasm32-wasip2 -p zed-npm-manager
```

## License

MIT
