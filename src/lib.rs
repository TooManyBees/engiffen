//! Engiffen is a library to convert sequences of images into animated Gifs.
//!
//! This library is a wrapper around the image and gif crates to convert
//! a sequence of images into an animated Gif.

#![doc(html_root_url = "https://docs.rs/engiffen/0.6.0")]

extern crate image;
extern crate gif;
extern crate color_quant;
extern crate lab;
extern crate rayon;
extern crate fnv;

use std::io;
#[cfg(feature = "debug-stderr")] use std::io::Write;
use std::{error, fmt, f32};
use std::borrow::Cow;
// use std::collections::HashMap;
use std::path::{Path, PathBuf};
use image::{GenericImage, DynamicImage};
use gif::{Frame, Encoder, Repeat, SetParameter};
use color_quant::NeuQuant;
use lab::Lab;
use rayon::prelude::*;
use fnv::FnvHashMap;

#[cfg(feature = "debug-stderr")] #[macro_use] mod macros;

#[cfg(feature = "debug-stderr")] use std::time::{Instant};

#[cfg(feature = "debug-stderr")]
fn ms(duration: Instant) -> u64 {
    let duration = duration.elapsed();
    duration.as_secs() * 1000 + duration.subsec_nanos() as u64 / 1000000
}

type RGBA = [u8; 4];

/// A color quantizing strategy.
///
/// `Naive` calculates color frequencies, picks the 256 most frequent colors
/// to be the palette, then reassigns the less frequently occuring colors to
/// the closest matching palette color.
///
/// `NeuQuant` uses the NeuQuant algorithm from the `color_quant` crate. It
/// trains a neural network using a pseudorandom subset of pixels, then
/// assigns each pixel its closest matching color in the palette.
///
/// # Usage
///
/// Pass this as the last argument to `engiffen` to select the quantizing
/// strategy.
///
/// The `NeuQuant` strategy produces the best looking images. Its interior
/// u32 value reduces the number of pixels that the algorithm uses to train,
/// which can greatly reduce its workload. Specifically, for a value of N,
/// only the pixels on every Nth column of every Nth row are considered, so
/// a value of 1 trains using every pixel, while a value of 2 trains using
/// 1/4 of all pixels.
///
/// The `Naive` strategy is fastest when you know that your input images
/// have a limited color range, but will produce terrible banding otherwise.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Quantizer {
    Naive,
    NeuQuant(u32),
}

/// An image, currently a wrapper around `image::DynamicImage`. If loaded from
/// disk through the `load_image` or `load_images` functions, its path property
/// contains the path used to read it from disk.
pub struct Image {
    inner: DynamicImage,
    pub path: Option<PathBuf>,
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Image {{ path: {:?}, dimensions: {} x {} }}", self.path, self.inner.width(), self.inner.height())
    }
}

#[derive(Debug)]
pub enum Error {
    NoImages,
    Mismatch((u32, u32), (u32, u32)),
    ImageLoad(image::ImageError),
    ImageWrite(io::Error),
}

impl From<image::ImageError> for Error {
    fn from(err: image::ImageError) -> Error {
        Error::ImageLoad(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::ImageWrite(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NoImages => write!(f, "No frames sent for engiffening"),
            Error::Mismatch(_, _) => write!(f, "Frames don't have the same dimensions"),
            Error::ImageLoad(ref e) => write!(f, "Image load error: {}", e),
            Error::ImageWrite(ref e) => write!(f, "Image write error: {}", e),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::NoImages => "No frames sent for engiffening",
            Error::Mismatch(_, _) => "Frames don't have the same dimensions",
            Error::ImageLoad(_) => "Unable to load image",
            Error::ImageWrite(_) => "Unable to write image",
        }
    }
}

/// Struct representing an animated Gif
#[derive(Eq, PartialEq, Clone, Hash)]
pub struct Gif {
    pub palette: Vec<u8>,
    pub transparency: Option<u8>,
    pub width: u16,
    pub height: u16,
    pub images: Vec<Vec<u8>>,
    pub delay: u16,
}

impl fmt::Debug for Gif {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Gif {{ palette: Vec<u8 x {:?}>, transparency: {:?}, width: {:?}, height: {:?}, images: Vec<Vec<u8> x {:?}>, delay: {:?} }}",
            self.palette.len(),
            self.transparency,
            self.width,
            self.height,
            self.images.len(),
            self.delay
        )
    }
}

