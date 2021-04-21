#[macro_use]
extern crate clap;

pub mod ast;
pub mod parser;
pub mod tok;

use clap::AppSettings;
use parser::RecursiveDescentParser;
use std::fs::File;
use std::path::Path;
use tok::{GreedyTokenizer, Token};

fn main() {
    let matches = clap_app!(lispy =>
        (version: "1.0")
        (author: "ocamlmycaml")
        (about: "Runs a limited subset of clojure")
        (@arg INPUT: +required "Sets the input file to use")
        (@subcommand tokenize =>
            (about: "Tokenize the file and print out the tokens")
        )
        (@subcommand parse =>
            (about: "Parse the file and print out the ASTs")
        )
    )
    .setting(AppSettings::SubcommandRequiredElseHelp)
    .get_matches();

    // Tokenizer stuff
    if matches.subcommand_matches("tokenize").is_some() {
        let tokenizer =
            GreedyTokenizer::new(read_file(matches.value_of("INPUT").unwrap())).unwrap();
        let mut tabs = 0;

        for token in tokenizer {
            let char_and_position = token.unwrap();

            // if we encounter a ), reduce tabs before printing
            if char_and_position.token == Token::CloseParen {
                tabs -= 1;
            }

            println!(
                "{}{}",
                (0..tabs).into_iter().map(|_| '\t').collect::<String>(),
                char_and_position
            );

            // if we encounter a (, increase tabs
            if char_and_position.token == Token::OpenParen {
                tabs += 1;
            }
        }
    }

    // Parser stuff
    if matches.subcommand_matches("parse").is_some() {
        let tokenizer =
            GreedyTokenizer::new(read_file(matches.value_of("INPUT").unwrap())).unwrap();
        let mut parser = RecursiveDescentParser::new(Box::new(tokenizer));

        loop {
            match parser.next_expression() {
                Ok(Some(something)) => println!("{:?}", something),
                Ok(None) => break,
                Err(err) => {
                    println!("Err: {:?}", err);
                    break;
                }
            }
        }
    }
}

fn read_file(file_path: &str) -> File {
    let path = Path::new(file_path);
    let display = path.display();

    // Open the path in read-only mode, returns `io::Result<File>`
    match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    }
}
