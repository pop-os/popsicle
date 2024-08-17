use std::{
    env, fs,
    process::{self, Command},
};

fn main() {
    for i in fs::read_dir("assets").unwrap() {
        println!("cargo:rerun-if-changed={}", i.unwrap().path().display());
    }

    let out_dir = env::var("OUT_DIR").unwrap();

    let status = Command::new("glib-compile-resources")
        .arg("--sourcedir=assets")
        .arg(format!("--target={}/compiled.gresource", out_dir))
        .arg("assets/resources.gresource.xml")
        .status()
        .unwrap();

    if !status.success() {
        eprintln!("glib-compile-resources failed with exit status {}", status);
        process::exit(1);
    }
}
