extern crate image;
extern crate gif;
extern crate color_quant;

use std::io::Write;
use std::borrow::Cow;
use image::{GenericImage, DynamicImage};
use gif::{Frame, Encoder, Repeat, SetParameter};
use color_quant::NeuQuant;

pub fn engiffen<W: Write>(imgs: &[DynamicImage], fps: usize, mut out: &mut W) {
    let gif_descriptor = palettize(&imgs);
    let delay = (1000 / fps) as u16;

    let width = gif_descriptor.width;
    let height = gif_descriptor.height;
    let mut encoder = Encoder::new(&mut out, width, height, &gif_descriptor.palette).unwrap();
    encoder.set(Repeat::Infinite).unwrap();
    for img in gif_descriptor.images {
        let mut frame = Frame::default();
        frame.delay = delay / 10;
        frame.width = width;
        frame.height = height;
        frame.buffer = Cow::Borrowed(&*img);
        frame.transparent = gif_descriptor.transparency;
        encoder.write_frame(&frame).unwrap();
    }
}

struct GifDescriptor {
    palette: Vec<u8>,
    transparency: Option<u8>,
    width: u16,
    height: u16,
    images: Vec<Vec<u8>>,
}

fn palettize(imgs: &[DynamicImage]) -> GifDescriptor {
    if imgs.is_empty() {
        panic!("No images sent for palettization!");
    }
    let (width, height) = {
        let img = imgs.iter().nth(0).unwrap();
        (img.width(), img.height())
    };
    let mut colors: Vec<u8> = Vec::with_capacity(width as usize * height as usize * imgs.len());
    for img in imgs {
        for (_, _, px) in img.pixels() {
            if px.data[3] == 0 {
                colors.push(0);
                colors.push(0);
                colors.push(0);
                colors.push(0);
            } else {
                colors.push(px.data[0]);
                colors.push(px.data[1]);
                colors.push(px.data[2]);
                colors.push(255);
            }
        }
    }

    let quant = NeuQuant::new(10, 256, &colors);
    let mut transparency = None;
    let palettized_imgs: Vec<Vec<u8>> = imgs.iter().map(|img| {
        img.pixels().map(|(_, _, px)| {
            let idx = quant.index_of(&px.data) as u8;
            if px.data[3] == 0 { transparency = Some(idx); }
            idx
        }).collect()
    }).collect();

    GifDescriptor {
        palette: quant.color_map_rgb(),
        transparency: transparency,
        width: width as u16,
        height: height as u16,
        images: palettized_imgs,
    }
}
