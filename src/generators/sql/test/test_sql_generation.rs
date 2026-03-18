use std::fs;
use std::path::Path;

use crate::core::generate::Generate;
use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::generators::sql::oml_sql::SqlGenerator;

const TEST_RESULTS_DIR: &str = "test_results";

fn ensure_test_results_dir() {
    fs::create_dir_all(TEST_RESULTS_DIR).expect("Failed to create test_results directory");
}

fn generate_and_write(oml_path: &str, file_name: &str) -> String {
    ensure_test_results_dir();

    let generator = SqlGenerator;

    let path = Path::new(oml_path);
    let (oml_objects, _imports) = OmlObject::get_from_file(path)
        .expect(&format!("Failed to parse OML file: {}", oml_path));

    let sql_output = generator.generate(&oml_objects, file_name)
        .expect(&format!("Failed to generate SQL for: {}", file_name));

    let output_path = format!("{}/{}.{}", TEST_RESULTS_DIR, file_name, generator.extension());
    fs::write(&output_path, &sql_output)
        .expect(&format!("Failed to write output file: {}", output_path));

    sql_output
}

#[test]
fn test_person_class_generates_sql_file() {
    let output = generate_and_write(
        "src/generators/sql/test_oml_files/person.oml",
        "Person",
    );

    // Header comment
    assert!(output.contains("-- This file has been generated from Person.oml"));

    // Table declaration with auto-increment primary key
    assert!(output.contains("CREATE TABLE Person ("));
    assert!(output.contains("id INT NOT NULL AUTO_INCREMENT PRIMARY KEY"));

    // Required fields are NOT NULL
    assert!(output.contains("name TEXT NOT NULL"));
    assert!(output.contains("age INT NOT NULL"));

    // Optional field allows NULL
    assert!(output.contains("nickname TEXT NULL"));
}

#[test]
fn test_point_struct_generates_sql_file() {
    let output = generate_and_write(
        "src/generators/sql/test_oml_files/point.oml",
        "Point",
    );

    assert!(output.contains("CREATE TABLE Point ("));
    assert!(output.contains("x DOUBLE NOT NULL"));
    assert!(output.contains("y DOUBLE NOT NULL"));
}

#[test]
fn test_color_enum_generates_lookup_table() {
    let output = generate_and_write(
        "src/generators/sql/test_oml_files/color.oml",
        "Color",
    );

    // Lookup table structure
    assert!(output.contains("CREATE TABLE Color ("));
    assert!(output.contains("id   INT          NOT NULL AUTO_INCREMENT PRIMARY KEY"));
    assert!(output.contains("name VARCHAR(255) NOT NULL"));

    // All enum values inserted
    assert!(output.contains("INSERT INTO Color (name) VALUES ('RED'), ('GREEN'), ('BLUE'), ('YELLOW');"));
}

#[test]
fn test_game_entity_generates_junction_table_for_list() {
    let output = generate_and_write(
        "src/generators/sql/test_oml_files/game_entity.oml",
        "GameEntity",
    );

    assert!(output.contains("CREATE TABLE GameEntity ("));

    // Static array float[3] expands to three columns
    assert!(output.contains("position_0 FLOAT NOT NULL"));
    assert!(output.contains("position_1 FLOAT NOT NULL"));
    assert!(output.contains("position_2 FLOAT NOT NULL"));

    // Dynamic list creates a junction table — no inline `tags` column
    assert!(!output.contains("\ttags "));
    assert!(output.contains("CREATE TABLE GameEntity_tags ("));
    assert!(output.contains("parent_id  INT NOT NULL"));
    assert!(output.contains("value      TEXT NOT NULL"));
    assert!(output.contains("FOREIGN KEY (parent_id) REFERENCES GameEntity(id)"));

    // Scalar fields
    assert!(output.contains("name TEXT NOT NULL"));
    assert!(output.contains("health INT NOT NULL"));
    assert!(output.contains("active BOOLEAN NOT NULL"));
}

// ── Inline unit tests ──────────────────────────────────────────────────────────

