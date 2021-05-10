use crate::ast::AST;
use crate::tok::{Token, Position, TokenAndSpan, Tokenizer, TokenizerError};

pub struct RecursiveDescentParser {
    tokenizer: Box<dyn Tokenizer>,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    MismatchedParens(Position),
    FunctionNeedsABody,
    UnexpectedEof(Position),
    UnexpectedTokenError {
        expected: Option<Token>,
        found: Option<Token>,
        from: Position,
        to: Position,
    },
    UnexpectedExpressionError {
        expected: Option<AST>,
        found: Option<AST>,
        position: Position,
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
                _ => Err(ParseError::UnknownError(String::from("Here we are but how")))
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

                            if rhs.len() > 1 {
                                return Err(ParseError::UnexpectedExpressionError {
                                    expected: None,
                                    found: rhs.get(1).cloned(),
                                    position: tokens_and_spans[parsed + 3].from.clone()
                                });
                            }

                            result.push(AST::EvaluateExpr {
                                callee: String::from("__assign"),
                                args: vec![AST::VariableExpr(name.clone()), rhs.pop().unwrap()],
                            });

                            // we also parsed the next two tokens
                            parsed += 1 + rec_parsed;
                        } else {
                            return Err(ParseError::UnexpectedTokenError {
                                expected: Some(Token::Identifier(String::from("_"))),
                                found: Some(tokens_and_spans[parsed + 1].token.clone()),
                                from: tokens_and_spans[parsed + 1].from.clone(),
                                to: tokens_and_spans[parsed + 1].to.clone(),
                            });
                        }
                    }

                    Token::Fn => {
                        if let Token::OpenParen = &tokens_and_spans[parsed + 1].token {
                            let mut total_tokens_parsed = 0;

                            // parse the args, make sure we have an open brancket and then get ourselves the tokens within them
                            let args_and_spans =
                                Self::find_tokens_within_brackets(&tokens_and_spans[parsed + 1..])?;
                            let mut parameters = vec![];
                            for arg_and_span in args_and_spans {
                                if let Token::Identifier(ref arg_name) = arg_and_span.token {
                                    parameters.push(String::from(arg_name))
                                } else {
                                    return Err(ParseError::UnexpectedTokenError {
                                        expected: Some(Token::Identifier(String::from("_"))),
                                        found: Some(arg_and_span.token.clone()),
                                        from: arg_and_span.from.clone(),
                                        to: arg_and_span.to.clone()
                                    });
                                }
                            }

                            total_tokens_parsed += 2 + parameters.len();  // include the bracket open and close

                            // parse the body of the function
                            if tokens_and_spans[parsed + total_tokens_parsed + 1].token
                                != Token::OpenParen
                            {
                                return Err(ParseError::UnexpectedTokenError {
                                    expected: Some(Token::OpenParen),
                                    found: Some(
                                        tokens_and_spans[parsed + total_tokens_parsed + 1]
                                            .token
                                            .clone(),
                                    ),
                                    from: tokens_and_spans[parsed + total_tokens_parsed + 1]
                                        .from
                                        .clone(),
                                    to: tokens_and_spans[parsed + total_tokens_parsed + 1]
                                        .to
                                        .clone(),
                                });
                            }

                            let function_body_tokens = Self::find_tokens_within_brackets(
                                &tokens_and_spans[parsed + total_tokens_parsed + 1..],
                            )?;
                            let (statements, rec_parsed) =
                                Self::recursively_evaluate(function_body_tokens)?;

                            if rec_parsed == 0 {
                                return Err(ParseError::FunctionNeedsABody);
                            }

                            total_tokens_parsed += 2 + rec_parsed;  // include the bracket open and close

                            result.push(AST::FunctionExpr {
                                parameters,
                                statements,
                            });

                            parsed += total_tokens_parsed;
                        } else {
                            return Err(ParseError::UnexpectedTokenError {
                                expected: Some(Token::OpenParen),
                                found: Some(tokens_and_spans[parsed + 1].token.clone()),
                                from: tokens_and_spans[parsed + 1].from.clone(),
                                to: tokens_and_spans[parsed + 1].to.clone(),
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
                            Some((AST::FunctionExpr {parameters, statements}, [])) => {
                                result.push(AST::FunctionExpr {
                                    parameters: parameters.clone(),
                                    statements: statements.clone()
                                })
                            }
                            _ => {
                                return Err(ParseError::UnexpectedExpressionError {
                                    expected: Some(AST::VariableExpr(String::from("_"))),
                                    found: stuff.first().cloned(),
                                    position: tokens_and_spans[parsed].from.clone(),
                                })
                            }
                        }
                    }

                    // close paren tokens indicate we should go up one level, and so return
                    Token::CloseParen => break,

                    Token::Unknown(chr) => return Err(ParseError::UnexpectedTokenError {
                        expected: None,
                        found: Some(Token::Unknown(chr)),
                        from: tokens_and_spans[parsed].from.clone(),
                        to: tokens_and_spans[parsed].to.clone(),
                    })

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
            Err(ParseError::MismatchedParens(
                extracted_tokens.last().unwrap().from.clone()
            ))
        } else {
            Ok(extracted_tokens)
        }
    }

    fn slice_until_tokens_match(
        tokens_and_spans: &[TokenAndSpan],
    ) -> Result<&[TokenAndSpan], ParseError> {
        let mut paren_count = 0;
        let mut end_idx = 0;

        for token_and_span in tokens_and_spans {
            match token_and_span.token {
                Token::OpenParen => paren_count += 1,
                Token::CloseParen => paren_count -= 1,
                _ => {}
            }

            // push end_idx forward
            end_idx += 1;

            // if we don't have open or closed parens remaining, let's return
            if paren_count <= 0 {
                break;
            }
        }

        // if we matched all parens, we're good
        if paren_count != 0 {
            Err(ParseError::MismatchedParens(tokens_and_spans[end_idx - 1].from.clone()))
        } else {
            Ok(&tokens_and_spans[0..end_idx])
        }
    }

    fn find_tokens_within_brackets(
        tokens_and_spans: &[TokenAndSpan],
    ) -> Result<&[TokenAndSpan], ParseError> {
        Self::slice_until_tokens_match(tokens_and_spans).map(|slc| &slc[1..slc.len() - 1])
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
                            line: 1,
                            position: 0,
                        },
                        to: Position {
                            line: 1,
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
                    line: 1,
                    position: 0,
                },
                to: Position {
                    line: 1,
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
                        line: 1,
                        position: 0
                    }
                );
                assert_eq!(
                    to,
                    Position {
                        line: 1,
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

    #[test]
    fn it_handles_unknown_token() {
        let tok = MockyTokenizer::new_with_zeros(vec![Token::Unknown('.')]);

        let mut parser = RecursiveDescentParser::new(Box::new(tok));
        assert_eq!(parser.next_expression(), Err(ParseError::UnexpectedTokenError {
            expected: None,
            found: Some(Token::Unknown('.')),
            from: Position { line: 1, position: 0 },
            to: Position { line: 1, position: 1 },
        }));
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
                line: 1,
                position: 0,
            },
            to: Position {
                line: 1,
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
                found: Some(AST::NumberExpr(1.0)),
                position: Position { line: 1, position: 0 }
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
                found: Some(Token::Fn),
                from: Position { line: 1, position: 0 },
                to: Position { line: 1, position: 1 },
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
                found: Some(AST::NumberExpr(2.0)),
                position: Position { line: 1, position: 0 }
            }
        );
    }

    #[test]
    fn it_parses_a_function_definition_into_a_function() {
        // function without args
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Fn,
            Token::OpenParen,
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
                parameters: vec![],
                statements: vec![AST::VariableExpr(String::from("contents"))]
            },
        );

        // function with args
        let tok = MockyTokenizer::new_with_zeros(vec![
            Token::OpenParen,
            Token::Fn,
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
                parameters: vec![String::from("arg1"), String::from("arg2")],
                statements: vec![AST::VariableExpr(String::from("contents"))]
            },
        );

        // TODO: handle errors
    }
}
