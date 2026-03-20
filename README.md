<!--
Suggested GitHub Topics: rust code-generation transpiler oml developer-tools
-->

# OML — Object Markup Language

> This collaborative project is actively under development.

OML is a language-agnostic class definition format and transpiler. Write a data structure or class prototype once in OML, then generate idiomatic implementations in Python, Rust, TypeScript, Java, Kotlin, C++, or SQL — automatically.

## What is OML
1
Modern software projects span multiple languages and services. Keeping data models in sync across a Python ML service, a TypeScript frontend, a Rust backend, and a SQL schema is tedious and error-prone. OML solves this by providing a single source of truth: write your class/struct definition once, and generate all the language-specific implementations from it.

**Supported output languages:**
- Python
- Rust
- TypeScript
- Java
- Kotlin
- C++
- SQL (table schema)

## Status

This project is actively under development.

## Usage / Example

Given an OML input file (e.g., `car.oml`):

```oml
import "engine.oml";

class Car {
    public string name;
    public Engine engine;
}
```

OML supports richer types too — fixed-size arrays, dynamic lists, optional fields, and cross-file imports. For example, `student.oml`:

```oml
// Academic records — static arrays for fixed-size grade slots,
// dynamic lists for enrolled courses and notes.

class Student {
    public string       name;
    public uint32       id;
    public float[5]     grades;         // up to 5 exam scores
    public list string  courses;        // enrolled course codes
    public list string  notes;          // free-form instructor notes
    public optional string advisor;
}

class Classroom {
    public string       room_code;
    public uint16[7]    capacity_by_day; // seats available Mon–Sun
    public list Student students;
}
```

Running the OML transpiler generates idiomatic code for each target language. For example, the Python output would be a dataclass/class, the Rust output a struct with derives, the TypeScript output an interface or class, and the SQL output a CREATE TABLE statement.

### CLI Usage

```bash
# Build first (see Building section)
./target/release/oml <input.oml> --lang python
./target/release/oml <input.oml> --lang rust
./target/release/oml <input.oml> --lang typescript
```

## Building

Requires Rust and Cargo (install via [rustup](https://rustup.rs/)).

```bash
git clone <repo-url>
cd OML
cargo build --release
```

Run tests:

```bash
cargo test
```

## Contributors

OML is a joint project:

- **Nikolay Tsonev** 
- **Bilal Waraich** 
