{
  description = "Complete native Linux userspace driver, GUI studio, and RGB music visualizer for Ajazz AK820 Max";

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
          gui = flake-utils.lib.mkApp { drv = self.packages.${system}.default; name = "ak820-gui"; };
          ak820ctl = flake-utils.lib.mkApp { drv = self.packages.${system}.default; name = "ak820ctl"; };
          ak820d = flake-utils.lib.mkApp { drv = self.packages.${system}.default; name = "ak820d"; };
          default = gui;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
            makeWrapper
          ];

          buildInputs = with pkgs; [
            alsa-lib
            systemd
            libudev-zero
            wayland
            libxkbcommon
            libGL
            vulkan-loader
            libx11
            libxcursor
            libxrandr
            libxi
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
            wayland
            libxkbcommon
            libGL
            vulkan-loader
            libx11
            libxcursor
            libxrandr
            libxi
            alsa-lib
          ]);

          shellHook = ''
            echo "=========================================================="
            echo " Ajazz AK820 Max Native Linux Studio (v1.1.0) Environment "
            echo "=========================================================="
            echo "Run 'cargo run --bin ak820-gui' to launch the GUI."
            echo "Run 'cargo run --bin ak820ctl -- status' to probe the keyboard."
            echo "Run 'cargo test' to execute all unit and integration tests."
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
