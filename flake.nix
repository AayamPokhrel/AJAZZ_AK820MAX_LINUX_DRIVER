{
  description = "Complete native Linux userspace driver and RGB music visualizer for Ajazz AK820 Max";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in {
        packages = rec {
          ajazz-ak820-max = pkgs.callPackage ./nix/package.nix { };
          default = ajazz-ak820-max;
        };

        apps = rec {
          ak820ctl = flake-utils.lib.mkApp { drv = self.packages.${system}.default; name = "ak820ctl"; };
          ak820d = flake-utils.lib.mkApp { drv = self.packages.${system}.default; name = "ak820d"; };
          default = ak820ctl;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
          ];

          buildInputs = with pkgs; [
            alsa-lib
            systemd
            libudev-zero
          ];

          shellHook = ''
            echo "====================================================="
            echo " Ajazz AK820 Max Native Linux Development Environment "
            echo "====================================================="
            echo "Run 'cargo test' to run the test suite."
            echo "Run 'cargo run --bin ak820ctl -- status' to probe the keyboard."
            echo "Run 'cargo run --bin ak820ctl -- music' to start music sync."
          '';
        };
      }
    ) // {
      nixosModules = rec {
        ajazz-ak820-max = import ./nix/module.nix;
        default = ajazz-ak820-max;
      };
    };
}
