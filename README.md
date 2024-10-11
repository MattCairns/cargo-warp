# cargo-warp

## Overview

The tool allows users to build Rust projects and transfer built files to a specified destination using rsync. It supports both standard Cargo builds and cross-compilation using the `cross` tool.

## Usage
```bash
Usage: cargo warp [OPTIONS] <DESTINATION>

Arguments:
  <DESTINATION>

Options:
  -c, --cross
  -p, --package <PACKAGE>
  -t, --target <TARGET>
  -h, --help               Print help
```

## Examples
Building and sending the project to a remote PC called `mypc` using the `aarch64-unknown-linux-gnu` target:
```bash
cargo warp mypc:~/. --cross -t aarch64-unknown-linux-gnu -p foo 
```

Building and sending the project to a remote PC called `mypc`:
```bash
cargo warp mypc:~/. -p foo 
```

