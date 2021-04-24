use std::fmt::Display;
use std::io::{self, Read};

const SPACE_CHAR: char = ' ';
const NEWLINE_CHAR: char = '\n';
const CARRIAGE_RETURN_CHAR: char = '\r';

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // standard symbols
    OpenParen,
    CloseParen,

    // reserved keywords
    Def,
    Fn,
    // If, // todo

    // more complex stuff
    Identifier(String),
    Number(f64),
    Unknown(char),
}

impl Token {
    fn from_str(string_value: &str) -> Option<Token> {
        match string_value {
            "def" => Some(Token::Def),
            "fn" => Some(Token::Fn),
            // "if" => Some(Token::If),
            _ => None,
        }
    }

    fn from_char(char_value: char) -> Option<Token> {
        match char_value {
            '+' => Some(Token::Identifier(String::from("+"))),
            '-' => Some(Token::Identifier(String::from("-"))),
            '*' => Some(Token::Identifier(String::from("*"))),
            '/' => Some(Token::Identifier(String::from("/"))),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Position {
    pub line: usize,
    pub position: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TokenAndSpan {
    pub token: Token,
    pub from: Position,
    pub to: Position,
}

impl Display for TokenAndSpan {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        if self.from == self.to {
            write!(
                formatter,
                "{:?}[line {} char {}]",
                self.token, self.from.line, self.from.position
            )
        } else {
            write!(
                formatter,
                "{:?}[line {} char {} -> line {} char {}]",
                self.token, self.from.line, self.from.position, self.to.line, self.to.position
            )
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct CharAndPosition {
    chr: Option<char>,
    line: usize,
    position: usize,
}

#[derive(Debug)]
pub enum TokenizerError {
    IoError(io::Error),
    ReadError {
        message: String,
        from: Position,
        to: Position,
    },
}

impl From<io::Error> for TokenizerError {
    fn from(io_error: io::Error) -> Self {
        Self::IoError(io_error)
    }
}

impl TokenizerError {
    fn from(
        text: String,
        from: Position,
        to: Position,
        float_parse_error: std::num::ParseFloatError,
    ) -> TokenizerError {
        TokenizerError::ReadError {
            message: format!("Unable to parse number '{}': {}", text, float_parse_error),
            from,
            to,
        }
    }
}

// hack: just get it working for tests
impl PartialEq for TokenizerError {
    fn eq(&self, rhs: &TokenizerError) -> bool {
        format!("{:?}", self) == format!("{:?}", rhs)
    }
}

pub trait Tokenizer: Iterator<Item = Result<TokenAndSpan, TokenizerError>> {}

impl<T: Iterator<Item = Result<TokenAndSpan, TokenizerError>>> Tokenizer for T {}

pub struct GreedyTokenizer<T>
where
    T: Read,
{
    inbuf: T,
    line: usize,
    position: usize,
    current_char: CharAndPosition,
}

impl<T> GreedyTokenizer<T>
where
    T: Read,
{
    pub fn new(inbuf: T) -> io::Result<Self> {
        let mut tok = GreedyTokenizer {
            inbuf,
            line: 1,
            position: 0,
            current_char: CharAndPosition {
                chr: None,
                line: 1,
                position: 0,
            },
        };

        // start it off
        tok.step_next_char()?;

        Ok(tok)
    }

    fn step_next_char(&mut self) -> io::Result<()> {
        let mut buffer: [u8; 1] = [0];
        let chars_read = self.inbuf.read(&mut buffer)?;

        if chars_read > 0 {
            let chr = buffer[0] as char;

            self.current_char = CharAndPosition {
                chr: Some(chr),
                line: self.line,
                position: self.position,
            };

            self.position += 1;
            if chr == '\n' || chr == '\r' {
                self.line += 1;
                self.position = 0;
            }
        } else {
            self.current_char = CharAndPosition {
                chr: None,
                line: self.line,
                position: self.position,
            };
        }

        Ok(())
    }

    fn fast_forward_comments_and_spaces(&mut self) -> Result<(), TokenizerError> {
        let start_tok = self.current_char;
        let mut tok = self.current_char;

        // remove any whitespace
        while tok.chr == Some(SPACE_CHAR)
            || tok.chr == Some(NEWLINE_CHAR)
            || tok.chr == Some(CARRIAGE_RETURN_CHAR)
        {
            self.step_next_char()?;
            tok = self.current_char;
        }

        // ignore comments - this could go to the end of the line
        if tok.chr == Some('#') {
            while tok.chr != Some(NEWLINE_CHAR)
                && tok.chr != Some(CARRIAGE_RETURN_CHAR)
                && tok.chr != None
            {
                self.step_next_char()?;
                tok = self.current_char;
            }
        }

        // if we ended up in a new line, we need to process more spaces
        if self.current_char != start_tok {
            self.fast_forward_comments_and_spaces()?;
        }

        Ok(())
    }

    fn move_to_next_token(&mut self) -> Result<Option<TokenAndSpan>, TokenizerError> {
        self.fast_forward_comments_and_spaces()?;

        let mut tok = self.current_char;

        // find parens
        if tok.chr == Some('(') {
            self.step_next_char()?;
            return Ok(Some(TokenAndSpan {
                token: Token::OpenParen,
                from: Position {
                    line: tok.line,
                    position: tok.position,
                },
                to: Position {
                    line: tok.line,
                    position: tok.position,
                },
            }));
        } else if tok.chr == Some(')') {
            self.step_next_char()?;
            return Ok(Some(TokenAndSpan {
                token: Token::CloseParen,
                from: Position {
                    line: tok.line,
                    position: tok.position,
                },
                to: Position {
                    line: tok.line,
                    position: tok.position,
                },
            }));
        }

        // recognize any identifiers
        if is_alphabetic(&tok) {
            let mut ident = String::new();
            let from = Position {
                line: tok.line,
                position: tok.position,
            };

            while is_identifier_like(&tok) {
                ident.push(tok.chr.unwrap());
                self.step_next_char()?;
                tok = self.current_char;
            }

            let to = Position {
                line: tok.line,
                position: tok.position - 1,
            };
            if let Some(reserved_token) = Token::from_str(&ident) {
                return Ok(Some(TokenAndSpan {
                    token: reserved_token,
                    from,
                    to,
                }));
            }

            return Ok(Some(TokenAndSpan {
                token: Token::Identifier(ident),
                from,
                to,
            }));
        }

        // recognizing any numeric things
        if is_number_like(&tok) {
            let mut numstr = String::new();
            let from = Position {
                line: tok.line,
                position: tok.position,
            };

            while is_number_like(&tok) {
                numstr.push(tok.chr.unwrap());
                self.step_next_char()?;
                tok = self.current_char;
            }
            let to = Position {
                line: tok.line,
                position: tok.position - 1,
            };

            match numstr.parse() {
                Ok(parsed) => {
                    return Ok(Some(TokenAndSpan {
                        token: Token::Number(parsed),
                        from,
                        to,
                    }))
                }
                Err(e) => return Err(TokenizerError::from(numstr, from, to, e)),
            }
        }

        // every other case is either a reserved char, EOF or simply an unknown char
        self.step_next_char()?;
        match tok.chr {
            Some(char_value) => match Token::from_char(char_value) {
                Some(token) => Ok(Some(TokenAndSpan {
                    token,
                    from: Position {
                        line: tok.line,
                        position: tok.position,
                    },
                    to: Position {
                        line: tok.line,
                        position: tok.position,
                    },
                })),
                None => Ok(Some(TokenAndSpan {
                    token: Token::Unknown(tok.chr.unwrap()),
                    from: Position {
                        line: tok.line,
                        position: tok.position,
                    },
                    to: Position {
                        line: tok.line,
                        position: tok.position,
                    },
                })),
            },
            None => Ok(None),
        }
    }
}

impl<T> Iterator for GreedyTokenizer<T>
where
    T: Read,
{
    type Item = Result<TokenAndSpan, TokenizerError>;

    fn next(&mut self) -> Option<Result<TokenAndSpan, TokenizerError>> {
        match self.move_to_next_token() {
            Ok(Some(item)) => Some(Ok(item)),
            Ok(None) => None,
            Err(item) => Some(Err(item)),
        }
    }
}

fn is_alphabetic(tok: &CharAndPosition) -> bool {
    if let Some(chr) = tok.chr {
        chr.is_alphabetic()
    } else {
        false
    }
}

fn is_identifier_like(tok: &CharAndPosition) -> bool {
    if let Some(chr) = tok.chr {
        chr.is_alphanumeric() || chr == '_'
    } else {
        false
    }
}

fn is_number_like(tok: &CharAndPosition) -> bool {
    if let Some(chr) = tok.chr {
        chr.is_numeric() || chr == '.'
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_handles_empty_buffer() -> Result<(), TokenizerError> {
        let inbuf = &b""[..];
        assert!(GreedyTokenizer::new(inbuf)?.next().is_none());

        let inbuf = &b"   "[..];
        assert!(GreedyTokenizer::new(inbuf)?.next().is_none());

        Ok(())
    }

    #[test]
    fn it_ignores_file_containing_only_comments() -> Result<(), TokenizerError> {
        let inbuf = &b"# blah"[..];
        assert!(GreedyTokenizer::new(inbuf)?.next().is_none());

        let inbuf = &b"  # blah"[..];
        assert!(GreedyTokenizer::new(inbuf)?.next().is_none());

        let mut handler = GreedyTokenizer::new(&b"  # only \n # comments"[..])?;
        assert!(handler.next().is_none());

        let mut handler = GreedyTokenizer::new(&b"  # only \r # comments"[..])?;
        assert!(handler.next().is_none());

        Ok(())
    }

    #[test]
    fn it_handles_parens() -> Result<(), TokenizerError> {
        let mut handler = GreedyTokenizer::new(&b"("[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::OpenParen,
                from: Position {
                    line: 1,
                    position: 0
                },
                to: Position {
                    line: 1,
                    position: 0
                }
            }
        );
        assert!(handler.next().is_none());

        let mut handler = GreedyTokenizer::new(&b"   ()  # whodat"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::OpenParen,
                from: Position {
                    line: 1,
                    position: 3
                },
                to: Position {
                    line: 1,
                    position: 3
                }
            }
        );
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::CloseParen,
                from: Position {
                    line: 1,
                    position: 4
                },
                to: Position {
                    line: 1,
                    position: 4
                }
            }
        );
        assert!(handler.next().is_none());

        Ok(())
    }

    #[test]
    fn it_handles_multiple_parens() -> Result<(), TokenizerError> {
        let mut handler = GreedyTokenizer::new(&b"(())"[..])?;

        // two open parens
        for position in 0..2 {
            assert_eq!(
                handler.next().unwrap()?,
                TokenAndSpan {
                    token: Token::OpenParen,
                    from: Position { line: 1, position },
                    to: Position { line: 1, position }
                }
            );
        }

        // two close parens
        for position in 2..4 {
            assert_eq!(
                handler.next().unwrap()?,
                TokenAndSpan {
                    token: Token::CloseParen,
                    from: Position { line: 1, position },
                    to: Position { line: 1, position }
                }
            );
        }

        // an eof at the end
        assert!(handler.next().is_none());

        Ok(())
    }

    #[test]
    fn it_handles_identifier_token() -> Result<(), TokenizerError> {
        let mut handler = GreedyTokenizer::new(&b"some_1dentifier"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::Identifier(String::from("some_1dentifier")),
                from: Position {
                    line: 1,
                    position: 0
                },
                to: Position {
                    line: 1,
                    position: 14
                }
            }
        );
        assert!(handler.next().is_none());

        let mut handler = GreedyTokenizer::new(&b"   w1432)  # whodat"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::Identifier(String::from("w1432")),
                from: Position {
                    line: 1,
                    position: 3
                },
                to: Position {
                    line: 1,
                    position: 7
                }
            }
        );
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::CloseParen,
                from: Position {
                    line: 1,
                    position: 8
                },
                to: Position {
                    line: 1,
                    position: 8
                }
            }
        );
        assert!(handler.next().is_none());

        Ok(())
    }

