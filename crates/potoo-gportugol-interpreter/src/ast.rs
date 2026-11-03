use crate::{DataType, SourceSpan};

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub name: String,
    pub name_span: SourceSpan,
    pub globals: Vec<VariableDecl>,
    pub main: Block,
    pub functions: Vec<FunctionDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableDecl {
    pub names: Vec<Identifier>,
    pub data_type: DataType,
    pub dimensions: Vec<usize>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub name_span: SourceSpan,
    pub parameters: Vec<VariableDecl>,
    pub locals: Vec<VariableDecl>,
    pub return_type: Option<DataType>,
    pub body: Block,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    Sequence {
        statements: Vec<Statement>,
        span: SourceSpan,
    },
    Assign {
        target: LValue,
        value: Expr,
        span: SourceSpan,
    },
    Call {
        call: Expr,
        span: SourceSpan,
    },
    Return {
        value: Option<Expr>,
        span: SourceSpan,
    },
    If {
        condition: Expr,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
        span: SourceSpan,
    },
    While {
        condition: Expr,
        body: Box<Statement>,
        span: SourceSpan,
    },
    Repeat {
        body: Box<Statement>,
        condition: Expr,
        span: SourceSpan,
    },
    For {
        target: LValue,
        start: Expr,
        end: Expr,
        step: i64,
        body: Box<Statement>,
        span: SourceSpan,
    },
}

impl Statement {
    pub fn span(&self) -> SourceSpan {
        match self {
            Self::Sequence { span, .. }
            | Self::Assign { span, .. }
            | Self::Call { span, .. }
            | Self::Return { span, .. }
            | Self::If { span, .. }
            | Self::While { span, .. }
            | Self::Repeat { span, .. }
            | Self::For { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LValue {
    pub name: String,
    pub name_span: SourceSpan,
    pub indexes: Vec<Expr>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Variable(LValue),
    Call {
        name: String,
        name_span: SourceSpan,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Integer(i64),
    Real(f64),
    Character(char),
    String(String),
    Logical(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Positive,
    Negative,
    Not,
    BitwiseNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
