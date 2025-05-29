//! The `grok` library allows you to quickly parse and match potentially unstructured data
//! into a structed result. It is especially helpful when parsing logfiles of all kinds. This
//! [Rust](http://rust-lang.org) version is mainly a port from the
//! [java version](https://github.com/thekrakken/java-grok)
//! which in drew inspiration from the original
//! [ruby version](https://github.com/logstash-plugins/logstash-filter-grok).
#![doc(html_root_url = "https://docs.rs/grok/2.0.0")]

include!(concat!(env!("OUT_DIR"), "/default_patterns.rs"));

use std::collections::{BTreeMap, HashMap};
use std::error::Error as StdError;
use std::fmt;

mod fancy_regex;
mod onig;
mod regex;

// If fancy-regex is enabled, we use it. Otherwise if onig is enabled, use that.
// Only use regex if it is enabled and no other feature is enabled.

#[cfg(feature = "fancy-regex")]
pub use fancy_regex::{
    FancyRegexMatches as Matches, FancyRegexMatchesIter as MatchesIter,
    FancyRegexPattern as Pattern,
};

#[cfg(all(feature = "onig", not(feature = "fancy-regex")))]
pub use onig::{OnigMatches as Matches, OnigMatchesIter as MatchesIter, OnigPattern as Pattern};

#[cfg(all(not(feature = "onig"), not(feature = "fancy-regex"), feature = "regex"))]
pub use regex::{
    RegexMatches as Matches, RegexMatchesIter as MatchesIter, RegexPattern as Pattern,
};

#[cfg(all(
    not(feature = "onig"),
    not(feature = "fancy-regex"),
    not(feature = "regex")
))]
compile_error!("No regex engine selected. Please enable one of the following features: fancy-regex, onig, regex");

const MAX_RECURSION: usize = 1024;

const GROK_PATTERN: &str = r"%\{(?<name>(?<pattern>[A-z0-9]+)(?::(?<alias>[A-z0-9_:;\/\s\.]+))?)(?:=(?<definition>(?:(?:[^{}]+|\.+)+)+))?\}";
const NAME_INDEX: usize = 1;
const PATTERN_INDEX: usize = 2;
const ALIAS_INDEX: usize = 3;
const DEFINITION_INDEX: usize = 4;

/// Returns the default patterns, also used by the default constructor of `Grok`.
pub fn patterns<'a>() -> &'a [(&'a str, &'a str)] {
    PATTERNS
}

/// The `Grok` struct is the main entry point into using this library.
#[derive(Debug)]
pub struct Grok {
    patterns: BTreeMap<String, String>,
}

impl Grok {
    /// Creates a new `Grok` instance with no patterns.
    pub fn empty() -> Self {
        Grok {
            patterns: BTreeMap::new(),
        }
    }

    /// Creates a new `Grok` instance and loads all the default patterns.
    pub fn with_default_patterns() -> Self {
        let mut grok = Grok::empty();
        for &(key, value) in PATTERNS {
            grok.add_pattern(String::from(key), String::from(value));
        }
        grok
    }

    /// Adds a custom pattern.
    pub fn add_pattern<S: Into<String>>(&mut self, name: S, pattern: S) {
        self.patterns.insert(name.into(), pattern.into());
    }

