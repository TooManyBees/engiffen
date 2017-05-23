extern crate engiffen;
extern crate image;
extern crate getopts;

use std::io::{self, Write};
use std::{env, fmt, process};
use std::fs::{read_dir, File};
use std::path::PathBuf;
use std::time::{Instant, Duration};
use parse_args::{parse_args, Args, SourceImages};

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
    let source_images: Vec<_> = match args.source {
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
    };

    let imgs = engiffen::load_images(&source_images);

    let now = Instant::now();
    let gif = engiffen::engiffen(&imgs, args.fps, args.quantizer)?;
    match args.out_file {
        Some(ref filename) => {
            let mut file = File::create(filename)
                .map_err(|_| RuntimeError::Destination(filename.to_owned()))?;
            gif.write(&mut file)
        },
        None => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            gif.write(&mut handle)
        }
    }?;
    let duration = now.elapsed();
    Ok((args.out_file.clone(), duration))
}

fn main() {
    let arg_strings: Vec<String> = env::args().collect();
    let args = parse_args(&arg_strings).map_err(|e| {
        writeln!(&mut io::stderr(), "{}", e).expect("failed to write to stderr");
        process::exit(1);
    }).unwrap();

    match run_engiffen(&args) {
        Ok((file, duration)) => {
            let ms = duration.as_secs() * 1000 + duration.subsec_nanos() as u64 / 1000000;
            let filename = file.unwrap_or("to stdout".to_owned());
            writeln!(&mut io::stderr(), "Wrote {} in {} ms", filename, ms).expect("failed to write to stderr");
        },
        Err(e) => {
            writeln!(&mut io::stderr(), "{}", e).expect("failed to write to stderr");
            process::exit(1);
        },
    }
}
