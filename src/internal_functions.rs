//! Declarative catalog and generic call resolution for compiler-provided functions.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::types::{ScalarType, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InternalFunctionId {
    IoWriteInt,
    IoWriteString,
    IoWriteLn,
    IoWriteReal,
    IoWriteLongReal,
    IoReadInt,
    IoReadReal,
    IoReadLongReal,
    IoEof,
    MathFlt,
    MathFloor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallContext {
    StatementOnly,
    ExpressionOnly,
    StatementOrExpression,
}

impl CallContext {
    fn accepts(self, actual: Self) -> bool {
        self == Self::StatementOrExpression || self == actual
    }
}

impl fmt::Display for CallContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::StatementOnly => "statement",
            Self::ExpressionOnly => "expression",
            Self::StatementOrExpression => "statement or expression",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatalogTypeRef {
    Integer,
    Boolean,
    Real,
    LongReal,
    String,
}

impl CatalogTypeRef {
    fn resolved(self) -> ResolvedType {
        let scalar = match self {
            Self::Integer => ScalarType::Integer,
            Self::Boolean => ScalarType::Boolean,
            Self::Real => ScalarType::Real,
            Self::LongReal => ScalarType::LongReal,
            Self::String => return ResolvedType::StringLiteral,
        };
        ResolvedType::Type(Type::Scalar(scalar))
    }
}

impl fmt::Display for CatalogTypeRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Integer => "INTEGER",
            Self::Boolean => "BOOLEAN",
            Self::Real => "REAL",
            Self::LongReal => "LONGREAL",
            Self::String => "STRING",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedType {
    Type(Type),
    StringLiteral,
}

