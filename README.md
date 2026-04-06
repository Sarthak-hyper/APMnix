# APMNix 🦴
Package manager GUI for VimuktiOS (NixOS based)

## Features
- Search 129,000+ NixOS packages
- One click User install (home.nix)
- One click System install (configuration.nix)
- Try packages without installing (nix-shell)
- Remove packages

## Dependencies
- gtk4
- libadwaita
- pkg-config
- openssl
- gcc
- rustc/cargo

## Build
```bash
nix-shell
cargo build --release
```

## Run
```bash
LIBGL_ALWAYS_SOFTWARE=1 GSK_RENDERER=cairo cargo run
```
