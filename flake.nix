{
  description = "A Nix flake for APMnix";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      # Systems you want to support
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      
      # Helper function to generate attributes for each system
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      
      # Nixpkgs instantiated for each system
      nixpkgsFor = forAllSystems (system: import nixpkgs { inherit system; });
    in
    {
      # 1. Define your package
      packages = forAllSystems (system: {
        default = nixpkgsFor.${system}.stdenv.mkDerivation {
          pname = "apmnix";
          version = "0.1.0";
          src = ./.;

          # Add build dependencies here
          nativeBuildInputs = [ ]; 
          buildInputs = [ ];

          installPhase = ''
            mkdir -p $out/bin
            # Add commands to install your binaries or files
          '';
        };
      });

      # 2. Define a development shell (run with 'nix develop')
      devShells = forAllSystems (system: {
        default = nixpkgsFor.${system}.mkShell {
          buildInputs = with nixpkgsFor.${system}; [
            git
            # Add other tools you need for development
          ];
        };
      });
    };
}
