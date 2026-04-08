// ─────────────────────────────────────────────────────────────
// ANWE v0.1 — PARSER
// Lexer, AST, and recursive descent parser for .anwe files.
//
// Reads Anwe source text → Tokens → Abstract Syntax Tree.
// The AST represents the program as the runtime sees it.
// ─────────────────────────────────────────────────────────────

pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;

pub use lexer::Lexer;
pub use parser::Parser;
pub use ast::Program;
