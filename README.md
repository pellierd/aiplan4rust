# AiPlan4Rust

**AiPlan4Rust** is a Rust-based compiler and analysis framework for **PDDL and HDDL** planning languages.

It provides a complete front-end pipeline for planning domains and problems:
parsing, semantic analysis, normalization, validation, and domain–problem linking,
with a structured intermediate representation and a command-line interface.

> ⚠️ AiPlan4Rust is **not a planner/solver**.  
> Its goal is to *analyze, validate, normalize, and serialize planning models* in a robust and extensible way.

---

## Key Capabilities

### Language Support
- PDDL (classical & extended constructs)
- HDDL (hierarchical task networks)
- Mixed domain / problem inputs

### Front-end & Syntax
- Lexer and parser generated with **LALRPOP**
- Rich concrete and abstract syntax trees
- Source spans and structured syntax diagnostics
- Tree renderers (default, syntax-oriented)

### Semantic Analysis
- Symbol interning and scoping
- Domain vs. problem symbol origins
- Type system with hierarchy checks
- Requirement validation
- Detection of:
  - undeclared symbols
  - unused symbols
  - invalid signatures
  - requirement violations
  - invalid task ordering (HTN)

### Normalization & Validation
- Modular normalization passes
- Typed list normalization
- Requirement-driven normalization
- Expression rewriting and simplification
- Validation layers:
  - syntax
  - semantic
  - normalization invariants

### Linking & Intermediate Representation
- Domain ↔ Problem consistency checks
- Construction of a **Linked / Lifted Intermediate Representation (LIR)**
- HTN task networks and methods
- Ready-to-serialize planning task model

### Serialization
- Multiple output formats:
  - `json`
  - `yaml`
  - `toml`
  - `cbor`
  - `messagepack`
- Structured artefact model with headers, metadata, and content
- Stable format abstraction independent of CLI

### Diagnostics
- Centralized diagnostic system
- Severity levels (error, warning, info)
- Suggestions and formatted messages
- CLI-oriented rendering

---

## Command-Line Interface

The `aiplan` CLI provides three main commands: `parse`, `link`, and `help`.

### Commands
- `parse` — Parse one or more PDDL/HDDL files and emit serialized artefacts
- `link` — Combine a domain and problem into a linked planning task
- `help` — Print help information

---

## Project Structure

```text
aiplan4rust/
├── artefact/        # Serialized artefact model (raw + IR)
├── cli/             # CLI commands and handlers
├── core/            # Generic arena and node infrastructure
├── diagnostic/      # Diagnostics, renderers, severities
├── interner/        # Symbol interning and identifiers
├── lang/            # Language-level definitions (types, requirements)
├── linking/         # Domain–problem linking
├── lir/             # Linked Intermediate Representation
├── normalization/   # Normalization passes
├── semantic/        # Semantic analysis and symbol tables
├── serialization/   # Output formats and serde support
├── syntax/          # Lexer, parser, AST, syntax trees
├── validation/      # Syntax, semantic, and normalization validators
├── frontend.rs      # High-level orchestration
├── bin/aiplan.rs    # CLI entry point
└── lib.rs
```

## Quickstart Commands

Below is a consolidated list of all the commands you’ll need to install dependencies, build, test, run, and profile **AiPlan4Rust** on macOS.

### 1. Update Rust toolchain
```bash
rustup update stable
```

### 2. Clone the repository and build in release mode
```bash
git clone https://github.com/yourorg/aiplan4rust.git
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
