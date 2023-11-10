use anyhow::{bail, Result};

#[derive(Debug)]
pub enum RegexClass {
    Char(char),
    AlphaNum,
    Digit,
    CharGroup((Vec<char>, bool)),
    OneOrMore(Box<RegexClass>),
    Optional(Box<RegexClass>),
    Sequence(Vec<RegexClass>),
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
    fn min_size(&self) -> usize {
        match self {
            RegexClass::Sequence(seq) => {
                seq.iter().fold(0, |acc, item| { acc + item.min_size() } )
            }
            RegexClass::Optional(..) => 0,
            RegexClass::OneOrMore(pat) => pat.min_size(),
            _ => 1,
        }
    }

    fn matches(&self, haystack: &str) -> (bool, usize) {
        match self {
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
                    let (matches, length) = pat.matches(&haystack[consumed..]);
                    if !matches {
                        break
                    } else {
                        consumed += length
                    }
                }

                (consumed > 0, consumed)
            }
            RegexClass::Optional(pat) => {
                (true, pat.matches(haystack).1)
            }
            RegexClass::Sequence(seq) => {
                let mut consumed = 0usize;

                for pat in seq {
                    let (matches, length) = pat.matches(&haystack[consumed..]);
                    if !matches {
                        return (false, 0)
                    } else {
                        consumed += length;
                    }
                }

                (true, consumed)
            }
        }
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

impl RegexPattern {
    pub fn parse(pattern: &str) -> Result<Self> {
        let mut at_start = false;
        let mut until_end = false;
        let mut seq = vec![];

        let pattern = if pattern.starts_with('^') {
            at_start = true;
            &pattern[1..]
        } else { pattern };
        let pattern = if pattern.ends_with('$') {
            until_end = true;
            &pattern[..pattern.len()-1]
        } else { pattern };

        let mut stream = pattern.chars();

        while let Some(chr) = stream.next() {
            match chr {
                '\\' => {
                    match stream.next() {
                        Some('d') => seq.push(RegexClass::Digit),
                        Some('w') => seq.push(RegexClass::AlphaNum),
                        Some(chr) => seq.push(RegexClass::Char(chr)),
                        None => bail!("trailing backlash (\\)"),
                    }
                }
                '[' => {
                    let mut set = vec![];
                    let mut pos: usize = 0;
                    let mut polarity = true;
                    let mut closed = false;
                    while let Some(chr) = stream.next() {
                        match chr {
                            ']' => {
                                seq.push(RegexClass::CharGroup((set, polarity)));
                                closed = true;
                                break;
                            },
                            '^' => if pos == 0 { polarity = false } else { set.push('^') }
                            _ => if !set.contains(&chr) { set.push(chr) }
                        }
                        pos = pos + 1;
                    }
                    if !closed {
                        bail!("brackets ([ ]) not balanced")
                    }
                }
                '+' => {
                    if let Some(pat) = seq.pop() {
                        seq.push(RegexClass::OneOrMore(Box::new(pat)))
                    } else {
                        bail!("repetition-operator operand invalid")
                    }
                }
                '?' => {
                    if let Some(pat) = seq.pop() {
                        seq.push(RegexClass::Optional(Box::new(pat)))
                    } else {
                        bail!("repetition-operator operand invalid")
                    }
                }
                _ => {
                    seq.push(RegexClass::Char(chr))
                }
            }
        };

        Ok(RegexPattern {
            at_start,
            until_end,
            sequence: RegexClass::Sequence(seq)
        })
    }

    pub fn is_contained_in(&self, haystack: &str) -> Result<bool> {
        let hlen = len_no_newline(haystack);
        let min_size = self.sequence.min_size();
        if min_size > hlen {
            return Ok(false)
        }

        match &self.sequence {
            RegexClass::Sequence(..) => {
                if self.at_start {
                    let (matches, length) = self.sequence.matches(haystack);
                    if self.until_end {
                        return Ok(matches && (length == hlen))
                    } else {
                        return Ok(matches)
                    }
                }

                for offset in 0..=(hlen - min_size) {
                    let (matches, length) = self.sequence.matches(&haystack[(offset)..]);
                    if matches {
                        if self.until_end && length != (hlen - offset) {
                            continue
                        }
                        return Ok(true)
                    }
                }
                Ok(false)
            }
            _ => {
                bail!("Top pattern must be a sequence")
            }
        }
    }
}