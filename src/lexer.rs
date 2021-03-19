use std::io::{self, Read};

const SPACE_CHAR: char = ' ';
const NEWLINE_CHAR: char = '\n';
const CARRIAGE_RETURN_CHAR: char = '\r';

#[derive(Debug, PartialEq)]
pub enum Token {
    // standard symbols
    Eof,
    OpenParen,
    CloseParen,

    // reserved keywords
    Ns,
    Def,
    Defn,
    Fn,
    Quote,
    If,

    // more complex stuff
    Identifier(String),
    Number(f64),
    Unknown(char),
}

impl Token {
    fn from(string_value: &str) -> Option<Token> {
        match string_value {
            "ns" => Some(Token::Ns),
            "def" => Some(Token::Def),
            "defn" => Some(Token::Defn),
            "fn" => Some(Token::Fn),
            "quote" => Some(Token::Quote),
            "if" => Some(Token::If),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Position {
    line: usize,
    position: usize,
}

#[derive(Debug, PartialEq)]
pub struct TokenAndSpan {
    pub token: Token,
    pub from: Position,
    pub to: Position,
}

struct CharAndPosition {
    chr: Option<char>,
    line: usize,
    position: usize,
}

#[derive(Debug)]
pub enum TokenizerError {
    IoError(io::Error),
    ParseError {
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
        TokenizerError::ParseError {
            message: format!("Unable to parse number '{}': {}", text, float_parse_error),
            from,
            to,
        }
    }
}

pub struct ParseHandler<T>
where
    T: Read,
{
    inbuf: T,
    line: usize,
    position: usize,
}

impl<T> ParseHandler<T>
where
    T: Read,
{
    pub fn new(inbuf: T) -> Self {
        ParseHandler {
            inbuf,
            line: 0,
            position: 0,
        }
    }

    pub fn get_token(&mut self) -> Result<TokenAndSpan, TokenizerError> {
        let mut tok = self.get_next_char()?;

        // remove any whitespace
        while tok.chr == Some(SPACE_CHAR) {
            tok = self.get_next_char()?;
        }

        // ignore comments
        if tok.chr == Some('#') {
            while tok.chr != Some(NEWLINE_CHAR)
                && tok.chr != Some(CARRIAGE_RETURN_CHAR)
                && tok.chr != None
            {
                tok = self.get_next_char()?;
            }
        }

        // find parens
        if tok.chr == Some('(') {
            return Ok(TokenAndSpan {
                token: Token::OpenParen,
                from: Position {
                    line: tok.line,
                    position: tok.position,
                },
                to: Position {
                    line: tok.line,
                    position: tok.position,
                },
            });
        } else if tok.chr == Some(')') {
            return Ok(TokenAndSpan {
                token: Token::CloseParen,
                from: Position {
                    line: tok.line,
                    position: tok.position,
                },
                to: Position {
                    line: tok.line,
                    position: tok.position,
                },
            });
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
                tok = self.get_next_char()?;
            }

            let to = Position {
                line: tok.line,
                position: tok.position - 1,
            };
            if let Some(reserved_token) = Token::from(&ident) {
                return Ok(TokenAndSpan {
                    token: reserved_token,
                    from,
                    to,
                });
            }

            return Ok(TokenAndSpan {
                token: Token::Identifier(ident),
                from,
                to,
            });
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
                tok = self.get_next_char()?;
            }
            let to = Position {
                line: tok.line,
                position: tok.position - 1,
            };

            match numstr.parse() {
                Ok(parsed) => {
                    return Ok(TokenAndSpan {
                        token: Token::Number(parsed),
                        from,
                        to,
                    })
                }
                Err(e) => return Err(TokenizerError::from(numstr, from, to, e)),
            }
        }

        // every other case is simply EOF and unknown char
        if tok.chr.is_none() {
            Ok(TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: tok.line,
                    position: tok.position,
                },
                to: Position {
                    line: tok.line,
                    position: tok.position,
                },
            })
        } else {
            Ok(TokenAndSpan {
                token: Token::Unknown(tok.chr.unwrap()),
                from: Position {
                    line: tok.line,
                    position: tok.position,
                },
                to: Position {
                    line: tok.line,
                    position: tok.position,
                },
            })
        }
    }

    fn get_next_char(&mut self) -> io::Result<CharAndPosition> {
        let mut buffer: [u8; 1] = [0];
        let chars_read = self.inbuf.read(&mut buffer)?;

        if chars_read > 0 {
            let chr = buffer[0] as char;

            let result = CharAndPosition {
                chr: Some(chr),
                line: self.line,
                position: self.position,
            };

            self.position += 1;
            if chr == '\n' || chr == '\r' {
                self.line += 1;
                self.position = 0;
            }

            Ok(result)
        } else {
            Ok(CharAndPosition {
                chr: None,
                line: self.line,
                position: self.position,
            })
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
        assert_eq!(
            ParseHandler::new(inbuf).get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 0
                },
                to: Position {
                    line: 0,
                    position: 0
                },
            }
        );

        let inbuf = &b"   "[..];
        assert_eq!(
            ParseHandler::new(inbuf).get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 3
                },
                to: Position {
                    line: 0,
                    position: 3
                },
            }
        );

        Ok(())
    }

    #[test]
    fn it_ignores_file_containing_only_comments() -> Result<(), TokenizerError> {
        let inbuf = &b"# blah"[..];
        assert_eq!(
            ParseHandler::new(inbuf).get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 6
                },
                to: Position {
                    line: 0,
                    position: 6
                },
            }
        );

        let inbuf = &b"  # blah"[..];
        assert_eq!(
            ParseHandler::new(inbuf).get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 8
                },
                to: Position {
                    line: 0,
                    position: 8
                },
            }
        );

        let mut handler = ParseHandler::new(&b"  # only \n # comments"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Unknown('\n'),
                from: Position {
                    line: 0,
                    position: 9
                },
                to: Position {
                    line: 0,
                    position: 9
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 1,
                    position: 11
                },
                to: Position {
                    line: 1,
                    position: 11
                }
            }
        );

        let mut handler = ParseHandler::new(&b"  # only \r # comments"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Unknown('\r'),
                from: Position {
                    line: 0,
                    position: 9
                },
                to: Position {
                    line: 0,
                    position: 9
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 1,
                    position: 11
                },
                to: Position {
                    line: 1,
                    position: 11
                }
            }
        );

        Ok(())
    }

    #[test]
    fn it_handles_parens() -> Result<(), TokenizerError> {
        let mut handler = ParseHandler::new(&b"("[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::OpenParen,
                from: Position {
                    line: 0,
                    position: 0
                },
                to: Position {
                    line: 0,
                    position: 0
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 1
                },
                to: Position {
                    line: 0,
                    position: 1
                }
            }
        );

        let mut handler = ParseHandler::new(&b"   )  # whodat"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::CloseParen,
                from: Position {
                    line: 0,
                    position: 3
                },
                to: Position {
                    line: 0,
                    position: 3
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 14
                },
                to: Position {
                    line: 0,
                    position: 14
                }
            }
        );

        Ok(())
    }

    #[test]
    fn it_handles_identifier_token() -> Result<(), TokenizerError> {
        let mut handler = ParseHandler::new(&b"some_1dentifier"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Identifier(String::from("some_1dentifier")),
                from: Position {
                    line: 0,
                    position: 0
                },
                to: Position {
                    line: 0,
                    position: 14
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 15
                },
                to: Position {
                    line: 0,
                    position: 15
                }
            }
        );

        let mut handler = ParseHandler::new(&b"   w1432  # whodat"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Identifier(String::from("w1432")),
                from: Position {
                    line: 0,
                    position: 3
                },
                to: Position {
                    line: 0,
                    position: 7
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 18
                },
                to: Position {
                    line: 0,
                    position: 18
                }
            }
        );

        Ok(())
    }

    #[test]
    fn it_handles_numeric_token() -> Result<(), TokenizerError> {
        let mut handler = ParseHandler::new(&b"120"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Number(120.0),
                from: Position {
                    line: 0,
                    position: 0
                },
                to: Position {
                    line: 0,
                    position: 2
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 3
                },
                to: Position {
                    line: 0,
                    position: 3
                }
            }
        );

        let mut handler = ParseHandler::new(&b"   3.14159  # delicious"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Number(3.14159),
                from: Position {
                    line: 0,
                    position: 3
                },
                to: Position {
                    line: 0,
                    position: 9
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 23
                },
                to: Position {
                    line: 0,
                    position: 23
                }
            }
        );

        Ok(())
    }

    #[test]
    fn it_throws_error_on_bad_numeric() -> Result<(), TokenizerError> {
        let mut handler = ParseHandler::new(&b"120.0.1"[..]);
        if let TokenizerError::ParseError { message, from, to } = handler.get_token().unwrap_err() {
            assert_eq!(
                &message,
                &"Unable to parse number '120.0.1': invalid float literal"
            );
            assert_eq!(
                from,
                Position {
                    line: 0,
                    position: 0
                }
            );
            assert_eq!(
                to,
                Position {
                    line: 0,
                    position: 6
                }
            );
        } else {
            panic!();
        }

        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 7
                },
                to: Position {
                    line: 0,
                    position: 7
                }
            }
        );

        let mut handler = ParseHandler::new(&b"  # feckin tool \n 120.0.1"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Unknown('\n'),
                from: Position {
                    line: 0,
                    position: 16
                },
                to: Position {
                    line: 0,
                    position: 16
                }
            }
        );
        if let TokenizerError::ParseError { message, from, to } = handler.get_token().unwrap_err() {
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
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
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

        Ok(())
    }

    #[test]
    fn it_handles_reserved_keyword_tokens() -> Result<(), TokenizerError> {
        let mut handler = ParseHandler::new(&b"defn"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Defn,
                from: Position {
                    line: 0,
                    position: 0
                },
                to: Position {
                    line: 0,
                    position: 3
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 4
                },
                to: Position {
                    line: 0,
                    position: 4
                }
            }
        );

        let mut handler = ParseHandler::new(&b"   if  # whodat"[..]);
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::If,
                from: Position {
                    line: 0,
                    position: 3
                },
                to: Position {
                    line: 0,
                    position: 4
                }
            }
        );
        assert_eq!(
            handler.get_token()?,
            TokenAndSpan {
                token: Token::Eof,
                from: Position {
                    line: 0,
                    position: 15
                },
                to: Position {
                    line: 0,
                    position: 15
                }
            }
        );

        Ok(())
    }
}