impl Gif {
    /// Writes the animated Gif to any output that implements Write.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::fs::File;
    /// # use engiffen::{Image, engiffen, Quantizer};
    /// # fn foo() -> Result<(), engiffen::Error> {
    /// # let images: Vec<Image> = vec![];
    /// let mut output = File::create("output.gif")?;
    /// let gif = engiffen(&images, 10, Quantizer::NeuQuant(2))?;
    /// gif.write(&mut output)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the `std::io::Result` of the underlying `write` function calls.
    pub fn write<W: io::Write>(&self, mut out: &mut W) -> Result<(), Error> {
        let mut encoder = Encoder::new(&mut out, self.width, self.height, &self.palette)?;
        encoder.set(Repeat::Infinite)?;
        for img in &self.images {
            let mut frame = Frame::default();
            frame.delay = self.delay / 10;
            frame.width = self.width;
            frame.height = self.height;
            frame.buffer = Cow::Borrowed(&*img);
            frame.transparent = self.transparency;
            encoder.write_frame(&frame)?;
        }
        Ok(())
    }
}

/// Loads an image from the given file path.
///
/// # Examples
///
/// ```rust,no_run
/// # use engiffen::{load_image, Image, Error};
/// # use std::path::PathBuf;
/// # fn foo() -> Result<Image, Error> {
/// let image = load_image("test/ball/ball01.bmp")?;
/// assert_eq!(image.path, Some(PathBuf::from("test/ball/ball01.bmp")));
/// # Ok(image)
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if the path can't be read or if the image can't be decoded
pub fn load_image<P>(path: P) -> Result<Image, Error>
    where P: AsRef<Path> {
    let img = image::open(&path)?;
    Ok(Image {
        inner: img,
        path: Some(path.as_ref().to_path_buf()),
    })
}

/// Loads images from a list of given paths. Errors encountered while loading files
/// are skipped.
///
/// # Examples
///
/// ```rust,no_run
/// # use engiffen::load_images;
/// let paths = vec!["tests/ball/ball06.bmp", "tests/ball/ball07.bmp", "tests/ball/ball08.bmp"];
/// let images = load_images(&paths);
/// assert_eq!(images.len(), 2); // The last path doesn't exist. It was silently skipped.
/// ```
///
/// Skips images that fail to load. If all images fail, returns an empty vector.
pub fn load_images<P>(paths: &[P]) -> Vec<Image>
    where P: AsRef<Path> {
    paths.iter()
        .map(|path| load_image(path))
        .filter_map(|img| img.ok())
        .collect()
}

/// Converts a sequence of images into a `Gif` at a given frame rate. The `quantizer`
/// parameter selects the algorithm that quantizes the palette into 256-colors.
///
/// # Examples
///
/// ```rust,no_run
/// # use engiffen::{load_images, engiffen, Gif, Error, Quantizer};
/// # fn foo() -> Result<Gif, Error> {
/// let paths = vec!["tests/ball/ball01.bmp", "tests/ball/ball02.bmp", "tests/ball/ball03.bmp"];
/// let images = load_images(&paths);
/// let gif = engiffen(&images, 10, Quantizer::NeuQuant(2))?;
/// assert_eq!(gif.images.len(), 3);
/// # Ok(gif)
/// # }
/// ```
///
/// # Errors
///
/// If any image dimensions differ, this function will return an Error::Mismatch
/// containing tuples of the conflicting image dimensions.
pub fn engiffen(imgs: &[Image], fps: usize, quantizer: Quantizer) -> Result<Gif, Error> {
    if imgs.is_empty() {
        return Err(Error::NoImages);
    }
    #[cfg(feature = "debug-stderr")] printerr!("Engiffening {} images", imgs.len());

    let (width, height) = {
        let ref first = imgs[0].inner;
        let first_dimensions = (first.width(), first.height());
        for img in imgs.iter() {
            let other_dimensions = (img.inner.width(), img.inner.height());
            if first_dimensions != other_dimensions {
                return Err(Error::Mismatch(first_dimensions, other_dimensions));
            }
        }
        first_dimensions
    };

    let (palette, palettized_imgs, transparency) = match quantizer {
        Quantizer::NeuQuant(sample_rate) => neuquant_palettize(&imgs, sample_rate, width, height),
        Quantizer::Naive => naive_palettize(&imgs),
    };

    let delay = (1000 / fps) as u16;

    Ok(Gif {
        palette: palette,
        transparency: transparency,
        width: width as u16,
        height: height as u16,
        images: palettized_imgs,
        delay: delay,
    })
}

