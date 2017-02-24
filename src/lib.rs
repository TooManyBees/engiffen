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

    let mut color_map: [u8; 256*3] = [0; 256*3];
    let mut transparency = None;
    for (idx, color) in gif_descriptor.palette.chunks(4).enumerate() {
        color_map[idx*3] = color[0];
        color_map[idx*3+1] = color[1];
        color_map[idx*3+2] = color[2];
        if color[3] == 0 {
            transparency = Some(idx as u8);
        }
    }
    let width = gif_descriptor.width;
    let height = gif_descriptor.height;
    let mut encoder = Encoder::new(&mut out, width, height, &color_map).unwrap();
    encoder.set(Repeat::Infinite).unwrap();
    for img in gif_descriptor.images {
        let mut frame = Frame::default();
        frame.delay = delay / 10;
        frame.width = width;
        frame.height = height;
        frame.buffer = Cow::Borrowed(&*img);
        frame.transparent = transparency;
        encoder.write_frame(&frame).unwrap();
    }
}

struct GifDescriptor {
    palette: Vec<u8>,
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
    let palettized_imgs: Vec<Vec<u8>> = imgs.iter().map(|img| {
        img.pixels().map(|(_, _, px)| {
            quant.index_of(&px.data) as u8
        }).collect()
    }).collect();

    GifDescriptor {
        palette: quant.color_map_rgba(),
        width: width as u16,
        height: height as u16,
        images: palettized_imgs,
    }
}
