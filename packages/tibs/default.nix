{
  inputs,
  pkgs,
  ...
}: let
  crane = inputs.crane.mkLib pkgs;
  skia-binaries = pkgs.fetchurl {
    url = "https://github.com/rust-skia/skia-binaries/releases/download/0.82.0/skia-binaries-7ca7f6be2332d3c9f3ad-x86_64-unknown-linux-gnu-gl.tar.gz";
    sha256 = "sha256-CVbBUf3JNebEYWJxlCDT+YKVthhX1M0kEhsgbsgi4+U=";
  };
in
  crane.buildPackage {
    src = crane.cleanCargoSource ./../..;
    
    # Add extra inputs here or any other derivation settings
    buildInputs = with pkgs; [
      mesa
      libGL
      freetype
      fontconfig
      glibc
      libinput
      libxkbcommon
    ];
    nativeBuildInputs = with pkgs; [
      cmake
      ninja
      pkg-config
      python3
      curl
    ];
    SKIA_BINARIES_URL = "file://${skia-binaries}";
  }
