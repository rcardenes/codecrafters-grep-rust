use anyhow::{bail, Result};

#[derive(Debug)]
pub enum RegexPattern {
    Char(char),
    AlphaNum,
    Digit,
    CharGroup((Vec<char>, bool)),
    Sequence(Vec<RegexPattern>),
}

impl RegexPattern {
    pub fn parse(pattern: &str) -> Result<Self> {
        let mut seq = vec![];
        let mut stream = pattern.chars();

        while let Some(chr) = stream.next() {
            match chr {
                '\\' => {
                    match stream.next() {
                        Some('d') => seq.push(RegexPattern::Digit),
                        Some('w') => seq.push(RegexPattern::AlphaNum),
                        Some(chr) => seq.push(RegexPattern::Char(chr)),
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
                                seq.push(RegexPattern::CharGroup((set, polarity)));
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
                _ => {
                    seq.push(RegexPattern::Char(chr))
                }
            }
        };

        Ok(RegexPattern::Sequence(seq))
    }

    fn min_size(&self) -> usize {
        match self {
            RegexPattern::Sequence(seq) => {
                seq.iter().fold(0, |acc, item| { acc + item.min_size() } )
            }
            _ => 1,
        }
    }

    fn matches(&self, haystack: &str) -> (bool, usize) {
        let mut it = haystack.chars();
        let result = match self {
            RegexPattern::Char(pat) => {
                it.next().is_some_and(|c| c == *pat)
            }
            RegexPattern::Digit => {
                it.next().is_some_and(|c| match c {
                    '0'..='9' => true,
                    _ => false
                })
            }
            RegexPattern::AlphaNum => {
                it.next().is_some_and(|c| match c {
                    '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => true,
                    _ => false
                })
            }
            RegexPattern::CharGroup((set, polarity)) => {
                it.next().is_some_and(|c| if set.contains(&c) {
                    *polarity
                } else {
                    !*polarity
                })
            }
            RegexPattern::Sequence(..) => todo!(),
        };

        (result, 1)
    }

    pub fn is_contained_in(&self, haystack: &str) -> Result<bool> {
        if self.min_size() > haystack.len() {
            return Ok(false)
        }
        match self {
            RegexPattern::Sequence(seq) => {
                for offset in 0..=(haystack.len() - self.min_size()) {
                    let mut valid = true;
                    let mut consumed = 0;

                    for pat in seq {
                        let (matches, length) = pat.matches(&haystack[(offset + consumed)..]);
                        if !matches {
                            valid = false;
                            break;
                        } else {
                            consumed += length;
                        }
                    }
                    if valid {
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