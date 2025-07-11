use std::env;
use std::fs;
use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let svg_path = PathBuf::from(crate_dir).join("ferris.svg");
    let svg_data = fs::read(&svg_path).expect("Failed to read SVG file");
    let svg_base64 = STANDARD.encode(svg_data);
    let data_uri = format!("data:image/svg+xml;base64,{svg_base64}");
    println!("cargo:rustc-env=ICON_SVG_BASE64={data_uri}");
}
