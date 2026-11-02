use std::collections::{HashMap, HashSet};

use crate::ast::{
    Block, Expr, ExprKind, ForGenerator, ForSource, FunctionBody, FunctionDecl, NodeId, Parameter,
    Pattern, PatternKind, Program, Stmt, StmtKind, StringPart, TypeDecl, TypeExpr, TypeExprKind,
};
use crate::types::{ClassInfo, FieldInfo, FunctionSignature, TypeInfo, ValueType};
use crate::{Diagnostic, DiagnosticKind, SourceSpan};

/// Infers and validates all declarations and expressions in a Potigol program.
pub fn check(program: &Program) -> Result<TypeInfo, Vec<Diagnostic>> {
    Checker::new().check_program(program)
}

#[derive(Clone)]
struct Scheme {
    variables: Vec<u32>,
    value_type: ValueType,
}

#[derive(Clone)]
struct Binding {
    scheme: Scheme,
    mutable: bool,
}

struct Checker {
    scopes: Vec<HashMap<String, Binding>>,
    substitutions: HashMap<u32, ValueType>,
    next_variable: u32,
    info: TypeInfo,
    known_types: HashSet<String>,
    class_declarations: HashMap<String, TypeDecl>,
    function_types: HashMap<NodeId, ValueType>,
    return_types: Vec<ValueType>,
    current_classes: Vec<String>,
    placeholder_parameters: Vec<Vec<ValueType>>,
}

impl Checker {
    fn new() -> Self {
        let mut checker = Self {
            scopes: vec![HashMap::new()],
            substitutions: HashMap::new(),
            next_variable: 0,
            info: TypeInfo::default(),
            known_types: HashSet::new(),
            class_declarations: HashMap::new(),
            function_types: HashMap::new(),
            return_types: Vec::new(),
            current_classes: Vec::new(),
            placeholder_parameters: Vec::new(),
        };
        checker.known_types.extend([
            "Inteiro".to_owned(),
            "Real".to_owned(),
            "BigInt".to_owned(),
            "Texto".to_owned(),
            "Caractere".to_owned(),
            "Lógico".to_owned(),
            "Nada".to_owned(),
        ]);
        checker
    }

    fn check_program(mut self, program: &Program) -> Result<TypeInfo, Vec<Diagnostic>> {
        self.collect_user_type_names(program)?;
        self.register_classes(program)?;
        self.register_top_level_functions(program)?;
        for statement in &program.statements {
            self.check_statement(statement)?;
        }
        self.resolve_type_info();
        Ok(self.info)
    }

    fn collect_user_type_names(&mut self, program: &Program) -> Result<(), Vec<Diagnostic>> {
        for statement in &program.statements {
            if let StmtKind::Type(declaration) = &statement.kind {
                if !self.known_types.insert(declaration.name.clone()) {
                    return Err(vec![self.error(
                        format!("o tipo '{}' já está definido", declaration.name),
                        declaration.span,
                    )]);
                }
                self.class_declarations
                    .insert(declaration.name.clone(), declaration.clone());
            }
        }
        Ok(())
    }

    fn register_classes(&mut self, program: &Program) -> Result<(), Vec<Diagnostic>> {
        let mut ordered = Vec::new();
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        for statement in &program.statements {
            let StmtKind::Type(declaration) = &statement.kind else {
                continue;
            };
            self.order_type_declaration(
                &declaration.name,
                &mut visiting,
                &mut visited,
                &mut ordered,
                declaration.span,
            )?;
        }

        for declaration in ordered {
            let mut fields = Vec::new();
            for field in &declaration.fields {
                let field_type = if let Some(annotation) = &field.annotation {
                    self.type_from_annotation(annotation)?
                } else {
                    self.fresh()
                };
                self.info.node_types.insert(field.id, field_type.clone());
                fields.push(FieldInfo {
                    name: field.name.clone(),
                    value_type: field_type,
                    mutable: field.mutable,
                    constructor_parameter: field.constructor_parameter,
                });
            }

            let mut inherited = declaration
                .parent
                .as_ref()
                .and_then(|parent| self.info.classes.get(parent))
                .cloned();
            let mut effective_fields = inherited
                .as_ref()
                .map_or_else(Vec::new, |class| class.fields.clone());
            for field in fields {
                if let Some(existing) = effective_fields
                    .iter_mut()
                    .find(|existing| existing.name == field.name)
                {
                    self.unify_assignable(
                        &existing.value_type,
                        &field.value_type,
                        declaration.span,
                    )?;
                    let inherited_mutability = existing.mutable;
                    *existing = FieldInfo {
                        mutable: inherited_mutability || field.mutable,
                        ..field
                    };
                } else {
                    effective_fields.push(field);
                }
            }

            let mut methods = inherited
                .take()
                .map_or_else(HashMap::new, |class| class.methods);
            for method in &declaration.methods {
                let method_type = self.function_type_for_declaration(method)?;
                if let Some(inherited) = methods.get(&method.name) {
                    let inherited = ValueType::Function(
                        inherited.parameters.clone(),
                        Box::new(inherited.return_type.clone()),
                    );
                    self.unify_assignable(&inherited, &method_type, method.span)?;
                }
                self.function_types.insert(method.id, method_type.clone());
                methods.insert(
                    method.name.clone(),
                    self.signature_from_function_type(method_type),
                );
            }
            self.info.classes.insert(
                declaration.name.clone(),
                ClassInfo {
                    name: declaration.name.clone(),
                    parent: declaration.parent.clone(),
                    abstract_type: declaration.abstract_type,
                    fields: effective_fields.clone(),
                    methods,
                },
            );
            let constructor_parameters = effective_fields
                .iter()
                .filter(|field| field.constructor_parameter)
                .map(|field| field.value_type.clone())
                .collect();
            let constructor = ValueType::Function(
                constructor_parameters,
                Box::new(ValueType::Named(declaration.name.clone())),
            );
            self.define(
                declaration.name.clone(),
                constructor,
                false,
                Vec::new(),
                declaration.span,
            )?;
        }
        Ok(())
    }

    fn order_type_declaration(
        &self,
        name: &str,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        ordered: &mut Vec<TypeDecl>,
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        if visited.contains(name) {
            return Ok(());
        }
        if !visiting.insert(name.to_owned()) {
            return Err(vec![self.error("herança circular de tipos", span)]);
        }
        let declaration = self.class_declarations.get(name).ok_or_else(|| {
            vec![self.error(format!("tipo pai '{name}' não foi declarado"), span)]
        })?;
        if let Some(parent) = &declaration.parent {
            if !self.class_declarations.contains_key(parent) {
                return Err(vec![self.error(
                    format!("tipo pai '{parent}' não foi declarado"),
                    declaration.span,
                )]);
            }
            self.order_type_declaration(parent, visiting, visited, ordered, declaration.span)?;
        }
        visiting.remove(name);
        visited.insert(name.to_owned());
        ordered.push(declaration.clone());
        Ok(())
    }

    fn register_top_level_functions(&mut self, program: &Program) -> Result<(), Vec<Diagnostic>> {
        for statement in &program.statements {
            if let StmtKind::Function(function) = &statement.kind {
                let function_type = self.function_type_for_declaration(function)?;
                self.function_types
                    .insert(function.id, function_type.clone());
                let signature = self.signature_from_function_type(function_type.clone());
                self.info.functions.insert(function.name.clone(), signature);
                self.define(
                    function.name.clone(),
                    function_type,
                    false,
                    Vec::new(),
                    function.span,
                )?;
            }
        }
        Ok(())
    }

    fn function_type_for_declaration(
        &mut self,
        declaration: &FunctionDecl,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let parameters = declaration
            .parameters
            .iter()
            .map(|parameter| self.parameter_type(parameter))
            .collect::<Result<Vec<_>, _>>()?;
        let return_type = if let Some(annotation) = &declaration.return_type {
            self.type_from_annotation(annotation)?
        } else {
            self.fresh()
        };
        Ok(ValueType::Function(parameters, Box::new(return_type)))
    }

    fn parameter_type(&mut self, parameter: &Parameter) -> Result<ValueType, Vec<Diagnostic>> {
        match &parameter.annotation {
            Some(annotation) => self.type_from_annotation(annotation),
            None => Ok(self.fresh()),
        }
    }

    fn signature_from_function_type(&mut self, value_type: ValueType) -> FunctionSignature {
        match self.resolve(value_type) {
            ValueType::Function(parameters, result) => FunctionSignature {
                parameters,
                return_type: *result,
            },
            _ => FunctionSignature {
                parameters: Vec::new(),
                return_type: ValueType::Nothing,
            },
        }
    }

    fn check_statement(&mut self, statement: &Stmt) -> Result<ValueType, Vec<Diagnostic>> {
        let value_type = match &statement.kind {
            StmtKind::Binding {
                pattern,
                annotation,
                mutable,
                value,
            } => {
                let value_type = if let (PatternKind::Tuple(_), ExprKind::Identifier(name)) =
                    (&pattern.kind, &value.kind)
                    && let Some(input_type) = input_type(name)
                {
                    self.input_pattern_type(pattern, input_type)
                } else {
                    self.check_expression(value)?
                };
                let binding_type = if let Some(annotation) = annotation {
                    let annotation_type = self.type_from_annotation(annotation)?;
                    self.annotated_pattern_type(pattern, &annotation_type, &value_type, value.span)?
                } else {
                    value_type
                };
                self.bind_pattern(pattern, binding_type, *mutable)?;
                ValueType::Nothing
            }
            StmtKind::Assignment { target, value } => {
                let target_type = self.check_assignment_target(target)?;
                let value_type = self.check_expression(value)?;
                self.check_assignment_types(&target_type, &value_type, value.span)?;
                ValueType::Nothing
            }
            StmtKind::Function(function) => {
                self.check_function(function, None)?;
                ValueType::Nothing
            }
            StmtKind::Type(declaration) => {
                self.check_type_declaration(declaration)?;
                ValueType::Nothing
            }
            StmtKind::Output { value, .. } => {
                let value_type = self.check_expression(value)?;
                self.ensure_displayable(&value_type, value.span)?;
                ValueType::Nothing
            }
            StmtKind::Return(value) => {
                let Some(expected) = self.return_types.last().cloned() else {
                    return Err(vec![
                        self.error("'retorne' fora de uma função", statement.span),
                    ]);
                };
                let actual = if let Some(value) = value {
                    self.check_expression(value)?
                } else {
                    ValueType::Nothing
                };
                self.unify_assignable(&expected, &actual, statement.span)?;
                actual
            }
            StmtKind::While { condition, body } => {
                let condition_type = self.check_expression(condition)?;
                self.unify(&ValueType::Logical, &condition_type, condition.span)?;
                self.check_block(body)?;
                ValueType::Nothing
            }
            StmtKind::For { generators, body } => {
                self.check_generators(generators)?;
                self.check_block(body)?;
                self.pop_scope();
                ValueType::Nothing
            }
            StmtKind::Expression(expression) => self.check_expression(expression)?,
        };
        self.info
            .node_types
            .insert(statement.id, value_type.clone());
        Ok(value_type)
    }

