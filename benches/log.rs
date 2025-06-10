#![allow(clippy::incompatible_msrv)]
// ^need 1.66 for `black_box`

use grok::Grok;

fn main() {
    divan::main();
}

#[divan::bench]
fn r#match(b: divan::Bencher) {
    let msg = "2016-09-19T18:19:00 [8.8.8.8:prd] DEBUG this is an example log message";

    let mut grok = Grok::default();
    let pattern = grok.compile(r"%{TIMESTAMP_ISO8601:timestamp} \[%{IPV4:ip}:%{WORD:environment}\] %{LOGLEVEL:log_level} %{GREEDYDATA:message}", false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match(b: divan::Bencher) {
    let msg = "2016-09-19T18:19:00 [8.8.8.8:prd] DEBUG this is an example log message";

    let mut grok = Grok::default();
    let pattern = grok.compile(r"%{TIMESTAMP_ISO8601:timestamp} \[%{IPV4:ip};%{WORD:environment}\] %{LOGLEVEL:log_level} %{GREEDYDATA:message}", false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn match_anchor(b: divan::Bencher) {
    let msg = "2016-09-19T18:19:00 [8.8.8.8:prd] DEBUG this is an example log message";

    let mut grok = Grok::default();
    let pattern = grok.compile(r"^%{TIMESTAMP_ISO8601:timestamp} \[%{IPV4:ip}:%{WORD:environment}\] %{LOGLEVEL:log_level} %{GREEDYDATA:message}$", false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match_anchor(b: divan::Bencher) {
    let msg = "2016-09-19T18:19:00 [8.8.8.8;prd] DEBUG this is an example log message";

    let mut grok = Grok::default();
    let pattern = grok.compile(r"^%{TIMESTAMP_ISO8601:timestamp} \[%{IPV4:ip}:%{WORD:environment}\] %{LOGLEVEL:log_level} %{GREEDYDATA:message}$", false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}
