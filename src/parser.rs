use crate::ast::AST;
use crate::tok::{Token, TokenAndSpan, Tokenizer, TokenizerError};

pub struct RecursiveDescentParser {
    tokenizer: Box<dyn Tokenizer>,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    MismatchedParens,
    UnexpectedEof,
    UnexpectedTokenError {
        expected: Option<Token>,
        found: Option<Token>,
    },
    UnexpectedExpressionError {
        expected: Option<AST>,
        found: Option<AST>,
    },
    TokenizerError(TokenizerError),
    UnknownError(String),
}

impl From<TokenizerError> for ParseError {
    fn from(tokenizer_error: TokenizerError) -> Self {
        ParseError::TokenizerError(tokenizer_error)
    }
}

impl RecursiveDescentParser {
    pub fn new(tokenizer: Box<dyn Tokenizer>) -> Self {
        Self { tokenizer }
    }

    pub fn next_expression(&mut self) -> Result<Option<Box<AST>>, ParseError> {
        let tokens_and_spans = Self::extract_until_brackets_match(&mut self.tokenizer)?;

        if tokens_and_spans.is_empty() {
            Ok(None)
        } else {
            let (mut asts, _) = Self::recursively_evaluate(&tokens_and_spans[..])?;
            match asts.len() {
                1 => Ok(Some(Box::new(asts.pop().unwrap()))),
                num_terms if num_terms > 1 => Err(ParseError::UnknownError(String::from("Not sure how we got here, but we have multiple statements with the same open/close brackets"))),
                _ => Err(ParseError::UnexpectedEof)
            }
        }
    }

    fn recursively_evaluate(
        tokens_and_spans: &[TokenAndSpan],
    ) -> Result<(Vec<AST>, usize), ParseError> {
        let mut result = Vec::with_capacity(tokens_and_spans.len());
        let mut parsed = 0;
        loop {
            if parsed < tokens_and_spans.len() {
                match tokens_and_spans[parsed].token {
                    Token::Number(val) => result.push(AST::NumberExpr(val)),
                    Token::Identifier(ref name) => {
                        result.push(AST::VariableExpr(String::from(name)))
                    }

                    Token::Def => {
                        if let Token::Identifier(name) = &tokens_and_spans[parsed + 1].token {
                            let (mut rhs, rec_parsed) =
                                Self::recursively_evaluate(&tokens_and_spans[parsed + 2..])?;

                            if rec_parsed > 1 {
                                return Err(ParseError::UnexpectedExpressionError {
                                    expected: None,
                                    found: rhs.get(1).cloned(),
                                });
                            }

                            result.push(AST::EvaluateExpr {
                                callee: String::from("__assign"),
                                args: vec![AST::VariableExpr(name.clone()), rhs.pop().unwrap()],
                            });

                            // we also parsed the next two tokens
                            parsed += 2;
                        } else {
                            return Err(ParseError::UnexpectedTokenError {
                                expected: Some(Token::Identifier(String::from("_"))),
                                found: Some(tokens_and_spans[parsed + 1].token.clone()),
                            });
                        }
                    }

                    // open paren tokens indicate we should go down one level in parsing things
                    Token::OpenParen => {
                        let (stuff, rec_parsed) =
                            Self::recursively_evaluate(&tokens_and_spans[parsed + 1..])?;
                        parsed += rec_parsed;

                        // if we have a variable and then some shit, let's return it as an EvaluateExpr
                        match stuff[..].split_first() {
                            Some((AST::VariableExpr(ref name), rest)) => {
                                result.push(AST::EvaluateExpr {
                                    callee: String::from(name),
                                    args: rest.to_vec(),
                                })
                            }
                            Some((AST::EvaluateExpr { callee, args }, [])) => {
                                result.push(AST::EvaluateExpr {
                                    callee: callee.clone(),
                                    args: args.clone(),
                                })
                            }
                            _ => {
                                return Err(ParseError::UnexpectedExpressionError {
                                    expected: Some(AST::VariableExpr(String::from("_"))),
                                    found: stuff.first().cloned(),
                                })
                            }
                        }
                    }

                    // close paren tokens indicate we should go up one level, and so return
                    Token::CloseParen => break,
                    _ => {}
                }
            } else {
                break;
            }

            parsed += 1;
        }

        Ok((result, parsed))
    }

