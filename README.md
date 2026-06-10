# AiPlan4Rust

**AiPlan4Rust** is a Rust-based compiler, analysis, and grounding framework for **PDDL and HDDL** planning languages.

It provides a complete industrial-grade front-end pipeline for planning domains and problems:
parsing, semantic analysis, normalization, validation, domain–problem linking, and reachability analysis through
grounding,
with a structured intermediate representation and a powerful command-line interface.

> ⚠️ AiPlan4Rust is **not a planner/solver**.  
> Its goal is to *analyze, validate, normalize, ground, and serialize planning models* in a robust and extensible way
> for downstream planners or execution systems.

![Tests](https://github.com/pellierd/aiplan4rust/actions/workflows/tests.yml/badge.svg)
![Quality](https://github.com/pellierd/aiplan4rust/actions/workflows/quality.yml/badge.svg)
![Security](https://github.com/pellierd/aiplan4rust/actions/workflows/security.yml/badge.svg)
![Docs](https://github.com/pellierd/aiplan4rust/actions/workflows/deploy-docs.yml/badge.svg)

---

## Key Capabilities

### Language Support

- PDDL (classical & extended constructs)
- HDDL (hierarchical task networks)
- Mixed domain / problem inputs

### Front-end & Syntax

- Lexer and parser generated with **LALRPOP**
- Memory-efficient Concrete and Abstract Syntax Trees backed by an **Arena allocator**
- Source spans and structured syntax diagnostics
- Tree renderers (default, syntax-oriented)

### Semantic Analysis

- Symbol interning and scoping
- Domain vs. problem symbol origins
- Strict type system with hierarchy checks
- Requirement extraction and validation
- Comprehensive static analysis (detection of undeclared/unused symbols, invalid signatures, requirement violations, and
  invalid HTN task orderings)

### Normalization & Validation

- Modular normalization passes (typed lists, require definitions, `either` type resolution)
- Expression rewriting and simplification
- Validation layers ensuring syntax, semantic, and normalization invariants

### Linking & Lifted Intermediate Representation (LIR)

- Domain ↔ Problem consistency checks
- Construction of a typed, optimized **Lifted Intermediate Representation (LIR)**
- Deep formula transformations (NNF, TNF, FNF tree-walking operations)

### Advanced Grounding Engine 🚧 *(In Progress)*

- State-of-the-art reachability analysis driven by a custom **Datalog engine**
- High-performance evaluator for inertia and predicate analysis
- Grounding passes optimizing expressions into **PNF** (*Prenex Normal Form*) and **QNF** (*Quantifier Normal Form*)
- Fast fluent registry and value range tracking

### Serialization

- Multiple output formats: `json`, `yaml`, `toml`, `cbor`, `messagepack`
- Structured artifact model with headers, metadata, and content
- Stable format abstraction independent of CLI

---

## Command-Line Interface

The `aiplan` CLI provides three main commands: `parse`, `link`, and `ground`.

### Commands

- `parse`  — Parse PDDL/HDDL files and emit raw or syntax-serialized artifacts
- `link`   — Combine a domain and problem into a unified LIR planning task
- `ground` — Perform reachability analysis and emit a fully grounded planning problem

---

## Project Structure

The project follows a clean, decoupled architecture separating the binary layer, the core compiler stages, and global
utilities:

```text
src/
├── bin/                 # CLI executable entry points
├── lib.rs               # Library root interface
└── aiplan4rust/         # Main framework core
    ├── cli/             # CLI app definitions, error handling, and serialization I/O
    │   └── commands/    # Subcommands mapping: parse, link, ground
    ├── compiler/        # The compiler pipeline stages
    │   ├── syntax/      # LALRPOP grammar, Lexer, Parser, and Arena AST
    │   ├── semantic/    # Symbol tables, Type checker, and Pass-based analyzers
    │   ├── linking/     # Cross-declaration checks between Domain and Problem
    │   ├── normalization/# Global restructuring and validation passes
    │   ├── lir/         # Lifted IR storage, trees, and formula normalizers (NNF, TNF)
    │   └── grounding/   # Datalog engine, reachability analysis, and PNF/QNF passes
    └── support/         # Shared workspace-wide utilities
        ├── diagnostic/  # Compiler diagnostics engine and layout renderers
        ├── interner/    # High-performance string interning engine
        └── lang/        # Language primitives (operators, basic types, requirements)

## Quickstart Commands

Below is a consolidated list of all the commands you’ll need to install dependencies, build, test, run, and profile *
*AiPlan4Rust** on macOS.

### 1. Update Rust toolchain

```bash
rustup update stable
```

### 2. Clone the repository and build in release mode

```bash
git clone https://github.com/pellierd/aiplan4rust
cd aiplan4rust
cargo build --release
```

### 3. Install optional tools

```bash
cargo install lalrpop           # for regenerating grammar if needed
cargo install flamegraph         # for profiling with cargo-flamegraph
```

### 4. Run the CLI on a PDDL domain/problem

## Parse one or more PDDL/HDDL files

```bash
cargo run --release -- parse path/to/domain.pddl path/to/problem.pddl

### Link a Domain and Problem into a Unified Artefact

```bash
cargo run --release -- link path/to/domain.pddl path/to/problem.pddl

### 5. Run all tests (unit + integration)
```bash
cargo test
```

### 6. Profile a specific integration test with performance analysis (requires sudo)

```bash
sudo cargo flamegraph --root --test frontend_parser_integration_tests```

### 7. View the generated flamegraph
```bash
open flamegraph.svg   # or open with your browser of choice
```
