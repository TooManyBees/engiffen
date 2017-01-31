extern crate image;
extern crate lab;
extern crate gif;

use std::collections::HashMap;
use std::io::Write;
use std::borrow::Cow;
use image::{GenericImage, DynamicImage, Pixel, Rgba};
use gif::{Frame, Encoder, Repeat, SetParameter};
use lab::Lab;

pub fn engiffen<W: Write>(imgs: &[DynamicImage], mut out: &mut W) {
    let gif_descriptor = palettize(&imgs);

    assert!(gif_descriptor.palette.len() <= 256, "Computed palette has more than 256 colors");

    let mut color_map: &mut [u8; 256*3] = &mut [0; 256*3];
    for (color, idx) in gif_descriptor.palette.iter() {
        color_map[(*idx as usize)*3] = color.data[0];
        color_map[(*idx as usize)*3+1] = color.data[1];
        color_map[(*idx as usize)*3+2] = color.data[2];
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
    let (palette, compressed) = compress_palette(counter);
    let palettized_imgs: Vec<Vec<u8>> = imgs.iter().map(|img| {
        img.pixels().map(|(_, _, px)| {
            *palette.get(&px.to_rgba()).unwrap()
        }).collect()
    }).collect();

    GifDescriptor {
        palette: compressed,
        width: width,
        height: height,
        images: palettized_imgs,
    }
}

fn not_too_close(lab: Lab, sensitivity: f32, palette: &HashMap<Rgba<u8>, u8>, labs: &HashMap<Rgba<u8>, Lab>) -> bool {
    palette.keys().map(|rgba| labs.get(rgba).unwrap()).all(|v| lab.squared_distance(&v) > sensitivity)
}

fn find_closest_rgb(px: &Rgba<u8>, palette: &mut HashMap<Rgba<u8>, u8>, labs: &mut HashMap<Rgba<u8>, Lab>) -> Rgba<u8> {
    let lab_new = Lab::from_rgba(px.data);
    let mut closest_rgb = None;
    {
        let mut closest_distance = std::f32::MAX;
        for (rgb, lab) in palette.keys().map(|k| (k, labs.get(k).unwrap())) {
            let dist = lab_new.squared_distance(lab);
            if dist < closest_distance {
                closest_distance = dist;
                closest_rgb = Some(rgb.clone());
            }
        }
    }
    labs.insert(*px, lab_new);
    closest_rgb.expect("Couldn't find ANY closest RGB colors?").clone()
}

fn compress_palette(colors: HashMap<Rgba<u8>, usize>) -> (HashMap<Rgba<u8>, u8>, HashMap<Rgba<u8>, u8>) {
    let mut ctr: Vec<(Rgba<u8>, usize)> = colors.into_iter().collect();
    ctr.sort_by(|a, b| b.1.cmp(&a.1));

    let mut palette = HashMap::with_capacity(256);
    let mut compressed_palette = palette.clone();
    let mut lab_colors_by_rgb = HashMap::with_capacity(256);

    if ctr.len() <= 256 {
        for (i, (px, _)) in ctr.into_iter().enumerate() {
            palette.insert(px, i as u8);
            compressed_palette.insert(px, i as u8);
        }
    } else {
        for (i, (px, _)) in ctr.into_iter().enumerate() {
            if compressed_palette.len() < 256 && not_too_close(Lab::from_rgba(px.data), 300.0, &palette, &lab_colors_by_rgb) {
                lab_colors_by_rgb.insert(px, Lab::from_rgba(px.data));
                palette.insert(px, i as u8);
                compressed_palette.insert(px, i as u8);
            } else {
                let closest_rgb = find_closest_rgb(&px, &mut compressed_palette, &mut lab_colors_by_rgb);
                let i_new = *(compressed_palette.get(&closest_rgb).expect("Closest RGB color wasn't in the palette to begin with."));
                palette.insert(px, i_new);
                // whew!
            }
        }
    }
    (palette, compressed_palette)
}

fn count_colors(img: &DynamicImage, counter: &mut HashMap<Rgba<u8>, usize>) {
    for (_, _, px) in img.pixels() {
        let ctr = counter.entry(px).or_insert(0);
        *ctr += 1;
    }
}