impl fmt::Display for ResolvedType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StringLiteral => formatter.write_str("STRING"),
            Self::Type(Type::Scalar(ScalarType::Integer)) => formatter.write_str("INTEGER"),
            Self::Type(Type::Scalar(ScalarType::Boolean)) => formatter.write_str("BOOLEAN"),
            Self::Type(Type::Scalar(ScalarType::Real)) => formatter.write_str("REAL"),
            Self::Type(Type::Scalar(ScalarType::LongReal)) => formatter.write_str("LONGREAL"),
            Self::Type(Type::Alias(name)) => formatter.write_str(name),
            Self::Type(Type::Array { .. }) => formatter.write_str("ARRAY"),
            Self::Type(Type::Record { .. }) => formatter.write_str("RECORD"),
            Self::Type(Type::Procedure { .. }) => formatter.write_str("PROCEDURE"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterCardinality {
    Required,
    Optional,
    Variadic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterMode {
    Value,
    Var,
    Literal,
}

impl fmt::Display for ParameterMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Value => "value",
            Self::Var => "VAR",
            Self::Literal => "literal",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypePredicate {
    Any,
    Numeric,
    Scalar,
}

impl fmt::Display for TypePredicate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Any => "any expression type",
            Self::Numeric => "numeric",
            Self::Scalar => "scalar",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVariableId(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeConstraint {
    Exact(CatalogTypeRef),
    OneOf(&'static [CatalogTypeRef]),
    Predicate(TypePredicate),
    TypeVariable(TypeVariableId, &'static TypeConstraint),
}

impl TypeConstraint {
    fn matches(
        self,
        actual: &ResolvedType,
        bindings: &mut HashMap<TypeVariableId, ResolvedType>,
    ) -> bool {
        match self {
            Self::Exact(expected) => expected.resolved() == *actual,
            Self::OneOf(expected) => expected
                .iter()
                .any(|candidate| candidate.resolved() == *actual),
            Self::Predicate(predicate) => predicate_matches(predicate, actual),
            Self::TypeVariable(id, constraint) => {
                if !constraint.matches(actual, bindings) {
                    return false;
                }
                match bindings.get(&id) {
                    Some(bound) => bound == actual,
                    None => {
                        bindings.insert(id, actual.clone());
                        true
                    }
                }
            }
        }
    }

    fn binds(self, id: TypeVariableId) -> bool {
        matches!(self, Self::TypeVariable(bound, _) if bound == id)
    }
}

impl fmt::Display for TypeConstraint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(expected) => write!(formatter, "{expected}"),
            Self::OneOf(expected) => {
                let formatted = expected
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" | ");
                formatter.write_str(&formatted)
            }
            Self::Predicate(predicate) => write!(formatter, "{predicate}"),
            Self::TypeVariable(id, constraint) => write!(formatter, "T{}: {constraint}", id.0),
        }
    }
}

fn predicate_matches(predicate: TypePredicate, actual: &ResolvedType) -> bool {
    match predicate {
        TypePredicate::Any => !matches!(actual, ResolvedType::StringLiteral),
        TypePredicate::Numeric => matches!(
            actual,
            ResolvedType::Type(Type::Scalar(
                ScalarType::Integer | ScalarType::Real | ScalarType::LongReal
            ))
        ),
        TypePredicate::Scalar => matches!(actual, ResolvedType::Type(Type::Scalar(_))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParameterSpec {
    pub cardinality: ParameterCardinality,
    pub mode: ParameterMode,
    pub type_constraint: TypeConstraint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeTransform {
    Identity,
    ToReal,
    ToLongReal,
}

impl TypeTransform {
    fn apply(self, source: &ResolvedType) -> ResolvedType {
        match self {
            Self::Identity => source.clone(),
            Self::ToReal => CatalogTypeRef::Real.resolved(),
            Self::ToLongReal => CatalogTypeRef::LongReal.resolved(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultSpec {
    None,
    Exact(CatalogTypeRef),
    TypeVariable(TypeVariableId),
    FromArgument {
        index: usize,
        transform: TypeTransform,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternalSignature {
    pub parameters: &'static [ParameterSpec],
    pub result: ResultSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternalFunctionDescriptor {
    pub id: InternalFunctionId,
    pub module_name: &'static str,
    pub member_name: &'static str,
    pub call_context: CallContext,
    pub signatures: &'static [InternalSignature],
}

impl InternalFunctionDescriptor {
    pub fn qualified_name(self) -> String {
        format!("{}.{}", self.module_name, self.member_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalArgument {
    pub ty: ResolvedType,
    pub is_literal: bool,
    pub is_assignable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalCallSite {
    pub context: CallContext,
    pub arguments: Vec<InternalArgument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInternalParameter {
    pub mode: ParameterMode,
    pub ty: ResolvedType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInternalCall {
    pub id: InternalFunctionId,
    pub signature_index: usize,
    pub parameters: Vec<ResolvedInternalParameter>,
    pub result: Option<ResolvedType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternalCallErrorKind {
    InvalidCallContext {
        expected: CallContext,
    },
    ArityMismatch,
    ParameterModeMismatch {
        position: usize,
        expected: ParameterMode,
    },
    ArgumentTypeMismatch {
        position: usize,
        expected: TypeConstraint,
    },
    AmbiguousSignature,
    InvalidDescriptor {
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalCallError {
    pub qualified_name: String,
    pub actual_context: CallContext,
    pub actual_argument_types: Vec<ResolvedType>,
    pub accepted_signatures: Vec<String>,
    pub kind: InternalCallErrorKind,
}

const ANY: TypeConstraint = TypeConstraint::Predicate(TypePredicate::Any);
#[cfg(test)]
const NUMERIC: TypeConstraint = TypeConstraint::Predicate(TypePredicate::Numeric);
const FLOOR_TYPES: &[CatalogTypeRef] = &[CatalogTypeRef::Real, CatalogTypeRef::LongReal];

const WRITE_INT_PARAMS: &[ParameterSpec] = &[ParameterSpec {
    cardinality: ParameterCardinality::Variadic,
    mode: ParameterMode::Value,
    type_constraint: ANY,
}];
const WRITE_STRING_PARAMS: &[ParameterSpec] = &[ParameterSpec {
    cardinality: ParameterCardinality::Required,
    mode: ParameterMode::Literal,
    type_constraint: TypeConstraint::Exact(CatalogTypeRef::String),
}];
const WRITE_REAL_PARAMS: &[ParameterSpec] = &[ParameterSpec {
    cardinality: ParameterCardinality::Required,
    mode: ParameterMode::Value,
    type_constraint: TypeConstraint::Exact(CatalogTypeRef::Real),
}];
const WRITE_LONG_REAL_PARAMS: &[ParameterSpec] = &[ParameterSpec {
    cardinality: ParameterCardinality::Required,
    mode: ParameterMode::Value,
    type_constraint: TypeConstraint::Exact(CatalogTypeRef::LongReal),
}];
const FLT_PARAMS: &[ParameterSpec] = &[ParameterSpec {
    cardinality: ParameterCardinality::Required,
    mode: ParameterMode::Value,
    type_constraint: TypeConstraint::Exact(CatalogTypeRef::Integer),
}];
const FLOOR_PARAMS: &[ParameterSpec] = &[ParameterSpec {
    cardinality: ParameterCardinality::Required,
    mode: ParameterMode::Value,
    type_constraint: TypeConstraint::OneOf(FLOOR_TYPES),
}];

const NO_ARGS_NO_RESULT: &[InternalSignature] = &[InternalSignature {
    parameters: &[],
    result: ResultSpec::None,
}];
const WRITE_INT_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: WRITE_INT_PARAMS,
    result: ResultSpec::None,
}];
const WRITE_STRING_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: WRITE_STRING_PARAMS,
    result: ResultSpec::None,
}];
const WRITE_REAL_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: WRITE_REAL_PARAMS,
    result: ResultSpec::None,
}];
const WRITE_LONG_REAL_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: WRITE_LONG_REAL_PARAMS,
    result: ResultSpec::None,
}];
const READ_INT_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: &[],
    result: ResultSpec::Exact(CatalogTypeRef::Integer),
}];
const READ_REAL_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: &[],
    result: ResultSpec::Exact(CatalogTypeRef::Real),
}];
const READ_LONG_REAL_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: &[],
    result: ResultSpec::Exact(CatalogTypeRef::LongReal),
}];
const FLT_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: FLT_PARAMS,
    result: ResultSpec::Exact(CatalogTypeRef::Real),
}];
const FLOOR_SIGNATURES: &[InternalSignature] = &[InternalSignature {
    parameters: FLOOR_PARAMS,
    result: ResultSpec::Exact(CatalogTypeRef::Integer),
}];

const DESCRIPTORS: &[InternalFunctionDescriptor] = &[
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoWriteInt,
        module_name: "IO",
        member_name: "WriteInt",
        call_context: CallContext::StatementOnly,
        signatures: WRITE_INT_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoWriteString,
        module_name: "IO",
        member_name: "WriteString",
        call_context: CallContext::StatementOnly,
        signatures: WRITE_STRING_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoWriteLn,
        module_name: "IO",
        member_name: "WriteLn",
        call_context: CallContext::StatementOnly,
        signatures: NO_ARGS_NO_RESULT,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoWriteReal,
        module_name: "IO",
        member_name: "WriteReal",
        call_context: CallContext::StatementOnly,
        signatures: WRITE_REAL_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoWriteLongReal,
        module_name: "IO",
        member_name: "WriteLongReal",
        call_context: CallContext::StatementOnly,
        signatures: WRITE_LONG_REAL_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoReadInt,
        module_name: "IO",
        member_name: "ReadInt",
        call_context: CallContext::ExpressionOnly,
        signatures: READ_INT_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoReadReal,
        module_name: "IO",
        member_name: "ReadReal",
        call_context: CallContext::ExpressionOnly,
        signatures: READ_REAL_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoReadLongReal,
        module_name: "IO",
        member_name: "ReadLongReal",
        call_context: CallContext::ExpressionOnly,
        signatures: READ_LONG_REAL_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::IoEof,
        module_name: "IO",
        member_name: "EOF",
        call_context: CallContext::ExpressionOnly,
        signatures: READ_INT_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::MathFlt,
        module_name: "MATH",
        member_name: "FLT",
        call_context: CallContext::ExpressionOnly,
        signatures: FLT_SIGNATURES,
    },
    InternalFunctionDescriptor {
        id: InternalFunctionId::MathFloor,
        module_name: "MATH",
        member_name: "FLOOR",
        call_context: CallContext::ExpressionOnly,
        signatures: FLOOR_SIGNATURES,
    },
];

pub fn lookup(module: &str, member: &str) -> Option<&'static InternalFunctionDescriptor> {
    DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.module_name == module && descriptor.member_name == member)
}

pub fn module_exists(module: &str) -> bool {
    DESCRIPTORS
        .iter()
        .any(|descriptor| descriptor.module_name == module)
}

pub fn descriptors() -> &'static [InternalFunctionDescriptor] {
    DESCRIPTORS
}

pub fn format_signature(
    descriptor: &InternalFunctionDescriptor,
    signature: &InternalSignature,
) -> String {
    let parameters = signature
        .parameters
        .iter()
        .map(format_parameter)
        .collect::<Vec<_>>()
        .join(", ");
    let result = match signature.result {
        ResultSpec::None => String::new(),
        ResultSpec::Exact(result) => format!(" -> {result}"),
        ResultSpec::TypeVariable(id) => format!(" -> T{}", id.0),
        ResultSpec::FromArgument { index, transform } => {
            format!(" -> argument {} via {transform:?}", index + 1)
        }
    };
    format!("{}({parameters}){result}", descriptor.member_name)
}

fn format_parameter(parameter: &ParameterSpec) -> String {
    let mode = match parameter.mode {
        ParameterMode::Value => String::new(),
        ParameterMode::Var => "VAR ".to_string(),
        ParameterMode::Literal => "literal ".to_string(),
    };
    let cardinality = match parameter.cardinality {
        ParameterCardinality::Required => String::new(),
        ParameterCardinality::Optional => "?".to_string(),
        ParameterCardinality::Variadic => "...".to_string(),
    };
    format!("{mode}{}{cardinality}", parameter.type_constraint)
}

pub fn resolve_call(
    descriptor: &InternalFunctionDescriptor,
    call_site: &InternalCallSite,
) -> Result<ResolvedInternalCall, InternalCallError> {
    let error = |kind| call_error(descriptor, call_site, kind);
    if !descriptor.call_context.accepts(call_site.context) {
        return Err(error(InternalCallErrorKind::InvalidCallContext {
            expected: descriptor.call_context,
        }));
    }

    let mut candidates = Vec::new();
    for (signature_index, signature) in descriptor.signatures.iter().enumerate() {
        for parameters in expand_parameters(signature.parameters, call_site.arguments.len()) {
            candidates.push((signature_index, signature, parameters));
        }
    }
    if candidates.is_empty() {
        return Err(error(InternalCallErrorKind::ArityMismatch));
    }

    let mut successes = Vec::new();
    let mut mode_error = None;
    let mut type_error = None;
    for (signature_index, signature, parameters) in candidates {
        let mut bindings = HashMap::new();
        let mut resolved_parameters = Vec::new();
        let mut failed = false;
        for (position, (parameter, argument)) in parameters
            .iter()
            .zip(call_site.arguments.iter())
            .enumerate()
        {
            let mode_matches = match parameter.mode {
                ParameterMode::Value => true,
                ParameterMode::Var => argument.is_assignable,
                ParameterMode::Literal => argument.is_literal,
            };
            if !mode_matches {
                mode_error.get_or_insert(InternalCallErrorKind::ParameterModeMismatch {
                    position: position + 1,
                    expected: parameter.mode,
                });
                failed = true;
                break;
            }
            if !parameter
                .type_constraint
                .matches(&argument.ty, &mut bindings)
            {
                type_error.get_or_insert(InternalCallErrorKind::ArgumentTypeMismatch {
                    position: position + 1,
                    expected: parameter.type_constraint,
                });
                failed = true;
                break;
            }
            resolved_parameters.push(ResolvedInternalParameter {
                mode: parameter.mode,
                ty: argument.ty.clone(),
            });
        }
        if failed {
            continue;
        }

        let result = match resolve_result(signature.result, &bindings, &call_site.arguments) {
            Ok(result) => result,
            Err(detail) => return Err(error(InternalCallErrorKind::InvalidDescriptor { detail })),
        };
        successes.push(ResolvedInternalCall {
            id: descriptor.id,
            signature_index,
            parameters: resolved_parameters,
            result,
        });
    }

    match successes.len() {
        0 => Err(error(
            mode_error
                .or(type_error)
                .expect("candidate must fail by mode or type"),
        )),
        1 => Ok(successes.pop().expect("one successful candidate")),
        _ => Err(error(InternalCallErrorKind::AmbiguousSignature)),
    }
}

fn expand_parameters(
    parameters: &'static [ParameterSpec],
    arity: usize,
) -> Vec<Vec<&'static ParameterSpec>> {
    let required = parameters
        .iter()
        .take_while(|parameter| parameter.cardinality == ParameterCardinality::Required)
        .count();
    let optional = parameters
        .iter()
        .filter(|parameter| parameter.cardinality == ParameterCardinality::Optional)
        .count();
    let variadic = parameters
        .last()
        .filter(|parameter| parameter.cardinality == ParameterCardinality::Variadic);

    if arity < required || (variadic.is_none() && arity > required + optional) {
        return Vec::new();
    }

    let remaining = arity - required;
    let optional_counts: Vec<usize> = if variadic.is_some() {
        (0..=optional.min(remaining)).collect()
    } else {
        vec![remaining]
    };
    optional_counts
        .into_iter()
        .map(|optional_count| {
            let mut expanded = parameters[..required + optional_count]
                .iter()
                .collect::<Vec<_>>();
            if let Some(variadic_parameter) = variadic {
                expanded.extend(std::iter::repeat_n(
                    variadic_parameter,
                    remaining - optional_count,
                ));
            }
            expanded
        })
        .collect()
}

