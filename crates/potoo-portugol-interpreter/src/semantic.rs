use std::collections::{HashMap, HashSet};

use crate::{Diagnostic, DiagnosticKind, ast::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueType {
    Scalar(Type),
    Array(Type, usize),
    Void,
    Unknown,
}

#[derive(Clone, Debug)]
struct VariableInfo {
    ty: Type,
    dimensions: usize,
    is_constant: bool,
    initialized: bool,
}

struct Analyzer<'a> {
    program: &'a Program,
    functions: HashMap<&'a str, Vec<&'a Function>>,
    libraries: HashMap<String, String>,
    scopes: Vec<HashMap<String, VariableInfo>>,
    return_type: Option<Type>,
    in_function: bool,
    loop_depth: usize,
    switch_depth: usize,
}

pub(super) fn analyze(program: &Program) -> Result<(), Vec<Diagnostic>> {
    Analyzer::new(program).analyze()
}

impl<'a> Analyzer<'a> {
    fn new(program: &'a Program) -> Self {
        let mut functions: HashMap<&str, Vec<&Function>> = HashMap::new();
        for function in &program.functions {
            functions.entry(&function.name).or_default().push(function);
        }
        let mut libraries = HashMap::new();
        for library in &program.libraries {
            libraries.insert(library.name.clone(), library.name.clone());
            if let Some(alias) = &library.alias {
                libraries.insert(alias.clone(), library.name.clone());
            }
        }
        Self {
            program,
            functions,
            libraries,
            scopes: vec![HashMap::new()],
            return_type: None,
            in_function: false,
            loop_depth: 0,
            switch_depth: 0,
        }
    }

    fn analyze(mut self) -> Result<(), Vec<Diagnostic>> {
        let supported_libraries = [
            "Arquivos",
            "Calendario",
            "Graficos",
            "Internet",
            "Matematica",
            "Mouse",
            "Objetos",
            "ServicosWeb",
            "Sons",
            "Teclado",
            "Texto",
            "Tipos",
            "Util",
        ];
        for library in &self.program.libraries {
            if !supported_libraries.contains(&library.name.as_str()) {
                return Err(vec![self.error_at(
                    DiagnosticKind::Type,
                    format!("biblioteca '{}' não encontrada", library.name),
                    1,
                    1,
                )]);
            }
        }
        if !self.functions.contains_key("inicio") {
            return Err(vec![self.error_at(
                DiagnosticKind::Type,
                "função 'inicio' não encontrada",
                1,
                1,
            )]);
        }
        self.check_function_signatures()?;

        for statement in &self.program.statements {
            self.statement(statement)?;
        }
        for function in &self.program.functions {
            self.function(function)?;
        }
        Ok(())
    }

