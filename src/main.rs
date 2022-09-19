use std::{
    env,
    ffi::OsStr,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::translator::Translator;

extern crate nom;

mod ast;
mod parser;
mod translator;

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() > 1, "Usage: {} <codefile.vm>", args[0]);
    let filename = &args[1];
    let mut file = File::open(filename).expect(&format!("File not found: {}", filename));
    let mut data = String::new();
    file.read_to_string(&mut data)
        .expect(&format!("Error while reading file: {}", filename));

    let ast = parser::parse(&data);

    let assembly = Path::new(filename)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap();
    let translation = Translator::new(assembly).translate(&ast);

    let outfilename = filename
        .rsplit_once('.')
        .map(|(fname, _)| fname)
        .unwrap_or(filename)
        .to_string()
        + ".asm";
    let mut outfile = File::create(outfilename).unwrap();

    for instruction in translation {
        writeln!(outfile, "{}", instruction).unwrap();
    }
}
