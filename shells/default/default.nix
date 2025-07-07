{
  pkgs,
  mkShell,
  lib,
  inputs,
  system,
  ...
}:
mkShell rec {
  packages = with pkgs; [
    mesa
    libGL.dev
    cmake
    pkg-config
    python3
    ninja
    fontconfig
    freetype

    rustc
    rustfmt
    cargo
    clippy
    rust-analyzer
    libinput
    libxkbcommon
    cairo
    hyprcursor
    inputs.hyprutils.packages.${system}.default
    inputs.aquamarine.packages.${system}.default
    libdrm.dev
    libgbm
    libunwind.dev
    libbacktrace
    pkgs.llvmPackages_16.libclang.lib
    pam
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath packages;
  LIBCLANG_PATH = "${pkgs.llvmPackages_16.libclang.lib}/lib";
}
