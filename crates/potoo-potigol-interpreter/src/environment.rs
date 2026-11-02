use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::Value;

pub(crate) type EnvironmentRef = Rc<RefCell<Environment>>;

#[derive(Clone)]
pub(crate) struct Binding {
    pub value: Value,
    pub mutable: bool,
}

pub(crate) struct Environment {
    values: HashMap<String, Binding>,
    parent: Option<EnvironmentRef>,
}

impl Environment {
    pub(crate) fn root() -> EnvironmentRef {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: None,
        }))
    }

    pub(crate) fn child(parent: EnvironmentRef) -> EnvironmentRef {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: Some(parent),
        }))
    }

    pub(crate) fn define(
        environment: &EnvironmentRef,
        name: impl Into<String>,
        value: Value,
        mutable: bool,
    ) -> Result<(), String> {
        let name = name.into();
        let mut environment = environment.borrow_mut();
        if environment.values.contains_key(&name) {
            return Err(format!("'{name}' já foi declarado neste escopo"));
        }
        environment.values.insert(name, Binding { value, mutable });
        Ok(())
    }

    pub(crate) fn lookup(environment: &EnvironmentRef, name: &str) -> Option<Binding> {
        let (binding, parent) = {
            let environment = environment.borrow();
            (
                environment.values.get(name).cloned(),
                environment.parent.clone(),
            )
        };
        binding.or_else(|| parent.and_then(|parent| Self::lookup(&parent, name)))
    }

    pub(crate) fn assign(
        environment: &EnvironmentRef,
        name: &str,
        value: Value,
    ) -> Result<(), String> {
        let parent = {
            let mut environment = environment.borrow_mut();
            if let Some(binding) = environment.values.get_mut(name) {
                if !binding.mutable {
                    return Err(format!("'{name}' não foi declarado como mutável"));
                }
                binding.value = value;
                return Ok(());
            }
            environment.parent.clone()
        };
        if let Some(parent) = parent {
            Self::assign(&parent, name, value)
        } else {
            Err(format!("identificador '{name}' não foi declarado"))
        }
    }

    pub(crate) fn visible_values(environment: &EnvironmentRef) -> Vec<(String, Value)> {
        let (values, parent) = {
            let environment = environment.borrow();
            (
                environment
                    .values
                    .iter()
                    .map(|(name, binding)| (name.clone(), binding.value.clone()))
                    .collect::<Vec<_>>(),
                environment.parent.clone(),
            )
        };
        let mut visible = values.into_iter().collect::<HashMap<_, _>>();
        if let Some(parent) = parent {
            for (name, value) in Self::visible_values(&parent) {
                visible.entry(name).or_insert(value);
            }
        }
        let mut values = visible.into_iter().collect::<Vec<_>>();
        values.sort_by(|left, right| left.0.cmp(&right.0));
        values
    }
}
