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
    pkgs.nodejs_22
    pkgs.playwright-driver.browsers
    pkgs.playwright-mcp
  ];

  env = {
    PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
    PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
    PLAYWRIGHT_MCP_BROWSER = "chromium";
  };

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

  # https://devenv.sh/processes/
  processes.let-timer-daemon.exec = "cargo run --bin let-timer-daemon";
}
