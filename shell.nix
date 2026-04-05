{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    gcc
    pkg-config
    gtk4
    libadwaita
    gobject-introspection
    openssl
    openssl.dev
  ];

  OPENSSL_DIR = "${pkgs.openssl.dev}";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
  OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";

  # Force software rendering in VM
  LIBGL_ALWAYS_SOFTWARE = "1";
  GSK_RENDERER = "cairo";
}
