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
    engiffen::engiffen(&imgs, 10, &mut out);
}
