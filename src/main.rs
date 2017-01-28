extern crate engiffen;
extern crate image;

use std::fs::{read_dir, File};

fn main() {
    let imgs: Vec<_> = read_dir("test/shrug").unwrap()
        .map(|e| e.unwrap().path())
        .map(|path| image::open(&path).unwrap())
        .collect();

    let mut out = File::create("test/out.gif").unwrap();
    engiffen::engiffen(&imgs, &mut out);
}
