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

cli:
    cargo build --manifest-path cli/Cargo.toml --release
    help2man --no-info {{target-dir / 'release' / bin}} > {{target-dir / 'release' / bin}}.1.partial || { rm -f {{target-dir / 'release' / bin}}.1.partial; exit 1; }
    gzip -c {{target-dir / 'release' / bin}}.1.partial > {{target-dir / 'release' / bin}}.1.gz || { rm -f {{target-dir / 'release' / bin}}.1.gz {{target-dir / 'release' / bin}}.1.partial; exit 1; }
    rm {{target-dir / 'release' / bin}}.1.partial

ui:
    cd ui && cargo build --release

flatpak:
    flatpak-builder --force-clean build/flatpak com.system76.Popsicle.json

flatpak-install:
    flatpak-builder --user --install --force-clean build/flatpak com.system76.Popsicle.json

cargo-sources:
    flatpak-cargo-generator Cargo.lock -o cargo-sources.json

install-cli: cli
    install -Dm0755 {{target-dir / 'release' / bin}} {{base-dir / 'bin' / bin}}
    install -Dm0644 {{target-dir / 'release' / (bin + '.1.gz')}} {{base-dir / 'share' / 'man' / 'man1' / (bin + '.1.gz')}}

install-ui: ui
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

clean:
    cargo clean

distclean: clean
    rm -rf .cargo vendor vendor.tar

vendor:
    mkdir -p .cargo
    cargo vendor | head -n -1 > .cargo/config
    echo 'directory = "vendor"' >> .cargo/config
    tar pcf vendor.tar vendor
    rm -rf vendor

update:
    cargo update
