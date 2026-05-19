# d — Interactively Manage Your Deno Versions

`d` is a simple, no-fuss Deno version manager. Download, cache, and switch between Deno versions with a single command.

## Features

- Install any released Deno version by number, alias, or `canary`
- Interactive version picker (arrow keys)
- Version caching — no re-downloading
- Symlink-based activation (no subshells, no profile magic)
- List local and remote versions
- Run a specific version without activating it

## Supported Platforms

| OS      | Architectures   |
| ------- | --------------- |
| Linux   | x86_64, aarch64 |
| macOS   | x86_64, aarch64 |
| Windows | x86_64, aarch64 |

## Installation

### Pre-built binary (no Rust required)

```bash
curl -fsSL https://raw.githubusercontent.com/THernandez03/d/main/install.sh | sh
```

This installs `d` to `~/.local/bin/d`. You can override the destination:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/THernandez03/d/main/install.sh | sh
```

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
export PATH="$HOME/.local/bin:$PATH"  # for the d binary
export PATH="$D_PREFIX/bin:$PATH"     # for managed Deno binaries
```

Optional environment variables:

| Variable      | Default         | Description                          |
| ------------- | --------------- | ------------------------------------ |
| `D_PREFIX`    | `~/.d`          | Root installation prefix             |
| `D_CACHE_DIR` | `~/.d/versions` | Where downloaded versions are stored |

## Usage

```bash
# Install and activate a version
d 1.40.0
d latest
d canary

# Interactive picker from cached versions
d

# List cached versions
d ls

# List remote versions
d ls-remote

# Fetch into cache without activating
d fetch 1.40.0

# Show path to a cached deno binary
d which v1.40.0

# Run a specific version
d run 1.40.0 -- --version

# Remove a cached version (interactive picker if no version given)
d remove v1.40.0
d rm v1.40.0        # alias

# Remove all except active
d prune

# Show info
d info

# Update d itself
d update

# Fully remove d + all cached versions (requires confirmation)
d uninstall
```

## Version Aliases

| Alias    | Resolves to                 |
| -------- | --------------------------- |
| `latest` | Latest stable release       |
| `stable` | Same as `latest`            |
| `lts`    | Same as `latest`            |
| `canary` | Deno's nightly canary build |
| `next`   | Same as `canary`            |
| `1.46`   | Latest patch in 1.46.x      |
| `1`      | Latest release in major 1   |

## Related Projects

| Project                                | Runtime                 |
| -------------------------------------- | ----------------------- |
| [n](https://github.com/THernandez03/n) | Node.js version manager |
| [b](https://github.com/THernandez03/b) | Bun version manager     |
| [z](https://github.com/THernandez03/z) | Zig version manager     |
| [r](https://github.com/THernandez03/r) | Rust version manager    |

## License

MIT
