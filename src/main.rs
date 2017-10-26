extern crate engiffen;
extern crate image;
extern crate getopts;
extern crate rand;
#[cfg(feature = "globbing")] extern crate glob;

use std::io::{self, BufWriter};
use std::{env, fmt, process};
use std::fs::{read_dir, File};
use std::path::PathBuf;
use std::time::{Instant, Duration};
use parse_args::{parse_args, Args, SourceImages, Modifier};

#[cfg(feature = "globbing")] use self::glob::glob;

use rand::distributions::exponential::Exp1;
use rand::distributions::{IndependentSample, Range};

mod parse_args;

#[derive(Debug)]
enum RuntimeError {
    Directory(PathBuf),
    Destination(String),
    Engiffen(engiffen::Error),
}

impl From<engiffen::Error> for RuntimeError {
    fn from(err: engiffen::Error) -> RuntimeError {
        RuntimeError::Engiffen(err)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RuntimeError::Directory(ref dir) => write!(f, "No such directory {:?}", dir),
            RuntimeError::Destination(ref dst) => write!(f, "Couldn't write to output '{}'", dst),
            RuntimeError::Engiffen(ref e) => e.fmt(f,)
        }
    }
}

fn run_engiffen(args: &Args) -> Result<((Option<String>, Duration)), RuntimeError> {
    let mut source_images: Vec<_> = match args.source {
        SourceImages::StartEnd(ref dir, ref start_path, ref end_path) => {
            let start_string = start_path.as_os_str();
            let end_string = end_path.as_os_str();

            let mut files: Vec<_> = read_dir(dir)
                .map_err(|_| RuntimeError::Directory(dir.clone()))?
                .filter_map(|e| e.ok())
                .collect();

            // Filesystem probably already sorted by name, but just in case
            files.sort_by_key(|f| f.file_name());

            files.iter()
            .skip_while(|path| path.file_name() < start_string)
            .take_while(|path| path.file_name() <= end_string)
            .map(|e| e.path())
            .collect()
        },
        SourceImages::List(ref list) => list.into_iter().map(PathBuf::from).collect(),
        #[cfg(feature = "globbing")]
        SourceImages::Glob(ref string) => {
            let paths: Vec<_> = glob(string).expect("glob parsing failed :(")
                .filter_map(std::result::Result::ok)
                .collect();
            #[cfg(feature = "debug-stderr")]
            eprintln!("Expanded {} into {} files.", string, paths.len());
            paths
        },
    };

    modify(&mut source_images, &args.modifiers);

    let imgs = engiffen::load_images(&source_images);

    let now = Instant::now();
    let gif = engiffen::engiffen(&imgs, args.fps, args.quantizer)?;
    match args.out_file {
        Some(ref filename) => {
            let mut file = BufWriter::new(
                File::create(filename)
                .map_err(|_| RuntimeError::Destination(filename.to_owned()))?
            );
            gif.write(&mut file)
        },
        None => {
            let stdout = io::stdout();
            let mut handle = BufWriter::new(stdout.lock());
            gif.write(&mut handle)
        }
    }?;
    let duration = now.elapsed();
    Ok((args.out_file.clone(), duration))
}

fn main() {
    let arg_strings: Vec<String> = env::args().collect();
    let args = parse_args(&arg_strings).map_err(|e| {
        eprintln!("{}", e);
        process::exit(1);
    }).unwrap();

    match run_engiffen(&args) {
        Ok((file, duration)) => {
            let ms = duration.as_secs() * 1000 + duration.subsec_nanos() as u64 / 1000000;
            let filename = file.unwrap_or("to stdout".to_owned());
            eprintln!("Wrote {} in {} ms", filename, ms);
        },
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        },
    }
}

fn modify<P>(source_images: &mut [P], modifiers: &[Modifier]) {
    for modifier in modifiers {
        match *modifier {
            Modifier::Reverse => reverse(source_images),
            Modifier::Shuffle => shuffle(source_images),
        }
    }
}

fn reverse<T>(src: &mut [T]) {
    let last_index = src.len()-1;
    for n in 0..(src.len()/2) {
        src.swap(n, last_index-n);
    }
}

fn shuffle<T>(src: &mut [T]) {
    use std::cmp::{max, min};

    let mut rng = rand::thread_rng();

    let lenf = src.len() as f64;

    for n in 1..(src.len()) {
        let i = src.len() - n;
        let Exp1(e) = rand::random();
        let frame_weight = i as f64 / lenf;
        if e * frame_weight > 0.5 {
            let range = Range::new(max(i - i/2, 0), min(src.len() - 1, i + i/2));
            let j = range.ind_sample(&mut rng);
            src.swap(i, j);
        }
    }
}
