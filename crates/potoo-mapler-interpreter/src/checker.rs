use std::collections::{HashMap, HashSet};

use crate::ast::{
    Expression, ExpressionKind, Module, Program, Statement, StatementKind, Type,
    VariableDeclaration,
};
use crate::{Diagnostic, DiagnosticKind};

pub(crate) fn check(program: &Program) -> Result<(), Vec<Diagnostic>> {
    let mut checker = Checker {
        variables: HashMap::new(),
        module_declarations: HashSet::new(),
        diagnostics: Vec::new(),
    };

    for variable in &program.variables {
        if checker.variables.contains_key(variable.name.as_str())
            || checker.module_declarations.contains(variable.name.as_str())
        {
            checker.error(variable.line, variable.column, "identificador duplicado");
        } else {
            checker.variables.insert(&variable.name, variable);
        }
    }
    for declaration in &program.module_declarations {
        if checker.variables.contains_key(declaration.name.as_str())
            || !checker.module_declarations.insert(&declaration.name)
        {
            checker.error(
                declaration.line,
                declaration.column,
                "identificador duplicado",
            );
        }
    }

    let mut definitions = HashMap::<&str, &Module>::new();
    for module in &program.modules {
        if definitions.insert(&module.name, module).is_some() {
            checker.error(
                module.line,
                module.column,
                "módulo definido mais de uma vez",
            );
        }
        if !checker.module_declarations.contains(module.name.as_str()) {
            checker.error(
                module.line,
                module.column,
                "módulo não declarado em 'variaveis'",
            );
        }
    }
    for declaration in &program.module_declarations {
        if !definitions.contains_key(declaration.name.as_str()) {
            checker.error(
                declaration.line,
                declaration.column,
                "módulo declarado sem definição",
            );
        }
    }

    checker.statements(&program.statements);
    for module in &program.modules {
        checker.statements(&module.statements);
    }

    if checker.diagnostics.is_empty() {
        Ok(())
    } else {
        Err(checker.diagnostics)
    }
}

