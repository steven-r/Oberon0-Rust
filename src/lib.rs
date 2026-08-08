#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod ast;
pub mod codegen;
pub mod expression_constant_handler;
pub mod hir;
pub mod internal_functions;
pub mod lower;
pub mod manifest;
pub mod parser;
pub mod scanner;
pub mod scope;
pub mod semantic;
pub mod symbols;
pub mod types;
