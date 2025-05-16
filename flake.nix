{
  description = "rangefs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs@{ nixpkgs, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem = { pkgs, ... }: {
        devShells = {
          default = pkgs.mkShell {
            packages = with pkgs; [
		          pkg-config
		          fuse3
            ];
          };
        };

        apps = {
          release = {
            type = "app";
            program = pkgs.writeShellScriptBin "release" ''
              set -e

              ver=''${1:-$(git cliff --bumped-version)}
              ver=''${ver##v}

              sed -i "/name = \"rangefs\"/{n;s/version = \".*\"/version = \"$ver\"/g}" Cargo.toml Cargo.lock
              git cliff --bump -o CHANGELOG.md
              git add -A
              git commit -m "chore(release): v$ver"
              git tag "v$ver"
            '';
          };
        };
      };
    };
}
