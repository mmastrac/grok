extern crate glob;

use glob::glob;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter;
use std::path::Path;

fn main() {
    let mut output = String::new();

    let mut lines = glob("patterns/*.pattern")
        .unwrap() // load filepaths
        // extract the filepath
        .map(|e| e.unwrap())
        // open file for path
        .map(|path| {
            (
                File::open(&path).unwrap(),
                path.file_stem().unwrap().to_os_string(),
            )
        })
        // flatten to actual lines
        .flat_map(|(f, path)| BufReader::new(f).lines().zip(iter::repeat(path)))
        .map(|(line, path)| (line.unwrap(), path))
        // filter comments
        .filter(|(line, _)| !line.starts_with('#'))
        // filter empty lines
        .filter(|(line, _)| !line.is_empty())
        // gather the key/value
        .map(|(line, path)| {
            let (a, b) = line.split_once(' ').unwrap();
            (a.to_string(), b.to_string(), path)
        })
        .collect::<Vec<_>>();
    lines.sort();

    fmt::write(
        &mut output,
        format_args!("static PATTERNS: &[(&str, &str)] = &[\n"),
    )
    .unwrap();

    for (key, value, _) in &lines {
        fmt::write(
            &mut output,
            format_args!("\t(\"{}\", r#\"{}\"#),\n", key, value),
        )
        .unwrap();
    }

    fmt::write(&mut output, format_args!("];\n")).unwrap();

    fmt::write(
        &mut output,
        format_args!("use std::borrow::Cow;\nstatic PATTERNS_COW: &[(Cow<'static, str>, Cow<'static, str>)] = &[\n"),
    )
    .unwrap();

    for (key, value, _) in &lines {
        fmt::write(
            &mut output,
            format_args!(
                "\t(Cow::Borrowed(\"{}\"), Cow::Borrowed(r#\"{}\"#)),\n",
                key, value
            ),
        )
        .unwrap();
    }

    fmt::write(&mut output, format_args!("];\n")).unwrap();

    lines.sort_by(|l1, l2| l1.2.cmp(&l2.2));

    fmt::write(&mut output, format_args!("#[doc=include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/patterns/README.md\"))] \npub mod patterns {{\n")).unwrap();
    for chunk in lines.chunk_by(|l1, l2| l1.2 == l2.2) {
        let name = chunk
            .first()
            .unwrap()
            .2
            .display()
            .to_string()
            .replace('-', "_");
        fmt::write(&mut output, format_args!("\npub mod {name} {{\n")).unwrap();
        for (key, value, _) in chunk {
            fmt::write(
                &mut output,
                format_args!("#[doc=r#\"`{value}`\"#] pub const {key}: &str = r#\"{value}\"#;\n"),
            )
            .unwrap();
        }
        fmt::write(&mut output, format_args!("\n}}\n")).unwrap();
    }
    fmt::write(&mut output, format_args!("\n}}\n")).unwrap();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("default_patterns.rs");
    let mut file = File::create(&dest_path).unwrap();
    file.write_all(output.as_bytes()).unwrap();
}
