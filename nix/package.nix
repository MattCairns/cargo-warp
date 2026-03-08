{
  lib,
  rustPlatform,
  rsync,
  makeWrapper,
}:
rustPlatform.buildRustPackage {
  pname = "cargo-warp";
  version = "0.1.11";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../tests
    ];
  };

  cargoHash = "sha256-Lp0Oljkdit0hHbGF4bu9OLK2WagTGiCa5z8jq+GV5d4=";

  nativeBuildInputs = [makeWrapper];

  postInstall = ''
    wrapProgram $out/bin/cargo-warp \
      --prefix PATH : ${lib.makeBinPath [rsync]}
  '';

  meta = {
    description = "Cargo subcommand to build and copy your project binary to a remote host";
    homepage = "https://github.com/MattCairns/cargo-warp";
    license = lib.licenses.mit;
    mainProgram = "cargo-warp";
  };
}
