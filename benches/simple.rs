#![feature(test)]

extern crate test;

use grok::Grok;
use test::Bencher;

#[bench]
fn bench_simple_pattern_match(b: &mut Bencher) {
    let mut grok = Grok::empty();
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
    let pattern = grok
        .compile("%{USERNAME}", false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against("user") {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_simple_pattern_no_match(b: &mut Bencher) {
    let mut grok = Grok::empty();
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
    let pattern = grok
        .compile("%{USERNAME}", false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against("$$$$") {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_simple_pattern_match_with_anchor(b: &mut Bencher) {
    let mut grok = Grok::empty();
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
    let pattern = grok
        .compile("^%{USERNAME}$", false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against("user") {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_simple_pattern_no_match_with_anchor(b: &mut Bencher) {
    let mut grok = Grok::empty();
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
    let pattern = grok
        .compile("^%{USERNAME}$", false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against("$$$$") {
            test::black_box(&found);
        }
    });
}