fn neuquant_palettize(imgs: &[Image], sample_rate: u32, width: u32, height: u32) -> (Vec<u8>, Vec<Vec<u8>>, Option<u8>) {
    let image_len = (width * height * 4 / sample_rate / sample_rate) as usize;
    let transparent_black = [0u8; 4];
    #[cfg(feature = "debug-stderr")] let time_push = Instant::now();
    let colors: Vec<u8> = imgs.par_iter().map(|img| {
        let mut temp: Vec<_> = Vec::with_capacity(image_len);
        for (x, y, px) in img.inner.pixels() {
            if sample_rate > 1 {
                if x % sample_rate != 0 || y % sample_rate != 0 {
                    continue;
                }
            }
            if px.data[3] == 0 {
                temp.extend_from_slice(&transparent_black);
            } else {
                temp.extend_from_slice(&px.data[..3]);
                temp.push(255);
            }
        }
        temp
    }).reduce(|| Vec::with_capacity(image_len * imgs.len()), |mut acc, img| {
        acc.extend_from_slice(&img);
        acc
    });
    #[cfg(feature = "debug-stderr")]
    printerr!("Neuquant: Concatenated {} bytes in {} ms.", colors.len(), ms(time_push));

    #[cfg(feature = "debug-stderr")] let time_quant = Instant::now();
    let quant = NeuQuant::new(10, 256, &colors);
    #[cfg(feature = "debug-stderr")]
    printerr!("Neuquant: Computed palette in {} ms.", ms(time_quant));

    #[cfg(feature = "debug-stderr")] let time_map = Instant::now();
    let mut transparency = None;
    let mut cache: FnvHashMap<RGBA, u8> = FnvHashMap::default();
    let palettized_imgs: Vec<Vec<u8>> = imgs.iter().map(|img| {
        img.inner.pixels().map(|(_, _, px)| {
            *cache.entry(px.data).or_insert_with(|| {
                let idx = quant.index_of(&px.data) as u8;
                if px.data[3] == 0 { transparency = Some(idx); }
                idx
            })
        }).collect()
    }).collect();
    #[cfg(feature = "debug-stderr")]
    printerr!("Neuquant: Mapped pixels to palette in {} ms.", ms(time_map));

    (quant.color_map_rgb(), palettized_imgs, transparency)
}

