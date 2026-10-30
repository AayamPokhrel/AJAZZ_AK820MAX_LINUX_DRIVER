{ lib
, rustPlatform
, pkg-config
, makeWrapper
, alsa-lib
, libpulseaudio
, systemd
, wayland
, libxkbcommon
, libGL
, vulkan-loader
, libx11
, libxcursor
, libxrandr
, libxi
}:

rustPlatform.buildRustPackage {
  pname = "ajazz-ak820-max";
  version = "1.1.1";

  src = lib.cleanSource ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];

  buildInputs = [
    alsa-lib
    libpulseaudio
    systemd
    wayland
    libxkbcommon
    libGL
    vulkan-loader
    libx11
    libxcursor
    libxrandr
    libxi
  ];

  postInstall = ''
    # Install udev rules for all AK820 Max variants (wired & wireless)
    install -Dm644 -t $out/lib/udev/rules.d/ nix/70-ajazz-ak820.rules

    # Install systemd user service for background daemon
    install -Dm644 -t $out/lib/systemd/user/ nix/ak820d.service

    # Install Freedesktop .desktop entry
    install -Dm644 assets/ajazz-ak820-max.desktop $out/share/applications/ajazz-ak820-max.desktop

    # Install scalable and raster icons
    install -Dm644 assets/icons/ajazz-ak820-max.svg $out/share/icons/hicolor/scalable/apps/ajazz-ak820-max.svg
    for size in 48x48 64x64 128x128 256x256 512x512; do
      install -Dm644 assets/icons/hicolor/$size/apps/ajazz-ak820-max.png $out/share/icons/hicolor/$size/apps/ajazz-ak820-max.png
    done

    # Wrap GUI and daemon binaries with runtime graphical & audio libraries
    for bin in ak820-gui ak820d ak820ctl; do
      wrapProgram $out/bin/$bin \
        --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [
          wayland
          libxkbcommon
          libGL
          vulkan-loader
          libx11
          libxcursor
          libxrandr
          libxi
          alsa-lib
          libpulseaudio
        ]}
    done
  '';

  meta = with lib; {
    description = "Complete native Linux userspace driver, GUI studio, and RGB music visualizer for Ajazz AK820 Max";
    homepage = "https://github.com/AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "ak820-gui";
  };
}
