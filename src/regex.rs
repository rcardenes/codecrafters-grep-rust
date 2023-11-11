use std::str::Chars;
use anyhow::{bail, Result};
use itertools::Itertools;

#[derive(Debug)]
pub enum RegexClass {
    Char(char),
    AlphaNum,
    Digit,
    Wildcard,
    CharGroup((Vec<char>, bool)),
    OneOrMore(Box<RegexClass>),
    Optional(Box<RegexClass>),
    Sequence(Vec<RegexClass>),
    Alternation(Vec<RegexClass>),
    OneOrMorePlaceholder,
    OptionalPlaceholder,
    Empty,
}

macro_rules! simple_match {
    ($expression:expr) => {
        if $expression {
            (true, 1)
        } else {
            (false, 0)
        }
    };
}

impl RegexClass {
    fn min_size(&self) -> Result<usize> {
        Ok(match self {
            RegexClass::Sequence(seq) => {
                seq.iter()
                    .map(|item| item.min_size())
                    .fold_ok(0, std::ops::Add::add)?
            }
            RegexClass::OneOrMore(pat) => pat.min_size()?,
            RegexClass::Alternation(seq) => {
                seq.iter()
                    .map(|item| item.min_size())
                    .fold_ok(std::usize::MAX, std::cmp::min)?
            }
            RegexClass::Optional(..) => 0,
            RegexClass::Wildcard |
            RegexClass::AlphaNum |
            RegexClass::Digit |
            RegexClass::Char(..) |
            RegexClass::CharGroup(..) => 1,
            RegexClass::Empty |
            RegexClass::OptionalPlaceholder |
            RegexClass::OneOrMorePlaceholder => bail!("placeholder values don't have a size")
        })
    }

    fn matches(&self, haystack: &str) -> Result<(bool, usize)>{
        Ok(match self {
            RegexClass::Char(pat) => {
                simple_match!(haystack.chars().next().is_some_and(|c| c == *pat))
            }
            RegexClass::Digit => {
                simple_match!(
                    haystack.chars().next().is_some_and(|c| match c {
                        '0'..='9' => true,
                        _ => false
                    })
                )
            }
            RegexClass::AlphaNum => {
                simple_match!(
                    haystack.chars().next().is_some_and(|c| match c {
                        '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => true,
                        _ => false
                    })
                )
            }
            RegexClass::Wildcard => {
                simple_match!(haystack.chars().next().is_some_and(|c| c != '\n'))
            }
            RegexClass::CharGroup((set, polarity)) => {
                simple_match!(
                    haystack.chars().next().is_some_and(|c| if set.contains(&c) {
                        *polarity
                    } else {
                        !*polarity
                    })
                )
            }
            RegexClass::OneOrMore(pat) => {
                let mut consumed = 0usize;

                loop {
                    let (matches, length) = pat.matches(&haystack[consumed..])?;
                    if !matches {
                        break
                    } else {
                        consumed += length
                    }
                }

                (consumed > 0, consumed)
            }
            RegexClass::Optional(pat) => {
                (true, pat.matches(haystack)?.1)
            }
            RegexClass::Sequence(seq) => {
                let mut consumed = 0usize;

                for pat in seq {
                    let (matches, length) = pat.matches(&haystack[consumed..])?;
                    if !matches {
                        return Ok((false, 0))
                    } else {
                        consumed += length;
                    }
                }

                (true, consumed)
            }
            RegexClass::Alternation(seq) => {
                for pat in seq {
                    let (matches, length) = pat.matches(&haystack)?;
                    if matches {
                        return Ok((true, length))
                    }
                }

                (false, 0)
            }
            RegexClass::Empty |
            RegexClass::OneOrMorePlaceholder |
            RegexClass::OptionalPlaceholder => bail!("placeholder class can't match anything")
        })
    }
}

fn len_no_newline(text: &str) -> usize {
    let mut index = text.len();
    while text[..index].ends_with('\n') {
        index -= 1
    }
    index
}

pub struct RegexPattern {
    at_start: bool,
    until_end: bool,
    sequence: RegexClass,
}

