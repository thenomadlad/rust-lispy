#[derive(Debug, PartialEq, Clone)]
pub enum AST {
    NumberExpr(f64),
    VariableExpr(String),
    EvaluateExpr {
        callee: String,
        args: Vec<AST>,
    },
    FunctionExpr {
        name: String,
        parameters: Vec<String>,
        body: Box<AST>,
    },
    ListExpr(Vec<AST>),
}
