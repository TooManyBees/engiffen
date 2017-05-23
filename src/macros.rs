macro_rules! printerr {
    ($($arg:tt)*) => (writeln!(&mut std::io::stderr(), $($arg)*).expect("failed to write to stderr"));
}
