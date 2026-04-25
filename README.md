# d — Interactively Manage Your Deno Versions

> Inspired by [tj/n](https://github.com/tj/n). Written in Rust.

`d` is a simple, no-fuss Deno version manager. Download, cache, and switch between Deno versions with a single command.

## Features

- Install any released Deno version by number, alias, or `canary`
- Interactive version picker (arrow keys)
- Version caching — no re-downloading
- Symlink-based activation (no subshells, no profile magic)
- List local and remote versions
- Run a specific version without activating it

## Supported Platforms

| OS      | Architectures          |
|---------|------------------------|
| Linux   | x86_64, aarch64        |
| macOS   | x86_64, aarch64        |
| Windows | x86_64, aarch64        |

## Installation

### From source (requires Rust)

```bash
cargo install --git https://github.com/THernandez03/d
```

### Manual

Download the latest binary from [Releases](https://github.com/THernandez03/d/releases) and place it in your `PATH`.

## Setup

Add `~/.d/bin` to your `PATH`:

```bash
# bash / zsh
export D_PREFIX="$HOME/.d"
export PATH="$D_PREFIX/bin:$PATH"
```

Optional environment variables:

| Variable      | Default         | Description                          |
|---------------|-----------------|--------------------------------------|
| `D_PREFIX`    | `~/.d`          | Root installation prefix             |
| `D_CACHE_DIR` | `~/.d/versions` | Where downloaded versions are stored |

## Usage

```bash
# Install a specific version
d 1.40.0
d install latest
d install canary

# Interactive picker from cached versions
d

# List cached versions
d ls

# List remote versions
d ls-remote

# Download without activating
d download 1.40.0

# Show path to a cached deno binary
d which v1.40.0

# Run a specific version
d run 1.40.0 -- --version

# Remove a cached version
d rm v1.40.0

# Remove all except active
d prune

# Diagnostics
d doctor

# Uninstall active deno symlink
d uninstall
```

## Version Aliases

| Alias    | Resolves to                        |
|----------|---------------------------------|
| `latest` | Latest stable release           |
| `stable` | Same as `latest`                |
| `lts`    | Same as `latest`                |
| `canary` | Deno's nightly canary build     |
| `next`   | Same as `canary`                |
| `1.46`   | Latest patch in 1.46.x          |
| `1`      | Latest release in major 1       |

## License

MIT