fn naive_palettize(imgs: &[Image]) -> (Vec<u8>, Vec<Vec<u8>>, Option<u8>) {
    #[cfg(feature = "debug-stderr")] let time_count = Instant::now();
    let frequencies: FnvHashMap<RGBA, usize> = imgs.par_iter().map(|img| {
        let mut fr: FnvHashMap<RGBA, usize> = FnvHashMap::default();
        for (_, _, pixel) in img.inner.pixels() {
            let num = fr.entry(pixel.data).or_insert(0);
            *num += 1;
        }
        fr
    }).reduce(|| FnvHashMap::default(), |mut acc, fr| {
        for (color, count) in fr {
            let num = acc.entry(color).or_insert(0);
            *num += count;
        }
        acc
    });
    #[cfg(feature = "debug-stderr")]
    printerr!("Naive: Counted color frequencies in {} ms", ms(time_count));
    #[cfg(feature = "debug-stderr")] let time_palette = Instant::now();
    let mut sorted_frequencies = frequencies.into_iter()
        .collect::<Vec<_>>();
    sorted_frequencies.sort_by(|a, b| b.1.cmp(&a.1));
    let sorted = sorted_frequencies.into_iter().map(|c| {
        (c.0, Lab::from_rgba(&c.0))
    }).collect::<Vec<_>>();

    let (palette, rest) = if sorted.len() > 256 {
        (&sorted[..256], &sorted[256..])
    } else {
        (&sorted[..], &[] as &[_])
    };

    let mut map: FnvHashMap<RGBA, u8> = FnvHashMap::default();
    for (i, color) in palette.iter().enumerate() {
        map.insert(color.0, i as u8);
    }
    for color in rest {
        let closest_index = palette.iter().enumerate().fold((0, f32::INFINITY), |closest, (idx, p)| {
            let dist = p.1.squared_distance(&color.1);
            if closest.1 < dist {
                closest
            } else {
                (idx, dist)
            }
        }).0;
        let closest_rgb = palette[closest_index].0;
        let index = *map.get(&closest_rgb).expect("A color we assigned to the palette is somehow missing from the palette index map.");
        map.insert(color.0, index);
    }
    #[cfg(feature = "debug-stderr")]
    printerr!("Naive: Computed palette in {} ms.", ms(time_palette));

    #[cfg(feature = "debug-stderr")]let time_index = Instant::now();
    let palettized_imgs: Vec<Vec<u8>> = imgs.par_iter().map(|img| {
        img.inner.pixels().map(|(_, _, px)| {
            *map.get(&px.data).expect("A color in an image was not added to the palette map.")
        }).collect()
    }).collect();
    #[cfg(feature = "debug-stderr")]
    printerr!("Naive: Mapped pixels to palette in {} ms", ms(time_index));

    let mut palette_as_bytes = Vec::with_capacity(palette.len() * 3);
    for color in palette {
        palette_as_bytes.extend_from_slice(&color.0[0..3]);
    }

    (palette_as_bytes, palettized_imgs, None)
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::{load_image, engiffen, Error, Quantizer};
    use std::fs::{read_dir, File};

    #[test]
    fn test_error_on_size_mismatch() {
        let imgs: Vec<_> = read_dir("tests/mismatched_size").unwrap()
        .map(|e| e.unwrap().path())
        .map(|path| load_image(&path).unwrap())
        .collect();

        let res = engiffen(&imgs, 30, Quantizer::NeuQuant(1));

        assert!(res.is_err());
        match res {
            Err(Error::Mismatch(one, another)) => {
                assert_eq!((one, another), ((100, 100), (50, 50)));
            },
            _ => unreachable!(),
        }
    }

    #[test] #[ignore]
    fn test_compress_palette() {
        // This takes a while to run when not in --release
        let imgs: Vec<_> = read_dir("tests/ball").unwrap()
            .map(|e| e.unwrap().path())
            .filter(|path| match path.extension() {
                Some(ext) if ext == "bmp" => true,
                _ => false,
            })
            .map(|path| load_image(&path).unwrap())
            .collect();

        let mut out = File::create("tests/ball.gif").unwrap();
        let gif = engiffen(&imgs, 10, Quantizer::NeuQuant(2));
        match gif {
            Ok(gif) => gif.write(&mut out),
            Err(_) => panic!("Test should have successfully made a gif."),
        };
    }

    #[test] #[ignore]
    fn test_simple_paletted_gif() {
        let imgs: Vec<_> = read_dir("tests/shrug").unwrap()
            .map(|e| e.unwrap().path())
            .filter(|path| match path.extension() {
                Some(ext) if ext == "tga" => true,
                _ => false,
            })
            .map(|path| load_image(&path).unwrap())
            .collect();

        let mut out = File::create("tests/shrug.gif").unwrap();
        let gif = engiffen(&imgs, 30, Quantizer::NeuQuant(2));
        match gif {
            Ok(gif) => gif.write(&mut out),
            Err(_) => panic!("Test should have successfully made a gif."),
        };
    }
}
