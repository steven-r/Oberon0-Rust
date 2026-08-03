use crate::ast::{Expr, TypeRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Scalar(ScalarType),
    Alias(String),
    Array {
        element_type: Box<Type>,
        length: Option<usize>,
    },
    Record {
        fields: Vec<(String, Type)>,
    },
    Procedure {
        parameters: Vec<ProcedureParameter>,
        result: Option<Box<Type>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarType {
    Integer,
    Boolean,
    Real,
    LongReal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureParameter {
    pub name: Option<String>,
    pub ty: Type,
    pub is_var: bool,
}

impl Type {
    pub fn from_ast_type_ref(type_ref: &TypeRef) -> Self {
        match type_ref {
            TypeRef::Integer => Self::Scalar(ScalarType::Integer),
            TypeRef::Boolean => Self::Scalar(ScalarType::Boolean),
            TypeRef::Real => Self::Scalar(ScalarType::Real),
            TypeRef::LongReal => Self::Scalar(ScalarType::LongReal),
            TypeRef::Array {
                element_type,
                length,
            } => Self::Array {
                element_type: Box::new(Self::from_ast_type_ref(element_type)),
                length: match length {
                    Expr::Integer(value) if *value >= 0 => Some(*value as usize),
                    _ => None,
                },
            },
            TypeRef::Named(name) => Self::Alias(name.clone()),
            TypeRef::Qualified { .. } => Self::Alias("<qualified>".to_string()),
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Scalar(ScalarType::Integer | ScalarType::Real | ScalarType::LongReal)
        )
    }

    pub fn resolve_aliases(&self, aliases: &std::collections::HashMap<String, Type>) -> Self {
        match self {
            Self::Alias(name) => aliases
                .get(name)
                .cloned()
                .map(|resolved| resolved.resolve_aliases(aliases))
                .unwrap_or_else(|| self.clone()),
            Self::Array {
                element_type,
                length,
            } => Self::Array {
                element_type: Box::new(element_type.resolve_aliases(aliases)),
                length: *length,
            },
            Self::Record { fields } => Self::Record {
                fields: fields
                    .iter()
                    .map(|(name, field_type)| (name.clone(), field_type.resolve_aliases(aliases)))
                    .collect(),
            },
            Self::Procedure { parameters, result } => Self::Procedure {
                parameters: parameters
                    .iter()
                    .map(|param| ProcedureParameter {
                        name: param.name.clone(),
                        ty: param.ty.resolve_aliases(aliases),
                        is_var: param.is_var,
                    })
                    .collect(),
                result: result
                    .as_ref()
                    .map(|result| Box::new(result.resolve_aliases(aliases))),
            },
            _ => self.clone(),
        }
    }

    /// Checks whether a value with one type can be used where the other type is expected.
    ///
    /// This is a semantic compatibility check for the current Oberon0 subset. It resolves
    /// aliases first and then applies the documented subset rules: scalar kinds must match
    /// exactly, and there is no implicit numeric conversion between INTEGER, REAL, and
    /// LONGREAL.
    pub fn is_compatible_with(
        &self,
        other: &Self,
        aliases: &std::collections::HashMap<String, Type>,
    ) -> bool {
        let expected = self.resolve_aliases(aliases);
        let actual = other.resolve_aliases(aliases);

        match (&expected, &actual) {
            (Self::Scalar(expected_scalar), Self::Scalar(actual_scalar)) => {
                expected_scalar == actual_scalar
            }
            (
                Self::Array {
                    element_type: expected_element,
                    length: expected_len,
                },
                Self::Array {
                    element_type: actual_element,
                    length: actual_len,
                },
            ) => {
                expected_len == actual_len
                    && expected_element.is_compatible_with(actual_element, aliases)
            }
            (
                Self::Record {
                    fields: expected_fields,
                },
                Self::Record {
                    fields: actual_fields,
                },
            ) => {
                expected_fields.len() == actual_fields.len()
                    && expected_fields.iter().zip(actual_fields.iter()).all(
                        |((expected_name, expected_field), (actual_name, actual_field))| {
                            expected_name == actual_name
                                && expected_field.is_compatible_with(actual_field, aliases)
                        },
                    )
            }
            (
                Self::Procedure {
                    parameters: expected_params,
                    result: expected_result,
                },
                Self::Procedure {
                    parameters: actual_params,
                    result: actual_result,
                },
            ) => {
                expected_params.len() == actual_params.len()
                    && expected_params.iter().zip(actual_params.iter()).all(
                        |(expected_param, actual_param)| {
                            expected_param.is_var == actual_param.is_var
                                && expected_param
                                    .ty
                                    .is_compatible_with(&actual_param.ty, aliases)
                        },
                    )
                    && match (expected_result, actual_result) {
                        (Some(expected_result), Some(actual_result)) => {
                            expected_result.is_compatible_with(actual_result, aliases)
                        }
                        (None, None) => true,
                        _ => false,
                    }
            }
            _ => expected == actual,
        }
    }
}

#[cfg(test)]
mod tests;
