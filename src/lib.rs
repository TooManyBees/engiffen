extern crate image;
extern crate gif;
extern crate color_quant;

use std::io;
use std::{error, fmt};
use std::borrow::Cow;
use image::{GenericImage, DynamicImage};
use gif::{Frame, Encoder, Repeat, SetParameter};
use color_quant::NeuQuant;

#[derive(Debug)]
pub enum Error {
    Write(io::Error),
    NoImages,
    Mismatch((u32, u32), (u32, u32)),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Write(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Write(_) => write!(f, "Couldn't write to output"),
            Error::NoImages => write!(f, "No frames sent for engiffening"),
            Error::Mismatch(_, _) => write!(f, "Frames don't have the same dimensions"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Write(ref err) => err.description(),
            Error::NoImages => "No frames sent for engiffening",
            Error::Mismatch(_, _) => "Frames don't have the same dimensions",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Write(ref err) => Some(err),
            _ => None,
        }
    }
}

pub fn engiffen<W: io::Write>(imgs: &[DynamicImage], fps: usize, mut out: &mut W) -> Result<(), Error> {
    let gif_descriptor = palettize(&imgs)?;
    let delay = (1000 / fps) as u16;

    let width = gif_descriptor.width;
    let height = gif_descriptor.height;
    let mut encoder = Encoder::new(&mut out, width, height, &gif_descriptor.palette)?;
    encoder.set(Repeat::Infinite)?;
    for img in gif_descriptor.images {
        let mut frame = Frame::default();
        frame.delay = delay / 10;
        frame.width = width;
        frame.height = height;
        frame.buffer = Cow::Borrowed(&*img);
        frame.transparent = gif_descriptor.transparency;
        encoder.write_frame(&frame)?;
    }
    Ok(())
}

struct GifDescriptor {
    palette: Vec<u8>,
    transparency: Option<u8>,
    width: u16,
    height: u16,
    images: Vec<Vec<u8>>,
}

fn palettize(imgs: &[DynamicImage]) -> Result<GifDescriptor, Error> {
    if imgs.is_empty() {
        return Err(Error::NoImages);
    }
    let (width, height) = {
        let first = imgs.iter().nth(0).unwrap();
        let first_dimensions = (first.width(), first.height());
        for img in imgs.iter() {
            let other_dimensions = (img.width(), img.height());
            if first_dimensions != other_dimensions {
                return Err(Error::Mismatch(first_dimensions, other_dimensions));
            }
        }
        first_dimensions
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

    Ok(GifDescriptor {
        palette: quant.color_map_rgb(),
        transparency: transparency,
        width: width as u16,
        height: height as u16,
        images: palettized_imgs,
    })
}

#[cfg(test)]
mod tests {
    use super::{engiffen, Error};
    use std::fs::{remove_file, read_dir, File};
    use image;

    #[test]
    fn test_error_on_size_mismatch() {
        let imgs: Vec<_> = read_dir("tests/mismatched_size").unwrap()
        .map(|e| e.unwrap().path())
        .map(|path| image::open(&path).unwrap())
        .collect();

        let out_file = "tests/test_out.gif";

        let res = engiffen(&imgs, 30, &mut File::create(&out_file).unwrap());

        assert!(res.is_err());
        match res {
            Err(Error::Mismatch(one, another)) => {
                assert_eq!((one, another), ((100, 100), (50, 50)));
            },
            _ => unreachable!(),
        }
        match remove_file(&out_file) {
            _ => {} // I don't care
        }
    }
}
