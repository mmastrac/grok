#![allow(clippy::incompatible_msrv)]
// ^need 1.66 for `black_box`

use grok::Grok;

fn main() {
    divan::main();
}

#[divan::bench]
fn r#match(b: divan::Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "GET /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" "Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)""#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}"#, false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match_start(b: divan::Bencher) {
    let msg = r#"tash-scale11x/css/fonts/Roboto-Regular.ttf HTTP/1.1" 200 41820 "http://semicomplete.com/presentations/logs"#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}"#, false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match_middle(b: divan::Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "111 /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" "Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)""#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}"#, false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match_end(b: divan::Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "GET /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" 1"#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}"#, false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn match_anchor(b: divan::Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "GET /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" "Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)""#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"^%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}$"#, false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match_start_anchor(b: divan::Bencher) {
    let msg = r#"tash-scale11x/css/fonts/Roboto-Regular.ttf HTTP/1.1" 200 41820 "http://semicomplete.com/presentations/logs"#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"^%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}$"#, false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match_middle_anchor(b: divan::Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "111 /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" "Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)""#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"^%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}$"#, false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}

#[divan::bench]
fn no_match_end_anchor(b: divan::Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "GET /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" 1"#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"^%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}$"#, false)
        .expect("Error while compiling!");

    b.bench(|| {
        if let Some(found) = pattern.match_against(msg) {
            divan::black_box(&found);
        }
    });
}
