extern crate engiffen;
extern crate image;

use std::fs::{read_dir, File};

fn main() {
    let imgs: Vec<_> = read_dir("tests/shrug").unwrap()
        .map(|e| e.unwrap().path())
        .map(|path| image::open(&path).unwrap())
        .collect();

    let mut out = File::create("tests/shrug.gif").unwrap();
    engiffen::engiffen(&imgs, 30, &mut out);
}