    fn input_pattern_type(&self, pattern: &Pattern, input: ValueType) -> ValueType {
        match &pattern.kind {
            PatternKind::Tuple(patterns) => ValueType::Tuple(
                patterns
                    .iter()
                    .map(|pattern| self.input_pattern_type(pattern, input.clone()))
                    .collect(),
            ),
            _ => input,
        }
    }

    fn annotated_pattern_type(
        &mut self,
        pattern: &Pattern,
        annotation: &ValueType,
        actual: &ValueType,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if let PatternKind::Tuple(patterns) = &pattern.kind {
            let annotations = match self.resolve(annotation.clone()) {
                ValueType::Tuple(types) if types.len() == patterns.len() => Some(types),
                _ => None,
            };
            let actuals = match self.resolve(actual.clone()) {
                ValueType::Tuple(types) if types.len() == patterns.len() => Some(types),
                ValueType::Variable(_) => {
                    let types = patterns.iter().map(|_| self.fresh()).collect::<Vec<_>>();
                    self.unify(actual, &ValueType::Tuple(types.clone()), pattern.span)?;
                    Some(types)
                }
                _ => None,
            };
            if matches!(self.resolve(annotation.clone()), ValueType::Tuple(_))
                && annotations.is_none()
            {
                return Err(vec![self.error(
                    "a quantidade de tipos da tupla não corresponde ao padrão",
                    pattern.span,
                )]);
            }
            let mut types = Vec::with_capacity(patterns.len());
            for index in 0..patterns.len() {
                let expected = annotations
                    .as_ref()
                    .map_or(annotation, |annotations| &annotations[index]);
                let value = actuals.as_ref().map_or(actual, |actuals| &actuals[index]);
                types.push(self.annotated_pattern_type(&patterns[index], expected, value, span)?);
            }
            return Ok(ValueType::Tuple(types));
        }
        self.unify_assignable(annotation, actual, span)?;
        Ok(annotation.clone())
    }

    fn check_assignment_types(
        &mut self,
        target: &ValueType,
        value: &ValueType,
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        if let ValueType::Tuple(targets) = self.resolve(target.clone()) {
            if let ValueType::Tuple(values) = self.resolve(value.clone()) {
                if targets.len() != values.len() {
                    return self.unify(&ValueType::Tuple(targets), &ValueType::Tuple(values), span);
                }
                for (target, value) in targets.iter().zip(&values) {
                    self.unify_assignable(target, value, span)?;
                }
            } else {
                for target in &targets {
                    self.unify_assignable(target, value, span)?;
                }
            }
            Ok(())
        } else {
            self.unify_assignable(target, value, span)
        }
    }

    fn check_function(
        &mut self,
        function: &FunctionDecl,
        owner: Option<&str>,
    ) -> Result<(), Vec<Diagnostic>> {
        let function_type = self
            .function_types
            .get(&function.id)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.function_type_for_declaration(function))?;
        if owner.is_none() && !self.is_defined_here(&function.name) {
            self.define(
                function.name.clone(),
                function_type.clone(),
                false,
                Vec::new(),
                function.span,
            )?;
        }
        let ValueType::Function(parameter_types, return_type) = self.resolve(function_type.clone())
        else {
            return Err(vec![
                self.error("assinatura de função inválida", function.span),
            ]);
        };

