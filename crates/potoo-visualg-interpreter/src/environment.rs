use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use crate::ast::{ArrayDimension, Expression, Type, Variable};
use crate::value::{ArrayValue, Value};
use crate::{Diagnostic, DiagnosticKind};

#[derive(Clone, Debug)]
pub(crate) struct Binding {
    pub(crate) data_type: Type,
    pub(crate) dimensions: Vec<ArrayDimension>,
    pub(crate) value: Rc<RefCell<Value>>,
}

#[derive(Default)]
struct Scope {
    bindings: HashMap<String, Binding>,
    slots: usize,
}

#[derive(Default)]
pub(crate) struct Environment {
    scopes: Vec<Scope>,
    allocated_slots: usize,
}

impl Environment {
    pub(crate) fn new() -> Self {
        Self {
            scopes: vec![Scope::default()],
            allocated_slots: 0,
        }
    }

    pub(crate) fn declare(
        &mut self,
        variable: &Variable,
        max_slots: usize,
    ) -> Result<(), Diagnostic> {
        if self
            .scopes
            .last()
            .expect("environment has a root scope")
            .bindings
            .contains_key(&variable.name)
        {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!(
                    "a variável '{}' já foi declarada neste escopo",
                    variable.name
                ),
                variable.line,
                variable.column,
            ));
        }

        let slots = variable_slots(variable)?;
        let total_slots = self.allocated_slots.checked_add(slots).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticKind::Runtime,
                "limite de variáveis excedido",
                variable.line,
                variable.column,
            )
        })?;
        if total_slots > max_slots {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!(
                    "programa excede o limite de variáveis e posições de vetores ({max_slots})"
                ),
                variable.line,
                variable.column,
            ));
        }
        let value = if variable.dimensions.is_empty() {
            Value::default_for(variable.data_type)
        } else {
            let element_count = slots;
            let elements = (0..element_count)
                .map(|_| Rc::new(RefCell::new(Value::default_for(variable.data_type))))
                .collect();
            Value::Array(ArrayValue {
                dimensions: variable.dimensions.clone(),
                elements,
            })
        };
        self.insert_binding(
            variable.name.clone(),
            Binding {
                data_type: variable.data_type,
                dimensions: variable.dimensions.clone(),
                value: Rc::new(RefCell::new(value)),
            },
            slots,
            max_slots,
            variable.line,
            variable.column,
        )?;
        Ok(())
    }

    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub(crate) fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            if let Some(scope) = self.scopes.pop() {
                self.allocated_slots = self.allocated_slots.saturating_sub(scope.slots);
            }
        }
    }

    pub(crate) fn check_additional_slots(
        &self,
        slots: usize,
        max_slots: usize,
        line: usize,
        column: usize,
    ) -> Result<(), Diagnostic> {
        let total_slots = self.allocated_slots.checked_add(slots).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticKind::Runtime,
                "limite de variáveis excedido",
                line,
                column,
            )
        })?;
        if total_slots > max_slots {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!(
                    "programa excede o limite de variáveis e posições de vetores ({max_slots})"
                ),
                line,
                column,
            ));
        }
        Ok(())
    }

    pub(crate) fn insert_binding(
        &mut self,
        name: String,
        binding: Binding,
        slots: usize,
        max_slots: usize,
        line: usize,
        column: usize,
    ) -> Result<(), Diagnostic> {
        let scope = self
            .scopes
            .last_mut()
            .expect("environment has a root scope");
        if scope.bindings.contains_key(&name) {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!("a variável '{name}' já foi declarada neste escopo"),
                line,
                column,
            ));
        }
        let total_slots = self.allocated_slots.checked_add(slots).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticKind::Runtime,
                "limite de variáveis excedido",
                line,
                column,
            )
        })?;
        if total_slots > max_slots {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!(
                    "programa excede o limite de variáveis e posições de vetores ({max_slots})"
                ),
                line,
                column,
            ));
        }
        scope.slots += slots;
        scope.bindings.insert(name, binding);
        self.allocated_slots = total_slots;
        Ok(())
    }

    pub(crate) fn lookup(
        &self,
        name: &str,
        expression: &Expression,
    ) -> Result<Binding, Diagnostic> {
        self.scopes
            .last()
            .and_then(|scope| scope.bindings.get(name))
            .or_else(|| {
                self.scopes
                    .first()
                    .and_then(|scope| scope.bindings.get(name))
            })
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticKind::Runtime,
                    format!("variável não declarada: '{name}'"),
                    expression.line,
                    expression.column,
                )
            })
    }

    pub(crate) fn snapshot(&self) -> Vec<(String, String)> {
        let mut visible_bindings = BTreeMap::new();
        if let Some(scope) = self.scopes.first() {
            for (name, binding) in &scope.bindings {
                visible_bindings.insert(name.clone(), binding.clone());
            }
        }
        if self.scopes.len() > 1 {
            if let Some(scope) = self.scopes.last() {
                for (name, binding) in &scope.bindings {
                    visible_bindings.insert(name.clone(), binding.clone());
                }
            }
        }
        let mut visible = BTreeMap::new();
        for (name, binding) in visible_bindings {
            let value = binding.value.borrow();
            if let Value::Array(array) = &*value {
                for (offset, cell) in array.elements.iter().enumerate() {
                    let indexes = array_index_label(offset, &binding.dimensions);
                    visible.insert(format!("{name}[{indexes}]"), cell.borrow().to_text());
                }
            } else {
                visible.insert(name, value.to_text());
            }
        }
        visible.into_iter().collect()
    }

    pub(crate) fn index_cell(
        &self,
        name: &str,
        indexes: &[i64],
        expression: &Expression,
        index_expressions: &[Expression],
    ) -> Result<Rc<RefCell<Value>>, Diagnostic> {
        let binding = self.lookup(name, expression)?;
        let value = binding.value.borrow();
        let Value::Array(array) = &*value else {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!("'{name}' não é um vetor"),
                expression.line,
                expression.column,
            ));
        };
        if indexes.len() != array.dimensions.len() {
            let location = index_expressions.first().unwrap_or(expression);
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!(
                    "o vetor '{name}' exige {} índice(s), mas recebeu {}",
                    array.dimensions.len(),
                    indexes.len()
                ),
                location.line,
                location.column,
            ));
        }

        let mut offset = 0_usize;
        for (dimension_index, (index, dimension)) in
            indexes.iter().zip(&array.dimensions).enumerate()
        {
            let index_location = index_expressions.get(dimension_index).unwrap_or(expression);
            if *index < dimension.lower || *index > dimension.upper {
                return Err(Diagnostic::new(
                    DiagnosticKind::Runtime,
                    format!(
                        "índice {index} fora dos limites {}..{} do vetor '{name}'",
                        dimension.lower, dimension.upper
                    ),
                    index_location.line,
                    index_location.column,
                ));
            }
            let relative = usize::try_from(index - dimension.lower).map_err(|_| {
                Diagnostic::new(
                    DiagnosticKind::Runtime,
                    "índice de vetor fora do limite permitido",
                    index_location.line,
                    index_location.column,
                )
            })?;
            let stride = array.dimensions[dimension_index + 1..]
                .iter()
                .try_fold(1_usize, |size, next| {
                    let dimension_size = dimension_size(*next)?;
                    size.checked_mul(dimension_size)
                })
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Runtime,
                        "tamanho do vetor fora do limite",
                        index_location.line,
                        index_location.column,
                    )
                })?;
            offset = offset
                .checked_add(relative.checked_mul(stride).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Runtime,
                        "índice de vetor fora do limite permitido",
                        index_location.line,
                        index_location.column,
                    )
                })?)
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Runtime,
                        "índice de vetor fora do limite permitido",
                        index_location.line,
                        index_location.column,
                    )
                })?;
        }
        array.elements.get(offset).cloned().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticKind::Runtime,
                "índice de vetor fora do limite permitido",
                expression.line,
                expression.column,
            )
        })
    }
}

