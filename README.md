# qexpr

Typed query expressions (query algebra) for retrieval systems.

This crate is intentionally **not** a parser. Parsing is product-specific. The goal here is a
small, stable AST for query meaning that multiple systems can compile into their own execution plans.

## Usage

```toml
[dependencies]
qexpr = "0.1.0"
```

Example:

```rust
use qexpr::{Near, Phrase, QExpr, Term};

let q = QExpr::And(vec![
    QExpr::Term(Term::new("alpha")),
    QExpr::Phrase(Phrase::new(vec![Term::new("new"), Term::new("york")])),
    QExpr::Near(Near::new(vec![Term::new("deep"), Term::new("learning")], 5, false)),
]);

qexpr::validate(&q).unwrap();
```

## Development

```bash
cargo test
```
