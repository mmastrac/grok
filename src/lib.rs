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

#[cfg(feature = "fancy-regex")]
mod fancy_regex;
#[cfg(feature = "onig")]
mod onig;
#[cfg(feature = "pcre2")]
mod pcre2;
#[cfg(feature = "regex")]
mod regex;

mod pattern_parser;

// Enable features in the following preferred order. If multiple features are
// enabled, the first one in the list is used.

// 0. pcre2
// 1. fancy-regex
// 3. onig
// 3. regex

#[doc(hidden)]
#[cfg(feature = "pcre2")]
use pcre2::{
    Pcre2Matches as MatchesInner, Pcre2MatchesIter as MatchesIterInner,
    Pcre2Pattern as InnerPattern,
};

#[doc(hidden)]
#[cfg(all(not(feature = "pcre2"), feature = "fancy-regex"))]
use fancy_regex::{
    FancyRegexMatches as MatchesInner, FancyRegexMatchesIter as MatchesIterInner,
    FancyRegexPattern as InnerPattern,
};

#[doc(hidden)]
#[cfg(all(not(feature = "pcre2"), not(feature = "fancy-regex"), feature = "onig"))]
use onig::{
    OnigMatches as MatchesInner, OnigMatchesIter as MatchesIterInner, OnigPattern as InnerPattern,
};

#[doc(hidden)]
#[cfg(all(
    not(feature = "pcre2"),
    not(feature = "fancy-regex"),
    not(feature = "onig"),
    feature = "regex"
))]
use regex::{
    RegexMatches as MatchesInner, RegexMatchesIter as MatchesIterInner,
    RegexPattern as InnerPattern,
};

use crate::pattern_parser::{grok_split, GrokComponent};

/// The `Pattern` represents a compiled regex, ready to be matched against arbitrary text.
pub struct Pattern {
    inner: InnerPattern,
    #[cfg(test)]
    text: String,
}

impl Pattern {
    /// Creates a new pattern from a raw regex string and an alias map to identify the
    /// fields properly.
    #[inline(always)]
    fn new(regex: &str, alias: HashMap<String, String>) -> Result<Self, Error> {
        let inner = InnerPattern::new(regex, &alias)?;
        Ok(Self {
            inner,
            #[cfg(test)]
            text: regex.to_string(),
        })
    }

    /// Matches this compiled `Pattern` against the text and returns the matches.
    #[inline(always)]
    pub fn match_against<'a>(&'a self, text: &'a str) -> Option<Matches<'a>> {
        Some(Matches {
            inner: self.inner.match_against(text)?,
        })
    }

    /// Returns all names this `Pattern` captures.
    #[inline(always)]
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.inner.capture_names()
    }
}

/// The `Matches` represent matched results from a `Pattern` against a provided text.
pub struct Matches<'a> {
    inner: MatchesInner<'a>,
}

impl<'a> Matches<'a> {
    /// Gets the value for the name (or) alias if found, `None` otherwise.
    #[inline(always)]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.inner.get(name)
    }

    /// Returns a tuple of key/value with all the matches found.
    #[inline(always)]
    pub fn iter(&'a self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.inner.iter()
    }

    /// Collects the matches into a collection supporting `FromIterator`.
    #[inline(always)]
    pub fn collect<O: FromIterator<(&'a str, &'a str)>>(&'a self) -> O {
        self.iter().collect()
    }

    /// Returns the number of matches.
    #[cfg(test)]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.iter().count()
    }
}

impl<'a> std::fmt::Debug for Matches<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let iter = self.inner.into_iter();
        f.debug_map().entries(iter).finish()
    }
}

impl<'a> IntoIterator for &'a Matches<'a> {
    type Item = (&'a str, &'a str);
    type IntoIter = MatchesIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        MatchesIter {
            inner: self.inner.into_iter(),
        }
    }
}

/// An `Iterator` over all matches, accessible via `Matches`.
pub struct MatchesIter<'a> {
    inner: MatchesIterInner<'a>,
}

