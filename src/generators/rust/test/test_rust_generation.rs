use std::fs;
use std::path::Path;

use crate::core::generate::Generate;
use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::generators::rust::oml_rust::RustGenerator;

const TEST_RESULTS_DIR: &str = "test_results";

fn ensure_test_results_dir() {
    fs::create_dir_all(TEST_RESULTS_DIR).expect("Failed to create test_results directory");
}

fn generate_and_write(oml_path: &str, file_name: &str) -> String {
    ensure_test_results_dir();

    let generator = RustGenerator;

    let path = Path::new(oml_path);
    let oml_objects = OmlObject::get_from_file(path)
        .expect(&format!("Failed to parse OML file: {}", oml_path));

    let rs_output = generator.generate(&oml_objects, file_name)
        .expect(&format!("Failed to generate Rust for: {}", file_name));

    let output_path = format!("{}/{}.{}", TEST_RESULTS_DIR, file_name, generator.extension());
    fs::write(&output_path, &rs_output)
        .expect(&format!("Failed to write output file: {}", output_path));

    rs_output
}

#[test]
fn test_person_class_generates_rs_file() {
    let output = generate_and_write(
        "src/generators/rust/test_oml_files/person.oml",
        "Person",
    );

    // Header comment
    assert!(output.contains("// This file has been generated from Person.oml"));

    // Derives and struct declaration
    assert!(output.contains("#[derive(Debug, Clone)]"));
    assert!(output.contains("pub struct Person {"));

    // Fields — person.oml has no explicit visibility so fields default to private (no pub)
    assert!(output.contains("name: String,"));
    assert!(output.contains("age: i32,"));
    // optional field wraps in Option<T>
    assert!(output.contains("nickname: Option<String>,"));
}

#[test]
fn test_point_struct_generates_rs_file() {
    let output = generate_and_write(
        "src/generators/rust/test_oml_files/point.oml",
        "Point",
    );

    assert!(output.contains("pub struct Point {"));

    // Public fields
    assert!(output.contains("\tpub x: f64,"));
    assert!(output.contains("\tpub y: f64,"));
}

#[test]
fn test_color_enum_generates_rs_file() {
    let output = generate_and_write(
        "src/generators/rust/test_oml_files/color.oml",
        "Color",
    );

    assert!(output.contains("#[derive(Debug, Clone, PartialEq)]"));
    assert!(output.contains("pub enum Color {"));
    assert!(output.contains("\tRed,"));
    assert!(output.contains("\tGreen,"));
    assert!(output.contains("\tBlue,"));
    assert!(output.contains("\tYellow,"));
}

#[test]
fn test_game_entity_arrays_generate_rs_file() {
    let output = generate_and_write(
        "src/generators/rust/test_oml_files/game_entity.oml",
        "GameEntity",
    );

    assert!(output.contains("pub struct GameEntity {"));

    // Static array: float[3] => [f32; 3]
    assert!(output.contains("position: [f32; 3],"));

    // Dynamic list: list string => Vec<String>
    assert!(output.contains("tags: Vec<String>,"));

    // Scalar fields
    assert!(output.contains("name: String,"));
    assert!(output.contains("health: i32,"));
    assert!(output.contains("active: bool,"));
}

// ── Inline unit tests ──────────────────────────────────────────────────────────

#[test]
fn test_enum_capitalises_variant_names() {
    let oml_object = OmlObject {
        oml_type: ObjectType::ENUM,
        name: "Direction".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), array_kind: ArrayKind::None, name: "north".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), array_kind: ArrayKind::None, name: "south".to_string() },
        ],
    };

    let output = RustGenerator.generate(std::slice::from_ref(&oml_object), "Direction").unwrap();
    assert!(output.contains("\tNorth,"));
    assert!(output.contains("\tSouth,"));
}

