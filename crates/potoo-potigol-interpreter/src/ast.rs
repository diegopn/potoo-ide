use crate::SourceSpan;

/// Stable identity assigned to every node in a parsed program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

/// A complete Potigol source file.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub id: NodeId,
    pub statements: Vec<Stmt>,
    pub span: SourceSpan,
}

/// One declaration, command, or expression statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub id: NodeId,
    pub span: SourceSpan,
    pub kind: StmtKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Binding {
        pattern: Pattern,
        annotation: Option<TypeExpr>,
        mutable: bool,
        value: Expr,
    },
    Assignment {
        target: Expr,
        value: Expr,
    },
    Function(FunctionDecl),
    Type(TypeDecl),
    TypeAlias {
        name: String,
        target: TypeExpr,
    },
    Import(String),
    Block(Block),
    Output {
        newline: bool,
        value: Expr,
    },
    Return(Option<Expr>),
    While {
        condition: Expr,
        body: Block,
    },
    For {
        generators: Vec<ForGenerator>,
        body: Block,
    },
    Expression(Expr),
}

/// A block's last expression is its value when used as a function body.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Block {
    pub id: NodeId,
    pub statements: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDecl {
    pub id: NodeId,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeExpr>,
    pub body: FunctionBody,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FunctionBody {
    Expression(Box<Expr>),
    Block(Block),
    Abstract,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub id: NodeId,
    pub name: String,
    pub annotation: Option<TypeExpr>,
    pub mutable: bool,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDecl {
    pub id: NodeId,
    pub name: String,
    pub parents: Vec<String>,
    pub abstract_type: bool,
    pub fields: Vec<TypeField>,
    pub methods: Vec<FunctionDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeField {
    pub id: NodeId,
    pub name: String,
    pub annotation: Option<TypeExpr>,
    pub mutable: bool,
    pub default_value: Option<Expr>,
    pub constructor_parameter: bool,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForGenerator {
    pub id: NodeId,
    pub pattern: Pattern,
    pub source: ForSource,
    pub condition: Option<Expr>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ForSource {
    Range {
        start: Expr,
        end: Expr,
        step: Option<Expr>,
    },
    Iterable(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub id: NodeId,
    pub span: SourceSpan,
    pub annotation: Option<TypeExpr>,
    pub kind: PatternKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternKind {
    Wildcard,
    Binding(String),
    Integer(i32),
    Real(f64),
    Character(char),
    Text(String),
    Logical(bool),
    Tuple(Vec<Pattern>),
    List(Vec<Pattern>),
    Cons(Box<Pattern>, Box<Pattern>),
    Constructor { name: String, fields: Vec<Pattern> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub id: NodeId,
    pub span: SourceSpan,
    pub kind: ExprKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Integer(i32),
    Real(f64),
    Character(char),
    Text(Vec<StringPart>),
    Logical(bool),
    Identifier(String),
    This,
    Placeholder,
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    Unary {
        operator: String,
        operand: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: String,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Member {
        receiver: Box<Expr>,
        name: String,
    },
    Index {
        receiver: Box<Expr>,
        indices: Vec<Expr>,
    },
    Lambda {
        parameters: Vec<Parameter>,
        body: FunctionBody,
    },
    Conditional {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    Match {
        value: Box<Expr>,
        cases: Vec<MatchCase>,
    },
    Comprehension {
        generators: Vec<ForGenerator>,
        body: Block,
    },
    Block(Block),
    While {
        condition: Box<Expr>,
        body: Block,
    },
    For {
        generators: Vec<ForGenerator>,
        body: Block,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum StringPart {
    Text(String),
    Expression(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchCase {
    pub id: NodeId,
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub value: Block,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeExpr {
    pub id: NodeId,
    pub span: SourceSpan,
    pub kind: TypeExprKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeExprKind {
    Named(String, Vec<TypeExpr>),
    Tuple(Vec<TypeExpr>),
    Function(Vec<TypeExpr>, Box<TypeExpr>),
}
