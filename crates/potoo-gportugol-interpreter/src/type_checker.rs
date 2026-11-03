use std::collections::HashMap;

use crate::{
    DataType, Diagnostic, DiagnosticKind, SourceSpan,
    ast::{
        BinaryOp, Block, Expr, ExprKind, LValue, Literal, Program, Statement, UnaryOp, VariableDecl,
    },
    types::{Type, is_numeric, promoted_semantic_numeric_type},
};

type Symbols = HashMap<String, Type>;

#[derive(Clone)]
struct Signature {
    parameters: Vec<Type>,
    return_type: Option<DataType>,
    span: SourceSpan,
}

pub fn check(program: &Program) -> Result<(), Vec<Diagnostic>> {
    Checker::new(program).check()
}

struct Checker<'a> {
    program: &'a Program,
    globals: Symbols,
    functions: HashMap<String, Signature>,
}

impl<'a> Checker<'a> {
    fn new(program: &'a Program) -> Self {
        Self {
            program,
            globals: Symbols::new(),
            functions: HashMap::new(),
        }
    }

    fn check(mut self) -> Result<(), Vec<Diagnostic>> {
        for declaration in &self.program.globals {
            for name in &declaration.names {
                if is_builtin_function(&name.name) {
                    return Err(vec![self.error(
                        DiagnosticKind::Type,
                        format!("'{}' is already declared as a function", name.name),
                        name.span,
                    )]);
                }
            }
            Self::declare(declaration, &mut self.globals).map_err(|error| vec![error])?;
        }
        for function in &self.program.functions {
            let signature = Signature {
                parameters: function
                    .parameters
                    .iter()
                    .flat_map(|declaration| {
                        declaration.names.iter().map(|_| {
                            Type::from_declaration(declaration.data_type, &declaration.dimensions)
                        })
                    })
                    .collect(),
                return_type: function.return_type,
                span: function.name_span,
            };
            if self.globals.contains_key(&function.name)
                || is_builtin_function(&function.name)
                || self
                    .functions
                    .insert(function.name.clone(), signature)
                    .is_some()
            {
                return Err(vec![self.error(
                    DiagnosticKind::Type,
                    format!("function '{}' is already declared", function.name),
                    function.name_span,
                )]);
            }
        }

        let mut diagnostics = self.block(&self.program.main, &Symbols::new(), None, true);
        for function in &self.program.functions {
            let mut locals = Symbols::new();
            let mut declarations_valid = true;
            for declaration in function.parameters.iter().chain(&function.locals) {
                if let Err(error) = Self::declare(declaration, &mut locals) {
                    diagnostics.push(error);
                    declarations_valid = false;
                    break;
                }
            }
            if declarations_valid {
                diagnostics.extend(self.block(
                    &function.body,
                    &locals,
                    function.return_type,
                    false,
                ));
            }
        }
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    fn declare(declaration: &VariableDecl, scope: &mut Symbols) -> Result<(), Diagnostic> {
        let ty = Type::from_declaration(declaration.data_type, &declaration.dimensions);
        if declaration.dimensions.contains(&0) {
            return Err(Diagnostic::new(
                DiagnosticKind::Type,
                "matrix dimensions must be greater than zero",
                declaration.span,
            ));
        }
        for name in &declaration.names {
            if scope.insert(name.name.clone(), ty.clone()).is_some() {
                return Err(Diagnostic::new(
                    DiagnosticKind::Type,
                    format!("'{}' is declared more than once in this scope", name.name),
                    name.span,
                ));
            }
        }
        Ok(())
    }

    fn block(
        &self,
        block: &Block,
        locals: &Symbols,
        return_type: Option<DataType>,
        main: bool,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for statement in &block.statements {
            if let Err(error) = self.statement(statement, locals, return_type, main) {
                diagnostics.push(error);
            }
        }
        diagnostics
    }

    fn statement(
        &self,
        statement: &Statement,
        locals: &Symbols,
        return_type: Option<DataType>,
        main: bool,
    ) -> Result<(), Diagnostic> {
        match statement {
            Statement::Sequence { statements, .. } => {
                for statement in statements {
                    self.statement(statement, locals, return_type, main)?;
                }
            }
            Statement::Assign { target, value, .. } => {
                let target_type = self.lvalue(target, locals)?;
                if !target_type.is_primitive() {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "a whole matrix cannot be assigned",
                        target.span,
                    ));
                }
                let value_type = self.expression(value, locals, Some(&target_type))?;
                if !target_type.compatible_with(&value_type) {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        format!("cannot assign {value_type:?} to {target_type:?}"),
                        value.span,
                    ));
                }
            }
            Statement::Call { call, .. } => {
                let ExprKind::Call {
                    name,
                    name_span,
                    args,
                } = &call.kind
                else {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "expected a function call",
                        call.span,
                    ));
                };
                self.call(name, *name_span, args, locals, None, call.span, false)?;
            }
            Statement::Return { value, span } => match (return_type, value) {
                (Some(expected), Some(value)) => {
                    if expected == DataType::Literal && is_untyped_leia(value) {
                        return Err(self.error(
                            DiagnosticKind::Type,
                            "leia is incompatible with a literal return type",
                            value.span,
                        ));
                    }
                    let expected_type = Type::Primitive(expected);
                    let actual = self.expression(value, locals, Some(&expected_type))?;
                    if !expected_type.compatible_with(&actual) {
                        return Err(self.error(
                            DiagnosticKind::Type,
                            format!("return value must be compatible with {expected:?}"),
                            value.span,
                        ));
                    }
                }
                (Some(_), None) => {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "a value-returning function must return a value",
                        *span,
                    ));
                }
                (None, Some(value)) => {
                    if main {
                        let expected = Type::Primitive(DataType::Integer);
                        let actual = self.expression(value, locals, None)?;
                        if actual != expected {
                            return Err(self.error(
                                DiagnosticKind::Type,
                                "the main block may only return an integer value",
                                value.span,
                            ));
                        }
                    } else {
                        return Err(self.error(
                            DiagnosticKind::Type,
                            "a function without a return type cannot return a value",
                            *span,
                        ));
                    }
                }
                (None, None) if main => {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "the main block may only return an integer value",
                        *span,
                    ));
                }
                (None, None) => {}
            },
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.condition(condition, locals)?;
                self.statement(then_branch, locals, return_type, main)?;
                if let Some(else_branch) = else_branch {
                    self.statement(else_branch, locals, return_type, main)?;
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.condition(condition, locals)?;
                self.statement(body, locals, return_type, main)?;
            }
            Statement::Repeat {
                body, condition, ..
            } => {
                self.statement(body, locals, return_type, main)?;
                self.condition(condition, locals)?;
            }
            Statement::For {
                target,
                start,
                end,
                step,
                body,
                ..
            } => {
                let target_type = self.lvalue(target, locals)?;
                if target_type != Type::Primitive(DataType::Integer) {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "the for-loop variable must be an integer",
                        target.span,
                    ));
                }
                self.integer_expression(start, locals)?;
                self.integer_expression(end, locals)?;
                if *step == 0 {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "the for-loop step cannot be zero",
                        statement.span(),
                    ));
                }
                self.statement(body, locals, return_type, main)?;
            }
        }
        Ok(())
    }

    fn condition(&self, expression: &Expr, locals: &Symbols) -> Result<(), Diagnostic> {
        let expected = Type::Primitive(DataType::Logical);
        let actual = self.expression(expression, locals, Some(&expected))?;
        if !actual.is_primitive() {
            return Err(self.error(
                DiagnosticKind::Type,
                "a condition must evaluate to a primitive value",
                expression.span,
            ));
        }
        Ok(())
    }

    fn integer_expression(&self, expression: &Expr, locals: &Symbols) -> Result<(), Diagnostic> {
        let actual = self.expression(expression, locals, None)?;
        if actual != Type::Primitive(DataType::Integer) {
            return Err(self.error(
                DiagnosticKind::Type,
                "expected an integer expression",
                expression.span,
            ));
        }
        Ok(())
    }

    fn expression(
        &self,
        expression: &Expr,
        locals: &Symbols,
        expected: Option<&Type>,
    ) -> Result<Type, Diagnostic> {
        match &expression.kind {
            ExprKind::Literal(literal) => Ok(Type::Primitive(match literal {
                Literal::Integer(_) => DataType::Integer,
                Literal::Real(_) => DataType::Real,
                Literal::Character(_) => DataType::Character,
                Literal::String(_) => DataType::Literal,
                Literal::Logical(_) => DataType::Logical,
            })),
            ExprKind::Variable(value) => self.lvalue(value, locals),
            ExprKind::Call {
                name,
                name_span,
                args,
            } => self
                .call(
                    name,
                    *name_span,
                    args,
                    locals,
                    expected,
                    expression.span,
                    true,
                )?
                .ok_or_else(|| {
                    self.error(
                        DiagnosticKind::Type,
                        format!("function '{name}' does not return a value"),
                        expression.span,
                    )
                }),
            ExprKind::Unary { op, value } => {
                if is_untyped_leia(value) {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "leia cannot be used as a unary operand",
                        expression.span,
                    ));
                }
                let value_type = self.expression(value, locals, None)?;
                let data_type = self.scalar_type(&value_type, value.span)?;
                match op {
                    UnaryOp::Positive | UnaryOp::Negative if is_numeric(data_type) => {
                        Ok(Type::Primitive(data_type))
                    }
                    UnaryOp::Not if is_numeric(data_type) || data_type == DataType::Literal => {
                        Ok(Type::Primitive(DataType::Logical))
                    }
                    UnaryOp::BitwiseNot if is_numeric(data_type) && data_type != DataType::Real => {
                        Ok(Type::Primitive(data_type))
                    }
                    _ => Err(self.error(
                        DiagnosticKind::Type,
                        "invalid operand type for unary operator",
                        expression.span,
                    )),
                }
            }
            ExprKind::Binary { left, op, right } => {
                if is_untyped_leia(left) || is_untyped_leia(right) {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "leia cannot be used as a binary operand",
                        expression.span,
                    ));
                }
                let left_type = self.expression(left, locals, None)?;
                let right_type = self.expression(right, locals, None)?;
                self.binary_type(*op, left_type, right_type, expression.span)
            }
        }
    }

    fn binary_type(
        &self,
        operator: BinaryOp,
        left: Type,
        right: Type,
        span: SourceSpan,
    ) -> Result<Type, Diagnostic> {
        let left_data = self.scalar_type(&left, span)?;
        let right_data = self.scalar_type(&right, span)?;
        let numeric = is_numeric(left_data) && is_numeric(right_data);
        let literals = left_data == DataType::Literal && right_data == DataType::Literal;
        let data_type = match operator {
            BinaryOp::Or | BinaryOp::And if numeric || literals => DataType::Logical,
            BinaryOp::BitwiseOr | BinaryOp::BitwiseXor | BinaryOp::BitwiseAnd
                if numeric && left_data != DataType::Real && right_data != DataType::Real =>
            {
                promoted_semantic_numeric_type(left_data, right_data)
            }
            BinaryOp::Equal | BinaryOp::NotEqual if numeric || literals => DataType::Logical,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
                if numeric || literals =>
            {
                DataType::Logical
            }
            BinaryOp::Remainder
                if numeric && left_data != DataType::Real && right_data != DataType::Real =>
            {
                promoted_semantic_numeric_type(left_data, right_data)
            }
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide
                if numeric =>
            {
                promoted_semantic_numeric_type(left_data, right_data)
            }
            _ => {
                return Err(self.error(
                    DiagnosticKind::Type,
                    "incompatible operand types for operator",
                    span,
                ));
            }
        };
        Ok(Type::Primitive(data_type))
    }

    fn call(
        &self,
        name: &str,
        name_span: SourceSpan,
        args: &[Expr],
        locals: &Symbols,
        expected: Option<&Type>,
        call_span: SourceSpan,
        value_required: bool,
    ) -> Result<Option<Type>, Diagnostic> {
        match name {
            "imprima" => {
                if args.is_empty() {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "imprima requires at least one argument",
                        call_span,
                    ));
                }
                for argument in args {
                    let argument_type = self.expression(argument, locals, None)?;
                    if !argument_type.is_primitive() {
                        return Err(self.error(
                            DiagnosticKind::Type,
                            "imprima only accepts primitive values",
                            argument.span,
                        ));
                    }
                }
                return if value_required {
                    Err(self.error(
                        DiagnosticKind::Type,
                        "imprima does not return a value",
                        call_span,
                    ))
                } else {
                    Ok(None)
                };
            }
            "leia" => {
                if !args.is_empty() {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        "leia does not accept arguments",
                        call_span,
                    ));
                }
                return Ok(Some(Type::Primitive(match expected {
                    Some(Type::Primitive(data_type)) => *data_type,
                    None => DataType::Literal,
                    _ => {
                        return Err(self.error(
                            DiagnosticKind::Type,
                            "leia needs a primitive target type",
                            call_span,
                        ));
                    }
                })));
            }
            _ => {}
        }

        let parameter_count = match name {
            "absoluto" | "arredonda" | "raiz_quadrada" => Some(1),
            "potencia" => Some(2),
            _ => None,
        };
        if let Some(parameter_count) = parameter_count {
            if args.len() != parameter_count {
                return Err(self.error(
                    DiagnosticKind::Type,
                    format!(
                        "function '{name}' expects {parameter_count} arguments, got {}",
                        args.len()
                    ),
                    call_span,
                ));
            }
            let expected = Type::Primitive(DataType::Real);
            for argument in args {
                let actual = self.expression(argument, locals, Some(&expected))?;
                if !expected.compatible_with(&actual) {
                    return Err(self.error(
                        DiagnosticKind::Type,
                        format!("argument is incompatible with function '{name}'"),
                        argument.span,
                    ));
                }
            }
            return Ok(Some(expected));
        }

        let Some(signature) = self.functions.get(name) else {
            return Err(self.error(
                DiagnosticKind::Type,
                format!("function '{name}' is not declared"),
                name_span,
            ));
        };
        if args.len() != signature.parameters.len() {
            return Err(self.error(
                DiagnosticKind::Type,
                format!(
                    "function '{name}' expects {} arguments, got {}",
                    signature.parameters.len(),
                    args.len()
                ),
                call_span,
            ));
        }
        for (argument, parameter_type) in args.iter().zip(&signature.parameters) {
            if parameter_type == &Type::Primitive(DataType::Literal) && is_untyped_leia(argument) {
                return Err(self.error(
                    DiagnosticKind::Type,
                    format!("argument is incompatible with parameter of '{name}'"),
                    argument.span,
                ));
            }
            let actual = self.expression(argument, locals, Some(parameter_type))?;
            if !parameter_type.compatible_with(&actual) {
                return Err(self.error(
                    DiagnosticKind::Type,
                    format!("argument is incompatible with parameter of '{name}'"),
                    argument.span,
                ));
            }
        }
        let Some(return_type) = signature.return_type else {
            return if value_required {
                Err(self.error(
                    DiagnosticKind::Type,
                    format!("function '{name}' does not return a value"),
                    signature.span,
                ))
            } else {
                Ok(None)
            };
        };
        Ok(Some(Type::Primitive(return_type)))
    }

    fn lvalue(&self, lvalue: &LValue, locals: &Symbols) -> Result<Type, Diagnostic> {
        let Some(declared) = locals
            .get(&lvalue.name)
            .or_else(|| self.globals.get(&lvalue.name))
        else {
            return Err(self.error(
                DiagnosticKind::Type,
                format!("variable '{}' is not declared", lvalue.name),
                lvalue.name_span,
            ));
        };
        let (element, dimensions) = match declared {
            Type::Array {
                element,
                dimensions,
            } => (element, dimensions),
            _ if lvalue.indexes.is_empty() => return Ok(declared.clone()),
            _ => {
                return Err(self.error(
                    DiagnosticKind::Type,
                    format!("scalar variable '{}' cannot be indexed", lvalue.name),
                    lvalue.span,
                ));
            }
        };
        if lvalue.indexes.len() > dimensions.len() {
            return Err(self.error(
                DiagnosticKind::Type,
                format!("too many indexes for '{}'", lvalue.name),
                lvalue.span,
            ));
        }
        for index in &lvalue.indexes {
            let index_type =
                self.expression(index, locals, Some(&Type::Primitive(DataType::Integer)))?;
            if !matches!(index_type, Type::Primitive(data_type) if is_numeric(data_type) && data_type != DataType::Real)
            {
                return Err(self.error(
                    DiagnosticKind::Type,
                    "matrix indexes must be integers",
                    index.span,
                ));
            }
        }
        if !lvalue.indexes.is_empty() && lvalue.indexes.len() < dimensions.len() {
            return Err(self.error(
                DiagnosticKind::Type,
                "matrix access must include every dimension",
                lvalue.span,
            ));
        }
        if lvalue.indexes.len() == dimensions.len() {
            Ok(Type::Primitive(*element))
        } else {
            Ok(Type::Array {
                element: *element,
                dimensions: dimensions[lvalue.indexes.len()..].to_vec(),
            })
        }
    }

    fn scalar_type(&self, ty: &Type, span: SourceSpan) -> Result<DataType, Diagnostic> {
        match ty {
            Type::Primitive(data_type) => Ok(*data_type),
            Type::Array { .. } => Err(self.error(
                DiagnosticKind::Type,
                "an entire matrix cannot be used as a value",
                span,
            )),
        }
    }

    fn error(
        &self,
        kind: DiagnosticKind,
        message: impl Into<String>,
        span: SourceSpan,
    ) -> Diagnostic {
        Diagnostic::new(kind, message, span)
    }
}

fn is_builtin_function(name: &str) -> bool {
    matches!(
        name,
        "leia" | "imprima" | "absoluto" | "arredonda" | "potencia" | "raiz_quadrada"
    )
}

fn is_untyped_leia(expression: &Expr) -> bool {
    matches!(
        &expression.kind,
        ExprKind::Call { name, args, .. } if name == "leia" && args.is_empty()
    )
}