#[test]
fn test_optional_field_wraps_in_option() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "User".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "name".to_string() },
            Variable { var_mod: vec![VariableModifier::OPTIONAL], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "email".to_string() },
        ],
    };

    let output = RustGenerator.generate(std::slice::from_ref(&oml_object), "User").unwrap();
    assert!(output.contains("\tpub name: String,"));
    assert!(output.contains("\tpub email: Option<String>,"));
}

#[test]
fn test_protected_visibility_maps_to_pub_crate() {
    let oml_object = OmlObject {
        oml_type: ObjectType::STRUCT,
        name: "Foo".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PROTECTED, var_type: "int32".to_string(), array_kind: ArrayKind::None, name: "value".to_string() },
        ],
    };

    let output = RustGenerator.generate(std::slice::from_ref(&oml_object), "Foo").unwrap();
    assert!(output.contains("\tpub(crate) value: i32,"));
}

#[test]
fn test_static_const_generates_impl_block_with_associated_const() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Config".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PRIVATE, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "name".to_string() },
            Variable { var_mod: vec![VariableModifier::STATIC, VariableModifier::CONST], visibility: VariableVisibility::PUBLIC, var_type: "int32".to_string(), array_kind: ArrayKind::None, name: "max".to_string() },
        ],
    };

    let output = RustGenerator.generate(std::slice::from_ref(&oml_object), "Config").unwrap();
    assert!(output.contains("impl Config {"));
    assert!(output.contains("pub const MAX: i32"));
    // Static field must NOT appear inside the struct body
    assert!(!output.contains("\tpub max:"));
}

#[test]
fn test_static_array_generates_fixed_size_array_type() {
    let oml_object = OmlObject {
        oml_type: ObjectType::STRUCT,
        name: "Matrix".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "float".to_string(), array_kind: ArrayKind::Static(4), name: "data".to_string() },
        ],
    };

    let output = RustGenerator.generate(std::slice::from_ref(&oml_object), "Matrix").unwrap();
    assert!(output.contains("\tpub data: [f32; 4],"));
}

#[test]
fn test_dynamic_list_generates_vec() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Container".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::Dynamic, name: "tags".to_string() },
        ],
    };

    let output = RustGenerator.generate(std::slice::from_ref(&oml_object), "Container").unwrap();
    assert!(output.contains("\tpub tags: Vec<String>,"));
}

#[test]
fn test_all_builtin_types_convert_to_rs() {
    let pairs: Vec<(&str, &str)> = vec![
        ("int8", "i8"), ("int16", "i16"), ("int32", "i32"), ("int64", "i64"),
        ("uint8", "u8"), ("uint16", "u16"), ("uint32", "u32"), ("uint64", "u64"),
        ("float", "f32"), ("double", "f64"),
        ("bool", "bool"), ("string", "String"), ("char", "char"),
    ];

    let variables: Vec<Variable> = pairs.iter().enumerate().map(|(i, (oml_type, _))| Variable {
        var_mod: vec![],
        visibility: VariableVisibility::PUBLIC,
        var_type: oml_type.to_string(),
        array_kind: ArrayKind::None,
        name: format!("field_{}", i),
    }).collect();

    let oml_object = OmlObject { oml_type: ObjectType::STRUCT, name: "AllTypes".to_string(), variables };
    let output = RustGenerator.generate(std::slice::from_ref(&oml_object), "AllTypes").unwrap();

    for (i, (_, expected)) in pairs.iter().enumerate() {
        let expected_field = format!("field_{}: {},", i, expected);
        assert!(output.contains(&expected_field), "Missing: {}", expected_field);
    }
}

#[test]
fn test_undecided_object_type_returns_error() {
    let oml_object = OmlObject { oml_type: ObjectType::UNDECIDED, name: "Bad".to_string(), variables: vec![] };
    assert!(RustGenerator.generate(std::slice::from_ref(&oml_object), "Bad").is_err());
}

#[test]
fn test_extension_is_rs() {
    assert_eq!(RustGenerator.extension(), "rs");
}