        self.push_scope();
        if let Some(owner) = owner {
            self.current_classes.push(owner.to_owned());
            if let Some(class) = self.info.classes.get(owner).cloned() {
                for field in class.fields {
                    self.define(
                        field.name,
                        field.value_type,
                        field.mutable,
                        Vec::new(),
                        function.span,
                    )?;
                }
                for (name, method) in class.methods {
                    self.define(
                        name,
                        ValueType::Function(method.parameters, Box::new(method.return_type)),
                        false,
                        Vec::new(),
                        function.span,
                    )?;
                }
            }
        }
        for (parameter, parameter_type) in function.parameters.iter().zip(parameter_types.iter()) {
            self.define(
                parameter.name.clone(),
                parameter_type.clone(),
                parameter.mutable,
                Vec::new(),
                parameter.span,
            )?;
            self.info
                .node_types
                .insert(parameter.id, parameter_type.clone());
        }
        self.return_types.push(*return_type.clone());
        let body_type = match &function.body {
            FunctionBody::Expression(expression) => self.check_expression(expression)?,
            FunctionBody::Block(block) => self.check_block(block)?,
        };
        self.unify_assignable(&return_type, &body_type, function.span)?;
        self.return_types.pop();
        self.pop_scope();
        if owner.is_some() {
            self.current_classes.pop();
        }
        let resolved = self.resolve(function_type);
        self.function_types.insert(function.id, resolved.clone());
        self.info.node_types.insert(function.id, resolved.clone());
        let signature = self.signature_from_function_type(resolved);
        if let Some(owner) = owner {
            if let Some(class) = self.info.classes.get_mut(owner) {
                class.methods.insert(function.name.clone(), signature);
            }
        } else {
            self.info.functions.insert(function.name.clone(), signature);
        }
        Ok(())
    }

    fn check_type_declaration(&mut self, declaration: &TypeDecl) -> Result<(), Vec<Diagnostic>> {
        self.push_scope();
        if let Some(class) = self.info.classes.get(&declaration.name).cloned() {
            for field in class.fields {
                self.define(
                    field.name,
                    field.value_type,
                    field.mutable,
                    Vec::new(),
                    declaration.span,
                )?;
            }
        }
        for field in &declaration.fields {
            if let Some(default_value) = &field.default_value {
                let value_type = self.check_expression(default_value)?;
                if let Some(class) = self.info.classes.get(&declaration.name) {
                    if let Some(field_type) = class
                        .fields
                        .iter()
                        .find(|item| item.name == field.name)
                        .map(|field| field.value_type.clone())
                    {
                        self.unify_assignable(&field_type, &value_type, default_value.span)?;
                    }
                }
            }
        }
        self.pop_scope();
        for method in &declaration.methods {
            self.check_function(method, Some(&declaration.name))?;
        }
        self.info
            .node_types
            .insert(declaration.id, ValueType::Named(declaration.name.clone()));
        Ok(())
    }

    fn check_block(&mut self, block: &Block) -> Result<ValueType, Vec<Diagnostic>> {
        self.push_scope();
        let mut result = ValueType::Nothing;
        for statement in &block.statements {
            result = self.check_statement(statement)?;
        }
        self.pop_scope();
        self.info.node_types.insert(block.id, result.clone());
        Ok(result)
    }

    fn check_expression(&mut self, expression: &Expr) -> Result<ValueType, Vec<Diagnostic>> {
        let value_type = match &expression.kind {
            ExprKind::Integer(_) => ValueType::Integer,
            ExprKind::Real(_) => ValueType::Real,
            ExprKind::Character(_) => ValueType::Character,
            ExprKind::Logical(_) => ValueType::Logical,
            ExprKind::Text(parts) => {
                for part in parts {
                    if let StringPart::Expression(interpolation) = part {
                        let interpolation_type = self.check_expression(interpolation)?;
                        self.ensure_displayable(&interpolation_type, interpolation.span)?;
                    }
                }
                ValueType::Text
            }
            ExprKind::Identifier(name) => self.identifier_type(name, expression.span)?,
            ExprKind::Placeholder => {
                if self.placeholder_parameters.is_empty() {
                    return Err(vec![self.error(
                        "'_' só pode aparecer como parâmetro de uma função de ordem superior",
                        expression.span,
                    )]);
                }
                let value_type = self.fresh();
                if let Some(parameters) = self.placeholder_parameters.last_mut() {
                    parameters.push(value_type.clone());
                }
                value_type
            }
            ExprKind::Tuple(elements) => ValueType::Tuple(
                elements
                    .iter()
                    .map(|element| self.check_expression(element))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            ExprKind::List(elements) => {
                let mut element_type = None;
                for element in elements {
                    let actual = self.check_expression(element)?;
                    element_type = Some(if let Some(current) = element_type {
                        self.merge_types(&current, &actual, element.span)?
                    } else {
                        actual
                    });
                }
                ValueType::List(Box::new(element_type.unwrap_or_else(|| self.fresh())))
            }
            ExprKind::Unary { operator, operand } => {
                let operand_type = self.check_expression(operand)?;
                match operator.as_str() {
                    "não" | "nao" => {
                        self.unify(&ValueType::Logical, &operand_type, operand.span)?;
                        ValueType::Logical
                    }
                    "+" | "-" => {
                        self.require_numeric(&operand_type, operand.span)?;
                        if matches!(self.resolve(operand_type.clone()), ValueType::Character) {
                            ValueType::Integer
                        } else {
                            operand_type
                        }
                    }
                    _ => {
                        return Err(vec![self.error(
                            format!("operador '{operator}' desconhecido"),
                            expression.span,
                        )]);
                    }
                }
            }
            ExprKind::Binary {
                left,
                operator,
                right,
            } => {
                let left_type = self.check_expression(left)?;
                let right_type = self.check_expression(right)?;
                self.check_binary(operator, &left_type, &right_type, expression.span)?
            }
            ExprKind::Call { callee, arguments } => {
                self.check_call(callee, arguments, expression.span)?
            }
            ExprKind::Member { receiver, name } => {
                let receiver_type = self.check_expression(receiver)?;
                self.member_type(&receiver_type, name, expression.span)?
            }
            ExprKind::Index { receiver, indices } => {
                if let Some(value_type) =
                    self.check_typed_empty_list(receiver, indices, expression.span)?
                {
                    value_type
                } else {
                    let receiver_type = self.check_expression(receiver)?;
                    let index_types = indices
                        .iter()
                        .map(|index| self.check_expression(index))
                        .collect::<Result<Vec<_>, _>>()?;
                    for (index, index_type) in indices.iter().zip(index_types.iter()) {
                        self.unify(&ValueType::Integer, index_type, index.span)?;
                    }
                    self.index_type(&receiver_type, indices, expression.span)?
                }
            }
            ExprKind::Lambda { parameters, body } => {
                self.push_scope();
                let mut parameter_types = Vec::with_capacity(parameters.len());
                for parameter in parameters {
                    let parameter_type = self.parameter_type(parameter)?;
                    self.define(
                        parameter.name.clone(),
                        parameter_type.clone(),
                        parameter.mutable,
                        Vec::new(),
                        parameter.span,
                    )?;
                    self.info
                        .node_types
                        .insert(parameter.id, parameter_type.clone());
                    parameter_types.push(parameter_type);
                }
                let body_type = self.check_expression(body)?;
                self.pop_scope();
                ValueType::Function(parameter_types, Box::new(body_type))
            }
            ExprKind::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_type = self.check_expression(condition)?;
                self.unify(&ValueType::Logical, &condition_type, condition.span)?;
                let then_type = self.check_block(then_branch)?;
                if let Some(else_branch) = else_branch {
                    let else_type = self.check_block(else_branch)?;
                    self.merge_types(&then_type, &else_type, expression.span)?
                } else {
                    ValueType::Nothing
                }
            }
            ExprKind::Match { value, cases } => {
                let value_type = self.check_expression(value)?;
                let mut result_type = None;
                for case in cases {
                    self.push_scope();
                    self.bind_pattern(&case.pattern, value_type.clone(), false)?;
                    if let Some(guard) = &case.guard {
                        let guard_type = self.check_expression(guard)?;
                        self.unify(&ValueType::Logical, &guard_type, guard.span)?;
                    }
                    let branch_type = self.check_block(&case.value)?;
                    result_type = Some(if let Some(current) = result_type {
                        self.merge_types(&current, &branch_type, case.span)?
                    } else {
                        branch_type.clone()
                    });
                    self.pop_scope();
                    self.info.node_types.insert(case.id, branch_type);
                }
                result_type.unwrap_or(ValueType::Nothing)
            }
            ExprKind::Comprehension { generators, value } => {
                self.check_generators(generators)?;
                let element_type = self.check_expression(value)?;
                self.pop_scope();
                ValueType::List(Box::new(element_type))
            }
        };
        self.info
            .node_types
            .insert(expression.id, value_type.clone());
        Ok(value_type)
    }

    fn check_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if let ExprKind::Identifier(name) = &callee.kind {
            if self
                .info
                .classes
                .get(name)
                .is_some_and(|class| class.abstract_type)
            {
                return Err(vec![self.error(
                    format!("o tipo abstrato '{name}' não pode ser instanciado"),
                    span,
                )]);
            }
            if let Some(value_type) = self.check_builtin_call(name, arguments, span)? {
                return Ok(value_type);
            }
        }
        if let ExprKind::Member { receiver, name } = &callee.kind {
            return self.check_member_call(receiver, name, arguments, span);
        }
        let callee_type = self.check_expression(callee)?;
        let argument_types = arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let expected = match self.resolve(callee_type.clone()) {
                    ValueType::Function(parameters, _) => parameters.get(index).cloned(),
                    _ => None,
                };
                if let Some(expected) = expected {
                    self.check_argument_as(argument, &expected)
                } else {
                    self.check_argument(argument)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.apply_type(callee_type, &argument_types, span)
    }

    fn check_argument_as(
        &mut self,
        argument: &Expr,
        expected: &ValueType,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let expected = self.resolve(expected.clone());
        if let (
            ExprKind::Lambda { parameters, body },
            ValueType::Function(expected_parameters, expected_result),
        ) = (&argument.kind, &expected)
            && parameters.len() == expected_parameters.len()
        {
            self.push_scope();
            for (parameter, expected) in parameters.iter().zip(expected_parameters) {
                let parameter_type = if let Some(annotation) = &parameter.annotation {
                    self.type_from_annotation(annotation)?
                } else {
                    expected.clone()
                };
                self.unify_assignable(expected, &parameter_type, parameter.span)?;
                self.define(
                    parameter.name.clone(),
                    parameter_type.clone(),
                    parameter.mutable,
                    Vec::new(),
                    parameter.span,
                )?;
                self.info.node_types.insert(parameter.id, parameter_type);
            }
            let result = self.check_expression(body)?;
            self.unify_assignable(expected_result, &result, body.span)?;
            self.pop_scope();
            Ok(ValueType::Function(
                expected_parameters.clone(),
                Box::new(result),
            ))
        } else {
            let actual = self.check_argument(argument)?;
            self.unify_assignable(&expected, &actual, argument.span)?;
            Ok(actual)
        }
    }

    fn check_argument(&mut self, argument: &Expr) -> Result<ValueType, Vec<Diagnostic>> {
        self.placeholder_parameters.push(Vec::new());
        let result = self.check_expression(argument);
        let parameters = self.placeholder_parameters.pop().unwrap_or_default();
        let result = result?;
        if parameters.is_empty() {
            Ok(result)
        } else {
            Ok(ValueType::Function(parameters, Box::new(result)))
        }
    }

    fn check_typed_empty_list(
        &mut self,
        receiver: &Expr,
        indices: &[Expr],
        span: SourceSpan,
    ) -> Result<Option<ValueType>, Vec<Diagnostic>> {
        let ExprKind::Member {
            receiver: factory,
            name,
        } = &receiver.kind
        else {
            return Ok(None);
        };
        if name != "vazia"
            || !matches!(&factory.kind, ExprKind::Identifier(factory_name) if factory_name == "Lista")
        {
            return Ok(None);
        }
        if indices.len() != 1 {
            return Err(vec![
                self.error("Lista.vazia precisa de exatamente um tipo", span),
            ]);
        }
        let element_type = self.type_argument(&indices[0])?;
        let list_type = ValueType::List(Box::new(element_type));
        self.info
            .node_types
            .insert(factory.id, ValueType::Named("Lista".to_owned()));
        self.info.node_types.insert(receiver.id, list_type.clone());
        Ok(Some(list_type))
    }

    fn type_argument(&mut self, expression: &Expr) -> Result<ValueType, Vec<Diagnostic>> {
        let value_type = match &expression.kind {
            ExprKind::Identifier(name) => self.type_name(name).ok_or_else(|| {
                vec![self.error(format!("tipo '{name}' não foi declarado"), expression.span)]
            })?,
            ExprKind::Tuple(elements) => ValueType::Tuple(
                elements
                    .iter()
                    .map(|element| self.type_argument(element))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            ExprKind::Index { receiver, indices } if matches!(&receiver.kind, ExprKind::Identifier(name) if name == "Lista") =>
            {
                if indices.len() != 1 {
                    return Err(vec![
                        self.error("Lista precisa de um tipo de elemento", expression.span),
                    ]);
                }
                self.info
                    .node_types
                    .insert(receiver.id, ValueType::Named("Lista".to_owned()));
                ValueType::List(Box::new(self.type_argument(&indices[0])?))
            }
            _ => {
                return Err(vec![
                    self.error("esperava um tipo para Lista.vazia", expression.span),
                ]);
            }
        };
        self.info
            .node_types
            .insert(expression.id, value_type.clone());
        Ok(value_type)
    }

    fn type_name(&self, name: &str) -> Option<ValueType> {
        match name.to_lowercase().as_str() {
            "inteiro" => Some(ValueType::Integer),
            "real" => Some(ValueType::Real),
            "bigint" => Some(ValueType::BigInteger),
            "texto" => Some(ValueType::Text),
            "caractere" | "caracter" => Some(ValueType::Character),
            "lógico" | "logico" => Some(ValueType::Logical),
            "nada" => Some(ValueType::Nothing),
            _ if self.known_types.contains(name) => Some(ValueType::Named(name.to_owned())),
            _ => None,
        }
    }

    fn check_builtin_call(
        &mut self,
        name: &str,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> Result<Option<ValueType>, Vec<Diagnostic>> {
        let result = match name {
            "Lista" | "lista" => {
                if arguments.len() != 2 {
                    return Err(vec![
                        self.error("Lista recebe tamanho e valor inicial", span),
                    ]);
                }
                let length_type = self.check_expression(&arguments[0])?;
                self.unify(&ValueType::Integer, &length_type, arguments[0].span)?;
                let element_type = self.check_expression(&arguments[1])?;
                ValueType::List(Box::new(element_type))
            }
            "BigInt" | "bigint" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("BigInt recebe um valor", span)]);
                }
                let source_type = self.check_expression(&arguments[0])?;
                if !matches!(
                    self.resolve(source_type),
                    ValueType::Integer | ValueType::Text | ValueType::BigInteger
                ) {
                    return Err(vec![
                        self.error("BigInt recebe Inteiro ou Texto", arguments[0].span),
                    ]);
                }
                ValueType::BigInteger
            }
            "leia_inteiros" => self.check_input_list(arguments, ValueType::Integer, span)?,
            "leia_reais" | "leia_numeros" | "leia_números" => {
                self.check_input_list(arguments, ValueType::Real, span)?
            }
            "leia_textos" => self.check_input_list(arguments, ValueType::Text, span)?,
            "min" | "menor" | "max" | "maior" => {
                if arguments.len() != 2 {
                    return Err(vec![
                        self.error(format!("{name} recebe dois argumentos"), span),
                    ]);
                }
                let first = self.check_expression(&arguments[0])?;
                let second = self.check_expression(&arguments[1])?;
                self.merge_types(&first, &second, span)?
            }
            "abs" | "raiz" | "sqrt" | "sen" | "cos" | "tan" | "tg" | "arcsen" | "arccos"
            | "arctg" | "log" | "log10" => {
                if arguments.len() != 1 {
                    return Err(vec![
                        self.error(format!("{name} recebe um argumento"), span),
                    ]);
                }
                let argument_type = self.check_expression(&arguments[0])?;
                self.require_numeric(&argument_type, arguments[0].span)?;
                if matches!(
                    name,
                    "raiz"
                        | "sqrt"
                        | "sen"
                        | "cos"
                        | "tan"
                        | "tg"
                        | "arcsen"
                        | "arccos"
                        | "arctg"
                        | "log"
                        | "log10"
                ) {
                    ValueType::Real
                } else {
                    argument_type
                }
            }
            "aleatório" | "aleatorio" => match arguments {
                [] => ValueType::Real,
                [argument] => {
                    let argument_type = self.check_expression(argument)?;
                    match self.resolve(argument_type.clone()) {
                        ValueType::List(element) | ValueType::MutableList(element) => *element,
                        ValueType::Integer => ValueType::Integer,
                        other => {
                            return Err(vec![
                                self.error(format!("aleatório não aceita {other}"), argument.span),
                            ]);
                        }
                    }
                }
                [minimum, maximum] => {
                    for argument in [minimum, maximum] {
                        let argument_type = self.check_expression(argument)?;
                        self.unify(&ValueType::Integer, &argument_type, argument.span)?;
                    }
                    ValueType::Integer
                }
                _ => {
                    return Err(vec![
                        self.error("aleatório aceita zero, um ou dois argumentos", span),
                    ]);
                }
            },
            "espera" => {
                if arguments.len() != 1 {
                    return Err(vec![
                        self.error("espera recebe um número de segundos", span),
                    ]);
                }
                let argument_type = self.check_expression(&arguments[0])?;
                self.require_numeric(&argument_type, arguments[0].span)?;
                ValueType::Nothing
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn check_input_list(
        &mut self,
        arguments: &[Expr],
        element_type: ValueType,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if arguments.len() > 1 {
            return Err(vec![
                self.error("leitura de lista aceita no máximo um argumento", span),
            ]);
        }
        if let Some(argument) = arguments.first() {
            let argument_type = self.check_expression(argument)?;
            if !matches!(
                self.resolve(argument_type),
                ValueType::Text | ValueType::Integer
            ) {
                return Err(vec![self.error(
                    "argumento de leitura deve ser Texto ou Inteiro",
                    argument.span,
                )]);
            }
        }
        Ok(ValueType::List(Box::new(element_type)))
    }

    fn identifier_type(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if let Some(binding) = self.lookup(name) {
            return Ok(self.instantiate(&binding.scheme));
        }
        let lower = name.to_lowercase();
        let builtin = match lower.as_str() {
            "leia_inteiro" => Some(ValueType::Integer),
            "leia_real" | "leia_numero" | "leia_número" => Some(ValueType::Real),
            "leia_texto" => Some(ValueType::Text),
            "leia_caractere" => Some(ValueType::Character),
            "leia_logico" => Some(ValueType::Logical),
            "eof" => Some(ValueType::Logical),
            "pi" => Some(ValueType::Real),
            "lista" => Some(ValueType::Named("Lista".to_owned())),
            "matriz" => Some(ValueType::Named("Matriz".to_owned())),
            "cubo" => Some(ValueType::Named("Cubo".to_owned())),
            _ => None,
        };
        if let Some(value_type) = builtin {
            return Ok(value_type);
        }
        Err(vec![self.error(
            format!("identificador '{name}' não foi declarado"),
            span,
        )])
    }

    fn check_binary(
        &mut self,
        operator: &str,
        left: &ValueType,
        right: &ValueType,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let left_resolved = self.resolve(left.clone());
        let right_resolved = self.resolve(right.clone());
        let result = match operator {
            "e" | "ou" => {
                self.unify(&ValueType::Logical, &left_resolved, span)?;
                self.unify(&ValueType::Logical, &right_resolved, span)?;
                ValueType::Logical
            }
            "==" | "<>" => {
                if matches!(left_resolved, ValueType::Variable(_))
                    || matches!(right_resolved, ValueType::Variable(_))
                {
                    self.unify(&left_resolved, &right_resolved, span)?;
                }
                ValueType::Logical
            }
            "<" | "<=" | ">" | ">=" => {
                if matches!(left_resolved, ValueType::Variable(_))
                    || matches!(right_resolved, ValueType::Variable(_))
                {
                    self.unify(&left_resolved, &right_resolved, span)?;
                } else if !compatible_comparison_types(&left_resolved, &right_resolved) {
                    return Err(vec![
                        self.error("tipos incompatíveis para comparação", span),
                    ]);
                }
                if !self.is_orderable(&left_resolved) {
                    return Err(vec![self.error("valores não podem ser ordenados", span)]);
                }
                ValueType::Logical
            }
            "::" => {
                let element = self.fresh();
                match &right_resolved {
                    ValueType::List(value) | ValueType::MutableList(value) => {
                        self.unify(&element, value, span)?;
                    }
                    _ => {
                        self.unify(
                            &ValueType::List(Box::new(element.clone())),
                            &right_resolved,
                            span,
                        )?;
                    }
                }
                self.unify(&element, &left_resolved, span)?;
                ValueType::List(Box::new(element))
            }
            "formato" => {
                self.unify(&ValueType::Text, &right_resolved, span)?;
                ValueType::Text
            }
            "+" => {
                if matches!(left_resolved, ValueType::Text) {
                    self.ensure_displayable(&right_resolved, span)?;
                    ValueType::Text
                } else {
                    match (&left_resolved, &right_resolved) {
                        (
                            ValueType::List(left) | ValueType::MutableList(left),
                            ValueType::List(right) | ValueType::MutableList(right),
                        ) => ValueType::List(Box::new(self.merge_types(left, right, span)?)),
                        _ => self.numeric_result(&left_resolved, &right_resolved, span)?,
                    }
                }
            }
            "-" | "*" => {
                if operator == "-"
                    && matches!(left_resolved, ValueType::Text)
                    && matches!(right_resolved, ValueType::Text)
                {
                    ValueType::Text
                } else if operator == "-"
                    && matches!(left_resolved, ValueType::Character)
                    && matches!(right_resolved, ValueType::Integer)
                {
                    ValueType::Integer
                } else if operator == "*" && matches!(left_resolved, ValueType::Text) {
                    self.unify(&ValueType::Integer, &right_resolved, span)?;
                    ValueType::Text
                } else {
                    self.numeric_result(&left_resolved, &right_resolved, span)?
                }
            }
            "^" => {
                if matches!(left_resolved, ValueType::BigInteger)
                    || matches!(right_resolved, ValueType::BigInteger)
                {
                    if !matches!(
                        left_resolved,
                        ValueType::Integer | ValueType::Character | ValueType::BigInteger
                    ) || !matches!(
                        right_resolved,
                        ValueType::Integer | ValueType::Character | ValueType::BigInteger
                    ) {
                        return Err(vec![self.error("potência de BigInt exige inteiros", span)]);
                    }
                    ValueType::BigInteger
                } else {
                    self.require_numeric(&left_resolved, span)?;
                    self.require_numeric(&right_resolved, span)?;
                    ValueType::Real
                }
            }
            "/" => {
                self.require_numeric(&left_resolved, span)?;
                self.require_numeric(&right_resolved, span)?;
                ValueType::Real
            }
            "div" => {
                let left = self.resolve(left_resolved);
                let right = self.resolve(right_resolved);
                if matches!(left, ValueType::BigInteger) || matches!(right, ValueType::BigInteger) {
                    if !matches!(
                        left,
                        ValueType::Integer | ValueType::Character | ValueType::BigInteger
                    ) || !matches!(
                        right,
                        ValueType::Integer | ValueType::Character | ValueType::BigInteger
                    ) {
                        return Err(vec![self.error("div exige inteiros", span)]);
                    }
                    ValueType::BigInteger
                } else {
                    self.require_integer_like(&left, span)?;
                    self.require_integer_like(&right, span)?;
                    ValueType::Integer
                }
            }
            "mod" => self.numeric_result(&left_resolved, &right_resolved, span)?,
            _ => {
                return Err(vec![
                    self.error(format!("operador '{operator}' desconhecido"), span),
                ]);
            }
        };
        Ok(result)
    }

    fn check_assignment_target(&mut self, target: &Expr) -> Result<ValueType, Vec<Diagnostic>> {
        let value_type = match &target.kind {
            ExprKind::Identifier(name) => {
                let Some(binding) = self.lookup(name) else {
                    return Err(vec![self.error(
                        format!("identificador '{name}' não foi declarado"),
                        target.span,
                    )]);
                };
                if !binding.mutable {
                    return Err(vec![self.error(
                        format!("'{name}' não foi declarado como mutável"),
                        target.span,
                    )]);
                }
                Ok(self.instantiate(&binding.scheme))
            }
            ExprKind::Tuple(elements) => Ok(ValueType::Tuple(
                elements
                    .iter()
                    .map(|element| self.check_assignment_target(element))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            ExprKind::Index { receiver, indices } => {
                if indices.len() != 1 {
                    return Err(vec![
                        self.error("um acesso aceita exatamente um índice", target.span),
                    ]);
                }
                let receiver_type = self.check_expression(receiver)?;
                for index in indices {
                    let index_type = self.check_expression(index)?;
                    self.unify(&ValueType::Integer, &index_type, index.span)?;
                }
                match self.resolve(receiver_type.clone()) {
                    ValueType::List(element) => {
                        self.check_assignment_target(receiver)?;
                        Ok(*element)
                    }
                    ValueType::MutableList(element) => Ok(*element),
                    ValueType::Variable(_) => {
                        let element = self.fresh();
                        self.unify(
                            &ValueType::List(Box::new(element.clone())),
                            &receiver_type,
                            receiver.span,
                        )?;
                        Ok(element)
                    }
                    other => Err(vec![self.error(
                        format!("{other} não permite atribuição por índice"),
                        target.span,
                    )]),
                }
            }
            ExprKind::Member { receiver, name } => {
                let receiver_type = self.check_expression(receiver)?;
                let ValueType::Named(type_name) = self.resolve(receiver_type) else {
                    return Err(vec![
                        self.error("o valor não possui atributos mutáveis", target.span),
                    ]);
                };
                let Some(class) = self.info.classes.get(&type_name) else {
                    return Err(vec![
                        self.error("o valor não possui atributos mutáveis", target.span),
                    ]);
                };
                let Some(field) = class.fields.iter().find(|field| field.name == *name) else {
                    return Err(vec![self.error(
                        format!("atributo '{name}' não existe em '{type_name}'"),
                        target.span,
                    )]);
                };
                if !field.mutable {
                    return Err(vec![
                        self.error(format!("atributo '{name}' é imutável"), target.span),
                    ]);
                }
                Ok(field.value_type.clone())
            }
            _ => Err(vec![self.error("alvo de atribuição inválido", target.span)]),
        }?;
        self.info.node_types.insert(target.id, value_type.clone());
        Ok(value_type)
    }

    fn bind_pattern(
        &mut self,
        pattern: &Pattern,
        expected: ValueType,
        mutable: bool,
    ) -> Result<(), Vec<Diagnostic>> {
        let value_type = self.resolve(expected);
        match &pattern.kind {
            PatternKind::Wildcard => {}
            PatternKind::Binding(name) => {
                let variables = if mutable {
                    Vec::new()
                } else {
                    self.generalized_variables(&value_type)
                };
                self.define(
                    name.clone(),
                    value_type.clone(),
                    mutable,
                    variables,
                    pattern.span,
                )?;
            }
            PatternKind::Integer(_) => {
                self.unify(&ValueType::Integer, &value_type, pattern.span)?
            }
            PatternKind::Real(_) => self.unify(&ValueType::Real, &value_type, pattern.span)?,
            PatternKind::Character(_) => {
                self.unify(&ValueType::Character, &value_type, pattern.span)?
            }
            PatternKind::Text(_) => self.unify(&ValueType::Text, &value_type, pattern.span)?,
            PatternKind::Logical(_) => {
                self.unify(&ValueType::Logical, &value_type, pattern.span)?
            }
            PatternKind::Tuple(patterns) => match &value_type {
                ValueType::Tuple(elements) if elements.len() == patterns.len() => {
                    for (pattern, element_type) in patterns.iter().zip(elements) {
                        self.bind_pattern(pattern, element_type.clone(), mutable)?;
                    }
                }
                ValueType::Tuple(_) => {
                    return Err(vec![self.error(
                        "quantidades diferentes na atribuição paralela",
                        pattern.span,
                    )]);
                }
                ValueType::Variable(_) => {
                    let elements = patterns.iter().map(|_| self.fresh()).collect::<Vec<_>>();
                    self.unify(
                        &ValueType::Tuple(elements.clone()),
                        &value_type,
                        pattern.span,
                    )?;
                    for (pattern, element_type) in patterns.iter().zip(elements) {
                        self.bind_pattern(pattern, element_type, mutable)?;
                    }
                }
                _ => {
                    for pattern in patterns {
                        self.bind_pattern(pattern, value_type.clone(), mutable)?;
                    }
                }
            },
            PatternKind::List(patterns) => {
                let element_type = match &value_type {
                    ValueType::List(element) | ValueType::MutableList(element) => {
                        (**element).clone()
                    }
                    _ => {
                        let element_type = self.fresh();
                        self.unify(
                            &ValueType::List(Box::new(element_type.clone())),
                            &value_type,
                            pattern.span,
                        )?;
                        element_type
                    }
                };
                for pattern in patterns {
                    self.bind_pattern(pattern, element_type.clone(), mutable)?;
                }
            }
            PatternKind::Cons(head, tail) => {
                let element_type = match &value_type {
                    ValueType::List(element) | ValueType::MutableList(element) => {
                        (**element).clone()
                    }
                    _ => {
                        let element_type = self.fresh();
                        self.unify(
                            &ValueType::List(Box::new(element_type.clone())),
                            &value_type,
                            pattern.span,
                        )?;
                        element_type
                    }
                };
                self.bind_pattern(head, element_type.clone(), mutable)?;
                self.bind_pattern(tail, ValueType::List(Box::new(element_type)), mutable)?;
            }
            PatternKind::Constructor { name, fields } => {
                let Some(class) = self.info.classes.get(name).cloned() else {
                    return Err(vec![self.error(
                        format!("construtor '{name}' não foi declarado"),
                        pattern.span,
                    )]);
                };
                self.unify(&ValueType::Named(name.clone()), &value_type, pattern.span)?;
                let constructor_fields = class
                    .fields
                    .iter()
                    .filter(|field| field.constructor_parameter)
                    .collect::<Vec<_>>();
                if fields.len() != constructor_fields.len() {
                    return Err(vec![self.error(
                        format!(
                            "'{name}' espera {} campos neste padrão",
                            constructor_fields.len()
                        ),
                        pattern.span,
                    )]);
                }
                for (field_pattern, field) in fields.iter().zip(constructor_fields) {
                    self.bind_pattern(field_pattern, field.value_type.clone(), mutable)?;
                }
            }
        }
        self.info.node_types.insert(pattern.id, value_type);
        Ok(())
    }

    fn check_generators(&mut self, generators: &[ForGenerator]) -> Result<(), Vec<Diagnostic>> {
        self.push_scope();
        for generator in generators {
            let element_type = match &generator.source {
                ForSource::Range { start, end, step } => {
                    let start_type = self.check_expression(start)?;
                    let end_type = self.check_expression(end)?;
                    self.unify(&ValueType::Integer, &start_type, start.span)?;
                    self.unify(&ValueType::Integer, &end_type, end.span)?;
                    if let Some(step) = step {
                        let step_type = self.check_expression(step)?;
                        self.unify(&ValueType::Integer, &step_type, step.span)?;
                    }
                    ValueType::Integer
                }
                ForSource::Iterable(iterable) => {
                    let iterable_type = self.check_expression(iterable)?;
                    match self.resolve(iterable_type.clone()) {
                        ValueType::List(element) | ValueType::MutableList(element) => *element,
                        ValueType::Text => ValueType::Character,
                        ValueType::Tuple(elements) => {
                            let element = self.fresh();
                            for item in elements {
                                self.unify(&element, &item, iterable.span)?;
                            }
                            element
                        }
                        ValueType::Variable(_) => {
                            let element = self.fresh();
                            self.unify(
                                &ValueType::List(Box::new(element.clone())),
                                &iterable_type,
                                iterable.span,
                            )?;
                            element
                        }
                        other => {
                            return Err(vec![self.error(
                                format!("{other} não pode ser percorrido"),
                                iterable.span,
                            )]);
                        }
                    }
                }
            };
            self.bind_pattern(&generator.pattern, element_type, false)?;
            if let Some(condition) = &generator.condition {
                let condition_type = self.check_expression(condition)?;
                self.unify(&ValueType::Logical, &condition_type, condition.span)?;
            }
            self.info
                .node_types
                .insert(generator.id, ValueType::Nothing);
        }
        Ok(())
    }

    fn index_type(
        &mut self,
        receiver_type: &ValueType,
        indices: &[Expr],
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if indices.len() != 1 {
            return Err(vec![
                self.error("um acesso aceita exatamente um índice", span),
            ]);
        }
        match self.resolve(receiver_type.clone()) {
            ValueType::List(element) | ValueType::MutableList(element) => Ok(*element),
            ValueType::Text => Ok(ValueType::Character),
            ValueType::Tuple(elements) => {
                if let ExprKind::Integer(index) = indices[0].kind {
                    if index <= 0 || index as usize > elements.len() {
                        return Err(vec![
                            self.error("índice da tupla fora dos limites", indices[0].span),
                        ]);
                    }
                    Ok(elements[index as usize - 1].clone())
                } else {
                    let element = self.fresh();
                    for item in elements {
                        self.unify(&element, &item, span)?;
                    }
                    Ok(element)
                }
            }
            ValueType::Variable(_) => {
                let element = self.fresh();
                self.unify(
                    &ValueType::List(Box::new(element.clone())),
                    receiver_type,
                    span,
                )?;
                Ok(element)
            }
            other => Err(vec![self.error(format!("{other} não aceita índice"), span)]),
        }
    }

    fn member_type(
        &mut self,
        receiver_type: &ValueType,
        name: &str,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let receiver_resolved = self.resolve(receiver_type.clone());
        if matches!(name, "qual_tipo" | "texto") {
            return Ok(ValueType::Text);
        }
        match receiver_resolved {
            ValueType::List(element) => self.list_member_type(*element, name, false, span),
            ValueType::MutableList(element) => self.list_member_type(*element, name, true, span),
            ValueType::Text => self.text_member_type(name, span),
            ValueType::Integer => match name {
                "inteiro" => Ok(ValueType::Integer),
                "real" => Ok(ValueType::Real),
                "caractere" | "caracter" => Ok(ValueType::Character),
                _ => Err(vec![self.error(
                    format!("tipo {receiver_type} não possui membro '{name}'"),
                    span,
                )]),
            },
            ValueType::BigInteger => match name {
                "inteiro" => Ok(ValueType::Integer),
                "real" => Ok(ValueType::Real),
                "caractere" | "caracter" => Ok(ValueType::Character),
                "pow" => Ok(ValueType::Function(
                    vec![ValueType::Integer],
                    Box::new(ValueType::BigInteger),
                )),
                _ => {
                    Err(vec![self.error(
                        format!("tipo BigInt não possui membro '{name}'"),
                        span,
                    )])
                }
            },
            ValueType::Real => match name {
                "inteiro" => Ok(ValueType::Integer),
                "real" => Ok(ValueType::Real),
                "caractere" | "caracter" => Ok(ValueType::Character),
                "piso" | "teto" => Ok(ValueType::Real),
                "arredonde" => Ok(ValueType::Integer),
                _ => {
                    Err(vec![self.error(
                        format!("tipo Real não possui membro '{name}'"),
                        span,
                    )])
                }
            },
            ValueType::Character => match name {
                "inteiro" => Ok(ValueType::Integer),
                "caractere" | "caracter" => Ok(ValueType::Character),
                _ => Err(vec![self.error(
                    format!("tipo Caractere não possui membro '{name}'"),
                    span,
                )]),
            },
            ValueType::Tuple(elements) => {
                let index = match name {
                    "primeiro" => Some(0),
                    "segundo" => Some(1),
                    "terceiro" => Some(2),
                    "quarto" => Some(3),
                    "quinto" => Some(4),
                    "sexto" => Some(5),
                    "sétimo" | "setimo" => Some(6),
                    "oitavo" => Some(7),
                    "nono" => Some(8),
                    "décimo" | "decimo" => Some(9),
                    _ => None,
                };
                if let Some(index) = index {
                    elements.get(index).cloned().ok_or_else(|| {
                        vec![self.error(format!("tupla não possui o elemento '{name}'"), span)]
                    })
                } else {
                    Err(vec![self.error(
                        format!("membro '{name}' não existe em tupla"),
                        span,
                    )])
                }
            }
            ValueType::Named(type_name) if type_name == "Lista" => {
                self.list_factory_member(name, span)
            }
            ValueType::Named(type_name) if type_name == "Matriz" => {
                self.matrix_factory_member(name, span)
            }
            ValueType::Named(type_name) if type_name == "Cubo" => {
                self.cube_factory_member(name, span)
            }
            ValueType::Named(type_name) => {
                let Some(class) = self.info.classes.get(&type_name) else {
                    return Err(vec![self.error(
                        format!("tipo '{type_name}' não possui membro '{name}'"),
                        span,
                    )]);
                };
                if let Some(field) = class.fields.iter().find(|field| field.name == name) {
                    return Ok(field.value_type.clone());
                }
                if let Some(method) = class.methods.get(name) {
                    if method.parameters.is_empty() {
                        return Ok(method.return_type.clone());
                    }
                    return Ok(ValueType::Function(
                        method.parameters.clone(),
                        Box::new(method.return_type.clone()),
                    ));
                }
                Err(vec![self.error(
                    format!("membro '{name}' não existe em '{type_name}'"),
                    span,
                )])
            }
            ValueType::Variable(_) => {
                let element = self.fresh();
                self.unify(
                    &ValueType::List(Box::new(element.clone())),
                    receiver_type,
                    span,
                )?;
                self.list_member_type(element, name, false, span)
            }
            other => {
                Err(vec![self.error(
                    format!("tipo {other} não possui membro '{name}'"),
                    span,
                )])
            }
        }
    }

    fn check_member_call(
        &mut self,
        receiver: &Expr,
        name: &str,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let receiver_type = self.check_expression(receiver)?;
        match self.resolve(receiver_type.clone()) {
            ValueType::List(element) => {
                self.check_list_method(*element, name, arguments, false, span)
            }
            ValueType::MutableList(element) => {
                self.check_list_method(*element, name, arguments, true, span)
            }
            ValueType::Text => self.check_text_method(name, arguments, span),
            ValueType::Integer | ValueType::Real | ValueType::BigInteger | ValueType::Character => {
                self.check_numeric_method(&receiver_type, name, arguments, span)
            }
            ValueType::Named(type_name)
                if matches!(type_name.as_str(), "Lista" | "Matriz" | "Cubo") =>
            {
                let method = self.member_type(&ValueType::Named(type_name), name, span)?;
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_argument(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                self.apply_type(method, &argument_types, span)
            }
            ValueType::Named(type_name) => {
                let Some(class) = self.info.classes.get(&type_name) else {
                    return Err(vec![self.error(
                        format!("tipo '{type_name}' não possui método '{name}'"),
                        span,
                    )]);
                };
                let Some(method) = class.methods.get(name) else {
                    return Err(vec![self.error(
                        format!("método '{name}' não existe em '{type_name}'"),
                        span,
                    )]);
                };
                let function = ValueType::Function(
                    method.parameters.clone(),
                    Box::new(method.return_type.clone()),
                );
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_argument(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                self.apply_type(function, &argument_types, span)
            }
            ValueType::Variable(_) => {
                let element = self.fresh();
                self.unify(
                    &ValueType::List(Box::new(element.clone())),
                    &receiver_type,
                    receiver.span,
                )?;
                self.check_list_method(element, name, arguments, false, span)
            }
            other => {
                Err(vec![self.error(
                    format!("tipo {other} não possui método '{name}'"),
                    span,
                )])
            }
        }
    }

    fn list_member_type(
        &mut self,
        element: ValueType,
        name: &str,
        mutable: bool,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let list_type = if mutable {
            ValueType::MutableList(Box::new(element.clone()))
        } else {
            ValueType::List(Box::new(element.clone()))
        };
        let result = match name {
            "tamanho" | "size" => ValueType::Integer,
            "cabeça" | "cabeca" | "primeiro" | "último" | "ultimo" => element,
            "cauda" => ValueType::List(Box::new(element)),
            "vazia" => ValueType::Logical,
            "inverta" | "ordene" => list_type,
            "mutável" | "mutavel" => ValueType::MutableList(Box::new(element)),
            "imutável" | "imutavel" => ValueType::List(Box::new(element)),
            "mapeie" => {
                let mapped = self.fresh();
                ValueType::Function(
                    vec![ValueType::Function(vec![element], Box::new(mapped.clone()))],
                    Box::new(ValueType::List(Box::new(mapped))),
                )
            }
            "selecione" | "pegue_enquanto" | "descarte_enquanto" => ValueType::Function(
                vec![ValueType::Function(
                    vec![element.clone()],
                    Box::new(ValueType::Logical),
                )],
                Box::new(ValueType::List(Box::new(element))),
            ),
            "pegue" | "descarte" => ValueType::Function(
                vec![ValueType::Integer],
                Box::new(ValueType::List(Box::new(element))),
            ),
            "injete" => {
                let accumulator = self.fresh();
                ValueType::Function(
                    vec![
                        accumulator.clone(),
                        ValueType::Function(
                            vec![accumulator.clone(), element],
                            Box::new(accumulator.clone()),
                        ),
                    ],
                    Box::new(accumulator),
                )
            }
            "posição" | "posicao" => {
                ValueType::Function(vec![element], Box::new(ValueType::Integer))
            }
            "contém" | "contem" => {
                ValueType::Function(vec![element], Box::new(ValueType::Logical))
            }
            "remova" => ValueType::Function(vec![ValueType::Integer], Box::new(list_type.clone())),
            "insira" | "atualize" => ValueType::Function(
                vec![ValueType::Integer, element.clone()],
                Box::new(list_type.clone()),
            ),
            "divida_quando" => ValueType::Function(
                vec![ValueType::Function(
                    vec![element.clone(), element.clone()],
                    Box::new(ValueType::Logical),
                )],
                Box::new(ValueType::List(Box::new(ValueType::List(Box::new(
                    element,
                ))))),
            ),
            "junte" => ValueType::Text,
            _ => {
                return Err(vec![
                    self.error(format!("lista não possui membro '{name}'"), span),
                ]);
            }
        };
        Ok(result)
    }

    fn check_list_method(
        &mut self,
        element: ValueType,
        name: &str,
        arguments: &[Expr],
        mutable: bool,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_argument(argument))
            .collect::<Result<Vec<_>, _>>()?;
        if name == "injete" {
            return self.check_inject_types(element, &argument_types, span);
        }
        if name == "junte" {
            if !matches!(arguments.len(), 0 | 1 | 3) {
                return Err(vec![
                    self.error("junte recebe zero, um ou três argumentos", span),
                ]);
            }
            for argument_type in &argument_types {
                self.unify(&ValueType::Text, argument_type, span)?;
            }
            return Ok(ValueType::Text);
        }
        if name == "ordene" {
            if argument_types.len() > 1 {
                return Err(vec![self.error("ordene aceita no máximo uma função", span)]);
            }
            if let Some(key_function) = argument_types.first() {
                match self.resolve(key_function.clone()) {
                    ValueType::Function(parameters, _) if parameters.len() == 2 => {
                        let result = self.apply_type(
                            key_function.clone(),
                            &[element.clone(), element.clone()],
                            span,
                        )?;
                        self.unify(&ValueType::Logical, &result, span)?;
                    }
                    ValueType::Function(parameters, _) if parameters.len() == 1 => {
                        let key =
                            self.apply_type(key_function.clone(), &[element.clone()], span)?;
                        if !self.is_orderable(&key) {
                            return Err(vec![
                                self.error("a chave de ordenação deve ser ordenável", span),
                            ]);
                        }
                    }
                    ValueType::Variable(_) => {
                        let key =
                            self.apply_type(key_function.clone(), &[element.clone()], span)?;
                        if !self.is_orderable(&key) {
                            return Err(vec![
                                self.error("a chave de ordenação deve ser ordenável", span),
                            ]);
                        }
                    }
                    _ => {
                        return Err(vec![
                            self.error("ordene espera uma função de chave ou comparadora", span),
                        ]);
                    }
                }
            }
            return Ok(if mutable {
                ValueType::MutableList(Box::new(element))
            } else {
                ValueType::List(Box::new(element))
            });
        }
        if arguments.is_empty() {
            return self.list_member_type(element, name, mutable, span);
        }
        let function = self.list_member_type(element, name, mutable, span)?;
        self.apply_type(function, &argument_types, span)
    }

    fn is_orderable(&mut self, value_type: &ValueType) -> bool {
        match self.resolve(value_type.clone()) {
            ValueType::Integer
            | ValueType::Real
            | ValueType::BigInteger
            | ValueType::Text
            | ValueType::Character
            | ValueType::Logical
            | ValueType::Variable(_) => true,
            ValueType::Tuple(elements) => elements.iter().all(|element| self.is_orderable(element)),
            ValueType::List(element) | ValueType::MutableList(element) => {
                self.is_orderable(&element)
            }
            _ => false,
        }
    }

    fn check_inject_types(
        &mut self,
        element: ValueType,
        arguments: &[ValueType],
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if arguments.len() == 1 {
            let first = self.resolve(arguments[0].clone());
            if matches!(first, ValueType::Function(_, _) | ValueType::Variable(_)) {
                let accumulator = self.fresh();
                let result = self.apply_type(first, &[accumulator.clone(), element], span)?;
                self.unify(&accumulator, &result, span)?;
                return Ok(accumulator);
            }
            let reducer =
                ValueType::Function(vec![first.clone(), element], Box::new(first.clone()));
            return Ok(ValueType::Function(vec![reducer], Box::new(first)));
        }
        if arguments.len() == 2 {
            let accumulator = arguments[0].clone();
            let result =
                self.apply_type(arguments[1].clone(), &[accumulator.clone(), element], span)?;
            self.unify(&accumulator, &result, span)?;
            return Ok(accumulator);
        }
        Err(vec![
            self.error("injete recebe um acumulador e uma função", span),
        ])
    }

    fn text_member_type(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let result = match name {
            "tamanho" => ValueType::Integer,
            "maiúsculo" | "maiusculo" | "minúsculo" | "minusculo" | "inverta" => ValueType::Text,
            "inteiro" => ValueType::Integer,
            "real" => ValueType::Real,
            "lista" => ValueType::List(Box::new(ValueType::Character)),
            "cabeça" | "cabeca" | "último" | "ultimo" => ValueType::Character,
            "cauda" | "ordene" | "qual_tipo" => ValueType::Text,
            "divida" => ValueType::List(Box::new(ValueType::Text)),
            _ => {
                return Err(vec![
                    self.error(format!("Texto não possui membro '{name}'"), span),
                ]);
            }
        };
        Ok(result)
    }

    fn check_text_method(
        &mut self,
        name: &str,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_argument(argument))
            .collect::<Result<Vec<_>, _>>()?;
        match name {
            "divida" => {
                if arguments.len() > 1 {
                    return Err(vec![
                        self.error("divida recebe no máximo um separador", span),
                    ]);
                }
                if let Some(argument_type) = argument_types.first() {
                    self.unify(&ValueType::Text, argument_type, span)?;
                }
                Ok(ValueType::List(Box::new(ValueType::Text)))
            }
            "pegue" | "descarte" => {
                if argument_types.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe um número"), span)]);
                }
                self.unify(&ValueType::Integer, &argument_types[0], span)?;
                Ok(ValueType::Text)
            }
            "posição" | "posicao" | "contém" | "contem" => {
                if argument_types.len() != 1
                    || !matches!(
                        self.resolve(argument_types[0].clone()),
                        ValueType::Character | ValueType::Text | ValueType::Variable(_)
                    )
                {
                    return Err(vec![
                        self.error(format!("{name} recebe um caractere ou texto"), span),
                    ]);
                }
                Ok(if matches!(name, "posição" | "posicao") {
                    ValueType::Integer
                } else {
                    ValueType::Logical
                })
            }
            "selecione" | "pegue_enquanto" | "descarte_enquanto" => {
                if argument_types.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe uma função"), span)]);
                }
                let result =
                    self.apply_type(argument_types[0].clone(), &[ValueType::Character], span)?;
                self.unify(&ValueType::Logical, &result, span)?;
                Ok(ValueType::Text)
            }
            "injete" => {
                if argument_types.len() != 1 {
                    return Err(vec![
                        self.error("injete recebe um acumulador antes da função", span),
                    ]);
                }
                let accumulator = argument_types[0].clone();
                Ok(ValueType::Function(
                    vec![ValueType::Function(
                        vec![accumulator.clone(), ValueType::Character],
                        Box::new(accumulator.clone()),
                    )],
                    Box::new(accumulator),
                ))
            }
            "remova" => {
                if argument_types.len() != 1 {
                    return Err(vec![self.error("remova recebe uma posição", span)]);
                }
                self.unify(&ValueType::Integer, &argument_types[0], span)?;
                Ok(ValueType::Text)
            }
            "insira" => {
                if argument_types.len() != 2 {
                    return Err(vec![self.error("insira recebe posição e valor", span)]);
                }
                self.unify(&ValueType::Integer, &argument_types[0], span)?;
                if !matches!(
                    self.resolve(argument_types[1].clone()),
                    ValueType::Character | ValueType::Text | ValueType::Variable(_)
                ) {
                    return Err(vec![self.error("insira espera Caractere ou Texto", span)]);
                }
                Ok(ValueType::Text)
            }
            "junte" => {
                if !matches!(argument_types.len(), 1 | 3) {
                    return Err(vec![self.error(
                        "junte recebe um separador ou prefixo, separador e sufixo",
                        span,
                    )]);
                }
                for argument in &argument_types {
                    self.unify(&ValueType::Text, argument, span)?;
                }
                Ok(ValueType::Text)
            }
            _ => {
                if !argument_types.is_empty() {
                    return Err(vec![
                        self.error(format!("'{name}' não recebe argumentos"), span),
                    ]);
                }
                self.text_member_type(name, span)
            }
        }
    }

    fn check_numeric_method(
        &mut self,
        receiver_type: &ValueType,
        name: &str,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if name == "arredonde" && matches!(self.resolve(receiver_type.clone()), ValueType::Real) {
            return match arguments {
                [] => Ok(ValueType::Integer),
                [digits] => {
                    let digits_type = self.check_expression(digits)?;
                    self.unify(&ValueType::Integer, &digits_type, digits.span)?;
                    Ok(ValueType::Real)
                }
                _ => Err(vec![
                    self.error("arredonde recebe no máximo uma precisão", span),
                ]),
            };
        }
        let member = self.member_type(receiver_type, name, span)?;
        if arguments.is_empty() {
            return Err(vec![
                self.error(format!("'{name}' é uma propriedade"), span),
            ]);
        }
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_argument(argument))
            .collect::<Result<Vec<_>, _>>()?;
        self.apply_type(member, &argument_types, span)
    }

    fn list_factory_member(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let element = self.fresh();
        match name {
            "imutável" | "imutavel" => Ok(ValueType::Function(
                vec![ValueType::Integer, element.clone()],
                Box::new(ValueType::List(Box::new(element))),
            )),
            "mutável" | "mutavel" => Ok(ValueType::Function(
                vec![ValueType::Integer, element.clone()],
                Box::new(ValueType::MutableList(Box::new(element))),
            )),
            "vazia" => Ok(ValueType::List(Box::new(element))),
            _ => Err(vec![
                self.error(format!("Lista não possui membro '{name}'"), span),
            ]),
        }
    }

    fn matrix_factory_member(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if !matches!(name, "imutável" | "imutavel") {
            return Err(vec![
                self.error(format!("Matriz não possui membro '{name}'"), span),
            ]);
        }
        let element = self.fresh();
        Ok(ValueType::Function(
            vec![ValueType::Integer, ValueType::Integer, element.clone()],
            Box::new(ValueType::List(Box::new(ValueType::List(Box::new(
                element,
            ))))),
        ))
    }

    fn cube_factory_member(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        if !matches!(name, "imutável" | "imutavel") {
            return Err(vec![
                self.error(format!("Cubo não possui membro '{name}'"), span),
            ]);
        }
        let element = self.fresh();
        Ok(ValueType::Function(
            vec![
                ValueType::Integer,
                ValueType::Integer,
                ValueType::Integer,
                element.clone(),
            ],
            Box::new(ValueType::List(Box::new(ValueType::List(Box::new(
                ValueType::List(Box::new(element)),
            ))))),
        ))
    }

    fn apply_type(
        &mut self,
        callee: ValueType,
        arguments: &[ValueType],
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        match self.resolve(callee.clone()) {
            ValueType::Function(parameters, result) => {
                let applied = arguments.len().min(parameters.len());
                for (parameter, argument) in parameters.iter().zip(arguments.iter()).take(applied) {
                    self.unify_assignable(parameter, argument, span)?;
                }
                if arguments.len() < parameters.len() {
                    return Ok(ValueType::Function(
                        parameters[arguments.len()..].to_vec(),
                        result,
                    ));
                }
                let value_type = *result;
                if arguments.len() == parameters.len() {
                    Ok(value_type)
                } else {
                    self.apply_type(value_type, &arguments[parameters.len()..], span)
                }
            }
            ValueType::Variable(_) => {
                let result = self.fresh();
                let inferred = ValueType::Function(arguments.to_vec(), Box::new(result.clone()));
                self.unify(&callee, &inferred, span)?;
                Ok(result)
            }
            other => {
                Err(vec![self.error(
                    format!("valor do tipo {other} não pode ser chamado"),
                    span,
                )])
            }
        }
    }

    fn type_from_annotation(
        &mut self,
        annotation: &TypeExpr,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let value_type = match &annotation.kind {
            TypeExprKind::Tuple(elements) => ValueType::Tuple(
                elements
                    .iter()
                    .map(|element| self.type_from_annotation(element))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            TypeExprKind::Function(parameters, result) => ValueType::Function(
                parameters
                    .iter()
                    .map(|parameter| self.type_from_annotation(parameter))
                    .collect::<Result<Vec<_>, _>>()?,
                Box::new(self.type_from_annotation(result)?),
            ),
            TypeExprKind::Named(name, arguments) => {
                let normalized = name.to_lowercase();
                match normalized.as_str() {
                    "inteiro" | "integer" => {
                        self.ensure_no_type_arguments(name, arguments, annotation.span)?;
                        ValueType::Integer
                    }
                    "real" => {
                        self.ensure_no_type_arguments(name, arguments, annotation.span)?;
                        ValueType::Real
                    }
                    "bigint" => {
                        self.ensure_no_type_arguments(name, arguments, annotation.span)?;
                        ValueType::BigInteger
                    }
                    "texto" | "text" => {
                        self.ensure_no_type_arguments(name, arguments, annotation.span)?;
                        ValueType::Text
                    }
                    "caractere" | "caracter" | "character" => {
                        self.ensure_no_type_arguments(name, arguments, annotation.span)?;
                        ValueType::Character
                    }
                    "lógico" | "logico" | "logical" => {
                        self.ensure_no_type_arguments(name, arguments, annotation.span)?;
                        ValueType::Logical
                    }
                    "nada" | "nothing" => {
                        self.ensure_no_type_arguments(name, arguments, annotation.span)?;
                        ValueType::Nothing
                    }
                    "lista" | "list" => {
                        if arguments.len() != 1 {
                            return Err(vec![
                                self.error("Lista precisa de um tipo de elemento", annotation.span),
                            ]);
                        }
                        ValueType::List(Box::new(self.type_from_annotation(&arguments[0])?))
                    }
                    _ if self.known_types.contains(name) => {
                        self.ensure_no_type_arguments(name, arguments, annotation.span)?;
                        ValueType::Named(name.clone())
                    }
                    _ => {
                        return Err(vec![self.error(
                            format!("tipo '{name}' não foi declarado"),
                            annotation.span,
                        )]);
                    }
                }
            }
        };
        self.info
            .node_types
            .insert(annotation.id, value_type.clone());
        Ok(value_type)
    }

    fn ensure_no_type_arguments(
        &self,
        name: &str,
        arguments: &[TypeExpr],
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        if arguments.is_empty() {
            Ok(())
        } else {
            Err(vec![self.error(
                format!("tipo '{name}' não recebe parâmetros"),
                span,
            )])
        }
    }

    fn numeric_result(
        &mut self,
        left: &ValueType,
        right: &ValueType,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let left = match self.resolve(left.clone()) {
            ValueType::Character => ValueType::Integer,
            value_type => value_type,
        };
        let right = match self.resolve(right.clone()) {
            ValueType::Character => ValueType::Integer,
            value_type => value_type,
        };
        if let ValueType::Variable(_) = left {
            if right.is_numeric() {
                self.unify(&left, &right, span)?;
                return Ok(right);
            }
        }
        if let ValueType::Variable(_) = right {
            if left.is_numeric() {
                self.unify(&right, &left, span)?;
                return Ok(left);
            }
        }
        if !left.is_numeric() || !right.is_numeric() {
            return Err(vec![self.error(
                format!("operação numérica exige números, recebeu {left} e {right}"),
                span,
            )]);
        }
        if matches!(left, ValueType::Real) || matches!(right, ValueType::Real) {
            Ok(ValueType::Real)
        } else if matches!(left, ValueType::BigInteger) || matches!(right, ValueType::BigInteger) {
            Ok(ValueType::BigInteger)
        } else {
            Ok(ValueType::Integer)
        }
    }

    fn require_numeric(
        &mut self,
        value_type: &ValueType,
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        let value_type = self.resolve(value_type.clone());
        if value_type.is_numeric() || matches!(value_type, ValueType::Character) {
            Ok(())
        } else if matches!(value_type, ValueType::Variable(_)) {
            self.unify(&value_type, &ValueType::Integer, span)
        } else {
            Err(vec![self.error(
                format!("esperava um número, recebeu {value_type}"),
                span,
            )])
        }
    }

    fn require_integer_like(
        &mut self,
        value_type: &ValueType,
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        let value_type = self.resolve(value_type.clone());
        if matches!(
            value_type,
            ValueType::Integer | ValueType::Character | ValueType::BigInteger
        ) {
            Ok(())
        } else if matches!(value_type, ValueType::Variable(_)) {
            self.unify(&value_type, &ValueType::Integer, span)
        } else {
            Err(vec![self.error(
                format!("esperava um inteiro, recebeu {value_type}"),
                span,
            )])
        }
    }

    fn ensure_displayable(
        &self,
        value_type: &ValueType,
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        if matches!(value_type, ValueType::Function(_, _)) {
            Err(vec![
                self.error("uma função não pode ser exibida diretamente", span),
            ])
        } else {
            Ok(())
        }
    }

    fn fresh(&mut self) -> ValueType {
        let variable = self.next_variable;
        self.next_variable = self.next_variable.wrapping_add(1);
        ValueType::Variable(variable)
    }

    fn resolve(&mut self, value_type: ValueType) -> ValueType {
        match value_type {
            ValueType::Variable(id) => {
                let Some(replacement) = self.substitutions.get(&id).cloned() else {
                    return ValueType::Variable(id);
                };
                let resolved = self.resolve(replacement);
                self.substitutions.insert(id, resolved.clone());
                resolved
            }
            ValueType::List(element) => ValueType::List(Box::new(self.resolve(*element))),
            ValueType::MutableList(element) => {
                ValueType::MutableList(Box::new(self.resolve(*element)))
            }
            ValueType::Tuple(elements) => ValueType::Tuple(
                elements
                    .into_iter()
                    .map(|element| self.resolve(element))
                    .collect(),
            ),
            ValueType::Function(parameters, result) => ValueType::Function(
                parameters
                    .into_iter()
                    .map(|parameter| self.resolve(parameter))
                    .collect(),
                Box::new(self.resolve(*result)),
            ),
            other => other,
        }
    }

    fn unify(
        &mut self,
        left: &ValueType,
        right: &ValueType,
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        let left = self.resolve(left.clone());
        let right = self.resolve(right.clone());
        if left == right {
            return Ok(());
        }
        match (left.clone(), right.clone()) {
            (ValueType::Variable(id), value_type) | (value_type, ValueType::Variable(id)) => {
                if self.occurs(id, &value_type) {
                    return Err(vec![self.error("tipo recursivo não é permitido", span)]);
                }
                self.substitutions.insert(id, value_type);
                Ok(())
            }
            (ValueType::List(left), ValueType::List(right))
            | (ValueType::MutableList(left), ValueType::MutableList(right)) => {
                self.unify(&left, &right, span)
            }
            (ValueType::Tuple(left), ValueType::Tuple(right)) if left.len() == right.len() => {
                for (left, right) in left.iter().zip(right.iter()) {
                    self.unify(left, right, span)?;
                }
                Ok(())
            }
            (
                ValueType::Function(left_parameters, left_result),
                ValueType::Function(right_parameters, right_result),
            ) if left_parameters.len() == right_parameters.len() => {
                for (left, right) in left_parameters.iter().zip(right_parameters.iter()) {
                    self.unify(left, right, span)?;
                }
                self.unify(&left_result, &right_result, span)
            }
            _ => Err(vec![self.error(
                format!("tipos incompatíveis: esperava {left}, recebeu {right}"),
                span,
            )]),
        }
    }

    fn unify_assignable(
        &mut self,
        expected: &ValueType,
        actual: &ValueType,
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        let expected = self.resolve(expected.clone());
        let actual = self.resolve(actual.clone());
        if matches!(expected, ValueType::Variable(_)) || matches!(actual, ValueType::Variable(_)) {
            self.unify(&expected, &actual, span)
        } else if expected == actual || can_implicitly_convert(&actual, &expected) {
            Ok(())
        } else if let (
            ValueType::Function(expected_parameters, expected_result),
            ValueType::Function(actual_parameters, actual_result),
        ) = (&expected, &actual)
            && expected_parameters.len() == actual_parameters.len()
        {
            for (expected, actual) in expected_parameters.iter().zip(actual_parameters) {
                self.unify_assignable(expected, actual, span)?;
            }
            self.unify_assignable(expected_result, actual_result, span)
        } else if let (ValueType::Named(expected), ValueType::Named(actual)) = (&expected, &actual)
            && self.is_subtype(actual, expected)
        {
            Ok(())
        } else {
            Err(vec![self.error(
                format!("tipos incompatíveis: esperava {expected}, recebeu {actual}"),
                span,
            )])
        }
    }

    fn is_subtype(&self, actual: &str, expected: &str) -> bool {
        let mut current = Some(actual);
        while let Some(name) = current {
            if name == expected {
                return true;
            }
            current = self
                .info
                .classes
                .get(name)
                .and_then(|class| class.parent.as_deref());
        }
        false
    }

    fn merge_types(
        &mut self,
        left: &ValueType,
        right: &ValueType,
        span: SourceSpan,
    ) -> Result<ValueType, Vec<Diagnostic>> {
        let left = self.resolve(left.clone());
        let right = self.resolve(right.clone());
        if let Some(common) = common_value_type(&left, &right) {
            Ok(common)
        } else {
            self.unify(&left, &right, span)?;
            Ok(self.resolve(left))
        }
    }

    fn occurs(&mut self, variable: u32, value_type: &ValueType) -> bool {
        match self.resolve(value_type.clone()) {
            ValueType::Variable(id) => id == variable,
            ValueType::List(element) | ValueType::MutableList(element) => {
                self.occurs(variable, &element)
            }
            ValueType::Tuple(elements) => elements
                .iter()
                .any(|element| self.occurs(variable, element)),
            ValueType::Function(parameters, result) => {
                parameters
                    .iter()
                    .any(|parameter| self.occurs(variable, parameter))
                    || self.occurs(variable, &result)
            }
            _ => false,
        }
    }

    fn generalize(&mut self, value_type: &ValueType) -> Scheme {
        let mut variables = self.free_type_variables(value_type);
        let environment = self.environment_free_variables();
        variables.retain(|variable| !environment.contains(variable));
        Scheme {
            variables: variables.into_iter().collect(),
            value_type: self.resolve(value_type.clone()),
        }
    }

    fn generalized_variables(&mut self, value_type: &ValueType) -> Vec<u32> {
        self.generalize(value_type).variables
    }

    fn free_type_variables(&mut self, value_type: &ValueType) -> HashSet<u32> {
        match self.resolve(value_type.clone()) {
            ValueType::Variable(id) => HashSet::from([id]),
            ValueType::List(element) | ValueType::MutableList(element) => {
                self.free_type_variables(&element)
            }
            ValueType::Tuple(elements) => {
                let mut variables = HashSet::new();
                for element in &elements {
                    variables.extend(self.free_type_variables(element));
                }
                variables
            }
            ValueType::Function(parameters, result) => {
                let mut variables = HashSet::new();
                for parameter in &parameters {
                    variables.extend(self.free_type_variables(parameter));
                }
                variables.extend(self.free_type_variables(&result));
                variables
            }
            _ => HashSet::new(),
        }
    }

    fn environment_free_variables(&mut self) -> HashSet<u32> {
        let schemes = self
            .scopes
            .iter()
            .flat_map(|scope| scope.values().map(|binding| binding.scheme.clone()))
            .collect::<Vec<_>>();
        let mut variables = HashSet::new();
        for scheme in schemes {
            let mut free = self.free_type_variables(&scheme.value_type);
            for quantified in scheme.variables {
                free.remove(&quantified);
            }
            variables.extend(free);
        }
        variables
    }

    fn instantiate(&mut self, scheme: &Scheme) -> ValueType {
        let replacements = scheme
            .variables
            .iter()
            .map(|variable| (*variable, self.fresh()))
            .collect::<HashMap<_, _>>();
        self.replace_variables(&scheme.value_type, &replacements)
    }

    fn replace_variables(
        &mut self,
        value_type: &ValueType,
        replacements: &HashMap<u32, ValueType>,
    ) -> ValueType {
        match value_type {
            ValueType::Variable(id) => replacements
                .get(id)
                .cloned()
                .unwrap_or_else(|| value_type.clone()),
            ValueType::List(element) => {
                ValueType::List(Box::new(self.replace_variables(element, replacements)))
            }
            ValueType::MutableList(element) => {
                ValueType::MutableList(Box::new(self.replace_variables(element, replacements)))
            }
            ValueType::Tuple(elements) => ValueType::Tuple(
                elements
                    .iter()
                    .map(|element| self.replace_variables(element, replacements))
                    .collect(),
            ),
            ValueType::Function(parameters, result) => ValueType::Function(
                parameters
                    .iter()
                    .map(|parameter| self.replace_variables(parameter, replacements))
                    .collect(),
                Box::new(self.replace_variables(result, replacements)),
            ),
            other => other.clone(),
        }
    }

    fn define(
        &mut self,
        name: String,
        value_type: ValueType,
        mutable: bool,
        variables: Vec<u32>,
        span: SourceSpan,
    ) -> Result<(), Vec<Diagnostic>> {
        if self.is_defined_here(&name) {
            return Err(vec![
                self.error(format!("'{name}' já foi declarado neste escopo"), span),
            ]);
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name,
                Binding {
                    scheme: Scheme {
                        variables,
                        value_type,
                    },
                    mutable,
                },
            );
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn is_defined_here(&self, name: &str) -> bool {
        self.scopes
            .last()
            .is_some_and(|scope| scope.contains_key(name))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn resolve_type_info(&mut self) {
        self.info.node_types = self
            .info
            .node_types
            .clone()
            .into_iter()
            .map(|(node, value_type)| (node, self.resolve(value_type)))
            .collect();
        self.info.functions = self
            .info
            .functions
            .clone()
            .into_iter()
            .map(|(name, signature)| {
                (
                    name,
                    FunctionSignature {
                        parameters: signature
                            .parameters
                            .into_iter()
                            .map(|ty| self.resolve(ty))
                            .collect(),
                        return_type: self.resolve(signature.return_type),
                    },
                )
            })
            .collect();
        let mut classes = std::mem::take(&mut self.info.classes);
        for class in classes.values_mut() {
            for field in &mut class.fields {
                field.value_type = self.resolve(field.value_type.clone());
            }
            for method in class.methods.values_mut() {
                method.parameters = method
                    .parameters
                    .iter()
                    .cloned()
                    .map(|ty| self.resolve(ty))
                    .collect();
                method.return_type = self.resolve(method.return_type.clone());
            }
        }
        self.info.classes = classes;
    }

    fn error(&self, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
        Diagnostic::new(DiagnosticKind::Type, message, span)
    }
}

fn input_type(name: &str) -> Option<ValueType> {
    match name {
        "leia_inteiro" => Some(ValueType::Integer),
        "leia_real" | "leia_numero" | "leia_número" => Some(ValueType::Real),
        "leia_texto" => Some(ValueType::Text),
        "leia_caractere" => Some(ValueType::Character),
        "leia_logico" => Some(ValueType::Logical),
        _ => None,
    }
}

fn compatible_comparison_types(left: &ValueType, right: &ValueType) -> bool {
    let numeric = |value_type: &ValueType| {
        matches!(
            value_type,
            ValueType::Integer | ValueType::Real | ValueType::BigInteger | ValueType::Character
        )
    };
    left == right || (numeric(left) && numeric(right))
}

fn can_implicitly_convert(actual: &ValueType, expected: &ValueType) -> bool {
    matches!(
        (actual, expected),
        (
            ValueType::Character,
            ValueType::Integer | ValueType::Real | ValueType::Text
        ) | (ValueType::Integer, ValueType::Real | ValueType::Text)
            | (
                ValueType::Real | ValueType::Logical | ValueType::BigInteger,
                ValueType::Text
            )
    )
}

fn common_value_type(left: &ValueType, right: &ValueType) -> Option<ValueType> {
    if left == right {
        return Some(left.clone());
    }
    let numeric = |value_type: &ValueType| {
        matches!(
            value_type,
            ValueType::Integer | ValueType::Real | ValueType::BigInteger | ValueType::Character
        )
    };
    if numeric(left) && numeric(right) {
        if matches!(left, ValueType::Real) || matches!(right, ValueType::Real) {
            return Some(ValueType::Real);
        }
        if matches!(left, ValueType::BigInteger) || matches!(right, ValueType::BigInteger) {
            return Some(ValueType::BigInteger);
        }
        return Some(ValueType::Integer);
    }
    if (matches!(left, ValueType::Text) && can_implicitly_convert(right, left))
        || (matches!(right, ValueType::Text) && can_implicitly_convert(left, right))
    {
        return Some(ValueType::Text);
    }
    match (left, right) {
        (ValueType::List(left), ValueType::List(right)) => {
            common_value_type(left, right).map(|element| ValueType::List(Box::new(element)))
        }
        (ValueType::MutableList(left), ValueType::MutableList(right)) => {
            common_value_type(left, right).map(|element| ValueType::MutableList(Box::new(element)))
        }
        (ValueType::Tuple(left), ValueType::Tuple(right)) if left.len() == right.len() => {
            Some(ValueType::Tuple(
                left.iter()
                    .zip(right)
                    .map(|(left, right)| common_value_type(left, right))
                    .collect::<Option<Vec<_>>>()?,
            ))
        }
        _ => None,
    }
}
