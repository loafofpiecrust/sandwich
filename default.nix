with import <nixpkgs> {
  # crossSystem = (import <nixpkgs/lib>).systems.examples.armv7l-hf-multiplatform;
};
mkShell {
  buildInputs = [
    pkg-config
    alsaLib
  ];
}
