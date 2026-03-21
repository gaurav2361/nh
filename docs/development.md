# Development Guide

This guide covers building, testing, and developing `nh`.

## Build Requirements

- Rust (latest stable)
- Nix (with Flakes enabled)
- `just` (optional, for task automation)
- `direnv` (optional, for automatic shell environment)

## Building with Nix (Recommended)

Building with Nix ensures all dependencies (including Darwin SDKs and libiconv) are correctly provided.

### Build the `nh` binary
```bash
nix build .#nh
```
The resulting binary will be at `./result/bin/nh`.

### Run without installing
```bash
nix run .#nh -- clean darwin --dry
```

## Developing with Cargo

If you want to use standard cargo commands, it is recommended to enter a Nix shell first to get all native dependencies.

### Entering the development shell
```bash
nix shell .#default
# OR if using direnv
direnv allow
```

### Common Development Tasks
The project uses `just` for common tasks:

- **Check code quality**: `just check`
- **Run tests**: `just test`
- **Apply automatic fixes**: `just fix`
- **Run the project**: `cargo run -p nh -- <args>`

## Darwin Specifics

On macOS, `nh` requires specific SDK frameworks. These are automatically handled by the `flake.nix` and `shell.nix`.

### Build issues on Darwin
If you encounter errors related to `apple_sdk_11_0` or missing `libiconv`:
1. Ensure you are using the updated `flake.nix` (using `rust-overlay`).
2. Run `nix flake update` to refresh dependencies.
3. Use `nix build .#nh --impure` if environment variables need to be inherited.

### Permission Denied errors during cleanup
The Darwin cleanup implementation in `nh-clean` is designed to gracefully skip files it doesn't have permission to access (like those in `.direnv` or system protected areas) without crashing the build.