    #[test]
    fn it_handles_numeric_token() -> Result<(), TokenizerError> {
        let mut handler = GreedyTokenizer::new(&b"120"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::Number(120.0),
                from: Position {
                    line: 1,
                    position: 0
                },
                to: Position {
                    line: 1,
                    position: 2
                }
            }
        );
        assert!(handler.next().is_none());

        let mut handler = GreedyTokenizer::new(&b"   3.14159)  # delicious"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::Number(3.14159),
                from: Position {
                    line: 1,
                    position: 3
                },
                to: Position {
                    line: 1,
                    position: 9
                }
            }
        );
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::CloseParen,
                from: Position {
                    line: 1,
                    position: 10
                },
                to: Position {
                    line: 1,
                    position: 10
                }
            }
        );
        assert!(handler.next().is_none());

        Ok(())
    }

    #[test]
    fn it_throws_error_on_bad_numeric() -> Result<(), TokenizerError> {
        let mut handler = GreedyTokenizer::new(&b"120.0.1"[..])?;
        if let TokenizerError::ReadError { message, from, to } =
            handler.next().unwrap().unwrap_err()
        {
            assert_eq!(
                &message,
                &"Unable to parse number '120.0.1': invalid float literal"
            );
            assert_eq!(
                from,
                Position {
                    line: 1,
                    position: 0
                }
            );
            assert_eq!(
                to,
                Position {
                    line: 1,
                    position: 6
                }
            );
        } else {
            panic!();
        }

        assert!(handler.next().is_none());

        let mut handler = GreedyTokenizer::new(&b"  # feckin tool \n 120.0.1"[..])?;
        if let TokenizerError::ReadError { message, from, to } =
            handler.next().unwrap().unwrap_err()
        {
            assert_eq!(
                &message,
                &"Unable to parse number '120.0.1': invalid float literal"
            );
            assert_eq!(
                from,
                Position {
                    line: 1,
                    position: 1
                }
            );
            assert_eq!(
                to,
                Position {
                    line: 1,
                    position: 7
                }
            );
        } else {
            panic!();
        }
        assert!(handler.next().is_none());

        Ok(())
    }

    #[test]
    fn it_handles_reserved_keyword_tokens() -> Result<(), TokenizerError> {
        let mut handler = GreedyTokenizer::new(&b"def"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::Def,
                from: Position {
                    line: 1,
                    position: 0
                },
                to: Position {
                    line: 1,
                    position: 2
                }
            }
        );
        assert!(handler.next().is_none());

        let mut handler = GreedyTokenizer::new(&b"   fn)  # whodat"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::Fn,
                from: Position {
                    line: 1,
                    position: 3
                },
                to: Position {
                    line: 1,
                    position: 4
                }
            }
        );
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::CloseParen,
                from: Position {
                    line: 1,
                    position: 5
                },
                to: Position {
                    line: 1,
                    position: 5
                }
            }
        );
        assert!(handler.next().is_none());

        Ok(())
    }

    #[test]
    fn it_handles_reserved_chars_tokens() -> Result<(), TokenizerError> {
        let mut handler = GreedyTokenizer::new(&b"+"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::Identifier(String::from("+")),
                from: Position {
                    line: 1,
                    position: 0
                },
                to: Position {
                    line: 1,
                    position: 0
                }
            }
        );
        assert!(handler.next().is_none());

        let mut handler = GreedyTokenizer::new(&b"   -)  # whodat"[..])?;
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::Identifier(String::from("-")),
                from: Position {
                    line: 1,
                    position: 3
                },
                to: Position {
                    line: 1,
                    position: 3
                }
            }
        );
        assert_eq!(
            handler.next().unwrap()?,
            TokenAndSpan {
                token: Token::CloseParen,
                from: Position {
                    line: 1,
                    position: 4
                },
                to: Position {
                    line: 1,
                    position: 4
                }
            }
        );
        assert!(handler.next().is_none());

        Ok(())
    }

    #[test]
    fn it_formats_token_and_span_to_string() {
        assert_eq!(
            format!(
                "{}",
                TokenAndSpan {
                    token: Token::CloseParen,
                    from: Position {
                        line: 1,
                        position: 1
                    },
                    to: Position {
                        line: 1,
                        position: 1
                    }
                }
            ),
            "CloseParen[line 0 char 1]"
        );

        assert_eq!(
            format!(
                "{}",
                TokenAndSpan {
                    token: Token::Number(1.0),
                    from: Position {
                        line: 1,
                        position: 1
                    },
                    to: Position {
                        line: 1,
                        position: 5
                    }
                }
            ),
            "Number(1.0)[line 0 char 1 -> line 0 char 5]"
        );
    }
}
