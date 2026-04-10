# APMNix 🦴
Package manager GUI for VimuktiOS (NixOS based)

## Its in its initial states
Right now it is in its initial state with hell lotta errors we are working on it with our top priority

## What to do, to make it Run
-First fault is, it cannot run with flakes
-Second is you need to make .dotfiles in the home directory /home/(your username)/.dotfiles
After copying your configuration.nix and hardware-configuration.nix to the .dotfiles directory
Run this command for congiguration.nix and do the same with hardware-configuration.nix
```bash
sudo rm /etc/nixos/configuration.nix
sudo ln -s /home/(your username)/.dotfiles/configuration.nix /etc/nixos/configuration.nix
```
I know its not a standard practice but we will improve this and there would be no need to run this bad-practice command
(Until then Sowwy 😿😿😿😿)

## Features
- Search 129,000+ NixOS packages
- One click User install (home.nix)
- One click System install (configuration.nix)
- Try packages without installing (nix-shell)
- Remove packages

## Build
```bash
nix-shell
cargo build --release
```

## Run
```bash
LIBGL_ALWAYS_SOFTWARE=1 GSK_RENDERER=cairo cargo run
```