fn resolve_result(
    result: ResultSpec,
    bindings: &HashMap<TypeVariableId, ResolvedType>,
    arguments: &[InternalArgument],
) -> Result<Option<ResolvedType>, String> {
    match result {
        ResultSpec::None => Ok(None),
        ResultSpec::Exact(result) => Ok(Some(result.resolved())),
        ResultSpec::TypeVariable(id) => bindings
            .get(&id)
            .cloned()
            .map(Some)
            .ok_or_else(|| format!("result type variable T{} is unbound", id.0)),
        ResultSpec::FromArgument { index, transform } => arguments
            .get(index)
            .map(|argument| Some(transform.apply(&argument.ty)))
            .ok_or_else(|| format!("result argument {} does not exist", index + 1)),
    }
}

fn call_error(
    descriptor: &InternalFunctionDescriptor,
    call_site: &InternalCallSite,
    kind: InternalCallErrorKind,
) -> InternalCallError {
    InternalCallError {
        qualified_name: descriptor.qualified_name(),
        actual_context: call_site.context,
        actual_argument_types: call_site
            .arguments
            .iter()
            .map(|argument| argument.ty.clone())
            .collect(),
        accepted_signatures: descriptor
            .signatures
            .iter()
            .map(|signature| format_signature(descriptor, signature))
            .collect(),
        kind,
    }
}

