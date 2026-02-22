{
  lib,
  rustPlatform,
  pkg-config,
  lockFile,
  ...
}:
let
  inherit (builtins) fromTOML readFile;

  cargoToml = fromTOML (readFile ../../anyrun-provider/Cargo.toml);
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  fs = lib.fileset;
  s = ../..;
in
rustPlatform.buildRustPackage {
  inherit pname version;

  src = fs.toSource {
    root = s;
    fileset = fs.unions [
      (s + /anyrun-provider)
      (s + /Cargo.toml)
      (s + /Cargo.lock)
    ];
  };

  strictDeps = true;

  cargoLock = {
    inherit lockFile;
    allowBuiltinFetchGit = true;
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [ ];

  cargoBuildFlags = [ "-p ${pname}" ];

  doCheck = false;

  CARGO_BUILD_INCREMENTAL = "false";
  RUST_BACKTRACE = "full";

  meta = {
    description = "Anyrun search provider";
    homepage = "https://github.com/anyrun-org/anyrun";
    license = [ lib.licenses.gpl3 ];
    mainProgram = "anyrun-provider";
  };
}
