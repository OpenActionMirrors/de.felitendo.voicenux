PLUGIN   = de.felitendo.voicenux.sdPlugin
VERSION := $(shell cat VERSION)

# Native OpenDeck. For the Flatpak use:
#   make install PLUGINS=~/.var/app/me.amankhanna.opendeck/config/opendeck/plugins
PLUGINS ?= $(HOME)/.config/opendeck/plugins

.PHONY: install uninstall link package check icons manifest clean

install:
	rm -rf "$(PLUGINS)/$(PLUGIN)"
	mkdir -p "$(PLUGINS)"
	cp -r $(PLUGIN) "$(PLUGINS)/$(PLUGIN)"
	@echo "Installed. Restart OpenDeck (or reload the plugin in its settings)."

uninstall:
	rm -rf "$(PLUGINS)/$(PLUGIN)"

# For development: OpenDeck runs the files from this folder. Needs developer mode in OpenDeck.
link:
	rm -rf "$(PLUGINS)/$(PLUGIN)"
	ln -s "$(CURDIR)/$(PLUGIN)" "$(PLUGINS)/$(PLUGIN)"

# dist/de.felitendo.voicenux.streamDeckPlugin, installable from the OpenDeck plugin page.
package:
	rm -rf dist && mkdir -p dist
	python3 -c 'import shutil; shutil.make_archive("dist/voicenux", "zip", ".", "$(PLUGIN)")'
	mv dist/voicenux.zip dist/de.felitendo.voicenux.streamDeckPlugin
	@echo "dist/de.felitendo.voicenux.streamDeckPlugin ($(VERSION))"

check:
	node --test 'test/*.test.mjs'

icons:
	node scripts/fit-glyphs.mjs
	node scripts/build-icons.mjs

manifest:
	node scripts/manifest.mjs

clean:
	rm -rf dist
