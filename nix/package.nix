{ lib
, rustPlatform
, pkg-config
, alsa-lib
, systemd
}:

rustPlatform.buildRustPackage {
  pname = "ajazz-ak820-max";
  version = "0.1.0";

  src = lib.cleanSource ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    alsa-lib
    systemd
  ];

  postInstall = ''
    # Install udev rules for all AK820 Max variants (wired & wireless)
    install -Dm644 -t $out/lib/udev/rules.d/ nix/70-ajazz-ak820.rules

    # Install systemd user service for background daemon
    install -Dm644 -t $out/lib/systemd/user/ nix/ak820d.service
  '';

  meta = with lib; {
    description = "Complete native Linux userspace driver and RGB music visualizer for Ajazz AK820 Max";
    homepage = "https://github.com/ajazz-linux/ajazz-ak820-max";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "ak820ctl";
  };
}