pub fn validate_catalog() -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for descriptor in DESCRIPTORS {
        if !ids.insert(descriptor.id) {
            return Err(format!(
                "duplicate internal function id: {:?}",
                descriptor.id
            ));
        }
        if !names.insert((descriptor.module_name, descriptor.member_name)) {
            return Err(format!(
                "duplicate internal function name: {}",
                descriptor.qualified_name()
            ));
        }
        if descriptor.signatures.is_empty() {
            return Err(format!("{} has no signatures", descriptor.qualified_name()));
        }
        for signature in descriptor.signatures {
            validate_signature(descriptor, signature)?;
        }
    }
    Ok(())
}

fn validate_signature(
    descriptor: &InternalFunctionDescriptor,
    signature: &InternalSignature,
) -> Result<(), String> {
    let mut optional_seen = false;
    let mut variadic_seen = false;
    for (index, parameter) in signature.parameters.iter().enumerate() {
        match parameter.cardinality {
            ParameterCardinality::Required if optional_seen || variadic_seen => {
                return Err(format!(
                    "{} has a required parameter after a non-required parameter",
                    descriptor.qualified_name()
                ));
            }
            ParameterCardinality::Required => {}
            ParameterCardinality::Optional if variadic_seen => {
                return Err(format!(
                    "{} has an optional parameter after a variadic parameter",
                    descriptor.qualified_name()
                ));
            }
            ParameterCardinality::Optional => optional_seen = true,
            ParameterCardinality::Variadic
                if variadic_seen || index + 1 != signature.parameters.len() =>
            {
                return Err(format!(
                    "{} has an invalid variadic parameter",
                    descriptor.qualified_name()
                ));
            }
            ParameterCardinality::Variadic => variadic_seen = true,
        }
    }

    if let ResultSpec::TypeVariable(id) = signature.result
        && !signature
            .parameters
            .iter()
            .any(|parameter| parameter.type_constraint.binds(id))
    {
        return Err(format!(
            "{} has an unbound result type variable T{}",
            descriptor.qualified_name(),
            id.0
        ));
    }
    if let ResultSpec::FromArgument { index, .. } = signature.result
        && index >= signature.parameters.len()
    {
        return Err(format!(
            "{} derives its result from a missing parameter",
            descriptor.qualified_name()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTEGER: TypeConstraint = TypeConstraint::Exact(CatalogTypeRef::Integer);
    const T0_NUMERIC: TypeConstraint = TypeConstraint::TypeVariable(TypeVariableId(0), &NUMERIC);

    fn argument(ty: CatalogTypeRef) -> InternalArgument {
        InternalArgument {
            ty: ty.resolved(),
            is_literal: false,
            is_assignable: false,
        }
    }

    fn descriptor(
        context: CallContext,
        signatures: &'static [InternalSignature],
    ) -> InternalFunctionDescriptor {
        InternalFunctionDescriptor {
            id: InternalFunctionId::MathFlt,
            module_name: "TEST",
            member_name: "Call",
            call_context: context,
            signatures,
        }
    }

    fn call(arguments: Vec<InternalArgument>) -> InternalCallSite {
        InternalCallSite {
            context: CallContext::ExpressionOnly,
            arguments,
        }
    }

    #[test]
    fn production_catalog_is_valid_unique_and_reachable() {
        validate_catalog().expect("production catalog should be valid");
        assert_eq!(DESCRIPTORS.len(), 11);
        for entry in DESCRIPTORS {
            assert_eq!(lookup(entry.module_name, entry.member_name), Some(entry));
        }
        assert!(module_exists("IO"));
        assert!(module_exists("MATH"));
        assert!(!module_exists("SYSTEM"));
        assert_eq!(lookup("IO", "Missing"), None);
    }

    #[test]
    fn production_signatures_format_deterministically() {
        let floor = lookup("MATH", "FLOOR").expect("FLOOR descriptor");
        assert_eq!(
            format_signature(floor, &floor.signatures[0]),
            "FLOOR(REAL | LONGREAL) -> INTEGER"
        );
        let write_int = lookup("IO", "WriteInt").expect("WriteInt descriptor");
        assert_eq!(
            format_signature(write_int, &write_int.signatures[0]),
            "WriteInt(any expression type...)"
        );
    }

    #[test]
    fn diagnostic_formatting_covers_catalog_vocabulary() {
        assert_eq!(CallContext::StatementOnly.to_string(), "statement");
        assert_eq!(CallContext::ExpressionOnly.to_string(), "expression");
        assert_eq!(
            CallContext::StatementOrExpression.to_string(),
            "statement or expression"
        );
        assert_eq!(CatalogTypeRef::Boolean.to_string(), "BOOLEAN");
        assert_eq!(CatalogTypeRef::String.to_string(), "STRING");
        assert_eq!(ParameterMode::Value.to_string(), "value");
        assert_eq!(ParameterMode::Var.to_string(), "VAR");
        assert_eq!(ParameterMode::Literal.to_string(), "literal");
        assert_eq!(TypePredicate::Numeric.to_string(), "numeric");
        assert_eq!(TypePredicate::Scalar.to_string(), "scalar");

        let resolved_types = [
            (ResolvedType::StringLiteral, "STRING"),
            (CatalogTypeRef::Integer.resolved(), "INTEGER"),
            (CatalogTypeRef::Boolean.resolved(), "BOOLEAN"),
            (CatalogTypeRef::Real.resolved(), "REAL"),
            (CatalogTypeRef::LongReal.resolved(), "LONGREAL"),
            (
                ResolvedType::Type(Type::Alias("Meters".to_string())),
                "Meters",
            ),
            (
                ResolvedType::Type(Type::Array {
                    element_type: Box::new(Type::Scalar(ScalarType::Integer)),
                    length: Some(1),
                }),
                "ARRAY",
            ),
            (
                ResolvedType::Type(Type::Record { fields: Vec::new() }),
                "RECORD",
            ),
            (
                ResolvedType::Type(Type::Procedure {
                    parameters: Vec::new(),
                    result: None,
                }),
                "PROCEDURE",
            ),
        ];
        for (resolved_type, expected) in resolved_types {
            assert_eq!(resolved_type.to_string(), expected);
        }

        assert_eq!(descriptors(), DESCRIPTORS);
        assert_eq!(
            TypeTransform::Identity.apply(&ResolvedType::StringLiteral),
            ResolvedType::StringLiteral
        );
        assert_eq!(
            TypeTransform::ToReal.apply(&CatalogTypeRef::Integer.resolved()),
            CatalogTypeRef::Real.resolved()
        );
    }

    #[test]
    fn malformed_result_specification_returns_a_structured_internal_error() {
        const SIGNATURES: &[InternalSignature] = &[InternalSignature {
            parameters: &[],
            result: ResultSpec::TypeVariable(TypeVariableId(7)),
        }];
        let entry = descriptor(CallContext::ExpressionOnly, SIGNATURES);
        let error = resolve_call(&entry, &call(Vec::new()))
            .expect_err("an unbound result variable is an invalid descriptor");
        assert!(matches!(
            error.kind,
            InternalCallErrorKind::InvalidDescriptor { ref detail }
                if detail == "result type variable T7 is unbound"
        ));
    }

    #[test]
    fn every_production_descriptor_resolves_a_representative_call_uniquely() {
        for entry in DESCRIPTORS {
            let (context, arguments, expected_result) = match entry.id {
                InternalFunctionId::IoWriteInt | InternalFunctionId::IoWriteLn => {
                    (CallContext::StatementOnly, Vec::new(), None)
                }
                InternalFunctionId::IoWriteString => (
                    CallContext::StatementOnly,
                    vec![InternalArgument {
                        ty: ResolvedType::StringLiteral,
                        is_literal: true,
                        is_assignable: false,
                    }],
                    None,
                ),
                InternalFunctionId::IoWriteReal => (
                    CallContext::StatementOnly,
                    vec![argument(CatalogTypeRef::Real)],
                    None,
                ),
                InternalFunctionId::IoWriteLongReal => (
                    CallContext::StatementOnly,
                    vec![argument(CatalogTypeRef::LongReal)],
                    None,
                ),
                InternalFunctionId::IoReadInt | InternalFunctionId::IoEof => (
                    CallContext::ExpressionOnly,
                    Vec::new(),
                    Some(CatalogTypeRef::Integer.resolved()),
                ),
                InternalFunctionId::IoReadReal => (
                    CallContext::ExpressionOnly,
                    Vec::new(),
                    Some(CatalogTypeRef::Real.resolved()),
                ),
                InternalFunctionId::IoReadLongReal => (
                    CallContext::ExpressionOnly,
                    Vec::new(),
                    Some(CatalogTypeRef::LongReal.resolved()),
                ),
                InternalFunctionId::MathFlt => (
                    CallContext::ExpressionOnly,
                    vec![argument(CatalogTypeRef::Integer)],
                    Some(CatalogTypeRef::Real.resolved()),
                ),
                InternalFunctionId::MathFloor => (
                    CallContext::ExpressionOnly,
                    vec![argument(CatalogTypeRef::Real)],
                    Some(CatalogTypeRef::Integer.resolved()),
                ),
            };
            let resolved = resolve_call(entry, &InternalCallSite { context, arguments })
                .expect("production descriptor should resolve uniquely");
            assert_eq!(resolved.id, entry.id);
            assert_eq!(resolved.result, expected_result);
        }
    }

    #[test]
    fn fixed_arity_and_context_failures_are_structured() {
        let flt = lookup("MATH", "FLT").expect("FLT descriptor");
        let wrong_context = resolve_call(
            flt,
            &InternalCallSite {
                context: CallContext::StatementOnly,
                arguments: vec![argument(CatalogTypeRef::Integer)],
            },
        )
        .expect_err("FLT is expression-only");
        assert!(matches!(
            wrong_context.kind,
            InternalCallErrorKind::InvalidCallContext { .. }
        ));

        let wrong_arity = resolve_call(flt, &call(Vec::new())).expect_err("FLT needs one argument");
        assert_eq!(wrong_arity.kind, InternalCallErrorKind::ArityMismatch);
        assert_eq!(
            wrong_arity.accepted_signatures,
            vec!["FLT(INTEGER) -> REAL"]
        );
    }

    #[test]
    fn write_int_preserves_zero_one_and_many_argument_compatibility() {
        let write_int = lookup("IO", "WriteInt").expect("WriteInt descriptor");
        for arguments in [
            Vec::new(),
            vec![argument(CatalogTypeRef::Integer)],
            vec![
                argument(CatalogTypeRef::Integer),
                argument(CatalogTypeRef::Boolean),
            ],
        ] {
            let resolved = resolve_call(
                write_int,
                &InternalCallSite {
                    context: CallContext::StatementOnly,
                    arguments,
                },
            )
            .expect("WriteInt compatibility call should resolve");
            assert_eq!(resolved.id, InternalFunctionId::IoWriteInt);
        }
    }

    #[test]
    fn exact_one_of_and_predicate_constraints_match() {
        let floor = lookup("MATH", "FLOOR").expect("FLOOR descriptor");
        assert!(resolve_call(floor, &call(vec![argument(CatalogTypeRef::Real)])).is_ok());
        assert!(resolve_call(floor, &call(vec![argument(CatalogTypeRef::LongReal)])).is_ok());
        let error = resolve_call(floor, &call(vec![argument(CatalogTypeRef::Integer)]))
            .expect_err("INTEGER is not accepted by FLOOR");
        assert!(matches!(
            error.kind,
            InternalCallErrorKind::ArgumentTypeMismatch { position: 1, .. }
        ));
    }

    #[test]
    fn literal_and_var_modes_are_enforced_before_types() {
        const PARAMETERS: &[ParameterSpec] = &[
            ParameterSpec {
                cardinality: ParameterCardinality::Required,
                mode: ParameterMode::Literal,
                type_constraint: INTEGER,
            },
            ParameterSpec {
                cardinality: ParameterCardinality::Required,
                mode: ParameterMode::Var,
                type_constraint: INTEGER,
            },
        ];
        const SIGNATURES: &[InternalSignature] = &[InternalSignature {
            parameters: PARAMETERS,
            result: ResultSpec::None,
        }];
        let entry = descriptor(CallContext::ExpressionOnly, SIGNATURES);
        let error = resolve_call(
            &entry,
            &call(vec![
                argument(CatalogTypeRef::Real),
                argument(CatalogTypeRef::Integer),
            ]),
        )
        .expect_err("first argument is not literal");
        assert_eq!(
            error.kind,
            InternalCallErrorKind::ParameterModeMismatch {
                position: 1,
                expected: ParameterMode::Literal
            }
        );

        let mut literal = argument(CatalogTypeRef::Integer);
        literal.is_literal = true;
        let error = resolve_call(
            &entry,
            &call(vec![literal.clone(), argument(CatalogTypeRef::Integer)]),
        )
        .expect_err("second argument is not assignable");
        assert_eq!(
            error.kind,
            InternalCallErrorKind::ParameterModeMismatch {
                position: 2,
                expected: ParameterMode::Var
            }
        );
        let mut assignable = argument(CatalogTypeRef::Integer);
        assignable.is_assignable = true;
        assert!(resolve_call(&entry, &call(vec![literal, assignable])).is_ok());
    }

    #[test]
    fn optional_and_variadic_parameters_expand_to_supplied_arity() {
        const OPTIONAL: &[ParameterSpec] = &[
            ParameterSpec {
                cardinality: ParameterCardinality::Required,
                mode: ParameterMode::Value,
                type_constraint: INTEGER,
            },
            ParameterSpec {
                cardinality: ParameterCardinality::Optional,
                mode: ParameterMode::Value,
                type_constraint: TypeConstraint::Exact(CatalogTypeRef::Boolean),
            },
        ];
        const OPTIONAL_SIGNATURES: &[InternalSignature] = &[InternalSignature {
            parameters: OPTIONAL,
            result: ResultSpec::None,
        }];
        let optional = descriptor(CallContext::ExpressionOnly, OPTIONAL_SIGNATURES);
        assert!(resolve_call(&optional, &call(vec![argument(CatalogTypeRef::Integer)])).is_ok());
        assert!(
            resolve_call(
                &optional,
                &call(vec![
                    argument(CatalogTypeRef::Integer),
                    argument(CatalogTypeRef::Boolean)
                ])
            )
            .is_ok()
        );

        const VARIADIC: &[ParameterSpec] = &[ParameterSpec {
            cardinality: ParameterCardinality::Variadic,
            mode: ParameterMode::Value,
            type_constraint: INTEGER,
        }];
        const VARIADIC_SIGNATURES: &[InternalSignature] = &[InternalSignature {
            parameters: VARIADIC,
            result: ResultSpec::None,
        }];
        let variadic = descriptor(CallContext::ExpressionOnly, VARIADIC_SIGNATURES);
        assert!(resolve_call(&variadic, &call(Vec::new())).is_ok());
        assert!(
            resolve_call(
                &variadic,
                &call(vec![
                    argument(CatalogTypeRef::Integer),
                    argument(CatalogTypeRef::Integer)
                ])
            )
            .is_ok()
        );
    }

    #[test]
    fn shared_type_variables_require_equal_types_and_resolve_results() {
        const PARAMETERS: &[ParameterSpec] = &[
            ParameterSpec {
                cardinality: ParameterCardinality::Required,
                mode: ParameterMode::Value,
                type_constraint: T0_NUMERIC,
            },
            ParameterSpec {
                cardinality: ParameterCardinality::Required,
                mode: ParameterMode::Value,
                type_constraint: T0_NUMERIC,
            },
        ];
        const SIGNATURES: &[InternalSignature] = &[InternalSignature {
            parameters: PARAMETERS,
            result: ResultSpec::TypeVariable(TypeVariableId(0)),
        }];
        let entry = descriptor(CallContext::ExpressionOnly, SIGNATURES);
        let resolved = resolve_call(
            &entry,
            &call(vec![
                argument(CatalogTypeRef::Real),
                argument(CatalogTypeRef::Real),
            ]),
        )
        .expect("matching type variables should resolve");
        assert_eq!(resolved.result, Some(CatalogTypeRef::Real.resolved()));
        assert!(matches!(
            resolve_call(
                &entry,
                &call(vec![
                    argument(CatalogTypeRef::Real),
                    argument(CatalogTypeRef::Integer)
                ])
            )
            .expect_err("different bindings should fail")
            .kind,
            InternalCallErrorKind::ArgumentTypeMismatch { position: 2, .. }
        ));
    }

    #[test]
    fn argument_derived_results_apply_named_transforms() {
        const PARAMETERS: &[ParameterSpec] = &[ParameterSpec {
            cardinality: ParameterCardinality::Required,
            mode: ParameterMode::Value,
            type_constraint: NUMERIC,
        }];
        const SIGNATURES: &[InternalSignature] = &[InternalSignature {
            parameters: PARAMETERS,
            result: ResultSpec::FromArgument {
                index: 0,
                transform: TypeTransform::ToLongReal,
            },
        }];
        let entry = descriptor(CallContext::ExpressionOnly, SIGNATURES);
        let resolved = resolve_call(&entry, &call(vec![argument(CatalogTypeRef::Integer)]))
            .expect("derived result should resolve");
        assert_eq!(resolved.result, Some(CatalogTypeRef::LongReal.resolved()));
    }

    #[test]
    fn overlapping_signatures_report_ambiguity() {
        const PARAMETERS: &[ParameterSpec] = &[ParameterSpec {
            cardinality: ParameterCardinality::Required,
            mode: ParameterMode::Value,
            type_constraint: INTEGER,
        }];
        const SIGNATURES: &[InternalSignature] = &[
            InternalSignature {
                parameters: PARAMETERS,
                result: ResultSpec::None,
            },
            InternalSignature {
                parameters: PARAMETERS,
                result: ResultSpec::None,
            },
        ];
        let entry = descriptor(CallContext::ExpressionOnly, SIGNATURES);
        let error = resolve_call(&entry, &call(vec![argument(CatalogTypeRef::Integer)]))
            .expect_err("overlapping signatures should be ambiguous");
        assert_eq!(error.kind, InternalCallErrorKind::AmbiguousSignature);
    }
}