    fn extract_until_brackets_match<T>(
        tokens_and_spans: &mut T,
    ) -> Result<Vec<TokenAndSpan>, ParseError>
    where
        T: Iterator<Item = Result<TokenAndSpan, TokenizerError>>,
    {
        let mut paren_count = 0;
        let mut extracted_tokens: Vec<TokenAndSpan> = vec![];

        for maybe_token_and_span in tokens_and_spans {
            let token_and_span = maybe_token_and_span?;
            match token_and_span.token {
                Token::OpenParen => paren_count += 1,
                Token::CloseParen => paren_count -= 1,
                _ => {}
            }

            // add token to the result
            extracted_tokens.push(token_and_span);

            // if we don't have open or closed parens remaining, let's return
            if paren_count <= 0 {
                break;
            }
        }

        // if we matched all parens, we're good
        if paren_count != 0 {
            Err(ParseError::MismatchedParens)
        } else {
            Ok(extracted_tokens)
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate rstest;

    use rstest::*;

    use super::*;
    use crate::tok::{Position, TokenAndSpan, TokenizerError};

    struct MockyTokenizer {
        returns: Vec<Result<TokenAndSpan, TokenizerError>>,
    }

    impl MockyTokenizer {
        fn new(tokens_and_spans: Vec<TokenAndSpan>) -> MockyTokenizer {
            MockyTokenizer {
                returns: tokens_and_spans.into_iter().map(Result::Ok).rev().collect(),
            }
        }

        fn new_with_zeros(tokens: Vec<Token>) -> MockyTokenizer {
            Self::new(
                tokens
                    .into_iter()
                    .map(|token| TokenAndSpan {
                        token,
                        from: Position {
                            line: 0,
                            position: 0,
                        },
                        to: Position {
                            line: 0,
                            position: 1,
                        },
                    })
                    .collect(),
            )
        }

        fn new_with_errors(
            tokens_and_spans: Vec<TokenAndSpan>,
            error: TokenizerError,
        ) -> MockyTokenizer {
            MockyTokenizer {
                returns: tokens_and_spans
                    .into_iter()
                    .map(Result::Ok)
                    .chain(vec![Result::Err(error)])
                    .rev()
                    .collect(),
            }
        }
    }

    impl Iterator for MockyTokenizer {
        type Item = Result<TokenAndSpan, TokenizerError>;

        fn next(&mut self) -> Option<Result<TokenAndSpan, TokenizerError>> {
            self.returns.pop()
        }
    }

    #[test]
    fn it_wraps_tokenizer_error_with_parse_error() {
        let tok = MockyTokenizer::new_with_errors(
            vec![],
            TokenizerError::ReadError {
                message: String::from("who dat"),
                from: Position {
                    line: 0,
                    position: 0,
                },
                to: Position {
                    line: 0,
                    position: 0,
                },
            },
        );
        let expr = RecursiveDescentParser::new(Box::new(tok)).next_expression();

        // expect the error is what we passed in wrapped in a ParseError
        assert!(expr.is_err());
        match expr.unwrap_err() {
            ParseError::TokenizerError(TokenizerError::ReadError { message, from, to }) => {
                assert_eq!(message, String::from("who dat"));
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
                        position: 0
                    }
                );
            }
            _ => panic!("Unexpected error here"),
        };
    }

