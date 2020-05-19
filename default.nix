with import <nixpkgs> {
  # crossSystem = (import <nixpkgs/lib>).systems.examples.armv7l-hf-multiplatform;
};
mkShell rec {
  buildInputs = [
    pkg-config
    alsaLib
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    libGL
  ];
  LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
}
