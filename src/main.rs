extern crate engiffen;
extern crate image;
extern crate getopts;

use std::{env, error, fmt, process};
use std::str::FromStr;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};
use std::time::Instant;
use getopts::Options;

use SourceImages::*;

#[derive(Debug)]
enum SourceImages {
    StartEnd(PathBuf, PathBuf, PathBuf),
    List(Vec<String>),
    StdIn,
}

#[derive(Debug)]
struct Args {
    source: SourceImages,
    fps: usize,
    out_file: String,
}

#[derive(Debug)]
enum ArgsError {
    Parse(getopts::Fail),
    Fps(std::num::ParseIntError),
    ImageRange(String),
}

impl From<getopts::Fail> for ArgsError {
    fn from(err: getopts::Fail) -> ArgsError {
        ArgsError::Parse(err)
    }
}

impl From<std::num::ParseIntError> for ArgsError {
    fn from(err: std::num::ParseIntError) -> ArgsError {
        ArgsError::Fps(err)
    }
}

impl fmt::Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ArgsError::Parse(ref err) => write!(f, "Options parse error: {}", err),
            ArgsError::Fps(_) => write!(f, "Unable to parse framerate as an integer"),
            ArgsError::ImageRange(ref s) => write!(f, "Bad image range: {}", s),
        }
    }
}

impl error::Error for ArgsError {
    fn description(&self) -> &str {
        match *self {
            ArgsError::Parse(ref err) => err.description(),
            ArgsError::Fps(ref err) => err.description(),
            ArgsError::ImageRange(_) => "Bad image range",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ArgsError::Parse(ref err) => Some(err),
            ArgsError::Fps(ref err) => Some(err),
            ArgsError::ImageRange(_) => None,
        }
    }
}

fn path_and_filename(input: &str) -> Result<(PathBuf, PathBuf), ArgsError> {
    let p = Path::new(&input);
    let parent = p.parent().unwrap_or(Path::new("."));
    let filename = p.file_name();
    if filename.is_none() {
        Err(ArgsError::ImageRange(format!("Invalid filename {:?}", input)))
    } else {
        Ok((parent.to_owned(), PathBuf::from(filename.unwrap())))
    }
}

fn parse_args() -> Result<Args, ArgsError> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("o", "outfile", "engiffen to this filename", "FILE");
    opts.optopt("f", "framerate", "frames per second", "30");
    opts.optflag("r", "range", "arguments specify start and end images");
    opts.optflag("h", "help", "display this help");

    let matches = try!{opts.parse(&args[1..])};
    if matches.opt_present("h") {
        let brief = format!("Usage: {} <files ...>", program);
        print!("{}", opts.usage(&brief));
        process::exit(0);
    }

    let fps: usize = if matches.opt_present("f") {
        try!{ usize::from_str(&matches.opt_str("f").unwrap()) }
    } else {
        30
    };

    let out_file = matches.opt_str("o").map(|f| f.clone()).unwrap_or("out.gif".to_string());
    let source = if matches.opt_present("r") {
        if matches.free.len() >= 2 {
            let (path_start, filename_start) = path_and_filename(&matches.free[0])?;
            let (path_end, filename_end) = path_and_filename(&matches.free[1])?;
            if path_start != path_end {
                return Err(ArgsError::ImageRange("start and end files are from different directories".to_string()))
            }
            if !path_start.exists() {
                return Err(ArgsError::ImageRange(format!("directory not readable: {:?}", path_start)))
            }
            StartEnd(path_start, filename_start, filename_end)
        } else if matches.free.len() == 1 {
            return Err(ArgsError::ImageRange("missing end filename".to_string()))
        } else {
            return Err(ArgsError::ImageRange("missing start and end filenames".to_string()))
        }
    } else if matches.free.is_empty() {
        StdIn
    } else {
        List(matches.free)
    };

    Ok(Args {
        source: source,
        fps: fps,
        out_file: out_file,
    })
}

fn main() {
    let args = parse_args().map_err(|e| {
        println!("Aborted! {}", e);
        process::exit(1);
    }).unwrap();
    let source_images: Vec<PathBuf> = match args.source {
        StartEnd(dir, start_path, end_path) => {
            read_dir(dir).unwrap()
            .map(|e| e.unwrap().path())
            .skip_while(|path| path.file_name().unwrap() < start_path)
            .take_while(|path| path.file_name().unwrap() <= end_path)
            .collect()
        },
        List(list) => list.into_iter().map(PathBuf::from).collect(),
        StdIn => vec![],
    };

    let imgs: Vec<_> = source_images.iter()
        .map(|path| image::open(&path).unwrap())
        .collect();

    let mut out = File::create(&args.out_file).unwrap();
    let now = Instant::now();
    match engiffen::engiffen(&imgs, args.fps, &mut out) {
        Err(e) => println!("{}", e),
        _ => {
            let duration = now.elapsed();
            let ms = duration.as_secs() * 1000 + duration.subsec_nanos() as u64 / 1000000;
            println!("Wrote {} in {} ms", &args.out_file, ms);
        },
    }
}