#[test]
fn test_enum_empty_generates_no_insert() {
    let oml_object = OmlObject {
        oml_type: ObjectType::ENUM,
        name: "Empty".to_string(),
        variables: vec![],
    };

    let output = SqlGenerator.generate(std::slice::from_ref(&oml_object), "Empty").unwrap();
    assert!(output.contains("CREATE TABLE Empty ("));
    assert!(!output.contains("INSERT INTO"));
}

#[test]
fn test_optional_field_allows_null() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "User".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "name".to_string() },
            Variable { var_mod: vec![VariableModifier::OPTIONAL], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "email".to_string() },
        ],
    };

    let output = SqlGenerator.generate(std::slice::from_ref(&oml_object), "User").unwrap();
    assert!(output.contains("name TEXT NOT NULL"));
    assert!(output.contains("email TEXT NULL"));
}

#[test]
fn test_static_array_expands_to_n_columns() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Rgb".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "uint8".to_string(), array_kind: ArrayKind::Static(3), name: "color".to_string() },
        ],
    };

    let output = SqlGenerator.generate(std::slice::from_ref(&oml_object), "Rgb").unwrap();
    assert!(output.contains("color_0 TINYINT UNSIGNED NOT NULL"));
    assert!(output.contains("color_1 TINYINT UNSIGNED NOT NULL"));
    assert!(output.contains("color_2 TINYINT UNSIGNED NOT NULL"));
    // Fourth column must NOT exist
    assert!(!output.contains("color_3"));
}

#[test]
fn test_dynamic_list_generates_junction_table_with_fk() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Post".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "title".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::Dynamic, name: "tags".to_string() },
        ],
    };

    let output = SqlGenerator.generate(std::slice::from_ref(&oml_object), "Post").unwrap();
    assert!(!output.contains("\ttags "));
    assert!(output.contains("CREATE TABLE Post_tags ("));
    assert!(output.contains("FOREIGN KEY (parent_id) REFERENCES Post(id)"));
}

#[test]
fn test_all_builtin_types_convert_to_sql() {
    let pairs: Vec<(&str, &str)> = vec![
        ("int8", "TINYINT"), ("int16", "SMALLINT"), ("int32", "INT"), ("int64", "BIGINT"),
        ("uint8", "TINYINT UNSIGNED"), ("uint16", "SMALLINT UNSIGNED"),
        ("uint32", "INT UNSIGNED"), ("uint64", "BIGINT UNSIGNED"),
        ("float", "FLOAT"), ("double", "DOUBLE"),
        ("bool", "BOOLEAN"), ("string", "TEXT"), ("char", "CHAR(1)"),
    ];

    let variables: Vec<Variable> = pairs.iter().enumerate().map(|(i, (oml_type, _))| Variable {
        var_mod: vec![],
        visibility: VariableVisibility::PUBLIC,
        var_type: oml_type.to_string(),
        array_kind: ArrayKind::None,
        name: format!("field_{}", i),
    }).collect();

    let oml_object = OmlObject { oml_type: ObjectType::CLASS, name: "AllTypes".to_string(), variables };
    let output = SqlGenerator.generate(std::slice::from_ref(&oml_object), "AllTypes").unwrap();

    for (i, (_, expected)) in pairs.iter().enumerate() {
        let expected_col = format!("field_{} {} NOT NULL", i, expected);
        assert!(output.contains(&expected_col), "Missing: {}", expected_col);
    }
}

#[test]
fn test_custom_type_maps_to_int_for_fk() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Order".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "Customer".to_string(), array_kind: ArrayKind::None, name: "customer".to_string() },
        ],
    };

    let output = SqlGenerator.generate(std::slice::from_ref(&oml_object), "Order").unwrap();
    // Custom types stored as INT (FK reference placeholder)
    assert!(output.contains("customer INT NOT NULL"));
}

#[test]
fn test_undecided_object_type_returns_error() {
    let oml_object = OmlObject { oml_type: ObjectType::UNDECIDED, name: "Bad".to_string(), variables: vec![] };
    assert!(SqlGenerator.generate(std::slice::from_ref(&oml_object), "Bad").is_err());
}

#[test]
fn test_extension_is_sql() {
    assert_eq!(SqlGenerator.extension(), "sql");
}
