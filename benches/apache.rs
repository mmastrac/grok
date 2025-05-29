#![feature(test)]

extern crate test;

use grok::Grok;
use test::Bencher;

#[bench]
fn bench_apache_log_match(b: &mut Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "GET /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" "Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)""#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}"#, false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against(msg) {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_apache_log_no_match_start(b: &mut Bencher) {
    let msg = r#"tash-scale11x/css/fonts/Roboto-Regular.ttf HTTP/1.1" 200 41820 "http://semicomplete.com/presentations/logs"#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}"#, false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against(msg) {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_apache_log_no_match_middle(b: &mut Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "111 /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" "Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)""#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}"#, false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against(msg) {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_apache_log_no_match_end(b: &mut Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "GET /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" 1"#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}"#, false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against(msg) {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_apache_log_match_anchor(b: &mut Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "GET /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" "Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)""#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"^%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}$"#, false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against(msg) {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_apache_log_no_match_start_anchor(b: &mut Bencher) {
    let msg = r#"tash-scale11x/css/fonts/Roboto-Regular.ttf HTTP/1.1" 200 41820 "http://semicomplete.com/presentations/logs"#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"^%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}$"#, false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against(msg) {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_apache_log_no_match_middle_anchor(b: &mut Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "111 /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" "Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)""#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"^%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}$"#, false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against(msg) {
            test::black_box(&found);
        }
    });
}

#[bench]
fn bench_apache_log_no_match_end_anchor(b: &mut Bencher) {
    let msg = r#"220.181.108.96 - - [13/Jun/2015:21:14:28 +0000] "GET /blog/geekery/xvfb-firefox.html HTTP/1.1" 200 10975 "-" 1"#;

    let mut grok = Grok::default();
    let pattern = grok.compile(r#"^%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{DATA:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} %{NUMBER:bytes} %{QS:referrer} %{QS:agent}$"#, false)
        .expect("Error while compiling!");

    b.iter(|| {
        if let Some(found) = pattern.match_against(msg) {
            test::black_box(&found);
        }
    });
}