    /// Compiles the given pattern, making it ready for matching.
    pub fn compile(&mut self, pattern: &str, with_alias_only: bool) -> Result<Pattern, Error> {
        let mut named_regex = String::from(pattern);
        let mut alias: HashMap<String, String> = HashMap::new();

        let mut index = 0;
        let mut iteration_left = MAX_RECURSION;
        let mut continue_iteration = true;

        let grok_regex = match ::onig::Regex::new(GROK_PATTERN) {
            Ok(r) => r,
            Err(_) => return Err(Error::RegexCompilationFailed(GROK_PATTERN.into())),
        };

        while continue_iteration {
            continue_iteration = false;
            if iteration_left == 0 {
                return Err(Error::RecursionTooDeep);
            }
            iteration_left -= 1;

            if let Some(m) = grok_regex.captures(&named_regex.clone()) {
                continue_iteration = true;
                let raw_pattern = match m.at(PATTERN_INDEX) {
                    Some(p) => p,
                    None => {
                        return Err(Error::GenericCompilationFailure(
                            "Could not find pattern in matches".into(),
                        ))
                    }
                };

                let mut name = match m.at(NAME_INDEX) {
                    Some(n) => String::from(n),
                    None => {
                        return Err(Error::GenericCompilationFailure(
                            "Could not find name in matches".into(),
                        ))
                    }
                };

                if let Some(definition) = m.at(DEFINITION_INDEX) {
                    self.add_pattern(raw_pattern, definition);
                    name = format!("{}={}", name, definition);
                }

                // Since a pattern with a given name can show up more than once, we need to
                // loop through the number of matches found and apply the transformations
                // on each of them.
                for _ in 0..named_regex.matches(&format!("%{{{}}}", name)).count() {
                    // Check if we have a definition for the raw pattern key and fail quickly
                    // if not.
                    let pattern_definition = match self.patterns.get(raw_pattern) {
                        Some(d) => d,
                        None => return Err(Error::DefinitionNotFound(raw_pattern.into())),
                    };

                    // If no alias is specified and all but with alias are ignored,
                    // the replacement tells the regex engine to ignore the matches.
                    // Otherwise, the definition is turned into a regex that the
                    // engine understands and uses a named group.

                    let replacement = if with_alias_only && m.at(ALIAS_INDEX).is_none() {
                        format!("(?:{})", pattern_definition)
                    } else {
                        // If an alias is specified by the user use that one to
                        // match the name<index> conversion, otherwise just use
                        // the name of the pattern definition directly.
                        alias.insert(
                            match m.at(ALIAS_INDEX) {
                                Some(a) => a.into(),
                                None => name.clone(),
                            },
                            format!("name{}", index),
                        );

                        format!("(?<name{}>{})", index, pattern_definition)
                    };

                    // Finally, look for the original %{...} style pattern and
                    // replace it with our replacement (only the first occurrence
                    // since we are iterating one by one).
                    named_regex = named_regex.replacen(&format!("%{{{}}}", name), &replacement, 1);

                    index += 1;
                }
            }
        }

        if named_regex.is_empty() {
            Err(Error::CompiledPatternIsEmpty(pattern.into()))
        } else {
            Pattern::new(&named_regex, &alias)
        }
    }
}

/// The Default implementation for Grok whuich will load the default patterns.
impl Default for Grok {
    fn default() -> Grok {
        Grok::with_default_patterns()
    }
}

/// Allows to initialize Grok with an iterator of patterns.
///
/// Example:
/// ```rs
/// let patterns = [("USERNAME", r"[a-zA-Z0-9._-]+")];
/// let mut grok = Grok::from_iter(patterns.into_iter());
/// ```
impl<S: Into<String>> FromIterator<(S, S)> for Grok {
    fn from_iter<I: IntoIterator<Item = (S, S)>>(iter: I) -> Self {
        let mut grok = Grok::empty();
        for (k, v) in iter {
            grok.add_pattern(k, v);
        }
        grok
    }
}

/// Allows to construct Grok with an array of patterns directly.
///
/// Example:
/// ```rs
/// let mut grok = Grok::from([("USERNAME", r"[a-zA-Z0-9._-]+")]);
/// ```
impl<S: Into<String>, const N: usize> From<[(S, S); N]> for Grok {
    fn from(arr: [(S, S); N]) -> Self {
        Self::from_iter(arr)
    }
}