    fn check_function_signatures(&self) -> Result<(), Vec<Diagnostic>> {
        for functions in self.functions.values() {
            for (index, function) in functions.iter().enumerate() {
                let mut parameters = HashSet::new();
                for parameter in &function.parameters {
                    if parameter.ty == Type::Void {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            "parâmetro não pode ter tipo vazio",
                            function.line,
                            function.column,
                        )]);
                    }
                    if !parameters.insert(parameter.name.as_str()) {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            format!("parâmetro '{}' declarado mais de uma vez", parameter.name),
                            function.line,
                            function.column,
                        )]);
                    }
                    if parameter.dimensions > 2 {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            "vetores podem ter no máximo duas dimensões",
                            function.line,
                            function.column,
                        )]);
                    }
                }
                if functions[index + 1..].iter().any(|other| {
                    function.parameters.len() == other.parameters.len()
                        && function
                            .parameters
                            .iter()
                            .zip(&other.parameters)
                            .all(|(left, right)| {
                                left.ty == right.ty
                                    && left.dimensions == right.dimensions
                                    && left.by_reference == right.by_reference
                            })
                }) {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        format!("assinatura duplicada para a função '{}'", function.name),
                        function.line,
                        function.column,
                    )]);
                }
            }
        }
        Ok(())
    }

    fn function(&mut self, function: &'a Function) -> Result<(), Vec<Diagnostic>> {
        let old_return_type = self.return_type;
        let old_in_function = self.in_function;
        self.return_type = function.return_type;
        self.in_function = true;
        self.scopes.push(HashMap::new());
        for parameter in &function.parameters {
            self.scopes.last_mut().expect("function scope").insert(
                parameter.name.clone(),
                VariableInfo {
                    ty: parameter.ty,
                    dimensions: parameter.dimensions,
                    is_constant: false,
                    initialized: true,
                },
            );
        }
        let always_returns = match &function.body.kind {
            StatementKind::Block(statements) => self.sequence(statements)?,
            _ => self.statement(&function.body)?,
        };
        self.scopes.pop();
        self.return_type = old_return_type;
        self.in_function = old_in_function;
        if function.return_type.is_some() && !always_returns {
            return Err(vec![self.error_at(
                DiagnosticKind::Type,
                format!(
                    "a função '{}' pode terminar sem retornar um valor",
                    function.name
                ),
                function.line,
                function.column,
            )]);
        }
        Ok(())
    }

    fn statement(&mut self, statement: &Statement) -> Result<bool, Vec<Diagnostic>> {
        match &statement.kind {
            StatementKind::Block(statements) => {
                self.scopes.push(HashMap::new());
                let result = self.sequence(statements);
                self.scopes.pop();
                result
            }
            StatementKind::Variables(variables) => {
                for variable in variables {
                    if variable.ty == Type::Void {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            "variável não pode ter tipo vazio",
                            statement.line,
                            statement.column,
                        )]);
                    }
                    if self
                        .scopes
                        .last()
                        .expect("active scope")
                        .contains_key(&variable.name)
                    {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            format!("variável '{}' declarada mais de uma vez", variable.name),
                            statement.line,
                            statement.column,
                        )]);
                    }
                    if variable.dimensions.len() > 2 {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            "vetores podem ter no máximo duas dimensões",
                            statement.line,
                            statement.column,
                        )]);
                    }
                    for dimension in &variable.dimensions {
                        let Some(dimension) = dimension else {
                            if variable.initializer.is_none() {
                                return Err(vec![self.error_at(
                                    DiagnosticKind::Type,
                                    "uma dimensão de vetor local precisa de tamanho",
                                    statement.line,
                                    statement.column,
                                )]);
                            }
                            continue;
                        };
                        let size_type = self.expression(dimension)?;
                        self.expect_scalar(size_type, Type::Integer, dimension)?;
                        if variable.initializer.is_none()
                            && constant_integer(dimension).is_some_and(|size| size <= 0)
                        {
                            return Err(vec![self.error_at(
                                DiagnosticKind::Type,
                                "o tamanho de vetor deve ser positivo",
                                dimension.line,
                                dimension.column,
                            )]);
                        }
                    }
                    if variable.is_constant && variable.initializer.is_none() {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            format!(
                                "a constante '{}' precisa de um valor inicial",
                                variable.name
                            ),
                            statement.line,
                            statement.column,
                        )]);
                    }
                    self.scopes.last_mut().expect("active scope").insert(
                        variable.name.clone(),
                        VariableInfo {
                            ty: variable.ty,
                            dimensions: variable.dimensions.len(),
                            is_constant: variable.is_constant,
                            initialized: !variable.dimensions.is_empty(),
                        },
                    );
                    if let Some(initializer) = &variable.initializer {
                        let actual = self.expression(initializer)?;
                        self.expect_variable_type(
                            variable.ty,
                            variable.dimensions.len(),
                            actual,
                            initializer,
                        )?;
                        self.mark_initialized(&variable.name);
                    }
                }
                Ok(false)
            }
            StatementKind::Expression(expression) => {
                self.expression(expression)?;
                Ok(false)
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_type = self.expression(condition)?;
                self.expect_scalar(condition_type, Type::Logical, condition)?;
                let before = self.scopes.clone();
                let then_returns = self.statement(then_branch)?;
                let after_then = self.scopes.clone();
                self.scopes = before.clone();
                let else_returns = if let Some(else_branch) = else_branch {
                    self.statement(else_branch)?
                } else {
                    false
                };
                let after_else = self.scopes.clone();
                self.scopes = merge_scopes(&before, &after_then, &after_else);
                Ok(else_branch.is_some() && then_returns && else_returns)
            }
            StatementKind::While { condition, body } => {
                let condition_type = self.expression(condition)?;
                self.expect_scalar(condition_type, Type::Logical, condition)?;
                let before = self.scopes.clone();
                self.loop_depth += 1;
                let body_returns = self.statement(body)?;
                self.loop_depth -= 1;
                self.scopes = before;
                Ok(matches!(condition.kind, ExpressionKind::Logical(true)) && body_returns)
            }
            StatementKind::DoWhile { body, condition } => {
                self.loop_depth += 1;
                let body_returns = self.statement(body)?;
                self.loop_depth -= 1;
                let condition_type = self.expression(condition)?;
                self.expect_scalar(condition_type, Type::Logical, condition)?;
                Ok(body_returns)
            }
            StatementKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.scopes.push(HashMap::new());
                if let Some(initializer) = initializer {
                    self.statement(initializer)?;
                }
                if let Some(condition) = condition {
                    let condition_type = self.expression(condition)?;
                    self.expect_scalar(condition_type, Type::Logical, condition)?;
                }
                if let Some(increment) = increment {
                    self.expression(increment)?;
                }
                let before = self.scopes.clone();
                self.loop_depth += 1;
                self.statement(body)?;
                self.loop_depth -= 1;
                self.scopes = before;
                self.scopes.pop();
                Ok(false)
            }
            StatementKind::Switch {
                expression,
                cases,
                default,
                default_position,
            } => {
                let switch_type = self.expression(expression)?;
                if matches!(switch_type, ValueType::Void | ValueType::Array(_, _)) {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "a expressão de 'escolha' precisa produzir um valor escalar",
                        expression.line,
                        expression.column,
                    )]);
                }
                self.switch_depth += 1;
                let before = self.scopes.clone();
                let mut case_returns = true;
                for case in cases {
                    let case_type = self.expression(&case.value)?;
                    self.expect_compatible(switch_type, case_type, &case.value)?;
                    self.scopes = before.clone();
                    case_returns &= self.sequence(&case.body)?;
                }
                self.scopes = before;
                let default_returns = self.sequence(default)?;
                self.switch_depth -= 1;
                Ok(default_position.is_some() && case_returns && default_returns)
            }
            StatementKind::Return(value) => {
                if !self.in_function {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "'retorne' não pode aparecer fora de uma função",
                        statement.line,
                        statement.column,
                    )]);
                }
                match (self.return_type, value) {
                    (Some(expected), Some(value)) => {
                        let actual = self.expression(value)?;
                        self.expect_scalar(actual, expected, value)?;
                    }
                    (Some(_), None) => {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            "a função precisa retornar um valor",
                            statement.line,
                            statement.column,
                        )]);
                    }
                    (None, Some(value)) => {
                        self.expression(value)?;
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            "uma função vazia não pode retornar um valor",
                            statement.line,
                            statement.column,
                        )]);
                    }
                    (None, None) => {}
                }
                Ok(true)
            }
            StatementKind::Break => {
                if self.loop_depth == 0 && self.switch_depth == 0 {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "'pare' não pode aparecer fora de um laço ou 'escolha'",
                        statement.line,
                        statement.column,
                    )]);
                }
                Ok(false)
            }
        }
    }

    fn sequence(&mut self, statements: &[Statement]) -> Result<bool, Vec<Diagnostic>> {
        let mut always_returns = false;
        for statement in statements {
            if always_returns {
                let before = self.scopes.clone();
                self.statement(statement)?;
                self.scopes = before;
            } else {
                always_returns = self.statement(statement)?;
            }
        }
        Ok(always_returns)
    }

    fn expression(&mut self, expression: &Expression) -> Result<ValueType, Vec<Diagnostic>> {
        match &expression.kind {
            ExpressionKind::Integer(value) => {
                if i32::try_from(*value).is_err() {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "literal inteiro fora do limite de 32 bits",
                        expression.line,
                        expression.column,
                    )]);
                }
                Ok(ValueType::Scalar(Type::Integer))
            }
            ExpressionKind::Real(_) => Ok(ValueType::Scalar(Type::Real)),
            ExpressionKind::Logical(_) => Ok(ValueType::Scalar(Type::Logical)),
            ExpressionKind::Character(_) => Ok(ValueType::Scalar(Type::Character)),
            ExpressionKind::String(_) => Ok(ValueType::Scalar(Type::String)),
            ExpressionKind::ArrayLiteral(values) => {
                let Some(first) = values.first() else {
                    return Ok(ValueType::Unknown);
                };
                let first_type = self.expression(first)?;
                for value in values.iter().skip(1) {
                    let actual = self.expression(value)?;
                    self.expect_compatible(first_type, actual, value)?;
                }
                Ok(match first_type {
                    ValueType::Scalar(ty) => ValueType::Array(ty, 1),
                    ValueType::Array(ty, dimensions) => {
                        ValueType::Array(ty, dimensions.saturating_add(1))
                    }
                    _ => ValueType::Unknown,
                })
            }
            ExpressionKind::Variable(name) => {
                if let Some(variable) = self.lookup(name) {
                    if !variable.initialized {
                        return Err(vec![self.error_at(
                            DiagnosticKind::Type,
                            format!("a variável '{}' pode não ter sido inicializada", name),
                            expression.line,
                            expression.column,
                        )]);
                    }
                    Ok(variable_type(variable))
                } else {
                    Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        format!("variável '{name}' não declarada"),
                        expression.line,
                        expression.column,
                    )])
                }
            }
            ExpressionKind::Unary { operator, operand } => {
                if *operator == UnaryOperator::Negate
                    && matches!(&operand.kind, ExpressionKind::Integer(value) if *value == i64::from(i32::MAX) + 1)
                {
                    return Ok(ValueType::Scalar(Type::Integer));
                }
                let value = self.expression(operand)?;
                match operator {
                    UnaryOperator::Not => {
                        self.expect_scalar(value, Type::Logical, operand)?;
                        Ok(ValueType::Scalar(Type::Logical))
                    }
                    UnaryOperator::BitNot => {
                        self.expect_scalar(value, Type::Integer, operand)?;
                        Ok(ValueType::Scalar(Type::Integer))
                    }
                    UnaryOperator::Positive | UnaryOperator::Negate => {
                        if !matches!(value, ValueType::Scalar(Type::Integer | Type::Real)) {
                            return Err(vec![self.error_at(
                                DiagnosticKind::Type,
                                "o operador unário precisa de um número",
                                expression.line,
                                expression.column,
                            )]);
                        }
                        Ok(value)
                    }
                }
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left_type = self.expression(left)?;
                let right_type = self.expression(right)?;
                self.binary_type(*operator, left_type, right_type, expression)
            }
            ExpressionKind::Assign {
                target,
                operator,
                value,
            } => {
                let (target_type, constant) = self.target(target)?;
                if constant {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "não é possível alterar uma constante",
                        target.line,
                        target.column,
                    )]);
                }
                if *operator != AssignmentOperator::Assign {
                    self.expression(target)?;
                }
                let value_type = self.expression(value)?;
                if *operator == AssignmentOperator::Assign {
                    self.expect_compatible(target_type, value_type, value)?;
                } else {
                    let binary = match operator {
                        AssignmentOperator::Add => BinaryOperator::Add,
                        AssignmentOperator::Subtract => BinaryOperator::Subtract,
                        AssignmentOperator::Multiply => BinaryOperator::Multiply,
                        AssignmentOperator::Divide => BinaryOperator::Divide,
                        AssignmentOperator::Assign => unreachable!(),
                    };
                    let result = self.binary_type(binary, target_type, value_type, expression)?;
                    self.expect_compatible(target_type, result, expression)?;
                }
                self.mark_target_initialized(target);
                Ok(target_type)
            }
            ExpressionKind::Call { callee, arguments } => {
                let Some(name) = callable_name(callee) else {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "a expressão chamada não é uma função",
                        expression.line,
                        expression.column,
                    )]);
                };
                self.call(&name, arguments, expression)
            }
            ExpressionKind::Index { target, index } => {
                let target_type = self.expression(target)?;
                let index_type = self.expression(index)?;
                self.expect_scalar(index_type, Type::Integer, index)?;
                match target_type {
                    ValueType::Array(ty, 1) => Ok(ValueType::Scalar(ty)),
                    ValueType::Array(ty, dimensions) => {
                        Ok(ValueType::Array(ty, dimensions.saturating_sub(1)))
                    }
                    _ => Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "o valor indexado não é um vetor",
                        target.line,
                        target.column,
                    )]),
                }
            }
            ExpressionKind::Member { target, name } => {
                let ExpressionKind::Variable(module) = &target.kind else {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "biblioteca desconhecida",
                        target.line,
                        target.column,
                    )]);
                };
                let Some(library) = self.libraries.get(module) else {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        format!("biblioteca '{module}' não incluída"),
                        target.line,
                        target.column,
                    )]);
                };
                self.constant_type(library, name).ok_or_else(|| {
                    vec![self.error_at(
                        DiagnosticKind::Type,
                        format!("constante '{}.{}' não encontrada", module, name),
                        expression.line,
                        expression.column,
                    )]
                })
            }
            ExpressionKind::Postfix { target, .. } | ExpressionKind::Prefix { target, .. } => {
                let (target_type, constant) = self.target(target)?;
                if constant {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "não é possível alterar uma constante",
                        target.line,
                        target.column,
                    )]);
                }
                self.expect_scalar(target_type, Type::Integer, target)?;
                self.expression(target)?;
                self.mark_target_initialized(target);
                Ok(target_type)
            }
        }
    }

    fn call(
        &mut self,
        name: &str,
        arguments: &[Expression],
        expression: &Expression,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if name == "escreva" || name == "escreva_linha" {
            for argument in arguments {
                self.expression(argument)?;
            }
            return Ok(ValueType::Void);
        }
        if name == "limpa" {
            self.check_arity(name, arguments, 0, expression)?;
            return Ok(ValueType::Void);
        }
        if name == "leia" {
            if arguments.is_empty() {
                return Err(vec![self.error_at(
                    DiagnosticKind::Type,
                    "'leia' precisa receber ao menos uma variável",
                    expression.line,
                    expression.column,
                )]);
            }
            for argument in arguments {
                let (_, constant) = self.target(argument)?;
                if constant {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "não é possível ler para uma constante",
                        argument.line,
                        argument.column,
                    )]);
                }
                self.mark_target_initialized(argument);
            }
            return Ok(ValueType::Void);
        }

        if let Some((module, function)) = name.rsplit_once('.') {
            let Some(library) = self.libraries.get(module).cloned() else {
                return Err(vec![self.error_at(
                    DiagnosticKind::Type,
                    format!("biblioteca '{module}' não incluída"),
                    expression.line,
                    expression.column,
                )]);
            };
            let Some((arity, result)) = builtin_signature(&library, function) else {
                return Err(vec![self.error_at(
                    DiagnosticKind::Type,
                    format!("função '{function}' não encontrada na biblioteca '{library}'"),
                    expression.line,
                    expression.column,
                )]);
            };
            if let Some(arity) = arity {
                self.check_arity(
                    &format!("{library}.{function}"),
                    arguments,
                    arity,
                    expression,
                )?;
            }
            for argument in arguments {
                self.expression(argument)?;
            }
            return Ok(result);
        }

        let Some(candidates) = self.functions.get(name).cloned() else {
            return Err(vec![self.error_at(
                DiagnosticKind::Type,
                format!("função '{name}' não encontrada"),
                expression.line,
                expression.column,
            )]);
        };
        let candidates: Vec<_> = candidates
            .into_iter()
            .filter(|function| function.parameters.len() == arguments.len())
            .collect();
        if candidates.is_empty() {
            let arities = self.functions[name]
                .iter()
                .map(|function| function.parameters.len().to_string())
                .collect::<Vec<_>>()
                .join(" ou ");
            return Err(vec![self.error_at(
                DiagnosticKind::Type,
                format!("a função '{name}' espera {arities} argumento(s)"),
                expression.line,
                expression.column,
            )]);
        }

        let mut arguments_types = Vec::with_capacity(arguments.len());
        for (index, argument) in arguments.iter().enumerate() {
            if candidates
                .iter()
                .all(|function| function.parameters[index].by_reference)
            {
                let (ty, constant) = self.target(argument)?;
                if constant {
                    return Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "não é possível passar uma constante por referência",
                        argument.line,
                        argument.column,
                    )]);
                }
                arguments_types.push(ty);
            } else {
                arguments_types.push(self.expression(argument)?);
            }
        }

        let mut matching: Vec<_> = candidates
            .into_iter()
            .filter_map(|function| {
                let mut score = 0_u32;
                for (parameter, actual) in function.parameters.iter().zip(&arguments_types) {
                    let expected = if parameter.dimensions == 0 {
                        ValueType::Scalar(parameter.ty)
                    } else {
                        ValueType::Array(parameter.ty, parameter.dimensions)
                    };
                    if parameter.by_reference && *actual == expected {
                        continue;
                    }
                    if !parameter.by_reference && *actual == expected {
                        continue;
                    }
                    if !parameter.by_reference
                        && *actual == ValueType::Scalar(Type::Integer)
                        && parameter.ty == Type::Real
                        && parameter.dimensions == 0
                    {
                        score += 1;
                        continue;
                    }
                    return None;
                }
                Some((score, function))
            })
            .collect();
        matching.sort_by_key(|(score, _)| *score);
        let best_score = matching.first().map(|(score, _)| *score);
        let best_count = best_score.map_or(0, |score| {
            matching
                .iter()
                .take_while(|(candidate_score, _)| *candidate_score == score)
                .count()
        });
        if best_count != 1 {
            return Err(vec![self.error_at(
                DiagnosticKind::Type,
                if best_count == 0 {
                    format!("nenhuma assinatura da função '{name}' aceita estes argumentos")
                } else {
                    format!("chamada ambígua para a função '{name}'")
                },
                expression.line,
                expression.column,
            )]);
        }
        let function = matching[0].1;
        for (parameter, argument) in function.parameters.iter().zip(arguments) {
            if parameter.by_reference {
                self.mark_target_initialized(argument);
            }
        }
        Ok(function
            .return_type
            .map(ValueType::Scalar)
            .unwrap_or(ValueType::Void))
    }

    fn binary_type(
        &self,
        operator: BinaryOperator,
        left: ValueType,
        right: ValueType,
        expression: &Expression,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        use BinaryOperator as Op;
        let invalid = || {
            vec![self.error_at(
                DiagnosticKind::Type,
                "tipos incompatíveis para esta operação",
                expression.line,
                expression.column,
            )]
        };
        match operator {
            Op::Add
                if left == ValueType::Scalar(Type::String)
                    || right == ValueType::Scalar(Type::String)
                    || left == ValueType::Scalar(Type::Character)
                        && right == ValueType::Scalar(Type::Character) =>
            {
                Ok(ValueType::Scalar(Type::String))
            }
            Op::Add | Op::Subtract | Op::Multiply | Op::Divide => {
                if is_numeric(left) && is_numeric(right) {
                    Ok(
                        if left == ValueType::Scalar(Type::Integer)
                            && right == ValueType::Scalar(Type::Integer)
                        {
                            ValueType::Scalar(Type::Integer)
                        } else {
                            ValueType::Scalar(Type::Real)
                        },
                    )
                } else {
                    Err(invalid())
                }
            }
            Op::Remainder
            | Op::BitAnd
            | Op::BitOr
            | Op::BitXor
            | Op::ShiftLeft
            | Op::ShiftRight => {
                if left == ValueType::Scalar(Type::Integer)
                    && right == ValueType::Scalar(Type::Integer)
                {
                    Ok(ValueType::Scalar(Type::Integer))
                } else {
                    Err(invalid())
                }
            }
            Op::Equal | Op::NotEqual => {
                self.expect_compatible(left, right, expression)?;
                Ok(ValueType::Scalar(Type::Logical))
            }
            Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual => {
                if is_numeric(left) && is_numeric(right)
                    || left == right
                        && matches!(left, ValueType::Scalar(Type::String | Type::Character))
                {
                    Ok(ValueType::Scalar(Type::Logical))
                } else {
                    Err(invalid())
                }
            }
            Op::And | Op::Or => {
                if left == ValueType::Scalar(Type::Logical)
                    && right == ValueType::Scalar(Type::Logical)
                {
                    Ok(ValueType::Scalar(Type::Logical))
                } else {
                    Err(invalid())
                }
            }
        }
    }

    fn target(&mut self, expression: &Expression) -> Result<(ValueType, bool), Vec<Diagnostic>> {
        match &expression.kind {
            ExpressionKind::Variable(name) => self
                .lookup(name)
                .map(|variable| (variable_type(variable), variable.is_constant))
                .ok_or_else(|| {
                    vec![self.error_at(
                        DiagnosticKind::Type,
                        format!("variável '{name}' não declarada"),
                        expression.line,
                        expression.column,
                    )]
                }),
            ExpressionKind::Index { target, index } => {
                let (target_type, constant) = self.target(target)?;
                let index_type = self.expression(index)?;
                self.expect_scalar(index_type, Type::Integer, index)?;
                match target_type {
                    ValueType::Array(ty, 1) => Ok((ValueType::Scalar(ty), constant)),
                    ValueType::Array(ty, dimensions) => {
                        Ok((ValueType::Array(ty, dimensions - 1), constant))
                    }
                    _ => Err(vec![self.error_at(
                        DiagnosticKind::Type,
                        "o valor indexado não é um vetor",
                        target.line,
                        target.column,
                    )]),
                }
            }
            _ => Err(vec![self.error_at(
                DiagnosticKind::Type,
                "esperada uma variável para atribuição",
                expression.line,
                expression.column,
            )]),
        }
    }

    fn constant_type(&self, library: &str, name: &str) -> Option<ValueType> {
        let valid = match library {
            "Matematica" => name == "PI",
            "Arquivos" => matches!(name, "MODO_LEITURA" | "MODO_ESCRITA" | "MODO_ACRESCENTAR"),
            "Objetos" => matches!(
                name,
                "TIPO_INTEIRO"
                    | "TIPO_CADEIA"
                    | "TIPO_CARACTER"
                    | "TIPO_REAL"
                    | "TIPO_LOGICO"
                    | "TIPO_OBJETO"
                    | "TIPO_VETOR"
            ),
            "ServicosWeb" => matches!(name, "OBTER" | "PUBLICAR" | "ATUALIZAR" | "EXCLUIR"),
            "Mouse" => matches!(name, "BOTAO_ESQUERDO" | "BOTAO_DIREITO" | "BOTAO_MEIO"),
            "Calendario" => matches!(
                name,
                "DIA_DOMINGO"
                    | "DIA_SEGUNDA_FEIRA"
                    | "DIA_TERCA_FEIRA"
                    | "DIA_QUARTA_FEIRA"
                    | "DIA_QUINTA_FEIRA"
                    | "DIA_SEXTA_FEIRA"
                    | "DIA_SABADO"
                    | "MES_JANEIRO"
                    | "MES_FEVEREIRO"
                    | "MES_MARCO"
                    | "MES_ABRIL"
                    | "MES_MAIO"
                    | "MES_JUNHO"
                    | "MES_JULHO"
                    | "MES_AGOSTO"
                    | "MES_SETEMBRO"
                    | "MES_OUTUBRO"
                    | "MES_NOVEMBRO"
                    | "MES_DEZEMBRO"
            ),
            "Teclado" => crate::keyboard::constant_code(name).is_some(),
            "Graficos" => crate::graphics_constants::constant_value(name).is_some(),
            _ => false,
        };
        valid.then_some(if library == "Matematica" {
            ValueType::Scalar(Type::Real)
        } else {
            ValueType::Scalar(Type::Integer)
        })
    }

    fn expect_variable_type(
        &self,
        expected: Type,
        dimensions: usize,
        actual: ValueType,
        expression: &Expression,
    ) -> Result<(), Vec<Diagnostic>> {
        let expected = if dimensions == 0 {
            ValueType::Scalar(expected)
        } else {
            ValueType::Array(expected, dimensions)
        };
        self.expect_compatible(expected, actual, expression)
    }

    fn expect_scalar(
        &self,
        actual: ValueType,
        expected: Type,
        expression: &Expression,
    ) -> Result<(), Vec<Diagnostic>> {
        self.expect_compatible(ValueType::Scalar(expected), actual, expression)
    }

    fn expect_compatible(
        &self,
        expected: ValueType,
        actual: ValueType,
        expression: &Expression,
    ) -> Result<(), Vec<Diagnostic>> {
        let compatible = expected == actual
            || expected == ValueType::Scalar(Type::Real)
                && actual == ValueType::Scalar(Type::Integer)
            || actual == ValueType::Unknown;
        if compatible {
            Ok(())
        } else {
            Err(vec![self.error_at(
                DiagnosticKind::Type,
                format!(
                    "tipo incompatível: esperado {}, recebido {}",
                    type_label(expected),
                    type_label(actual)
                ),
                expression.line,
                expression.column,
            )])
        }
    }

    fn check_arity(
        &self,
        name: &str,
        arguments: &[Expression],
        expected: usize,
        expression: &Expression,
    ) -> Result<(), Vec<Diagnostic>> {
        if arguments.len() == expected {
            Ok(())
        } else {
            Err(vec![self.error_at(
                DiagnosticKind::Type,
                format!(
                    "a função '{name}' espera {expected} argumento(s), recebeu {}",
                    arguments.len()
                ),
                expression.line,
                expression.column,
            )])
        }
    }

    fn lookup(&self, name: &str) -> Option<&VariableInfo> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn mark_initialized(&mut self, name: &str) {
        if let Some(variable) = self
            .scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(name))
        {
            variable.initialized = true;
        }
    }

    fn mark_target_initialized(&mut self, target: &Expression) {
        match &target.kind {
            ExpressionKind::Variable(name) => self.mark_initialized(name),
            ExpressionKind::Index { .. } => {}
            _ => {}
        }
    }

    fn error_at(
        &self,
        kind: DiagnosticKind,
        message: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Diagnostic {
        Diagnostic::new(kind, message, line, column)
    }
}

