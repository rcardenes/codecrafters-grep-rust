use std::env;
use std::io;
use std::process;
use grep_starter_rust::regex::RegexPattern;

// Usage: echo <input_text> | your_grep.sh -E <pattern>
fn main() {
    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();
    match RegexPattern::parse(&pattern) {
        Ok(pat) => {
            if pat.is_contained_in(&input_line) {
                process::exit(0)
            } else {
                process::exit(1)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(1)
        }
    }

}
