default_prefix = /usr/local
prefix ?= $(default_prefix)
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
libdir = $(exec_prefix)/lib
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)
DEBUG ?= 0
RELEASE = debug

ifeq (0,$(DEBUG))
	ARGS = --release
	RELEASE = release
endif

TARGET = target/$(RELEASE)

.PHONY: all clean distclean install uninstall update

BIN=popsicle
GTK_BIN=popsicle-gtk
PKEXEC_BIN=popsicle-pkexec
POLICY=com.system76.pkexec.popsicle.policy
ICONS=\
	512x512/apps/$(BIN).png \
	16x16@2x/apps/$(BIN).png \
	32x32@2x/apps/$(BIN).png \
	32x32/apps/$(BIN).png \
	48x48@2x/apps/$(BIN).png \
	24x24/apps/$(BIN).png \
	48x48/apps/$(BIN).png \
	16x16/apps/$(BIN).png \
	24x24@2x/apps/$(BIN).png \
	512x512@2x/apps/$(BIN).png

all: cli gtk

cli: $(TARGET)/$(BIN) $(TARGET)/$(BIN).1.gz

gtk: $(TARGET)/$(GTK_BIN)

clean:
	cargo clean

distclean: clean
	rm -rf .cargo vendor

install-cli: cli
	install -D -m 0755 "$(TARGET)/$(BIN)" "$(DESTDIR)$(bindir)/$(BIN)"
	install -D -m 0755 "$(TARGET)/$(BIN).1.gz" "$(DESTDIR)$(datadir)/man/man1/$(BIN).1.gz"

install-gtk: gtk
	install -D -m 0755 "$(TARGET)/$(GTK_BIN)" "$(DESTDIR)$(bindir)/$(GTK_BIN)"
	install -D -m 0755 "gtk/assets/popsicle-pkexec" "$(DESTDIR)$(bindir)/$(PKEXEC_BIN)"
	install -D -m 0644 "gtk/assets/popsicle.desktop" "$(DESTDIR)$(datadir)/applications/popsicle.desktop"
	install -D -m 0644 "gtk/assets/$(POLICY)" "$(DESTDIR)$(datadir)/polkit-1/actions/$(POLICY)"
	for icon in $(ICONS); do \
		install -D -m 0644 "gtk/assets/icons/$$icon" "$(DESTDIR)$(datadir)/icons/hicolor/$$icon"; \
	done

	# Fix paths in assets
	sed -i -e 's#$(default_prefix)#$(prefix)#g' $(DESTDIR)$(datadir)/applications/popsicle.desktop \
		&& sed -i -e 's#$(default_prefix)#$(prefix)#g' $(DESTDIR)$(datadir)/polkit-1/actions/$(POLICY) \
		&& sed -i -e 's#$(default_prefix)#$(prefix)#g' $(DESTDIR)$(bindir)/$(PKEXEC_BIN)

install: all install-cli install-gtk

uninstall-cli:
	rm -f "$(DESTDIR)$(bindir)/$(BIN)"
	rm -f "$(DESTDIR)$(datadir)/man/man1/$(BIN).1.gz"

uninstall-gtk:
	rm -f "$(DESTDIR)$(bindir)/$(GTK_BIN)"
	rm -f "$(DESTDIR)$(bindir)/$(PKEXEC_BIN)"
	rm -f "$(DESTDIR)$(datadir)/applications/popsicle.desktop"
	rm -f "$(DESTDIR)$(datadir)/polkit-1/actions/$(POLICY)"
	for icon in $(ICONS); do \
		rm -f "$(DESTDIR)$(datadir)/icons/hicolor/$$icon"; \
	done

uninstall: uninstall-cli uninstall-gtk

update:
	cargo update

.cargo/config: vendor_config
	mkdir -p .cargo
	cp $< $@

vendor: .cargo/config
	cargo vendor --explicit-version --locked
	touch vendor

$(TARGET)/$(BIN):
	echo $(TARGET): $(DEBUG): $(ARGS)
	if [ -d vendor ]; \
	then \
		cargo build --manifest-path cli/Cargo.toml $(ARGS) --frozen; \
	else \
		cargo build --manifest-path cli/Cargo.toml $(ARGS); \
	fi

$(TARGET)/$(GTK_BIN):
	if [ -d vendor ]; \
	then \
		cargo build --manifest-path gtk/Cargo.toml $(ARGS) --frozen; \
	else \
		cargo build --manifest-path gtk/Cargo.toml $(ARGS); \
	fi

$(TARGET)/$(BIN).1.gz: $(TARGET)/$(BIN)
	help2man --no-info $< | gzip -c > $@.partial
	mv $@.partial $@
