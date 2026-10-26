Name:           ajazz-ak820-max
Version:        1.1.1
Release:        1%{?dist}
Summary:        Native Linux driver, GUI studio, and RGB music visualizer for Ajazz AK820 Max

License:        MIT or Apache-2.0
URL:            https://github.com/AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  pkgconfig
BuildRequires:  alsa-lib-devel
BuildRequires:  systemd-devel
BuildRequires:  libX11-devel
BuildRequires:  libXcursor-devel
BuildRequires:  libXrandr-devel
BuildRequires:  libXi-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  wayland-devel

Requires:       alsa-lib
Requires:       systemd-libs
Requires:       libX11
Requires:       libXcursor
Requires:       libXrandr
Requires:       libXi
Requires:       libxkbcommon
Requires:       wayland

%description
Complete native Linux userspace driver, GUI studio, and RGB music visualizer
for the Ajazz AK820 Max mechanical keyboard, supporting both BY-COMBO /
SinoWealth (3554:FA09 / 258A:010C) and Shinetek (1A2C:A036 / 1A2C:8FFF)
hardware controller revisions.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --workspace

%install
# Binaries
install -Dm755 target/release/ak820-gui %{buildroot}%{_bindir}/ak820-gui
install -Dm755 target/release/ak820ctl %{buildroot}%{_bindir}/ak820ctl
install -Dm755 target/release/ak820d %{buildroot}%{_bindir}/ak820d

# Udev rules
install -Dm644 nix/70-ajazz-ak820.rules %{buildroot}%{_udevrulesdir}/70-ajazz-ak820.rules

# Systemd user service
install -Dm644 nix/ak820d.service %{buildroot}%{_userunitdir}/ak820d.service

# Desktop entry
install -Dm644 assets/ajazz-ak820-max.desktop %{buildroot}%{_datadir}/applications/ajazz-ak820-max.desktop

# Icons
install -Dm644 assets/icons/ajazz-ak820-max.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/ajazz-ak820-max.svg
for size in 48x48 64x64 128x128 256x256 512x512; do
  install -Dm644 assets/icons/hicolor/$size/apps/ajazz-ak820-max.png \
    %{buildroot}%{_datadir}/icons/hicolor/$size/apps/ajazz-ak820-max.png
done

%post
udevadm control --reload-rules || :
udevadm trigger --subsystem-match=hidraw || :
update-desktop-database %{_datadir}/applications || :
gtk-update-icon-cache -q -t -f %{_datadir}/icons/hicolor || :

%postun
udevadm control --reload-rules || :
udevadm trigger --subsystem-match=hidraw || :
update-desktop-database %{_datadir}/applications || :
gtk-update-icon-cache -q -t -f %{_datadir}/icons/hicolor || :

%files
%license LICENSE
%doc README.md docs/
%{_bindir}/ak820-gui
%{_bindir}/ak820ctl
%{_bindir}/ak820d
%{_udevrulesdir}/70-ajazz-ak820.rules
%{_userunitdir}/ak820d.service
%{_datadir}/applications/ajazz-ak820-max.desktop
%{_datadir}/icons/hicolor/scalable/apps/ajazz-ak820-max.svg
%{_datadir}/icons/hicolor/*/apps/ajazz-ak820-max.png

%changelog
* Mon Oct 26 2026 Aayam Pokhrel <pokhrelaayam1010@gmail.com> - 1.1.1-1
- 1:1 vendor binary reverse-engineered hardware communication protocol (Report ID 6 Feature Report, 520 bytes).
- Resolved ENODEV (error 19), dynamic device re-probing, full hardware lighting, sleep timeout, and per-key custom matrix.

* Sun Oct 25 2026 Aayam Pokhrel <pokhrelaayam1010@gmail.com> - 1.1.0-1
- Version 1.1.0 release with native Linux GUI studio, desktop integration and updatable packaging.
