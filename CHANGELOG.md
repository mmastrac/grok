# Change Log

All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## 2.2.0 - 2025-07-04

 * Rewrote the pattern parsing to avoid using regular expressions, making all
   regular expression engine features optional (though at least one is still
   required).
 * Ensure correctness in capture names, even when using duplicate names
   generated from pattern names. These were previously unspecified:
   * (breaking) Duplicate pattern names result in matches with incrementing names in the format:
     `NAME`, `NAME[1]`, `NAME[2]`, etc.
   * (breaking) Duplicate pattern names from aliases are now guaranteed to be
     "last one wins".
 * (breaking) `Matches::len()` was removed as it was previously reporting the
   pattern name count. Use `Matches::iter().count()` instead.
 * (breaking) `Matches::is_empty()` was removed. Use `Matches::iter().count() == 0` instead.

## 2.1.0 - 2025-05-29

 * Add support for `pcre2` feature which is significantly faster than `onig`.
 * Add support for `regex` and `fancy_regex` features which allow for pure Rust
   operation.
 * Some default patterns were updated from upstream sources for better
   compatibility across engines.

## 2.0.2 - 2025-05-29

 * Workaround for an issue with `onig` where certain patterns could panic.

## 2.0.1 - Unreleased

 * Updated `onig` to `6.4`.

## 2.0.0 - 2022-06-07

 * Minimum Rust version is `1.56`, Rust Edition switched to 2021.
 * (breaking) Renamed `Grok::with_patterns()` to `Grok::with_default_patterns()`.
 * (breaking) Renamed `Grok::insert_definition` to `Grok::add_pattern` to stop mixing "definition" with "pattern".
 * (breaking) `Matches::iter()` is now only returning the matches and not also the other patters with an empty string as the value.
 * Added `IntoIter` for `&Matches` for more convenient match iteration (i.e. for loop).
 * Added `Pattern::capture_names` which returns the names the compiled pattern captures.
 * Updated `onig` to `6.3`.
 * `master` branch is now called `main`.

## 1.2.0 - 2021-03-21

 * Updated `onig` to `6.1`.
 * Allow to inspect the default built patterns. Thanks @magodo!
 * Use the non_exhaustive attribute on `Error` as suggested by clippy.

## 1.0.1 - 2019-10-31

 * Use `Regex::foreach_names` instead of `Regex::capture_names` to work on 32 bit platforms. Thanks @a-rodin! 

## 1.1.0 - 2019-10-30

 * Updated `onig` to `5.0`.
 * Use `Regex::foreach_names` instead of `Regex::capture_names` to work on 32 bit platforms. Thanks @a-rodin! 

## 1.0.0 - 2019-03-28

 * Updated `onig` to `4.3`.

## 0.5.0 - 2018-02-19

 * Updated `onig` to `3.1`.

## 0.4.1 - 2017-11-15

 * Fixed a bug where the named pattern on compilation is also accessible from the iterator.

## 0.4.0 - 2017-11-15

 * Allow to specify named patterns when compiling, without inserting the definition beforehand.

## 0.3.0 - 2017-09-13

 * `regex` has been switched to `onig` so we have full compatibility with all the other grok patterns.
 * Added `Grok::with_patterns()` which loads all the default patterns. `Grok::defalt()` also uses that now.
 * `iter()` is available on `Matches` which yields a `(&str, &str)` kv pair of match/result.

## 0.2.0 - 2017-09-06

 * Instead of panicing, all methods that could return a `Result<T, grok::Error>`.
 * `Grok::new()` has been renamed to `Grok::empty()` (or `Grok::default()`)
 * `is_empty()` is available in the `Matches` API to check if there are matches at all.
 * `len()` is available in the `Matches` API to get the total number of matches.

## 0.1.0 - 2017-09-05

 * Initial Release