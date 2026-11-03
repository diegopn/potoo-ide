#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Integer,
    Real,
    Logical,
    Character,
    String,
    Void,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub libraries: Vec<Library>,
    pub functions: Vec<Function>,
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub struct Library {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub return_type: Option<Type>,
    pub parameters: Vec<Parameter>,
    pub body: Statement,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub ty: Type,
    pub name: String,
    pub by_reference: bool,
    pub dimensions: usize,
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub enum StatementKind {
    Block(Vec<Statement>),
    Variables(Vec<Variable>),
    Expression(Expression),
    If {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    While {
        condition: Expression,
        body: Box<Statement>,
    },
    DoWhile {
        body: Box<Statement>,
        condition: Expression,
    },
    For {
        initializer: Option<Box<Statement>>,
        condition: Option<Expression>,
        increment: Option<Expression>,
        body: Box<Statement>,
    },
    Switch {
        expression: Expression,
        cases: Vec<SwitchCase>,
        default: Vec<Statement>,
        default_position: Option<usize>,
    },
    Return(Option<Expression>),
    Break,
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub ty: Type,
    pub name: String,
    pub is_constant: bool,
    pub dimensions: Vec<Option<Expression>>,
    pub initializer: Option<Expression>,
}

#[derive(Clone, Debug)]
pub struct SwitchCase {
    pub value: Expression,
    pub body: Vec<Statement>,
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
    Character(char),
    String(String),
    ArrayLiteral(Vec<Expression>),
    Variable(String),
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    Assign {
        target: Box<Expression>,
        operator: AssignmentOperator,
        value: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
    },
    Member {
        target: Box<Expression>,
        name: String,
    },
    Postfix {
        target: Box<Expression>,
        operator: PostfixOperator,
    },
    Prefix {
        target: Box<Expression>,
        operator: PostfixOperator,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Positive,
    Negate,
    Not,
    BitNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignmentOperator {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostfixOperator {
    Increment,
    Decrement,
}