    #[test]
    fn it_handles_empty_token_stream() {
        let tok = MockyTokenizer::new(vec![]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(parser.next_expression().unwrap(), None);
    }

    #[rstest]
    // numeric bois
    #[case(Token::Number(-1.0), AST::NumberExpr(-1.0))]
    #[case(Token::Number(0.0), AST::NumberExpr(0.0))]
    #[case(Token::Number(188.0), AST::NumberExpr(188.0))]
    // string bois
    #[case(
        Token::Identifier(String::from("something")),
        AST::VariableExpr(String::from("something"))
    )]
    fn it_parses_leaf_tokens(#[case] token: Token, #[case] expr: AST) {
        let tok = MockyTokenizer::new(vec![TokenAndSpan {
            token,
            from: Position {
                line: 0,
                position: 0,
            },
            to: Position {
                line: 0,
                position: 1,
            },
        }]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(*parser.next_expression().unwrap().unwrap(), expr);
    }

    #[test]
    fn it_parses_no_args_expressions() {
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Identifier(String::from("something")),
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            *parser.next_expression().unwrap().unwrap(),
            AST::EvaluateExpr {
                callee: String::from("something"),
                args: vec![]
            }
        );

        // it throws an error if the first expression is not an identifier
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Number(1.0),
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            parser.next_expression().unwrap_err(),
            ParseError::UnexpectedExpressionError {
                expected: Some(AST::VariableExpr(String::from("_"))),
                found: Some(AST::NumberExpr(1.0))
            }
        );
    }

    #[test]
    fn it_parses_expressions_with_args() {
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Identifier(String::from("something")),
            Token::Number(1.0),
            Token::Identifier(String::from("something_else")),
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            *parser.next_expression().unwrap().unwrap(),
            AST::EvaluateExpr {
                callee: String::from("something"),
                args: vec![
                    AST::NumberExpr(1.0),
                    AST::VariableExpr(String::from("something_else"))
                ]
            }
        );
    }

    #[test]
    fn it_parses_expressions_with_args_that_are_expressions() {
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Identifier(String::from("something")),
            Token::Number(1.0),
            Token::OpenParen,
            Token::Identifier(String::from("something_else")),
            Token::Number(2.0),
            Token::CloseParen,
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            *parser.next_expression().unwrap().unwrap(),
            AST::EvaluateExpr {
                callee: String::from("something"),
                args: vec![
                    AST::NumberExpr(1.0),
                    AST::EvaluateExpr {
                        callee: String::from("something_else"),
                        args: vec![AST::NumberExpr(2.0)]
                    }
                ]
            }
        );
    }

    #[test]
    fn it_returns_multiple_statements_as_separate_expressions() {
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Identifier(String::from("something")),
            Token::Number(1.0),
            Token::CloseParen,
            Token::OpenParen,
            Token::Identifier(String::from("something_else")),
            Token::Number(2.0),
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            *parser.next_expression().unwrap().unwrap(),
            AST::EvaluateExpr {
                callee: String::from("something"),
                args: vec![AST::NumberExpr(1.0),]
            },
        );
        assert_eq!(
            *parser.next_expression().unwrap().unwrap(),
            AST::EvaluateExpr {
                callee: String::from("something_else"),
                args: vec![AST::NumberExpr(2.0)]
            }
        );
    }

    #[test]
    fn it_parses_def_statements_into_assignment_operation() {
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Def,
            Token::Identifier(String::from("whodat")),
            Token::Number(1.0),
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            *parser.next_expression().unwrap().unwrap(),
            AST::EvaluateExpr {
                callee: String::from("__assign"),
                args: vec![
                    AST::VariableExpr(String::from("whodat")),
                    AST::NumberExpr(1.0),
                ]
            },
        );

        // it throws an error if i use a non-identifier type as name
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Def,
            Token::Fn,
            Token::Number(1.0),
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            parser.next_expression().unwrap_err(),
            ParseError::UnexpectedTokenError {
                expected: Some(Token::Identifier(String::from("_"))),
                found: Some(Token::Fn)
            }
        );

        // it throws an error if we provide too many args
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Def,
            Token::Identifier(String::from("too_many_args")),
            Token::Number(1.0),
            Token::Number(2.0),
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            parser.next_expression().unwrap_err(),
            ParseError::UnexpectedExpressionError {
                expected: None,
                found: Some(AST::NumberExpr(2.0))
            }
        );
    }

    #[test]
    #[ignore]
    fn it_parses_a_function_definition_into_a_prototype() {
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Fn,
            Token::Identifier(String::from("func_name")),
            Token::OpenParen,
            Token::Identifier(String::from("arg1")),
            Token::Identifier(String::from("arg2")),
            Token::CloseParen,
            Token::OpenParen,
            Token::Identifier(String::from("contents")),
            Token::CloseParen,
            Token::CloseParen,
        ]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(
            *parser.next_expression().unwrap().unwrap(),
            AST::FunctionExpr {
                name: String::from("func_name"),
                parameters: vec![String::from("arg1"), String::from("arg2")],
                body: Box::new(AST::VariableExpr(String::from("contents")))
            },
        );
    }
}
