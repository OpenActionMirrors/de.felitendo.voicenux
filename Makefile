PLUGIN   = de.felitendo.voicenux.sdPlugin
VERSION := $(shell cat VERSION)

# Native OpenDeck. For the Flatpak use:
#   make install PLUGINS=~/.var/app/me.amankhanna.opendeck/config/opendeck/plugins
PLUGINS ?= $(HOME)/.config/opendeck/plugins

.PHONY: build install uninstall link package check icons manifest clean

# The plugin for this PC, into $(PLUGIN)/bin/<target>/.
build:
	cargo xtask build

install: build
	rm -rf "$(PLUGINS)/$(PLUGIN)"
	mkdir -p "$(PLUGINS)"
	cp -r $(PLUGIN) "$(PLUGINS)/$(PLUGIN)"
	@echo "Installed. Restart OpenDeck (or reload the plugin in its settings)."

uninstall:
	rm -rf "$(PLUGINS)/$(PLUGIN)"

# For development: OpenDeck runs the files from this folder. Needs developer mode in OpenDeck.
link: build
	rm -rf "$(PLUGINS)/$(PLUGIN)"
	ln -s "$(CURDIR)/$(PLUGIN)" "$(PLUGINS)/$(PLUGIN)"

# dist/de.felitendo.voicenux.streamDeckPlugin for x86_64 and aarch64, installable from the
# OpenDeck plugin page. Needs zig and cargo-zigbuild.
package:
	cargo xtask package

check:
	cargo test --workspace

icons:
	cargo xtask fit-glyphs
	cargo xtask icons

manifest:
	cargo xtask manifest

clean:
	rm -rf dist $(PLUGIN)/bin
	cargo clean
