mod cargo 'cargo.just'

default_prefix := '/usr/local'
prefix := default_prefix
rootdir := env('DESTDIR', '')
base-dir := absolute_path(clean(rootdir / prefix))
target-dir := env('CARGO_TARGET_DIR', 'target')

bin := 'popsicle'
ui-bin := 'popsicle-ui'
appid := 'com.system76.Popsicle'
appdata := appid + '.appdata.xml'
desktop := appid + '.desktop'

default: all

all: cli ui

cli: (cargo::build-release '--manifest-path' 'cli/Cargo.toml') manpage

manpage:
    help2man --no-info {{target-dir / 'release' / bin}} > {{target-dir / 'release' / bin}}.1.partial || { rm -f {{target-dir / 'release' / bin}}.1.partial; exit 1; }
    gzip -c {{target-dir / 'release' / bin}}.1.partial > {{target-dir / 'release' / bin}}.1.gz || { rm -f {{target-dir / 'release' / bin}}.1.gz {{target-dir / 'release' / bin}}.1.partial; exit 1; }
    rm {{target-dir / 'release' / bin}}.1.partial

ui: (cargo::build-release '--manifest-path' 'ui/Cargo.toml')

build-debug *args: (cargo::build-debug '--workspace' args)

build-release *args: (build-debug '--release' args) manpage

build-vendored *args: cargo::vendor-extract
    LOCKSTEP_XML_PATH="${PWD}/vendor/atspi-common/xml" cargo build --workspace --release --frozen --offline {{args}}
    just manpage

check *args: (cargo::check args)

check-json: cargo::check-json

run *args: (cargo::run args)

test *args: (cargo::test args)

flatpak:
    flatpak-builder --force-clean build/flatpak com.system76.Popsicle.json

flatpak-install:
    flatpak-builder --user --install --force-clean build/flatpak com.system76.Popsicle.json

cargo-sources:
    flatpak-cargo-generator Cargo.lock -o cargo-sources.json

install-cli:
    install -Dm0755 {{target-dir / 'release' / bin}} {{base-dir / 'bin' / bin}}
    install -Dm0644 {{target-dir / 'release' / (bin + '.1.gz')}} {{base-dir / 'share' / 'man' / 'man1' / (bin + '.1.gz')}}

install-ui:
    install -Dm0755 {{target-dir / 'release' / ui-bin}} {{base-dir / 'bin' / ui-bin}}
    install -Dm0644 ui/target/xdgen/app.desktop {{base-dir / 'share' / 'applications' / desktop}}
    install -Dm0644 ui/target/xdgen/app.metainfo.xml {{base-dir / 'share' / 'metainfo' / appdata}}
    find resources/icons/hicolor -type f | while IFS= read -r icon; do install -Dm0644 "$icon" "{{base-dir}}/share/icons/hicolor/${icon#resources/icons/hicolor/}"; done

install: install-cli install-ui

uninstall-cli:
    rm -f {{base-dir / 'bin' / bin}} {{base-dir / 'share' / 'man' / 'man1' / (bin + '.1.gz')}}

uninstall-ui:
    rm -f {{base-dir / 'bin' / ui-bin}} {{base-dir / 'share' / 'applications' / desktop}} {{base-dir / 'share' / 'metainfo' / appdata}}

uninstall: uninstall-cli uninstall-ui

clean: cargo::clean

distclean: cargo::clean-dist

vendor: cargo::vendor

update:
    cargo update
