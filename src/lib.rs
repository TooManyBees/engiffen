extern crate image;
extern crate gif;

use std::collections::HashMap;
use std::io::Write;
use std::borrow::Cow;
use image::{GenericImage, DynamicImage, Pixel, Rgba};
use gif::{Frame, Encoder, Repeat, SetParameter};

pub fn engiffen<W: Write>(imgs: &[DynamicImage], mut out: &mut W) {
    let gif_descriptor = palettize(&imgs);

    let mut color_map: &mut [u8; 256*3] = &mut [0; 256*3];
    for (color, idx) in gif_descriptor.palette.iter() {
        color_map[(idx*3) as usize] = color.data[0];
        color_map[(idx*3+1) as usize] = color.data[1];
        color_map[(idx*3+2) as usize] = color.data[2];
    }
    let width = gif_descriptor.width;
    let height = gif_descriptor.height;
    let mut encoder = Encoder::new(&mut out, width, height, color_map).unwrap();
    encoder.set(Repeat::Infinite).unwrap();
    for img in gif_descriptor.images {
        let mut frame = Frame::default();
        frame.width = width;
        frame.height = height;
        frame.buffer = Cow::Borrowed(&*img);
        encoder.write_frame(&frame).unwrap();
    }
}

struct GifDescriptor {
    palette: HashMap<Rgba<u8>, u8>,
    width: u16,
    height: u16,
    images: Vec<Vec<u8>>
}

fn palettize(imgs: &[DynamicImage]) -> GifDescriptor {
    if imgs.is_empty() {
        panic!("No images sent for palettization!");
    }
    let mut counter: HashMap<Rgba<u8>, usize> = HashMap::new();
    let (width, height) = {
        let img = imgs.iter().nth(0).unwrap();
        (img.width() as u16, img.height() as u16)
    };
    for img in imgs {
        count_colors(&img, &mut counter);
    }
    let palette = compress_palette(counter);
    let palettized_imgs: Vec<Vec<u8>> = imgs.iter().map(|img| {
        img.pixels().map(|(_, _, px)| {
            *palette.get(&px.to_rgba()).unwrap()
        }).collect()
    }).collect();

    GifDescriptor {
        palette: palette,
        width: width,
        height: height,
        images: palettized_imgs,
    }
}

fn compress_palette(colors: HashMap<Rgba<u8>, usize>) -> HashMap<Rgba<u8>, u8> {
    let mut ctr: Vec<(Rgba<u8>, usize)> = colors.into_iter().collect();
    if ctr.len() > 256 {
        panic!("Don't know how to engiffen more than 256 colors yet!");
    }
    ctr.sort_by(|a, b| b.1.cmp(&a.1));
    let mut palette = HashMap::with_capacity(256);
    for (i, (px, _)) in ctr.into_iter().enumerate() {
        palette.insert(px, i as u8);
    }
    palette
}

fn count_colors(img: &DynamicImage, counter: &mut HashMap<Rgba<u8>, usize>) {
    for (_, _, px) in img.pixels() {
        let ctr = counter.entry(px.to_rgba()).or_insert(0);
        *ctr += 1;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
