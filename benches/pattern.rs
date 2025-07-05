#![allow(clippy::incompatible_msrv)]
// ^need 1.66 for `black_box`

use grok::Grok;

fn main() {
    divan::main();
}

#[divan::bench]
fn create_with_default_patterns(b: divan::Bencher) {
    let grok = Grok::with_default_patterns();
    divan::black_box(grok);
    b.bench(|| {
        let grok = Grok::with_default_patterns();
        divan::black_box(grok);
    });
}

#[divan::bench]
fn parse_complex_pattern(b: divan::Bencher) {
    let grok = Grok::with_default_patterns();
    b.bench(|| {
        let pattern = grok.compile("%{BACULA_LOGLINE}", false).unwrap();
        divan::black_box(pattern);
    });
}

#[divan::bench]
fn parse_complex_pattern_alias_only(b: divan::Bencher) {
    let grok = Grok::with_default_patterns();
    b.bench(|| {
        let pattern = grok.compile("%{BACULA_LOGLINE}", true).unwrap();
        divan::black_box(pattern);
    });
}
