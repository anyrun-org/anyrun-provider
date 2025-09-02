{
  description = "Anyrun search results provider";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    inputs:
    let
      fsys =
        f:
        inputs.nixpkgs.lib.attrsets.genAttrs [
          "x86_64-linux"
          "armv7l-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ] (s: f s);
    in
    {
      devShells = fsys (
        a:
        let
          pkgs = inputs.nixpkgs.legacyPackages.${a};
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
