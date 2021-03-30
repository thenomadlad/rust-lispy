#[macro_use]
extern crate clap;

pub mod ast;
pub mod parser;
pub mod tok;

use clap::AppSettings;
use parser::RecursiveDescentParser;
use std::fs::File;
use std::path::Path;
use tok::{GreedyTokenizer, Token, Tokenizer};

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

    let mut tokenizer =
        GreedyTokenizer::new(read_file(matches.value_of("INPUT").unwrap())).unwrap();

    // Tokenizer stuff
    if matches.subcommand_matches("tokenize").is_some() {
        let mut tabs = 0;
        loop {
            let char_and_position = tokenizer.get_token().unwrap();

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

            // if we encounter Eof, break
            if char_and_position.token == Token::Eof {
                break;
            }
        }
    }

    // Parser stuff
    if matches.subcommand_matches("parse").is_some() {
        let mut parser = RecursiveDescentParser::new(Box::new(tokenizer));
        loop {
            match parser.next_expression() {
                Ok(Some(something)) => println!("{:?}", something),
                Ok(None) => break,
                Err(err) => {
                    println!("{:?}", err);
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
