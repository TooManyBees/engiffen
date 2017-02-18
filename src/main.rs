extern crate engiffen;
extern crate image;
extern crate getopts;
// extern crate glob;

use std::{env, error, fmt, process};
use std::str::FromStr;
use std::fs::{read_dir, File};
use std::path::PathBuf;
// use glob::glob;
use getopts::Options;

use SourceImages::*;

#[derive(Debug)]
enum SourceImages {
    StartEnd(String, String),
    // Glob(String),
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
    // Glob(glob::PatternError),
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
            // ArgsError::Glob(ref err) => write!(f, "Glob pattern error: {}", err),
            ArgsError::Fps(_) => write!(f, "Unable to parse framerate as an integer"),
            ArgsError::ImageRange(ref s) => write!(f, "Incomplete range of images: {}", s),
        }
    }
}

impl error::Error for ArgsError {
    fn description(&self) -> &str {
        match *self {
            ArgsError::Parse(ref err) => err.description(),
            // ArgsError::Glob(ref err) => err.description(),
            ArgsError::Fps(ref err) => err.description(),
            ArgsError::ImageRange(_) => "Incomplete range of images",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ArgsError::Parse(ref err) => Some(err),
            // ArgsError::Glob(ref err) => Some(err),
            ArgsError::Fps(ref err) => Some(err),
            ArgsError::ImageRange(_) => None,
        }
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
        let brief = format!("Usage: {} <glob>", program);
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
            StartEnd(matches.free[0].clone(), matches.free[1].clone())
        } else if matches.free.len() == 1 {
            return Err(ArgsError::ImageRange("end filename".to_string()))
        } else {
            return Err(ArgsError::ImageRange("start and end filenames".to_string()))
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
        // Glob(pattern) => {
        //     // already tested pattern validity in parse_args()
        //     glob(&pattern).unwrap()
        //     .filter(|e| {
        //         e.is_ok()
        //     })
        //     .map(|e| e.unwrap())
        //     .collect()
        // },
        StartEnd(start_pattern, end_pattern) => {
            let start_path = PathBuf::from(&start_pattern);
            let end_path = PathBuf::from(&end_pattern);
            read_dir(".").unwrap()
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

    let mut out = File::create(args.out_file).unwrap();
    engiffen::engiffen(&imgs, args.fps, &mut out);
}
