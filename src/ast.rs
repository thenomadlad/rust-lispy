#[derive(Debug, PartialEq, Clone)]
pub enum AST {
    NumberExpr(f64),
    VariableExpr(String),
    EvaluateExpr {
        callee: String,
        args: Vec<AST>,
    },
    FunctionExpr {
        parameters: Vec<String>,
        statements: Vec<AST>,
    },
    ListExpr(Vec<AST>),
}
