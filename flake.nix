{
  description = "gpql dev shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAll = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAll (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            packages = with pkgs; [
              cargo
              rustc
              rustfmt
              clippy
              bun
              mosquitto
              rustfs
              cmake
            ];

            buildInputs = with pkgs; [
              glib
              gtk3
              libsoup_3
              webkitgtk_4_1
              gdk-pixbuf
              pango
              cairo
              atk
              openssl
              dbus
              libayatana-appindicator
            ];
          };
        }
      );
    };
}