/// Errors that can occur when using this library.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// The recursion while compiling has exhausted the limit.
    RecursionTooDeep,
    /// After compiling, the resulting compiled regex pattern is empty.
    CompiledPatternIsEmpty(String),
    /// A corresponding pattern definition could not be found for the given name.
    DefinitionNotFound(String),
    /// If the compilation for a specific regex in the underlying engine failed.
    RegexCompilationFailed(String),
    /// Something is messed up during the compilation phase.
    GenericCompilationFailure(String),
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::RecursionTooDeep => "compilation recursion reached the limit",
            Error::CompiledPatternIsEmpty(_) => "compiled pattern is empty",
            Error::DefinitionNotFound(_) => "pattern definition not found while compiling",
            Error::RegexCompilationFailed(_) => "regex compilation in the engine failed",
            Error::GenericCompilationFailure(_) => {
                "something happened during the compilation phase"
            }
        }
    }

    fn cause(&self) -> Option<&dyn StdError> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::RecursionTooDeep => write!(
                f,
                "Recursion while compiling reached the limit of {}",
                MAX_RECURSION
            ),
            Error::CompiledPatternIsEmpty(ref p) => write!(
                f,
                "The given pattern \"{}\" ended up compiling into an empty regex",
                p
            ),
            Error::DefinitionNotFound(ref d) => write!(
                f,
                "The given pattern definition name \"{}\" could not be found in the definition map",
                d
            ),
            Error::RegexCompilationFailed(ref r) => write!(
                f,
                "The given regex \"{}\" failed compilation in the underlying engine",
                r
            ),
            Error::GenericCompilationFailure(ref d) => write!(
                f,
                "Something unexpected happened during the compilation phase: \"{}\"",
                d
            ),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_simple_anonymous_pattern() {
        let mut grok = Grok::empty();
        grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
        let pattern = grok
            .compile("%{USERNAME}", false)
            .expect("Error while compiling!");

        let matches = pattern.match_against("root").expect("No matches found!");
        assert_eq!("root", matches.get("USERNAME").unwrap());
        assert_eq!(1, matches.len());
        let matches = pattern
            .match_against("john doe")
            .expect("No matches found!");
        assert_eq!("john", matches.get("USERNAME").unwrap());
        assert_eq!(1, matches.len());
    }

    #[test]
    fn test_from_iter() {
        let patterns = [("USERNAME", r"[a-zA-Z0-9._-]+")];
        let mut grok = Grok::from_iter(patterns.into_iter());
        let pattern = grok
            .compile("%{USERNAME}", false)
            .expect("Error while compiling!");

        let matches = pattern.match_against("root").expect("No matches found!");
        assert_eq!("root", matches.get("USERNAME").unwrap());
        assert_eq!(1, matches.len());
        let matches = pattern
            .match_against("john doe")
            .expect("No matches found!");
        assert_eq!("john", matches.get("USERNAME").unwrap());
        assert_eq!(1, matches.len());
    }

    #[test]
    fn test_from() {
        let mut grok = Grok::from([("USERNAME", r"[a-zA-Z0-9._-]+")]);
        let pattern = grok
            .compile("%{USERNAME}", false)
            .expect("Error while compiling!");

        let matches = pattern.match_against("root").expect("No matches found!");
        assert_eq!("root", matches.get("USERNAME").unwrap());
        assert_eq!(1, matches.len());
        let matches = pattern
            .match_against("john doe")
            .expect("No matches found!");
        assert_eq!("john", matches.get("USERNAME").unwrap());
        assert_eq!(1, matches.len());
    }

    #[test]
    fn test_simple_named_pattern() {
        let mut grok = Grok::empty();
        grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
        let pattern = grok
            .compile("%{USERNAME:usr}", false)
            .expect("Error while compiling!");

        let matches = pattern.match_against("root").expect("No matches found!");
        assert_eq!("root", matches.get("usr").unwrap());
        assert_eq!(1, matches.len());
        let matches = pattern
            .match_against("john doe")
            .expect("No matches found!");
        assert_eq!("john", matches.get("usr").unwrap());
        assert_eq!(1, matches.len());
    }

    #[test]
    fn test_alias_anonymous_pattern() {
        let mut grok = Grok::empty();
        grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
        grok.add_pattern("USER", r"%{USERNAME}");
        let pattern = grok
            .compile("%{USER}", false)
            .expect("Error while compiling!");

        let matches = pattern.match_against("root").expect("No matches found!");
        assert_eq!("root", matches.get("USER").unwrap());
        let matches = pattern
            .match_against("john doe")
            .expect("No matches found!");
        assert_eq!("john", matches.get("USER").unwrap());
    }

    #[test]
    fn test_ailas_named_pattern() {
        let mut grok = Grok::empty();
        grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
        grok.add_pattern("USER", r"%{USERNAME}");
        let pattern = grok
            .compile("%{USER:usr}", false)
            .expect("Error while compiling!");

        let matches = pattern.match_against("root").expect("No matches found!");
        assert_eq!("root", matches.get("usr").unwrap());
        let matches = pattern
            .match_against("john doe")
            .expect("No matches found!");
        assert_eq!("john", matches.get("usr").unwrap());
    }

    #[test]
    fn test_composite_or_pattern() {
        let mut grok = Grok::empty();
        grok.add_pattern("MAC", r"(?:%{CISCOMAC}|%{WINDOWSMAC}|%{COMMONMAC})");
        grok.add_pattern("CISCOMAC", r"(?:(?:[A-Fa-f0-9]{4}\.){2}[A-Fa-f0-9]{4})");
        grok.add_pattern("WINDOWSMAC", r"(?:(?:[A-Fa-f0-9]{2}-){5}[A-Fa-f0-9]{2})");
        grok.add_pattern("COMMONMAC", r"(?:(?:[A-Fa-f0-9]{2}:){5}[A-Fa-f0-9]{2})");
        let pattern = grok
            .compile("%{MAC}", false)
            .expect("Error while compiling!");

        let matches = pattern
            .match_against("5E:FF:56:A2:AF:15")
            .expect("No matches found!");
        assert_eq!("5E:FF:56:A2:AF:15", matches.get("MAC").unwrap());
        assert_eq!(4, matches.len());
        let matches = pattern
            .match_against("hello! 5E:FF:56:A2:AF:15 what?")
            .expect("No matches found!");
        assert_eq!("5E:FF:56:A2:AF:15", matches.get("MAC").unwrap());
        assert_eq!(true, pattern.match_against("5E:FF").is_none());
    }

    #[test]
    fn test_multiple_patterns() {
        let mut grok = Grok::empty();
        grok.add_pattern("YEAR", r"(\d\d){1,2}");
        grok.add_pattern("MONTH", r"\b(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:tember)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)\b");
        grok.add_pattern("DAY", r"(?:Mon(?:day)?|Tue(?:sday)?|Wed(?:nesday)?|Thu(?:rsday)?|Fri(?:day)?|Sat(?:urday)?|Sun(?:day)?)");
        let pattern = grok
            .compile("%{DAY} %{MONTH} %{YEAR}", false)
            .expect("Error while compiling!");

        let matches = pattern
            .match_against("Monday March 2012")
            .expect("No matches found!");
        assert_eq!("Monday", matches.get("DAY").unwrap());
        assert_eq!("March", matches.get("MONTH").unwrap());
        assert_eq!("2012", matches.get("YEAR").unwrap());
        assert_eq!(None, matches.get("unknown"));
    }

    #[test]
    fn test_with_alias_only() {
        let mut grok = Grok::empty();
        grok.add_pattern("MAC", r"(?:%{CISCOMAC}|%{WINDOWSMAC}|%{COMMONMAC})");
        grok.add_pattern("CISCOMAC", r"(?:(?:[A-Fa-f0-9]{4}\.){2}[A-Fa-f0-9]{4})");
        grok.add_pattern("WINDOWSMAC", r"(?:(?:[A-Fa-f0-9]{2}-){5}[A-Fa-f0-9]{2})");
        grok.add_pattern("COMMONMAC", r"(?:(?:[A-Fa-f0-9]{2}:){5}[A-Fa-f0-9]{2})");
        let pattern = grok
            .compile("%{MAC:macaddr}", true)
            .expect("Error while compiling!");

        let matches = pattern
            .match_against("5E:FF:56:A2:AF:15")
            .expect("No matches found!");
        assert_eq!("5E:FF:56:A2:AF:15", matches.get("macaddr").unwrap());
        assert_eq!(1, matches.len());
        let matches = pattern
            .match_against("hello! 5E:FF:56:A2:AF:15 what?")
            .expect("No matches found!");
        assert_eq!("5E:FF:56:A2:AF:15", matches.get("macaddr").unwrap());
        assert_eq!(true, pattern.match_against("5E:FF").is_none());
    }

    #[test]
    fn test_match_iterator() {
        let mut grok = Grok::empty();
        grok.add_pattern("YEAR", r"(\d\d){1,2}");
        grok.add_pattern("MONTH", r"\b(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:tember)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)\b");
        grok.add_pattern("DAY", r"(?:Mon(?:day)?|Tue(?:sday)?|Wed(?:nesday)?|Thu(?:rsday)?|Fri(?:day)?|Sat(?:urday)?|Sun(?:day)?)");
        grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
        grok.add_pattern("SPACE", r"\s*");

        let pattern = grok
            .compile(
                "%{DAY:day} %{MONTH:month} %{YEAR:year}%{SPACE}%{USERNAME:user}?",
                true,
            )
            .expect("Error while compiling!");
        let matches = pattern
            .match_against("Monday March 2012")
            .expect("No matches found!");
        let mut found = 0;
        for (k, v) in matches.iter() {
            match k {
                "day" => assert_eq!("Monday", v),
                "month" => assert_eq!("March", v),
                "year" => assert_eq!("2012", v),
                e => panic!("{:?}", e),
            }
            found += 1;
        }
        assert_eq!(3, found);
    }

    #[test]
    fn test_matches_into_iter() {
        let mut grok = Grok::empty();
        grok.add_pattern("YEAR", r"(\d\d){1,2}");
        grok.add_pattern("MONTH", r"\b(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:tember)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)\b");
        grok.add_pattern("DAY", r"(?:Mon(?:day)?|Tue(?:sday)?|Wed(?:nesday)?|Thu(?:rsday)?|Fri(?:day)?|Sat(?:urday)?|Sun(?:day)?)");
        grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
        grok.add_pattern("SPACE", r"\s*");

        let pattern = grok
            .compile(
                "%{DAY:day} %{MONTH:month} %{YEAR:year}%{SPACE}%{USERNAME:user}?",
                true,
            )
            .expect("Error while compiling!");
        let matches = pattern
            .match_against("Monday March 2012")
            .expect("No matches found!");
        let mut found = 0;
        for (k, v) in &matches {
            match k {
                "day" => assert_eq!("Monday", v),
                "month" => assert_eq!("March", v),
                "year" => assert_eq!("2012", v),
                e => panic!("{:?}", e),
            }
            found += 1;
        }
        assert_eq!(3, found);
    }

    #[test]
    fn test_loaded_default_patterns() {
        let mut grok = Grok::with_default_patterns();
        let pattern = grok
            .compile("%{DAY} %{MONTH} %{YEAR}", false)
            .expect("Error while compiling!");

        let matches = pattern
            .match_against("Monday March 2012")
            .expect("No matches found!");
        assert_eq!("Monday", matches.get("DAY").unwrap());
        assert_eq!("March", matches.get("MONTH").unwrap());
        assert_eq!("2012", matches.get("YEAR").unwrap());
        assert_eq!(None, matches.get("unknown"));
    }

    #[test]
    fn test_compilation_of_all_default_patterns() {
        let mut grok = Grok::default();
        let mut num_checked = 0;
        for &(key, _) in PATTERNS {
            let pattern = format!("%{{{}}}", key);
            grok.compile(&pattern, false).expect(&format!(
                "Pattern {} key {} failed to compile!",
                pattern, key
            ));
            num_checked += 1;
        }
        assert!(num_checked > 0);
    }

    #[test]
    fn test_adhoc_pattern() {
        let mut grok = Grok::default();
        let pattern = grok
            .compile(r"\[(?<threadname>[^\]]+)\]", false)
            .expect("Error while compiling!");

        let matches = pattern
            .match_against("[thread1]")
            .expect("No matches found!");
        assert_eq!("thread1", matches.get("threadname").unwrap());
    }

    #[test]
    fn test_adhoc_pattern_in_iter() {
        let mut grok = Grok::default();
        let pattern = grok
            .compile(r"\[(?<threadname>[^\]]+)\]", false)
            .expect("Error while compiling!");

        let matches = pattern
            .match_against("[thread1]")
            .expect("No matches found!");
        let mut found = 0;
        for (k, v) in matches.iter() {
            assert_eq!("threadname", k);
            assert_eq!("thread1", v);
            found += 1;
        }
        assert_eq!(1, found);
    }

    #[test]
    fn test_capture_names() {
        let mut grok = Grok::empty();
        grok.add_pattern("YEAR", r"(\d\d){1,2}");
        grok.add_pattern("MONTH", r"\b(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:tember)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)\b");
        grok.add_pattern("DAY", r"(?:Mon(?:day)?|Tue(?:sday)?|Wed(?:nesday)?|Thu(?:rsday)?|Fri(?:day)?|Sat(?:urday)?|Sun(?:day)?)");
        grok.add_pattern("USERNAME", r"[a-zA-Z0-9._-]+");
        grok.add_pattern("SPACE", r"\s*");

        let pattern = grok
            .compile("%{YEAR}%{SPACE}%{USERNAME:user}?", false)
            .expect("Error while compiling!");

        let expected = vec!["SPACE", "YEAR", "user"];
        let actual = pattern.capture_names().collect::<Vec<_>>();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_capture_error() {
        let mut grok = Grok::with_default_patterns();
        let pattern = grok
            .compile("Path: %{PATH}$", false)
            .expect("Error while compiling!");
        let matches = pattern
            .match_against("Path: /AAAAA/BBBBB/CCCCC/DDDDDDDDDDDDDD EEEEEEEEEEEEEEEEEEEEEEEE/");

        assert!(matches.is_none());
    }
}
