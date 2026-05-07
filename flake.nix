{
  description = "APMnix - Effortless system integration and tooling";

  inputs = {
    # Using unstable for the latest packages and features
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      # Define the architectures you want to support
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      
      # Helper function to generate attributes for all systems
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      
      # Instantiate nixpkgs for each system
      nixpkgsFor = forAllSystems (system: import nixpkgs { inherit system; });
    in
    {
      # ==========================================
      # 1. PACKAGE DEFINITION (nix build)
      # ==========================================
      packages = forAllSystems (system: {
        default = nixpkgsFor.${system}.stdenv.mkDerivation {
          pname = "apmnix";
          version = "0.1.0";
          src = ./.; # Pulls source from the current git directory

          # makeWrapper allows us to inject dependencies into the environment
          nativeBuildInputs = [ nixpkgsFor.${system}.makeWrapper ];
          
          # Add compile-time dependencies here
          buildInputs = [ ];

          installPhase = ''
            mkdir -p $out/bin
            
            # COPY YOUR ACTUAL SCRIPT/BINARY HERE. 
            # Replace 'apmnix-core' with the actual file in your repo.
            cp apmnix-core $out/bin/.apmnix-wrapped
            
            chmod +x $out/bin/.apmnix-wrapped

            # Create a wrapper so your tool always has access to necessary binaries
            # Add waydroid, bash, or anything else it needs to run in the makeBinPath list
            makeWrapper $out/bin/.apmnix-wrapped $out/bin/apmnix \
              --prefix PATH : ${nixpkgsFor.${system}.lib.makeBinPath [ nixpkgsFor.${system}.bash ]}
          '';
        };
      });

      # ==========================================
      # 2. APP ENTRY POINT (nix run)
      # ==========================================
      apps = forAllSystems (system: {
        default = {
          type = "app";
          # This tells 'nix run' exactly which executable to launch
          program = "${self.packages.${system}.default}/bin/apmnix";
        };
      });

      # ==========================================
      # 3. DEVELOPMENT SHELL (nix develop)
      # ==========================================
      devShells = forAllSystems (system: {
        default = nixpkgsFor.${system}.mkShell {
          # Tools loaded when someone works on the code
          buildInputs = with nixpkgsFor.${system}; [
            git
            bash
            # Add linters, compilers, or testing tools here
          ];
          
          shellHook = ''
            echo "🚀 Welcome to the APMnix development shell!"
          '';
        };
      });

      # ==========================================
      # 4. OVERLAY (For custom system integrations)
      # ==========================================
      overlays.default = final: prev: {
        # Allows users to add APMnix to their global 'pkgs' 
        apmnix = self.packages.${prev.system}.default;
      };

      # ==========================================
      # 5. NixOS MODULE (The "Effortless" setup)
      # ==========================================
      nixosModules.default = { config, lib, pkgs, ... }: with lib; {
        options.programs.apmnix = {
          enable = mkEnableOption "APMnix system integration";
        };

        config = mkIf config.programs.apmnix.enable {
          # Automatically install the package globally
          environment.systemPackages = [ self.packages.${pkgs.system}.default ];
          
          # You can enforce system-level dependencies here
          # For example, if APMnix requires Waydroid to be active:
          # virtualisation.waydroid.enable = mkDefault true;
        };
      };
    };
}
