pub enum RegexPattern {
    Char(char),
    Digit,
    Empty,
}

impl RegexPattern {
    pub fn is_contained_in(&self, haystack: &str) -> bool {
        match self {
            RegexPattern::Char(pat) => {
                haystack.contains(*pat)
            }
            RegexPattern::Digit => {
                let digits = '0'..='9';
                let mut chars = haystack.chars();

                while let Some(chr) = chars.next() {
                    if digits.contains(&chr) {
                        return true
                    }
                }
                false
            }
            RegexPattern::Empty => true
        }
    }
}