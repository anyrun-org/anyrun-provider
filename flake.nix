{
  description = "Anyrun search results provider";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, self }:
    let
      fsys =
        f:
        nixpkgs.lib.attrsets.genAttrs [
          "x86_64-linux"
          "armv7l-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ] (s: f s);
    in
    {
      packages = fsys (
        a:
        let
          pkgs = nixpkgs.legacyPackages.${a};
          lib = pkgs.lib;

          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          pname = cargoToml.package.name;
          version = cargoToml.package.version;

          pkg = pkgs.rustPlatform.buildRustPackage {
            inherit pname version;

            src = builtins.path {
              path = lib.sources.cleanSource self;
              name = "${pname}-${version}";
            };

            strictDeps = true;

            cargoLock = {
              lockFile = ./Cargo.lock;
              # Temporary while packages aren't yet stabilized
              allowBuiltinFetchGit = true;
            };

            nativeBuildInputs = with pkgs; [
              rustc
              cargo
            ];

            CARGO_BUILD_INCREMENTAL = "false";
            RUST_BACKTRACE = "full";

            meta = {
              description = "A program for loading and interacting with Anyrun plugins";
              homepage = "https://github.com/anyrun-org/anyrun-provider";
              mainProgram = "anyrun-provider";
              license = [ lib.licenses.gpl3 ];
            };
          };

        in
        {
          anyrun-provider = pkg;
          default = pkg;
        }
      );
      devShells = fsys (
        a:
        let
          pkgs = nixpkgs.legacyPackages.${a};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustc
              cargo
              clippy
              rustfmt
            ];
          };
        }
      );

    };
}
