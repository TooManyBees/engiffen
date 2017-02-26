extern crate engiffen;
extern crate image;

use std::fs::{read_dir, File};

#[test]
#[ignore]
fn test_compress_palette() {
    // This takes a while to run when not in --release
    let imgs: Vec<_> = read_dir("tests/ball").unwrap()
        .map(|e| e.unwrap().path())
        .map(|path| image::open(&path).unwrap())
        .collect();

    let mut out = File::create("tests/ball.gif").unwrap();
    #[allow(unused_must_use)]
    engiffen::engiffen(&imgs, 10, &mut out);
}

#[test]
#[ignore]
fn test_simple_paletted_gif() {
    let imgs: Vec<_> = read_dir("tests/shrug").unwrap()
        .map(|e| e.unwrap().path())
        .map(|path| image::open(&path).unwrap())
        .collect();

    let mut out = File::create("tests/shrug.gif").unwrap();
    #[allow(unused_must_use)]
    engiffen::engiffen(&imgs, 30, &mut out);
}
