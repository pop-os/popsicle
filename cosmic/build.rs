use std::{env, fs, path::Path};
use xdgen::{App, Context, FluentString};

fn main() {
    let ctx = Context::new("../i18n", env::var("CARGO_PKG_NAME").unwrap()).unwrap();
    let app = App::new(FluentString("app-title"))
        .comment(FluentString("app-comment"))
        .keywords(FluentString("app-keywords"));

    let desktop_entry = app.expand_desktop("resources/app.desktop", &ctx).unwrap();
    let metainfo = app.expand_metainfo("resources/app.metainfo.xml", &ctx).unwrap();

    let output = Path::new("target/xdgen/");
    fs::create_dir_all(output).unwrap();
    fs::write(output.join("app.desktop"), desktop_entry).unwrap();
    fs::write(output.join("app.metainfo.xml"), metainfo).unwrap();
}