fn merge_scopes(
    before: &[HashMap<String, VariableInfo>],
    left: &[HashMap<String, VariableInfo>],
    right: &[HashMap<String, VariableInfo>],
) -> Vec<HashMap<String, VariableInfo>> {
    before
        .iter()
        .enumerate()
        .map(|(scope_index, scope)| {
            scope
                .iter()
                .map(|(name, variable)| {
                    let initialized = left
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|left| left.initialized)
                        && right
                            .get(scope_index)
                            .and_then(|scope| scope.get(name))
                            .is_some_and(|right| right.initialized);
                    let mut variable = variable.clone();
                    variable.initialized = initialized;
                    (name.clone(), variable)
                })
                .collect()
        })
        .collect()
}

fn variable_type(variable: &VariableInfo) -> ValueType {
    if variable.dimensions == 0 {
        ValueType::Scalar(variable.ty)
    } else {
        ValueType::Array(variable.ty, variable.dimensions)
    }
}

fn is_numeric(value: ValueType) -> bool {
    matches!(value, ValueType::Scalar(Type::Integer | Type::Real))
}

fn type_label(value: ValueType) -> &'static str {
    match value {
        ValueType::Scalar(Type::Integer) => "inteiro",
        ValueType::Scalar(Type::Real) => "real",
        ValueType::Scalar(Type::Logical) => "lógico",
        ValueType::Scalar(Type::Character) => "caractere",
        ValueType::Scalar(Type::String) => "cadeia",
        ValueType::Scalar(Type::Void) => "vazio",
        ValueType::Array(_, _) => "vetor",
        ValueType::Void => "vazio",
        ValueType::Unknown => "desconhecido",
    }
}

