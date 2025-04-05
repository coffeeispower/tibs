{
  pkgs,
  mkShell,
  lib,
  ...
}:
mkShell rec {
  packages = with pkgs; [
    mesa
    libGL
    cmake
    glfw
    pkg-config
    xorg.libX11
    xorg.libXrandr
    xorg.libXinerama
    xorg.libXcursor
    xorg.libXi
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
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath packages;
}
