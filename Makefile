default_prefix = /usr/local
prefix ?= $(default_prefix)
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
libdir = $(exec_prefix)/lib
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)

CLI_SOURCES = $(shell find cli -type f -wholename '*src/*.rs') cli/Cargo.toml
GTK_SOURCES = $(shell find gtk -type f -wholename '*src/*.rs') gtk/Cargo.toml
SHR_SOURCES = $(shell find src -type f -wholename '*src/*.rs') Cargo.toml Cargo.lock

RELEASE = debug
DEBUG ?= 0
ifeq (0,$(DEBUG))
	ARGS = --release
	RELEASE = release
endif

VENDORED ?= 0
ifeq (1,$(VENDORED))
    ARGS += --frozen
endif

TARGET = target/$(RELEASE)

.PHONY: all clean distclean install uninstall update

BIN=popsicle
APPID=com.system76.Popsicle
APPDATA=$(APPID).appdata.xml
DESKTOP=$(APPID).desktop
GTK_BIN=popsicle-gtk
PKEXEC_BIN=popsicle-pkexec
POLICY=com.system76.pkexec.popsicle.policy
ICONS=\
	512x512/apps/$(APPID).png \
	16x16@2x/apps/$(APPID).png \
	32x32@2x/apps/$(APPID).png \
	32x32/apps/$(APPID).png \
	48x48@2x/apps/$(APPID).png \
	24x24/apps/$(APPID).png \
	48x48/apps/$(APPID).png \
	16x16/apps/$(APPID).png \
	24x24@2x/apps/$(APPID).png \
	512x512@2x/apps/$(APPID).png

all: cli gtk

cli: $(TARGET)/$(BIN) $(TARGET)/$(BIN).1.gz $(CLI_SOURCES) $(SHR_SOURCES)

gtk: $(TARGET)/$(GTK_BIN) $(GTK_SOURCES) $(SHR_SOURCES)

clean:
	cargo clean

distclean: clean
	rm -rf .cargo vendor vendor.tar.xz

vendor:
	mkdir -p .cargo
	cargo vendor | head -n -1 > .cargo/config
	echo 'directory = "vendor"' >> .cargo/config
	tar pcfJ vendor.tar.xz vendor
	rm -rf vendor

install-cli: cli
	install -Dm 0755 "$(TARGET)/$(BIN)" "$(DESTDIR)$(bindir)/$(BIN)"
	install -Dm 0755 "$(TARGET)/$(BIN).1.gz" "$(DESTDIR)$(datadir)/man/man1/$(BIN).1.gz"

install-gtk: gtk
	install -Dm 0755 "$(TARGET)/$(GTK_BIN)" "$(DESTDIR)$(bindir)/$(GTK_BIN)"
	install -Dm 0755 "gtk/assets/popsicle-pkexec" "$(DESTDIR)$(bindir)/$(PKEXEC_BIN)"
	install -Dm 0644 "gtk/assets/$(DESKTOP)" "$(DESTDIR)$(datadir)/applications/$(DESKTOP)"
	install -Dm 0644 "gtk/assets/$(POLICY)" "$(DESTDIR)$(datadir)/polkit-1/actions/$(POLICY)"
	install -Dm 0644 "gtk/assets/$(APPDATA)" "$(DESTDIR)$(datadir)/metainfo/$(APPDATA)"
	for icon in $(ICONS); do \
		install -D -m 0644 "gtk/assets/icons/$$icon" "$(DESTDIR)$(datadir)/icons/hicolor/$$icon"; \
	done

	# Fix paths in assets
	sed -i -e 's#$(default_prefix)#$(prefix)#g' $(DESTDIR)$(datadir)/applications/$(DESKTOP) \
		&& sed -i -e 's#$(default_prefix)#$(prefix)#g' $(DESTDIR)$(datadir)/polkit-1/actions/$(POLICY) \
		&& sed -i -e 's#$(default_prefix)#$(prefix)#g' $(DESTDIR)$(bindir)/$(PKEXEC_BIN)

install: all install-cli install-gtk

uninstall-cli:
	rm -f "$(DESTDIR)$(bindir)/$(BIN)"
	rm -f "$(DESTDIR)$(datadir)/man/man1/$(BIN).1.gz"

uninstall-gtk:
	rm -f "$(DESTDIR)$(bindir)/$(GTK_BIN)"
	rm -f "$(DESTDIR)$(bindir)/$(PKEXEC_BIN)"
	rm -f "$(DESTDIR)$(datadir)/applications/$(DESKTOP)"
	rm -f "$(DESTDIR)$(datadir)/polkit-1/actions/$(POLICY)"
	for icon in $(ICONS); do \
		rm -f "$(DESTDIR)$(datadir)/icons/hicolor/$$icon"; \
	done

uninstall: uninstall-cli uninstall-gtk

update:
	cargo update

extract:
ifeq ($(VENDORED),1)
	tar pxf vendor.tar.xz
endif

$(TARGET)/$(BIN): extract
	cargo build --manifest-path cli/Cargo.toml $(ARGS)

$(TARGET)/$(GTK_BIN): extract
	cargo build --manifest-path gtk/Cargo.toml $(ARGS)

$(TARGET)/$(BIN).1.gz: $(TARGET)/$(BIN)
	help2man --no-info $< | gzip -c > $@.partial
	mv $@.partial $@
