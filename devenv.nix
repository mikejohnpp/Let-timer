{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  # https://devenv.sh/packages/
  packages = [
    pkgs.git
    pkgs.librsvg
    pkgs.webkitgtk_4_1
  ];

  languages.javascript = {
    enable = true;
    lsp.enable = true;
    npm.enable = true;
    npm.install.enable = true;
  };

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    lsp.enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };

}
