# AiPlan4Rust

AiPlan4Rust is a Rust-based compiler and analysis framework for PDDL (and HDDL) planning domains and problems. It parses domain and problem files, performs semantic analysis, links them into a unified planning task, and provides a CLI front‑end, diagnostics, and visualization.

---

## Features

- **Syntax**  
  - Lexer & LALRPOP parser  
  - Abstract Syntax Tree (AST) representation  
  - Flexible grammar elements (operators, requirements, types)

- **Semantic**  
  - High‑level Intermediate Representation (HIR) arena  
  - Symbol table with `SymbolSource` tracking (Domain vs. Problem)  
  - Modular “checks” suite (undeclared symbols, type hierarchy, requirement violations, etc.)  
  - Type normalization and inference

- **Linking**  
  - Domain ↔ Problem consistency checks  
  - Builds a `LiftedPlanningTask` ready for solving  
  - Phase‑specific checks

- **Diagnostics**  
  - Centralized `DiagnosticManager` & `DiagnosticProvider`  
  - Severity levels (Error, Warning, Info)  
  - Pluggable renderers (console, structured output)

- **CLI**  
  - `aiplan` command‑line tool  
  - Supports parsing, analysis, linking, and PDDL pretty‑printing

---

## Project Structure

```text
aiplan4rust/
├── cli/                 # Command‑line interface
├── diagnostic/          # Error & warning management
│   ├── kind.rs          # enum Kind → DiagnosticKind
│   ├── provider.rs      # DiagnosticProvider trait
│   ├── renderer.rs      # Console & JSON renderers
│   ├── severity.rs      # enum Severity
│   └── mod.rs
├── linking/             # Domain ↔ Problem linking & checks
│   ├── checks/          # Linking-specific semantic checks
│   ├── linker.rs        # Linker implementation
│   ├── linker_result.rs
│   ├── lifted_planning_task.rs
│   └── mod.rs
├── semantic/            # Semantic analysis & HIR
│   ├── analyzer.rs      # Orchestrates AST → HIR → checks
│   ├── analyzer_result.rs
│   ├── checks/          # General semantic checks
│   ├── hir/             # HIR arena & nodes
│   │   ├── arena.rs     # `HirArena`, `HirNodeId`
│   │   ├── node.rs      # `HirNode`
│   │   ├── kind.rs      # `HirKind`
│   │   └── mod.rs
│   ├── normalization/   # Type normalization passes
│   ├── symbol/          # Symbols, scopes, origins
│   ├── symbol_table/    # `SymbolTable` & builder
│   ├── type_checker.rs
│   └── mod.rs
├── syntax/              # Front‑end: lexer, parser, AST
│   ├── lexer/           # `token.rs`, `lexer.rs`
│   ├── elements/        # Grammar elements
│   ├── parser.rs        # LALRPOP parser wrapper
│   ├── parser_result.rs
│   ├── span.rs
│   ├── ast/             # `ast.rs`, `ast_node.rs`, `ast_kind.rs`
│   └── mod.rs
├── file_format.rs
├── frontend.rs          # High‑level orchestration
├── pddl_display.rs      # PDDL pretty‑printing utilities
├── mod.rs
└── bin/                 # Example binaries (e.g. `aiplan.rs`)

# Quickstart Commands

Below is a consolidated list of all the commands you’ll need to install dependencies, build, test, run, and profile **AiPlan4Rust** on macOS.

```bash
# 1. Update Rust toolchain
rustup update stable

# 2. Clone the repository and build in release mode
git clone https://github.com/yourorg/aiplan4rust.git
cd aiplan4rust
cargo build --release

# 3. Install optional tools
cargo install lalrpop           # for regenerating grammar if needed
cargo install flamegraph         # for profiling with cargo-flamegraph

# 4. Run the CLI on a PDDL domain/problem
cargo run --release -- \
  --domain path/to/domain.pddl \
  --problem path/to/problem.pddl \
  --output task.json

# 5. Run all tests (unit + integration)
cargo test

# 6. Profile integration tests on macOS (requires sudo)
sudo RUST_BACKTRACE=1 RUSTFLAGS="-g" \
  cargo flamegraph --dtrace --root / -- \
    test --test integration_tests --release

# 7. View the generated flamegraph
open flamegraph.svg   # or open with your browser of choice