impl<'a> Iterator for MatchesIter<'a> {
    type Item = (&'a str, &'a str);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[cfg(all(
    not(feature = "onig"),
    not(feature = "fancy-regex"),
    not(feature = "regex"),
    not(feature = "pcre2")
))]
compile_error!("No regex engine selected. Please enable one of the following features: fancy-regex, onig, regex");

const MAX_RECURSION: usize = 1024;

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
    ///
    /// Specify `with_alias_only` to only include the aliases in the matches
    /// rather that all named patterns.
    pub fn compile(&mut self, pattern: &str, with_alias_only: bool) -> Result<Pattern, Error> {
        let mut named_regex = String::with_capacity(pattern.len() * 4);
        let mut aliases: HashMap<String, String> = HashMap::new();
        let mut aliases_extra: HashMap<String, usize> = HashMap::new();

        let mut pattern_stack = Vec::with_capacity(16);

        pattern_stack.push(grok_split(pattern));
        let mut index = 0;
        let mut pattern_overrides = BTreeMap::new();

        while let Some(mut it) = pattern_stack.pop() {
            if let Some(next) = it.next() {
                pattern_stack.push(it);
                use GrokComponent::*;
                match next {
                    GrokPattern {
                        name,
                        alias,
                        #[expect(unused)]
                        capture, // TODO: use
                        definition,
                        ..
                    } => {
                        if !definition.is_empty() {
                            // We can cleverly reborrow the definition here because we know that
                            // the lifetime is compatible.
                            pattern_stack.push(grok_split(definition));
                            pattern_overrides.insert(name.to_string(), definition);
                        } else if let Some(pattern) = pattern_overrides.get(name) {
                            // Again, cleverly reborrow the pattern
                            pattern_stack.push(grok_split(*pattern));
                        } else {
                            let Some(pattern) = self.patterns.get(name) else {
                                return Err(Error::DefinitionNotFound(name.to_string()));
                            };
                            pattern_stack.push(grok_split(pattern));
                        }

                        if with_alias_only && alias.is_empty() {
                            named_regex.push_str("(?:");
                        } else {
                            let match_name = format!("_n_{index}");
                            index += 1;

                            let orig_key = if alias.is_empty() { name } else { alias };

                            let count = aliases_extra.entry(orig_key.to_string()).or_insert(0);
                            let key = if *count == 0 {
                                orig_key.to_string()
                            } else {
                                format!("{orig_key}[{count}]")
                            };
                            *count += 1;

                            // This is unlikely but will really mess things up if it happens.
                            if *count > 1 && aliases_extra.contains_key(&key) {
                                return Err(Error::GenericCompilationFailure(format!(
                                    "Alias {key} already exists"
                                )));
                            }

                            aliases.insert(match_name.clone(), key);

                            named_regex.push_str("(?<");
                            named_regex.push_str(&match_name);
                            named_regex.push('>');
                        }
                    }
                    RegularExpression { string, .. } => {
                        named_regex.push_str(string);
                    }
                    PatternError(e) => {
                        return Err(Error::GenericCompilationFailure(format!("{e:?}")));
                    }
                }
            } else {
                named_regex.push(')');
            }

            if pattern_stack.len() > MAX_RECURSION {
                return Err(Error::RecursionTooDeep);
            }
        }

        named_regex.pop();

        if named_regex.is_empty() {
            Err(Error::CompiledPatternIsEmpty(pattern.into()))
        } else {
            Pattern::new(&named_regex, aliases)
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
    fn test_alias_named_pattern() {
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
        assert_eq!(2, matches.len());
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
        assert_eq!(
            pattern.capture_names().collect::<Vec<_>>(),
            vec!["DAY", "MONTH", "YEAR"]
        );

        let matches = pattern
            .match_against("Monday March 2012")
            .expect("No matches found!");
        assert_eq!(matches.len(), 3);
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
            .match_against("Monday March 2012 user")
            .expect("No matches found!");
        assert_eq!(matches.len(), 4);
        let mut found = 0;
        for (k, v) in matches.iter() {
            match k {
                "day" => assert_eq!("Monday", v),
                "month" => assert_eq!("March", v),
                "year" => assert_eq!("2012", v),
                "user" => assert_eq!("user", v),
                e => panic!("{:?}", e),
            }
            found += 1;
        }
        assert_eq!(4, found);
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
            .match_against("Monday March 2012 username")
            .expect("No matches found!");
        assert_eq!(matches.len(), 4);
        let mut found = 0;
        for (k, v) in &matches {
            match k {
                "day" => assert_eq!("Monday", v),
                "month" => assert_eq!("March", v),
                "year" => assert_eq!("2012", v),
                "user" => assert_eq!("username", v),
                e => panic!("{:?}", e),
            }
            found += 1;
        }
        assert_eq!(4, found);
    }

    #[cfg(not(feature = "regex"))]
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

    #[cfg(not(feature = "regex"))]
    #[test]
    fn test_compilation_of_all_default_patterns() {
        let mut grok = Grok::default();
        let mut num_checked = 0;
        let mut errors = vec![];
        for &(key, _) in PATTERNS {
            let pattern = format!("%{{{}}}", key);
            match grok.compile(&pattern, false) {
                Ok(_) => (),
                Err(e) => errors.push((key, e)),
            }
            num_checked += 1;
        }
        assert!(num_checked > 0);
        if !errors.is_empty() {
            for (key, e) in errors {
                eprintln!("Pattern {} failed to compile: {}", key, e);
            }
            panic!("Not all patterns compiled successfully");
        }
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
        assert_eq!(matches.len(), 1);
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
        assert_eq!(matches.len(), 1);
        for (k, v) in matches.iter() {
            assert_eq!("threadname", k);
            assert_eq!("thread1", v);
            found += 1;
        }
        assert_eq!(1, found);
    }

    /// If multiple captures have the same name, the last one wins.
    #[test]
    fn test_adhoc_pattern_conflict() {
        let mut grok = Grok::with_default_patterns();
        let pattern = grok
            .compile(r"(?<capture>\w+) %{GREEDYDATA:capture}", true)
            .unwrap();
        assert_eq!(vec!["capture"], Vec::from_iter(pattern.capture_names()));
        let matches = pattern.match_against("word1 word2").unwrap();
        assert_eq!("word2", matches.get("capture").unwrap());
    }

    #[test]
    fn test_capture_repeat() {
        let mut grok = Grok::with_default_patterns();
        let pattern = grok.compile(r"%{INT}{1,3}", false).unwrap();
        let matches = pattern.match_against("+1+2+3").unwrap();
        assert_eq!("+3", matches.get("INT").unwrap());
    }

    #[test]
    fn test_pattern_with_definition() {
        let mut grok = Grok::default();
        let pattern = grok
            .compile(r"%{NEW_PATTERN:first=\w+} %{NEW_PATTERN:second}", false)
            .unwrap();
        let matches = pattern.match_against("word1 word2").unwrap();
        assert_eq!("word1", matches.get("first").unwrap());
        assert_eq!("word2", matches.get("second").unwrap());
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

    #[cfg(not(feature = "regex"))]
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

    #[cfg(not(feature = "regex"))]
    #[test]
    fn test_match_deep_patterns() {
        let mut grok = Grok::with_default_patterns();
        let pattern = grok
            .compile("%{BACULA_LOGLINE}", false)
            .expect("Error while compiling!");

        let capture_names = pattern.capture_names().collect::<Vec<_>>();
        assert_eq!(163, capture_names.len());
        eprintln!("{capture_names:?}");
        assert!(
            !capture_names.iter().any(|s| s.starts_with("name")),
            "Found a name<n> in {capture_names:?}"
        );

        // %{BACULA_TIMESTAMP:bts} %{BACULA_HOST:hostname} JobId %{INT:jobid}: (%{BACULA_LOG_BEGIN_PRUNE_FILES})

        let log_line = "03-Jan 11:22 HostName JobId 1234: Begin pruning Files.";
        let matches = pattern.match_against(log_line).unwrap();
        assert_eq!("03-Jan 11:22", matches.get("bts").unwrap());
        assert_eq!("HostName", matches.get("hostname").unwrap());
        assert_eq!("1234", matches.get("jobid").unwrap());

        assert_eq!(
            "Begin pruning Files.",
            matches.get("BACULA_LOG_BEGIN_PRUNE_FILES").unwrap()
        );
        assert_eq!(
            "03-Jan 11:22 HostName JobId 1234: Begin pruning Files.",
            matches.get("BACULA_LOGLINE").unwrap()
        );
        assert_eq!("03", matches.get("MONTHDAY").unwrap());
        assert_eq!("Jan", matches.get("MONTH").unwrap());

        assert_eq!(None, matches.get("BACULA_LOG_END_VOLUME"));
        assert_eq!(None, matches.get("doesn't exist"));

        let matches = HashMap::<&str, &str>::from_iter(matches.iter());
        assert_eq!(9, matches.len());

        // BACULA_LOG_END_VOLUME End of medium on Volume \"%{BACULA_VOLUME:volume}\" Bytes=%{BACULA_CAPACITY} Blocks=%{BACULA_CAPACITY} at %{MONTHDAY}-%{MONTH}-%{YEAR} %{HOUR}:%{MINUTE}.

        let log_line = "03-Feb 11:22 HostName JobId 1234: End of medium on Volume \"Volume1\" Bytes=1000000000 Blocks=1000000 at 01-Mar-2026 01:02.";
        let matches = pattern.match_against(log_line).unwrap();
        eprintln!("{:?}", matches);
    }

    #[cfg(not(feature = "regex"))]
    #[test]
    fn test_compile_deep_patterns() {
        let mut grok = Grok::with_default_patterns();
        let pattern = grok
            .compile("%{BACULA_LOGLINE}", false)
            .expect("Error while compiling!");

        assert_eq!(pattern.text, include_str!("../testdata/BACULA_LOGLINE"));

        let pattern = grok
            .compile("%{BACULA_LOGLINE}", true)
            .expect("Error while compiling!");

        assert_eq!(
            pattern.text,
            include_str!("../testdata/BACULA_LOGLINE.aliasesonly")
        );

        let pattern = grok
            .compile("%{ELB_ACCESS_LOG}", false)
            .expect("Error while compiling!");

        assert_eq!(pattern.text, include_str!("../testdata/ELB_ACCESS_LOG"));
    }
}
