//! `qexpr`: typed query expressions (query algebra).
//!
//! Goal: provide a small, stable AST for common retrieval query operators without
//! committing to any particular index backend or scoring model.
//!
//! This is intentionally **not** a parser. Parsing (syntax) is product-specific.
//! This crate is about a shared, typed *meaning* that multiple systems can compile
//! down to their preferred execution plan.

#![warn(missing_docs)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A query expression for retrieval.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QExpr {
    /// A single term.
    Term(Term),
    /// A phrase (ordered sequence of terms).
    ///
    /// Semantics require positional information in the target index to evaluate exactly.
    Phrase(Phrase),
    /// Proximity query: terms must occur within a window.
    ///
    /// This is the semantic payload behind operators like `NEAR/k`.
    /// Evaluation requires positional information in the target index (or a verifier stage).
    Near(Near),
    /// Conjunction: all children must match.
    And(Vec<QExpr>),
    /// Disjunction: any child may match.
    Or(Vec<QExpr>),
    /// Negation: exclude matches of inner expression.
    Not(Box<QExpr>),
    /// Field scoping (e.g. `title:term`).
    ///
    /// Evaluation requires field-aware indexing (or a compiler that rewrites into field-specific terms).
    Field(FieldName, Box<QExpr>),
}

/// A normalized term token.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Term(pub String);

impl Term {
    /// Create a term (caller is responsible for normalization/tokenization policy).
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns true if the term is empty or whitespace.
    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }
}

/// A phrase of ordered terms.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Phrase {
    /// Ordered terms.
    pub terms: Vec<Term>,
}

impl Phrase {
    /// Create a phrase.
    pub fn new(terms: Vec<Term>) -> Self {
        Self { terms }
    }

    /// Returns true if the phrase has no terms (or all terms are blank).
    pub fn is_blank(&self) -> bool {
        self.terms.is_empty() || self.terms.iter().all(|t| t.is_blank())
    }
}

/// A proximity query over ordered terms.
///
/// This represents constraints like “the terms occur within `window` tokens”.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Near {
    /// Terms participating in the proximity constraint.
    ///
    /// Must have length >= 2.
    pub terms: Vec<Term>,
    /// Window size in tokens.
    ///
    /// Interpretation: there exists an assignment of positions (one per term occurrence)
    /// such that `max(pos) - min(pos) <= window`.
    pub window: u32,
    /// If true, enforce term order (like an ordered NEAR / “WITHIN k in order”).
    ///
    /// If false, order is ignored (unordered NEAR/k).
    pub ordered: bool,
}

impl Near {
    /// Create a proximity query.
    pub fn new(terms: Vec<Term>, window: u32, ordered: bool) -> Self {
        Self {
            terms,
            window,
            ordered,
        }
    }

    /// Returns true if the constraint is structurally blank/invalid.
    pub fn is_blank(&self) -> bool {
        self.terms.len() < 2 || self.terms.iter().all(|t| t.is_blank()) || self.window == 0
    }
}

/// A field name.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldName(pub String);

impl FieldName {
    /// Create a field name.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns true if the field name is empty or whitespace.
    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }
}

/// Structural validation errors for `QExpr`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateError {
    /// A `Term` node contained a blank term.
    BlankTerm,
    /// A `Phrase` node contained no usable terms.
    BlankPhrase,
    /// A `Near` node contained fewer than 2 usable terms or an invalid window.
    BlankNear,
    /// An `And`/`Or` node had no children.
    EmptyJunction,
    /// A `Field` node had a blank field name.
    BlankFieldName,
}

/// Validate a query expression for basic structural invariants.
///
/// This does **not** attempt semantic checks like "is phrase supported by the target index".
pub fn validate(expr: &QExpr) -> Result<(), ValidateError> {
    match expr {
        QExpr::Term(t) => {
            if t.is_blank() {
                Err(ValidateError::BlankTerm)
            } else {
                Ok(())
            }
        }
        QExpr::Phrase(p) => {
            if p.is_blank() {
                Err(ValidateError::BlankPhrase)
            } else {
                Ok(())
            }
        }
        QExpr::Near(n) => {
            if n.is_blank() {
                Err(ValidateError::BlankNear)
            } else {
                Ok(())
            }
        }
        QExpr::And(xs) | QExpr::Or(xs) => {
            if xs.is_empty() {
                return Err(ValidateError::EmptyJunction);
            }
            for x in xs {
                validate(x)?;
            }
            Ok(())
        }
        QExpr::Not(x) => validate(x),
        QExpr::Field(name, inner) => {
            if name.is_blank() {
                return Err(ValidateError::BlankFieldName);
            }
            validate(inner)
        }
    }
}
