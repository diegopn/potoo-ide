#[derive(Clone, Debug)]
pub struct Program {
    pub name: String,
    pub main: Vec<Statement>,
    pub routines: Vec<Routine>,
}

#[derive(Clone, Debug)]
pub struct Routine {
    pub name: String,
    pub return_type: Option<Type>,
    pub parameters: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub by_reference: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct VariableDeclaration {
    pub name: String,
    pub ty: Type,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Integer,
    Real,
    Logical,
    Character,
    Text,
    Array(Box<Type>, Vec<Option<usize>>),
}

impl Type {
    pub fn scalar(&self) -> &Type {
        match self {
            Self::Array(inner, _) => inner.scalar(),
            scalar => scalar,
        }
    }

    pub fn element(&self) -> Option<Type> {
        match self {
            Self::Array(inner, dimensions) if dimensions.len() > 1 => {
                Some(Self::Array(inner.clone(), dimensions[1..].to_vec()))
            }
            Self::Array(inner, _) => Some((**inner).clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub enum StatementKind {
    Declare(Vec<VariableDeclaration>),
    Assign {
        target: Expression,
        value: Expression,
    },
    Write {
        newline: bool,
        values: Vec<Expression>,
    },
    Read {
        targets: Vec<Expression>,
        character_only: bool,
    },
    If {
        branches: Vec<(Expression, Vec<Statement>)>,
        otherwise: Vec<Statement>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    DoWhile {
        body: Vec<Statement>,
        condition: Expression,
    },
    For {
        variable: String,
        start: Expression,
        end: Expression,
        step: Expression,
        body: Vec<Statement>,
    },
    Choose {
        value: Expression,
        cases: Vec<(Expression, Vec<Statement>)>,
        otherwise: Vec<Statement>,
    },
    Call {
        name: String,
        arguments: Vec<Expression>,
    },
    Return(Option<Expression>),
    Break,
    ClearScreen,
}

#[derive(Clone, Debug)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub enum ExpressionKind {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Character(u16),
    Text(String),
    Variable(String),
    Index(Box<Expression>, Box<Expression>),
    Call {
        name: String,
        arguments: Vec<Expression>,
    },
    Unary {
        operator: String,
        operand: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: String,
        right: Box<Expression>,
    },
    Formatted {
        value: Box<Expression>,
        width: usize,
        decimals: Option<usize>,
    },
}
