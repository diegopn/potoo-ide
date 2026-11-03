#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Integer,
    Real,
    Logical,
    Chain,
    Character,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VectorBounds {
    pub lower: i64,
    pub upper: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableDeclaration {
    pub name: String,
    pub data_type: Type,
    pub vector: Option<VectorBounds>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleDeclaration {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub variables: Vec<VariableDeclaration>,
    pub module_declarations: Vec<ModuleDeclaration>,
    pub statements: Vec<Statement>,
    pub modules: Vec<Module>,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: String,
    pub statements: Vec<Statement>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub enum StatementKind {
    Write(Vec<Expression>),
    Read(Expression),
    Assign {
        target: Expression,
        value: Expression,
    },
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    Repeat {
        condition: Expression,
        body: Vec<Statement>,
    },
    For {
        variable: String,
        start: Expression,
        end: Expression,
        step: Expression,
        body: Vec<Statement>,
    },
    CallModule(String),
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
    String(String),
    Logical(bool),
    Identifier(String),
    Index {
        name: String,
        index: Box<Expression>,
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
}
