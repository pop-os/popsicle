prefix ?= /usr/local
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
libdir = $(exec_prefix)/lib
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)

.PHONY: all clean distclean install uninstall update

BIN=muff

all: target/release/$(BIN) target/release/$(BIN).1.gz

clean:
	cargo clean

distclean: clean
	rm -rf .cargo vendor

install: all
	install -D -m 0755 "target/release/$(BIN)" "$(DESTDIR)$(bindir)/$(BIN)"
	install -D -m 0755 "target/release/$(BIN).1.gz" "$(DESTDIR)$(datadir)/man/man1/$(BIN).1.gz"

uninstall:
	rm -f "$(DESTDIR)$(bindir)/$(BIN)"
	rm -f "$(DESTDIR)$(datadir)/man/man1/$(BIN).1.gz"

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
		cargo build --release --frozen; \
	else \
		cargo build --release; \
	fi

target/release/$(BIN).1.gz: target/release/$(BIN)
	help2man --no-info $< | gzip -c > $@.partial
	mv $@.partial $@
