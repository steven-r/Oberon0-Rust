#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod ast;
pub mod codegen;
pub mod hir;
pub mod lower;
pub mod manifest;
pub mod parser;
pub mod scanner;
pub mod scope;
pub mod semantic;
pub mod symbols;
pub mod expression_constant_handler;
