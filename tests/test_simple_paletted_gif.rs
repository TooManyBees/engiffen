extern crate engiffen;
extern crate image;

use std::fs::{read_dir, File};

#[test]
fn test_simple_paletted_gif() {
    let imgs: Vec<_> = read_dir("tests/shrug").unwrap()
        .map(|e| e.unwrap().path())
        .map(|path| image::open(&path).unwrap())
        .collect();

    let mut out = File::create("tests/shrug.gif").unwrap();
    engiffen::engiffen(&imgs, &mut out);
}
