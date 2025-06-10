#![allow(clippy::incompatible_msrv)]
// ^need 1.66 for `black_box`

use grok::Grok;

fn main() {
    divan::main();
}

#[divan::bench]
fn r#match(b: divan::Bencher) {
    let mut grok = Grok::empty();
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
    let pattern = grok
        .compile("%{USERNAME}", false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against("user") {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match(b: divan::Bencher) {
    let mut grok = Grok::empty();
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
    let pattern = grok
        .compile("%{USERNAME}", false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against("$$$$") {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn match_anchor(b: divan::Bencher) {
    let mut grok = Grok::empty();
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
    let pattern = grok
        .compile("^%{USERNAME}$", false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against("user") {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match_anchor(b: divan::Bencher) {
    let mut grok = Grok::empty();
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
    let pattern = grok
        .compile("^%{USERNAME}$", false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against("$$$$") {
            divan::black_box(&found);
        }
    });
}