fn array_index_label(mut offset: usize, dimensions: &[ArrayDimension]) -> String {
    let mut indexes = vec![0_i64; dimensions.len()];
    for dimension_index in (0..dimensions.len()).rev() {
        let dimension = dimensions[dimension_index];
        let size = dimension_size(dimension).unwrap_or(1).max(1);
        let relative = offset % size;
        offset /= size;
        indexes[dimension_index] = dimension
            .lower
            .saturating_add(i64::try_from(relative).unwrap_or(i64::MAX));
    }
    indexes
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn array_element_count(dimensions: &[ArrayDimension]) -> Option<usize> {
    dimensions.iter().try_fold(1_usize, |count, dimension| {
        count.checked_mul(dimension_size(*dimension)?)
    })
}

pub(crate) fn variable_slots(variable: &Variable) -> Result<usize, Diagnostic> {
    if variable.dimensions.is_empty() {
        Ok(1)
    } else {
        array_element_count(&variable.dimensions).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticKind::Runtime,
                "tamanho do vetor fora do limite",
                variable.line,
                variable.column,
            )
        })
    }
}

fn dimension_size(dimension: ArrayDimension) -> Option<usize> {
    let size = dimension
        .upper
        .checked_sub(dimension.lower)?
        .checked_add(1)?;
    usize::try_from(size).ok()
}