fn parse_fragment(chars: &mut Chars) -> Result<RegexClass> {
    if let Some(chr) = chars.next() {
        match chr {
            '\\' => {
                match chars.next() {
                    Some('d') => Ok(RegexClass::Digit),
                    Some('w') => Ok(RegexClass::AlphaNum),
                    Some(chr) => Ok(RegexClass::Char(chr)),
                    None => bail!("trailing backlash (\\)"),
                }
            }
            '[' => {
                let mut set = vec![];
                let mut polarity = true;
                while let Some(chr) = chars.next() {
                    match chr {
                        ']' => return Ok(RegexClass::CharGroup((set, polarity))),
                        '^' => if set.is_empty() { polarity = false } else { set.push('^') }
                        _ => if !set.contains(&chr) { set.push(chr) }
                    }
                }
                bail!("brackets ([ ]) not balanced")
            }
            '(' => {
                let mut alternatives = vec![];
                loop {
                    if let Ok((seq, stopped_at)) = parse_sequence(chars, "|)") {
                        alternatives.push(seq);
                        if stopped_at == Some(')') {
                            break;
                        }
                    } else {
                        bail!("parentheses not balanced")
                    }
                }
                Ok(RegexClass::Alternation(alternatives))
            }
            '+' => Ok(RegexClass::OneOrMorePlaceholder),
            '?' => Ok(RegexClass::OptionalPlaceholder),
            '.' => Ok(RegexClass::Wildcard),
            _ => Ok(RegexClass::Char(chr)),
        }
    } else {
        Ok(RegexClass::Empty)
    }
}

fn parse_sequence(chars: &mut Chars, stop: &str) -> Result<(RegexClass, Option<char>)> {
    let mut seq = vec![];
    loop {
        let next = parse_fragment(chars)?;
        match next {
            RegexClass::Empty => {
                if stop.len() == 0 {
                    return Ok((RegexClass::Sequence(seq), None));
                } else {
                    bail!("unexpected end of stream")
                }
            }
            RegexClass::Char(chr) => {
                if stop.contains(chr) {
                    return Ok((RegexClass::Sequence(seq), Some(chr)))
                } else {
                    seq.push(RegexClass::Char(chr))
                }
            }
            RegexClass::OptionalPlaceholder => {
                if let Some(pat) = seq.pop() {
                    seq.push(RegexClass::Optional(Box::new(pat)))
                } else {
                    bail!("repetition-operator operand invalid")
                }
            }
            RegexClass::OneOrMorePlaceholder => {
                if let Some(pat) = seq.pop() {
                    seq.push(RegexClass::OneOrMore(Box::new(pat)))
                } else {
                    bail!("repetition-operator operand invalid")
                }
            }
            pat => seq.push(pat),
        }
    }
}

impl RegexPattern {
    pub fn parse(pattern: &str) -> Result<Self> {
        let mut at_start = false;
        let mut until_end = false;

        let pattern = if pattern.starts_with('^') {
            at_start = true;
            &pattern[1..]
        } else { pattern };
        let pattern = if pattern.ends_with('$') {
            until_end = true;
            &pattern[..pattern.len()-1]
        } else { pattern };

        let (sequence, _) = parse_sequence(&mut pattern.chars(), "")?;
        Ok(RegexPattern {
            at_start,
            until_end,
            sequence
        })
    }

    pub fn is_contained_in(&self, haystack: &str) -> Result<bool> {
        let hlen = len_no_newline(haystack);
        let min_size = self.sequence.min_size()?;
        if min_size > hlen {
            return Ok(false)
        }

        if self.at_start {
            let (matches, length) = self.sequence.matches(haystack)?;
            if self.until_end {
                return Ok(matches && (length == hlen))
            } else {
                return Ok(matches)
            }
        }

        for offset in 0..=(hlen - min_size) {
            let (matches, length) = self.sequence.matches(&haystack[(offset)..])?;
            if matches {
                if self.until_end && length != (hlen - offset) {
                    continue
                }
                return Ok(true)
            }
        }
        Ok(false)
    }
}