fn callable_name(expression: &Expression) -> Option<String> {
    match &expression.kind {
        ExpressionKind::Variable(name) => Some(name.clone()),
        ExpressionKind::Member { target, name } => match &target.kind {
            ExpressionKind::Variable(module) => Some(format!("{module}.{name}")),
            _ => None,
        },
        _ => None,
    }
}

fn constant_integer(expression: &Expression) -> Option<i64> {
    match &expression.kind {
        ExpressionKind::Integer(value) => Some(*value),
        ExpressionKind::Unary {
            operator: UnaryOperator::Negate,
            operand,
        } => constant_integer(operand)?.checked_neg(),
        _ => None,
    }
}

fn builtin_signature(library: &str, function: &str) -> Option<(Option<usize>, ValueType)> {
    let name = function;
    let (arity, result) = match library {
        "Matematica" => {
            let arity = match name {
                "potencia" | "raiz" | "arredondar" | "logaritmo" | "maior_numero"
                | "menor_numero" => 2,
                "seno" | "cosseno" | "tangente" | "valor_absoluto" => 1,
                _ => return None,
            };
            (arity, ValueType::Scalar(Type::Real))
        }
        "Texto" => {
            let (arity, ty) = match name {
                "numero_caracteres" | "posicao_texto" => {
                    (if name == "posicao_texto" { 3 } else { 1 }, Type::Integer)
                }
                "caixa_alta"
                | "caixa_baixa"
                | "substituir"
                | "preencher_a_esquerda"
                | "extrair_subtexto" => (
                    if matches!(
                        name,
                        "substituir" | "preencher_a_esquerda" | "extrair_subtexto"
                    ) {
                        3
                    } else {
                        1
                    },
                    Type::String,
                ),
                "obter_caracter" => (2, Type::Character),
                _ => return None,
            };
            (arity, ValueType::Scalar(ty))
        }
        "Tipos" => {
            let (arity, ty) = match name {
                "cadeia_e_real"
                | "cadeia_e_logico"
                | "cadeia_e_caracter"
                | "inteiro_e_caracter"
                | "inteiro_para_logico"
                | "caracter_e_inteiro"
                | "caracter_e_logico"
                | "caracter_para_logico" => (1, Type::Logical),
                "cadeia_e_inteiro" | "cadeia_para_inteiro" | "inteiro_para_cadeia" => (
                    2,
                    if name == "inteiro_para_cadeia" {
                        Type::String
                    } else {
                        Type::Integer
                    },
                ),
                "cadeia_para_caracter" | "inteiro_para_caracter" | "logico_para_caracter" => {
                    (1, Type::Character)
                }
                "cadeia_para_real" | "inteiro_para_real" => (1, Type::Real),
                "cadeia_para_logico" => (1, Type::Logical),
                "caracter_para_cadeia" | "logico_para_cadeia" | "real_para_cadeia" => {
                    (1, Type::String)
                }
                "caracter_para_inteiro" | "logico_para_inteiro" | "real_para_inteiro" => {
                    (1, Type::Integer)
                }
                _ => return None,
            };
            (arity, ValueType::Scalar(ty))
        }
        "Util" => match name {
            "numero_elementos" | "numero_linhas" | "numero_colunas" | "aguarde" => (
                1,
                if name == "aguarde" {
                    ValueType::Void
                } else {
                    ValueType::Scalar(Type::Integer)
                },
            ),
            "sorteia" => (2, ValueType::Scalar(Type::Integer)),
            "tempo_decorrido" => (0, ValueType::Scalar(Type::Integer)),
            "obter_diretorio_usuario" => (0, ValueType::Scalar(Type::String)),
            _ => return None,
        },
        "Calendario" => match name {
            "dia_mes_atual" | "dia_semana_atual" | "mes_atual" | "ano_atual" | "minuto_atual"
            | "segundo_atual" | "milisegundo_atual" => (0, ValueType::Scalar(Type::Integer)),
            "hora_atual" => (1, ValueType::Scalar(Type::Integer)),
            "dia_semana_completo" | "dia_semana_curto" | "dia_semana_abreviado" => {
                (3, ValueType::Scalar(Type::String))
            }
            _ => return None,
        },
        "Arquivos" => match name {
            "abrir_arquivo"
            | "escrever_linha"
            | "fechar_arquivo"
            | "substituir_texto"
            | "apagar_arquivo"
            | "criar_pasta"
            | "listar_pastas"
            | "listar_arquivos"
            | "listar_arquivos_por_tipo" => (
                match name {
                    "abrir_arquivo" | "escrever_linha" => 2,
                    "substituir_texto" => 4,
                    "listar_arquivos_por_tipo" => 3,
                    "listar_pastas" | "listar_arquivos" => 2,
                    _ => 1,
                },
                if name == "abrir_arquivo" {
                    ValueType::Scalar(Type::Integer)
                } else {
                    ValueType::Void
                },
            ),
            "fim_arquivo" | "arquivo_existe" => (1, ValueType::Scalar(Type::Logical)),
            "ler_linha" | "selecionar_arquivo" => (
                if name == "selecionar_arquivo" { 2 } else { 1 },
                ValueType::Scalar(Type::String),
            ),
            _ => return None,
        },
        "Objetos" => {
            let arity = match name {
                "criar_objeto" | "liberar" => 0,
                "criar_objeto_via_json"
                | "criar_objeto_via_xml"
                | "liberar_objeto"
                | "obter_json" => 1,
                "atribuir_propriedade" => 3,
                "obter_propriedade_tipo_objeto_em_vetor"
                | "obter_propriedade_tipo_caracter_em_vetor"
                | "obter_propriedade_tipo_logico_em_vetor"
                | "obter_propriedade_tipo_real_em_vetor"
                | "obter_propriedade_tipo_inteiro_em_vetor"
                | "obter_propriedade_tipo_cadeia_em_vetor" => 3,
                "obter_propriedade_tipo_inteiro"
                | "obter_propriedade_tipo_real"
                | "obter_propriedade_tipo_logico"
                | "obter_propriedade_tipo_caracter"
                | "obter_propriedade_tipo_cadeia"
                | "obter_propriedade_tipo_objeto"
                | "obter_tamanho_vetor_propriedade"
                | "contem_propriedade"
                | "tipo_propriedade" => 2,
                _ => return None,
            };
            let ty = match name {
                "liberar" | "liberar_objeto" | "atribuir_propriedade" => ValueType::Void,
                "obter_propriedade_tipo_real" | "obter_propriedade_tipo_real_em_vetor" => {
                    ValueType::Scalar(Type::Real)
                }
                "obter_propriedade_tipo_logico"
                | "obter_propriedade_tipo_logico_em_vetor"
                | "contem_propriedade" => ValueType::Scalar(Type::Logical),
                "obter_propriedade_tipo_caracter" | "obter_propriedade_tipo_caracter_em_vetor" => {
                    ValueType::Scalar(Type::Character)
                }
                "obter_propriedade_tipo_cadeia"
                | "obter_propriedade_tipo_cadeia_em_vetor"
                | "obter_json" => ValueType::Scalar(Type::String),
                _ => ValueType::Scalar(Type::Integer),
            };
            (arity, ty)
        }
        "Internet" => match name {
            "definir_tempo_limite" | "obter_texto" | "endereco_disponivel" => (
                1,
                match name {
                    "definir_tempo_limite" => ValueType::Void,
                    "endereco_disponivel" => ValueType::Scalar(Type::Logical),
                    _ => ValueType::Scalar(Type::String),
                },
            ),
            "baixar_imagem" => (2, ValueType::Scalar(Type::String)),
            _ => return None,
        },
        "ServicosWeb" => match name {
            "obter_dados" | "excluir_dados" | "abrir_conexao" | "adicionar_parametros" => (
                1,
                match name {
                    "abrir_conexao" | "adicionar_parametros" => ValueType::Void,
                    _ => ValueType::Scalar(Type::String),
                },
            ),
            "publicar_dados" | "atualizar_dados" | "adicionar_cabecalho" => (
                2,
                if name == "adicionar_cabecalho" {
                    ValueType::Void
                } else {
                    ValueType::Scalar(Type::String)
                },
            ),
            "fazer_requisicao" => (1, ValueType::Scalar(Type::String)),
            _ => return None,
        },
        "Mouse" => match name {
            "botao_pressionado" => (1, ValueType::Scalar(Type::Logical)),
            "algum_botao_pressionado" => (0, ValueType::Scalar(Type::Logical)),
            "ler_botao" | "posicao_x" | "posicao_y" => (0, ValueType::Scalar(Type::Integer)),
            "ocultar_cursor" | "exibir_cursor" => (0, ValueType::Void),
            _ => return None,
        },
        "Teclado" => match name {
            "tecla_pressionada" => (1, ValueType::Scalar(Type::Logical)),
            "alguma_tecla_pressionada" => (0, ValueType::Scalar(Type::Logical)),
            "ler_tecla" => (0, ValueType::Scalar(Type::Integer)),
            "caracter_tecla" => (1, ValueType::Scalar(Type::Character)),
            _ => return None,
        },
        "Sons" => match name {
            "carregar_som" => (1, ValueType::Scalar(Type::Integer)),
            "liberar_som" | "pausar_som" | "interromper_som" | "definir_volume" => {
                (1, ValueType::Void)
            }
            "reproduzir_som" => (2, ValueType::Scalar(Type::Integer)),
            "obter_tamanho_musica" | "obter_posicao_atual_musica" | "obter_volume_reproducao" => {
                (1, ValueType::Scalar(Type::Integer))
            }
            "obter_volume" => (0, ValueType::Scalar(Type::Integer)),
            "definir_posicao_atual_musica" | "definir_volume_reproducao" => (2, ValueType::Void),
            _ => return None,
        },
        "Graficos" => {
            let (arity, returns_integer) = match name {
                "iniciar_modo_grafico"
                | "definir_titulo_janela"
                | "carregar_fonte"
                | "definir_icone_janela"
                | "definir_cor"
                | "definir_fonte_texto"
                | "definir_tamanho_texto"
                | "definir_opacidade"
                | "definir_rotacao"
                | "proximo_frame_gif" => (1, false),
                "carregar_imagem" => (1, true),
                "liberar_imagem" => (1, false),
                "fechar_janela"
                | "minimizar_janela"
                | "restaurar_janela"
                | "ocultar_borda_janela"
                | "exibir_borda_janela"
                | "encerrar_modo_grafico"
                | "limpar"
                | "renderizar"
                | "entrar_modo_tela_cheia"
                | "sair_modo_tela_cheia"
                | "largura_janela"
                | "altura_janela"
                | "largura_tela"
                | "altura_tela" => (
                    0,
                    matches!(
                        name,
                        "largura_janela" | "altura_janela" | "largura_tela" | "altura_tela"
                    ),
                ),
                "definir_dimensoes_janela"
                | "desenhar_poligono"
                | "definir_quadro_gif"
                | "salvar_imagem" => (2, false),
                "renderizar_imagem" | "obter_RGB" | "obter_quadro_gif" => (2, true),
                "desenhar_retangulo" => (6, false),
                "desenhar_elipse" => (5, false),
                "desenhar_ponto" => (2, false),
                "desenhar_linha" => (4, false),
                "transformar_imagem" => (5, true),
                "redimensionar_imagem" => (4, true),
                "obter_cor_pixel" => (3, true),
                "transformar_porcao_imagem" => (9, true),
                "desenhar_imagem" | "desenhar_quadro_atual_gif" => (3, false),
                "desenhar_porcao_imagem" => (7, false),
                "obter_intervalo_gif"
                | "obter_numero_quadros_gif"
                | "obter_numero_quadro_atual_gif" => (1, true),
                "desenhar_texto" => (3, false),
                "definir_gradiente" | "definir_estilo_texto" | "criar_cor" => {
                    (3, matches!(name, "criar_cor"))
                }
                "largura_texto" | "altura_texto" | "largura_imagem" | "altura_imagem" => (1, true),
                _ => return None,
            };
            (
                arity,
                if returns_integer {
                    ValueType::Scalar(Type::Integer)
                } else {
                    ValueType::Void
                },
            )
        }
        _ => return None,
    };
    Some((Some(arity), result))
}