struct Checker<'a> {
    variables: HashMap<&'a str, &'a VariableDeclaration>,
    module_declarations: HashSet<&'a str>,
    diagnostics: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            match &statement.kind {
                StatementKind::Write(values) => {
                    for value in values {
                        self.expression(value);
                    }
                }
                StatementKind::Read(target) => {
                    self.target_type(target);
                }
                StatementKind::Assign { target, value } => {
                    if let (Some(target_type), Some(value_type)) =
                        (self.target_type(target), self.expression(value))
                        && !Self::assignable(target_type, value_type, value)
                    {
                        self.error(
                            value.line,
                            value.column,
                            "o valor não corresponde ao tipo da variável",
                        );
                    }
                }
                StatementKind::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    self.require_type(condition, Type::Logical, "a condição precisa ser lógica");
                    self.statements(then_branch);
                    self.statements(else_branch);
                }
                StatementKind::While { condition, body }
                | StatementKind::Repeat { condition, body } => {
                    self.require_type(condition, Type::Logical, "a condição precisa ser lógica");
                    self.statements(body);
                }
                StatementKind::For {
                    variable,
                    start,
                    end,
                    step,
                    body,
                } => {
                    match self.variables.get(variable.as_str()) {
                        Some(declaration)
                            if declaration.data_type == Type::Integer
                                && declaration.vector.is_none() => {}
                        Some(declaration) => self.error(
                            declaration.line,
                            declaration.column,
                            "a variável de controle do 'para' precisa ser inteira",
                        ),
                        None => self.error(
                            statement.line,
                            statement.column,
                            &format!("a variável '{variable}' não foi declarada"),
                        ),
                    }
                    for bound in [start, end, step] {
                        self.require_type(
                            bound,
                            Type::Integer,
                            "o limite do 'para' precisa ser inteiro",
                        );
                    }
                    self.statements(body);
                }
                StatementKind::CallModule(name) => {
                    if !self.module_declarations.contains(name.as_str()) {
                        self.error(
                            statement.line,
                            statement.column,
                            &format!("o módulo '{name}' não foi declarado"),
                        );
                    }
                }
            }
        }
    }

    fn target_type(&mut self, target: &Expression) -> Option<Type> {
        match &target.kind {
            ExpressionKind::Identifier(name) => match self.variables.get(name.as_str()) {
                Some(declaration) if declaration.vector.is_none() => Some(declaration.data_type),
                Some(_) => {
                    self.error(
                        target.line,
                        target.column,
                        "o vetor precisa ser acessado por um índice",
                    );
                    None
                }
                None => {
                    self.error(
                        target.line,
                        target.column,
                        &format!("a variável '{name}' não foi declarada"),
                    );
                    None
                }
            },
            ExpressionKind::Index { name, index } => {
                self.require_type(
                    index,
                    Type::Integer,
                    "o índice do vetor precisa ser inteiro",
                );
                match self.variables.get(name.as_str()) {
                    Some(declaration) if declaration.vector.is_some() => {
                        Some(declaration.data_type)
                    }
                    Some(_) => {
                        self.error(
                            target.line,
                            target.column,
                            "a variável não foi declarada como vetor",
                        );
                        None
                    }
                    None => {
                        self.error(
                            target.line,
                            target.column,
                            &format!("a variável '{name}' não foi declarada"),
                        );
                        None
                    }
                }
            }
            _ => {
                self.error(target.line, target.column, "esperada uma variável");
                None
            }
        }
    }

    fn expression(&mut self, expression: &Expression) -> Option<Type> {
        match &expression.kind {
            ExpressionKind::Integer(_) => Some(Type::Integer),
            ExpressionKind::Real(_) => Some(Type::Real),
            ExpressionKind::String(_) => Some(Type::Chain),
            ExpressionKind::Logical(_) => Some(Type::Logical),
            ExpressionKind::Identifier(name) => match self.variables.get(name.as_str()) {
                Some(declaration) if declaration.vector.is_none() => Some(declaration.data_type),
                Some(_) => {
                    self.error(
                        expression.line,
                        expression.column,
                        "o vetor precisa ser acessado por um índice",
                    );
                    None
                }
                None => {
                    self.error(
                        expression.line,
                        expression.column,
                        &format!("a variável '{name}' não foi declarada"),
                    );
                    None
                }
            },
            ExpressionKind::Index { name, index } => {
                self.require_type(
                    index,
                    Type::Integer,
                    "o índice do vetor precisa ser inteiro",
                );
                match self.variables.get(name.as_str()) {
                    Some(declaration) if declaration.vector.is_some() => {
                        Some(declaration.data_type)
                    }
                    Some(_) => {
                        self.error(
                            expression.line,
                            expression.column,
                            "a variável não foi declarada como vetor",
                        );
                        None
                    }
                    None => {
                        self.error(
                            expression.line,
                            expression.column,
                            &format!("a variável '{name}' não foi declarada"),
                        );
                        None
                    }
                }
            }
            ExpressionKind::Unary { operator, operand } => {
                let Some(operand_type) = self.expression(operand) else {
                    return None;
                };
                match (operator.as_str(), operand_type) {
                    ("-", Type::Integer | Type::Real) => Some(operand_type),
                    ("nao", Type::Logical) => Some(Type::Logical),
                    ("-", _) => {
                        self.error(
                            expression.line,
                            expression.column,
                            "o sinal '-' exige um número",
                        );
                        None
                    }
                    _ => {
                        self.error(
                            expression.line,
                            expression.column,
                            "'nao' exige um valor lógico",
                        );
                        None
                    }
                }
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left_type = self.expression(left);
                let right_type = self.expression(right);
                self.binary_type(expression, operator, left_type, right_type)
            }
        }
    }

    fn binary_type(
        &mut self,
        expression: &Expression,
        operator: &str,
        left: Option<Type>,
        right: Option<Type>,
    ) -> Option<Type> {
        let (Some(left), Some(right)) = (left, right) else {
            return None;
        };
        use Type::{Integer, Logical, Real};
        let numeric = |data_type| matches!(data_type, Integer | Real);
        match operator {
            "+" | "-" | "*" | "/" if numeric(left) && numeric(right) => {
                Some(if left == Real || right == Real {
                    Real
                } else {
                    Integer
                })
            }
            "=" if left == Type::Chain && right == Type::Chain => Some(Logical),
            "e" | "ou" if left == Logical && right == Logical => Some(Logical),
            "=" if numeric(left) && numeric(right) => Some(Logical),
            "<" | "<=" | ">" | ">=" if numeric(left) && numeric(right) => Some(Logical),
            "+" | "-" | "*" | "/" => {
                self.error(
                    expression.line,
                    expression.column,
                    "a operação exige valores numéricos",
                );
                None
            }
            "e" | "ou" => {
                self.error(
                    expression.line,
                    expression.column,
                    "a operação exige valores lógicos",
                );
                None
            }
            "<" | "<=" | ">" | ">=" => {
                self.error(
                    expression.line,
                    expression.column,
                    "a comparação exige valores numéricos",
                );
                None
            }
            _ => {
                self.error(
                    expression.line,
                    expression.column,
                    "tipos incompatíveis na comparação",
                );
                None
            }
        }
    }

    fn require_type(&mut self, expression: &Expression, expected: Type, message: &str) {
        if let Some(actual) = self.expression(expression)
            && actual != expected
        {
            self.error(expression.line, expression.column, message);
        }
    }

    fn assignable(target: Type, value: Type, expression: &Expression) -> bool {
        target == value
            || (target == Type::Character
                && value == Type::Chain
                && matches!(&expression.kind, ExpressionKind::String(text) if text.chars().count() == 1))
    }

    fn error(&mut self, line: usize, column: usize, message: &str) {
        self.diagnostics
            .push(Diagnostic::new(DiagnosticKind::Type, message, line, column));
    }
}
