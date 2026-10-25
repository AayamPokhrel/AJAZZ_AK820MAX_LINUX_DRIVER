#!/usr/bin/env bash
# ==============================================================================
# Ajazz AK820 Max — Native Linux Installation, Update & Uninstallation Engine
# ==============================================================================

set -euo pipefail

PREFIX="${PREFIX:-/usr/local}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ACTION="${1:-install}"

BOLD="\033[1m"
GREEN="\033[1;32m"
CYAN="\033[1;36m"
YELLOW="\033[1;33m"
RED="\033[1;31m"
RESET="\033[0m"

log_info() {
    echo -e "${CYAN}==>${RESET} ${BOLD}$*${RESET}"
}

log_success() {
    echo -e "${GREEN}==>${RESET} ${BOLD}$*${RESET}"
}

log_warn() {
    echo -e "${YELLOW}==>${RESET} ${BOLD}$*${RESET}"
}

log_error() {
    echo -e "${RED}==>${RESET} ${BOLD}$*${RESET}"
}

check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "This operation requires root privileges. Please run with sudo: sudo $0 $ACTION"
        exit 1
    fi
}

do_build() {
    log_info "Building Ajazz AK820 Max suite in release mode..."
    cd "$SCRIPT_DIR"
    if command -v cargo >/dev/null 2>&1; then
        cargo build --release --workspace
    elif command -v nix >/dev/null 2>&1; then
        nix develop --command cargo build --release --workspace
    else
        log_error "Neither cargo nor nix was found in PATH. Please install Rust or Nix."
        exit 1
    fi
}

do_install() {
    check_root

    # Ensure build binaries exist
    if [ ! -f "$SCRIPT_DIR/target/release/ak820-gui" ] || \
       [ ! -f "$SCRIPT_DIR/target/release/ak820ctl" ] || \
       [ ! -f "$SCRIPT_DIR/target/release/ak820d" ]; then
        do_build
    fi

    log_info "Installing binaries into ${PREFIX}/bin/..."
    install -d "${PREFIX}/bin"
    install -m 755 "$SCRIPT_DIR/target/release/ak820-gui" "${PREFIX}/bin/ak820-gui"
    install -m 755 "$SCRIPT_DIR/target/release/ak820ctl" "${PREFIX}/bin/ak820ctl"
    install -m 755 "$SCRIPT_DIR/target/release/ak820d" "${PREFIX}/bin/ak820d"

    log_info "Installing udev rules..."
    local UDEV_DIR="/etc/udev/rules.d"
    [ -d "$UDEV_DIR" ] || UDEV_DIR="/usr/lib/udev/rules.d"
    install -d "$UDEV_DIR"
    install -m 644 "$SCRIPT_DIR/nix/70-ajazz-ak820.rules" "$UDEV_DIR/70-ajazz-ak820.rules"

    log_info "Installing systemd user service..."
    local SYSTEMD_DIR="/usr/lib/systemd/user"
    [ -d "$SYSTEMD_DIR" ] || SYSTEMD_DIR="/etc/systemd/user"
    install -d "$SYSTEMD_DIR"
    install -m 644 "$SCRIPT_DIR/nix/ak820d.service" "$SYSTEMD_DIR/ak820d.service"

    log_info "Installing Freedesktop .desktop entry..."
    install -d "${PREFIX}/share/applications"
    install -m 644 "$SCRIPT_DIR/assets/ajazz-ak820-max.desktop" "${PREFIX}/share/applications/ajazz-ak820-max.desktop"

    log_info "Installing scalable & bitmap application icons..."
    install -d "${PREFIX}/share/icons/hicolor/scalable/apps"
    install -m 644 "$SCRIPT_DIR/assets/icons/ajazz-ak820-max.svg" \
        "${PREFIX}/share/icons/hicolor/scalable/apps/ajazz-ak820-max.svg"

    for size in 48x48 64x64 128x128 256x256 512x512; do
        install -d "${PREFIX}/share/icons/hicolor/$size/apps"
        if [ -f "$SCRIPT_DIR/assets/icons/hicolor/$size/apps/ajazz-ak820-max.png" ]; then
            install -m 644 "$SCRIPT_DIR/assets/icons/hicolor/$size/apps/ajazz-ak820-max.png" \
                "${PREFIX}/share/icons/hicolor/$size/apps/ajazz-ak820-max.png"
        fi
    done

    log_info "Reloading udev rules..."
    if command -v udevadm >/dev/null 2>&1; then
        udevadm control --reload-rules || true
        udevadm trigger --subsystem-match=hidraw || true
    fi

    log_info "Updating desktop application and icon caches..."
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "${PREFIX}/share/applications" 2>/dev/null || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -t -f "${PREFIX}/share/icons/hicolor" 2>/dev/null || true
    fi

    log_success "Installation of Ajazz AK820 Max Suite v1.1.0 completed successfully!"
    echo
    echo -e "  To launch the GUI:          ${BOLD}ak820-gui${RESET}"
    echo -e "  To query keyboard status:   ${BOLD}ak820ctl status${RESET}"
    echo -e "  To enable user daemon:      ${BOLD}systemctl --user enable --now ak820d.service${RESET}"
    echo
}

do_update() {
    log_info "Updating Ajazz AK820 Max suite from remote..."
    cd "$SCRIPT_DIR"

    if [ -d ".git" ]; then
        log_info "Pulling latest changes from git repository..."
        git pull --rebase origin main || log_warn "Git pull failed or local edits present; continuing build."
    fi

    log_info "Rebuilding updated binaries..."
    do_build

    log_info "Installing updated files..."
    do_install

    # Restart user daemon if running
    if [ -n "${SUDO_USER:-}" ]; then
        su - "$SUDO_USER" -c "systemctl --user restart ak820d.service 2>/dev/null" || true
    fi

    log_success "Update complete! You are now running the latest version."
}

do_uninstall() {
    check_root
    log_info "Removing Ajazz AK820 Max suite from ${PREFIX}..."

    rm -f "${PREFIX}/bin/ak820-gui"
    rm -f "${PREFIX}/bin/ak820ctl"
    rm -f "${PREFIX}/bin/ak820d"
    rm -f "/etc/udev/rules.d/70-ajazz-ak820.rules"
    rm -f "/usr/lib/udev/rules.d/70-ajazz-ak820.rules"
    rm -f "/usr/lib/systemd/user/ak820d.service"
    rm -f "/etc/systemd/user/ak820d.service"
    rm -f "${PREFIX}/share/applications/ajazz-ak820-max.desktop"
    rm -f "${PREFIX}/share/icons/hicolor/scalable/apps/ajazz-ak820-max.svg"
    rm -f "${PREFIX}/share/icons/hicolor/*/apps/ajazz-ak820-max.png"

    if command -v udevadm >/dev/null 2>&1; then
        udevadm control --reload-rules || true
        udevadm trigger --subsystem-match=hidraw || true
    fi

    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "${PREFIX}/share/applications" 2>/dev/null || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -t -f "${PREFIX}/share/icons/hicolor" 2>/dev/null || true
    fi

    log_success "Ajazz AK820 Max suite uninstalled cleanly."
}

case "$ACTION" in
    install)
        do_install
        ;;
    update)
        do_update
        ;;
    uninstall)
        do_uninstall
        ;;
    build)
        do_build
        ;;
    *)
        echo "Usage: $0 {install|update|uninstall|build}"
        exit 1
        ;;
esac
