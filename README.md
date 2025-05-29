grok
====

The `grok` library allows you to quickly parse and match potentially unstructured data into a structed result. It is especially helpful when parsing logfiles of all kinds. This [Rust](http://rust-lang.org) version is mainly a port from the [java version](https://github.com/thekrakken/java-grok) which in turn drew inspiration from the original [ruby version](https://github.com/logstash-plugins/logstash-filter-grok).

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Latest Version](https://img.shields.io/crates/v/grok.svg)](https://crates.io/crates/grok)
[![Documentation](https://docs.rs/grok/badge.svg)](https://docs.rs/grok)
![Continuous Integration](https://github.com/mmastrac/grok/actions/workflows/ci.yml/badge.svg?branch=main)

## Usage
Add this to your `Cargo.toml`:

```toml
[dependencies]
grok = "2.0"
```

Here is a simple example which stores a pattern, compiles it and then matches a line on it:

```rust
use grok::Grok;

fn main() {
    // Instantiate Grok
    let mut grok = Grok::default();

    // Add a pattern which might be a regex or an alias
    grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");

    // Compile the definitions into the pattern you want
    let pattern = grok
        .compile("%{USERNAME}", false)
        .expect("Error while compiling!");

    //  Match the compiled pattern against a string
    match pattern.match_against("root") {
        Some(m) => println!("Found username {:?}", m.get("USERNAME")),
        None => println!("No matches found!"),
    }
}
```

Note that compiling the pattern is an expensive operation, so very similar to plain regex handling the `compile`
operation should be performed once and then the `match_against` method on the pattern can be called repeatedly
in a loop or iterator. The returned pattern is not bound to the lifetime of the original grok instance so it can
be passed freely around. For performance reasons the `Match` returned is bound to the pattern lifetime so keep
them close together or clone/copy out the containing results as needed.

## Further Information

This library supports multiple regex engines through feature flags. By default,
it uses [onig](https://crates.io/crates/onig), which is a Rust binding for the
powerful [Oniguruma](https://github.com/kkos/oniguruma) regex library. You can
also use the standard Rust regex engine or fancy-regex by enabling the
respective features:

The default engine, and at this time the most performant one, is `onig`:

```toml
[dependencies]
grok = { version = "2.0", features = ["onig"] }
```

The `fancy-regex` engine is a more complete Rust regex library supporting
backtracking:

```toml
[dependencies]
grok = { version = "2.0", default-features = false, features = ["fancy-regex"] }
```

The `regex` engine is supported, but it does not support backtracking, so many
patterns are unusable. This is not recommended for most use cases:

```toml
[dependencies]
grok = { version = "2.0", default-features = false, features = ["regex"] }
```

## License
`grok` is distributed under the terms of the Apache License (Version 2.0). 
See LICENSE for details.
