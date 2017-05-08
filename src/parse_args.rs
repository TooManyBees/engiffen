extern crate getopts;

use getopts::Options;
use std::path::{Path, PathBuf};
use std::{error, fmt};
use std::str::FromStr;
use std;

use self::SourceImages::*;

#[derive(Debug, PartialEq)]
pub enum SourceImages {
    StartEnd(PathBuf, PathBuf, PathBuf),
    List(Vec<String>),
    StdIn,
}

#[derive(Debug)]
pub struct Args {
    pub source: SourceImages,
    pub fps: usize,
    pub out_file: String,
}

#[derive(Debug, PartialEq)]
pub enum ArgsError {
    Parse(getopts::Fail),
    Fps(std::num::ParseIntError),
    ImageRange(String),
    DisplayHelp(String),
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
            ArgsError::DisplayHelp(ref msg) => write!(f, "{}", msg),
        }
    }
}

impl error::Error for ArgsError {
    fn description(&self) -> &str {
        match *self {
            ArgsError::Parse(ref err) => err.description(),
            ArgsError::Fps(ref err) => err.description(),
            ArgsError::ImageRange(_) => "Bad image range",
            ArgsError::DisplayHelp(_) => "Display help message"
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ArgsError::Parse(ref err) => Some(err),
            ArgsError::Fps(ref err) => Some(err),
            ArgsError::ImageRange(_) => None,
            ArgsError::DisplayHelp(_) => None,
        }
    }
}

pub fn parse_args(args: &[String]) -> Result<Args, ArgsError> {
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("o", "outfile", "engiffen to this filename", "FILE");
    opts.optopt("f", "framerate", "frames per second", "30");
    opts.optflag("r", "range", "arguments specify start and end images");
    opts.optflag("h", "help", "display this help");

    let matches = opts.parse(&args[1..])?;
    if matches.opt_present("h") {
        let brief = format!("Usage: {} <files ...>", program);
        return Err(ArgsError::DisplayHelp(opts.usage(&brief)));
    }

    let fps: usize = if let Some(fps_str) = matches.opt_str("f") {
        usize::from_str(&fps_str)?
    } else {
        30
    };

    let out_file = matches.opt_str("o").map(|f| f.clone()).unwrap_or("out.gif".to_string());
    let source = if matches.opt_present("r") {
        if matches.free.len() >= 2 {
            let (path_start, filename_start) = path_and_filename(&matches.free[0])?;
            let (path_end, filename_end) = path_and_filename(&matches.free[1])?;
            if path_start != path_end {
                return Err(ArgsError::ImageRange("start and end files are from different directories".to_string()));
            }
            StartEnd(path_start, filename_start, filename_end)
        } else if matches.free.len() == 1 {
            return Err(ArgsError::ImageRange("missing end filename".to_string()));
        } else {
            return Err(ArgsError::ImageRange("missing start and end filenames".to_string()));
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

fn path_and_filename(input: &str) -> Result<(PathBuf, PathBuf), ArgsError> {
    let p = Path::new(&input);
    let parent = match p.parent() {
        Some(s) => {
            if s == Path::new("") {
                Path::new(".")
            } else {
                s
            }
        },
        None => Path::new(".")
    };
    if let Some(filename) = p.file_name() {
        Ok((parent.to_owned(), PathBuf::from(filename)))
    } else {
        Err(ArgsError::ImageRange(format!("Invalid filename {:?}", input)))
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::{parse_args, SourceImages, ArgsError, Args};
    use std::path::PathBuf;

    fn make_args(args: &str) -> Vec<String> {
        args.split(" ").map(|s| s.to_owned()).collect()
    }

    fn assert_err_eq(actual: Result<Args, ArgsError>, expected: ArgsError) {
        assert!(actual.is_err());
        assert_eq!(actual.err().unwrap(), expected);
    }

    #[test]
    fn test_outfile() {
        let args = parse_args(&make_args("engiffen -o bees.gif"));
        assert!(args.is_ok());
        assert_eq!(args.unwrap().out_file, "bees.gif");
    }

    #[test]
    fn test_fps() {
        let args = parse_args(&make_args("engiffen -f 45"));
        assert!(args.is_ok());
        assert_eq!(args.unwrap().fps, 45);
    }

    #[test]
    fn test_fps_missing() {
        use std::str::FromStr;

        let args = parse_args(&make_args("engiffen -f barry"));
        let parse_error = usize::from_str("barry").err().unwrap();
        assert_err_eq(args, ArgsError::Fps(parse_error));
    }

    #[test]
    fn test_file_list() {
        let args = parse_args(&make_args("engiffen this.jpg that.jpg other.jpg"));
        assert!(args.is_ok());
        assert_eq!(
            args.unwrap().source,
            SourceImages::List(vec![
                "this.jpg".to_owned(),
                "that.jpg".to_owned(),
                "other.jpg".to_owned()
            ])
        );
    }

    #[test]
    fn test_file_range() {
        let args = parse_args(&make_args("engiffen -r thing001.jpg thing010.jpg"));
        assert!(args.is_ok());
        assert_eq!(
            args.unwrap().source,
            SourceImages::StartEnd(
                PathBuf::from("."),
                PathBuf::from("thing001.jpg"),
                PathBuf::from("thing010.jpg")
            )
        );
    }

    #[test]
    fn test_file_range_remote_directory() {
        let args = parse_args(&make_args("engiffen -r ../dir/thing001.jpg ../dir/thing010.jpg"));
        assert!(args.is_ok());
        assert_eq!(
            args.unwrap().source,
            SourceImages::StartEnd(
                PathBuf::from("../dir"),
                PathBuf::from("thing001.jpg"),
                PathBuf::from("thing010.jpg")
            )
        );
    }

    #[test]
    fn test_file_range_different_directories() {
        let args = parse_args(&make_args("engiffen -r ./thing001.jpg ../thing010.jpg"));
        assert_err_eq(args, ArgsError::ImageRange("start and end files are from different directories".to_string()));
    }

    #[test]
    fn test_file_range_incomplete() {
        let args = parse_args(&make_args("engiffen -r ./thing001.jpg"));
        assert_err_eq(args, ArgsError::ImageRange("missing end filename".to_string()));
    }

    #[test]
    fn test_file_range_missing() {
        let args = parse_args(&make_args("engiffen -r"));
        assert_err_eq(args, ArgsError::ImageRange("missing start and end filenames".to_string()));
    }

    #[test]
    fn test_help() {
        let args = parse_args(&make_args("engiffen -h"));
        // Such a long DisplayHelp message that will probably change as more
        // options get added. Just check the error's type instead.
        match args {
            Err(ArgsError::DisplayHelp(_)) => assert!(true),
            Err(_) => panic!("Wrong error type returned"),
            Ok(_) => panic!("Should not have returned an Ok args result"),
        }
    }
}
