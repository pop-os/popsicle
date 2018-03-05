prefix ?= /usr/local
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
libdir = $(exec_prefix)/lib
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)

.PHONY: all clean distclean install uninstall update

BIN=muff
GTK_BIN=muff-gtk

all: cli gtk

cli: target/release/$(BIN) target/release/$(BIN).1.gz

gtk: target/release/$(GTK_BIN)

clean:
	cargo clean

distclean: clean
	rm -rf .cargo vendor

install-cli: cli
	install -D -m 0755 "target/release/$(BIN)" "$(DESTDIR)$(bindir)/$(BIN)"
	install -D -m 0755 "target/release/$(BIN).1.gz" "$(DESTDIR)$(datadir)/man/man1/$(BIN).1.gz"

install-gtk: gtk
	install -D -m 0755 "target/release/$(GTK_BIN)" "$(DESTDIR)$(bindir)/$(GTK_BIN)"
	install -D -m 0644 "gtk/assets/flash.png" "$(DESTDIR)$(datadir)/muff/flash.png"
	install -D -m 0644 "gtk/assets/image.png" "$(DESTDIR)$(datadir)/muff/image.png"
	install -D -m 0644 "gtk/assets/usb.png" "$(DESTDIR)$(datadir)/muff/usb.png"

install: all install-cli install-gtk

uninstall-cli:
	rm -f "$(DESTDIR)$(bindir)/$(BIN)"
	rm -f "$(DESTDIR)$(datadir)/man/man1/$(BIN).1.gz"

uninstall-gtk:
	rm -f "$(DESTDIR)$(bindir)/$(GTK_BIN)"

uninstall: uninstall-cli uninstall-gtk

update:
	cargo update

.cargo/config: vendor_config
	mkdir -p .cargo
	cp $< $@

vendor: .cargo/config
	cargo vendor
	touch vendor

target/release/$(BIN):
	if [ -d vendor ]; \
	then \
		cargo build --manifest-path cli/Cargo.toml --release --frozen; \
	else \
		cargo build --manifest-path cli/Cargo.toml --release; \
	fi

target/release/$(GTK_BIN):
	if [ -d vendor ]; \
	then \
		cargo build --manifest-path gtk/Cargo.toml --release --frozen; \
	else \
		cargo build --manifest-path gtk/Cargo.toml --release; \
	fi

target/release/$(BIN).1.gz: target/release/$(BIN)
	help2man --no-info $< | gzip -c > $@.partial
	mv $@.partial $@
