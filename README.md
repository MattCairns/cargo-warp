# cargo-warp

## Overview

The tool allows users to build Rust projects and transfer built files to a specified destination using rsync. It supports both standard Cargo builds and cross-compilation using the `cross` tool.

## Usage
```bash
Usage:
  cargo warp [OPTIONS] <DESTINATION>
  cargo warp config init [--force]

Arguments:
  <DESTINATION>

Options:
  -c, --cross[=<CROSS>]
  -p, --package <PACKAGE>
  -t, --target <TARGET>
  -r, --release[=<RELEASE>]
  -h, --help               Print help
```

`--cross` and `--release` can be passed as a flag (`--cross`) or with an explicit value (`--cross=false`).

## Configuration

Create a starter config file:

```bash
cargo warp config init
```

By default, cargo-warp reads `config.toml` from:

- `$XDG_CONFIG_HOME/cargo-warp/config.toml`
- fallback: `~/.config/cargo-warp/config.toml`

Example config:

```toml
[defaults]
release = true

[hosts.mypc]
cross = true
target = "aarch64-unknown-linux-gnu"

[hosts."*.lab.local"]
package = "deploy-agent"

[hosts."10.0.*"]
release = false
```

Host matching supports exact keys and wildcards (`*`, `?`).

When a destination is an SSH alias, cargo-warp attempts to resolve it via `ssh -G <host>` and also checks the resolved `HostName`.

Effective setting precedence:

1. CLI flags
2. Matched host config
3. `[defaults]`
4. Built-in defaults

Host match priority:

1. Exact destination host
2. Exact resolved SSH `HostName`
3. Wildcard destination host
4. Wildcard resolved SSH `HostName`

## Examples
Building and sending the project to a remote PC called `mypc` using the `aarch64-unknown-linux-gnu` target:
```bash
cargo warp mypc:~/. --cross -t aarch64-unknown-linux-gnu -p foo
```

Building and sending the project to a remote PC called `mypc`:
```bash
cargo warp mypc:~/. -p foo
```
