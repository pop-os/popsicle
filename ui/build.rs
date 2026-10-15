use std::{env, fs, path::PathBuf};
use xdgen::{App, Context, FluentString};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let ctx = Context::new(
        manifest_dir.join("../i18n"),
        env::var("CARGO_PKG_NAME").unwrap(),
    )
    .unwrap();
    let app = App::new(FluentString("app-title"))
        .comment(FluentString("app-comment"))
        .keywords(FluentString("app-keywords"));

    let desktop_entry = app
        .expand_desktop(manifest_dir.join("../resources/app.desktop"), &ctx)
        .unwrap();
    let metainfo = app
        .expand_metainfo(manifest_dir.join("../resources/app.metainfo.xml"), &ctx)
        .unwrap();

    let output = manifest_dir.join("target/xdgen");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("app.desktop"), desktop_entry).unwrap();
    fs::write(output.join("app.metainfo.xml"), metainfo).unwrap();
}
