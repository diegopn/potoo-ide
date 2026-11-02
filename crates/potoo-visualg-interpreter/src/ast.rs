#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Integer,
    Real,
    Character,
    Logical,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub globals: Vec<Variable>,
    pub arquivo: Option<DataFileDeclaration>,
    pub routines: Vec<Routine>,
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub struct DataFileDeclaration {
    pub path: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub name: String,
    pub data_type: Type,
    pub dimensions: Vec<ArrayDimension>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayDimension {
    pub lower: i64,
    pub upper: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutineKind {
    Procedure,
    Function,
}

#[derive(Clone, Debug)]
pub struct Routine {
    pub name: String,
    pub kind: RoutineKind,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub locals: Vec<Variable>,
    pub body: Vec<Statement>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub data_type: Type,
    pub by_reference: bool,
    pub dimensions: usize,
}

#[derive(Clone, Debug)]
pub struct ChoiceCase {
    pub values: Vec<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub enum StatementKind {
    Write {
        newline: bool,
        values: Vec<FormattedExpression>,
    },
    Read(Vec<Expression>),
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
    },
    Choice {
        expression: Expression,
        cases: Vec<ChoiceCase>,
        default: Vec<Statement>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    Repeat {
        body: Vec<Statement>,
        condition: Option<Expression>,
    },
    For {
        variable: String,
        start: Expression,
        end: Expression,
        step: Option<Expression>,
        body: Vec<Statement>,
    },
    Break,
    Aleatorio {
        enabled: bool,
        minimum: i64,
        maximum: i64,
    },
    Timer(Option<u64>),
    Pause,
    Debug(Expression),
    Echo(bool),
    Chronometer(bool),
    ClearScreen,
    Return(Option<Expression>),
    Expression(Expression),
}

#[derive(Clone, Debug)]
pub struct FormattedExpression {
    pub expression: Expression,
    pub width: Option<Expression>,
    pub precision: Option<Expression>,
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
    Unary {
        operator: String,
        operand: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: String,
        right: Box<Expression>,
    },
    Assign {
        target: Box<Expression>,
        value: Box<Expression>,
    },
    Call {
        name: String,
        arguments: Vec<Expression>,
    },
    Index {
        name: String,
        indexes: Vec<Expression>,
    },
}
