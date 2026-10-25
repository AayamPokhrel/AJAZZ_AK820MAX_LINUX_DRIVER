# ==============================================================================
# Makefile for Ajazz AK820 Max Suite (ak820-gui, ak820ctl, ak820d)
# ==============================================================================

PREFIX ?= /usr/local
CARGO ?= $(shell command -v cargo 2>/dev/null || echo "nix develop --command cargo")

.PHONY: all build test gui cli daemon install update uninstall clean package-arch

all: build

build:
	$(CARGO) build --release --workspace

test:
	$(CARGO) test --workspace

gui:
	$(CARGO) run --bin ak820-gui

cli:
	$(CARGO) run --bin ak820ctl -- status

daemon:
	$(CARGO) run --bin ak820d

install:
	@sudo PREFIX=$(PREFIX) ./scripts/install.sh install

update:
	@sudo PREFIX=$(PREFIX) ./scripts/install.sh update

uninstall:
	@sudo PREFIX=$(PREFIX) ./scripts/install.sh uninstall

clean:
	$(CARGO) clean

package-arch:
	cd packaging/arch && makepkg -si
