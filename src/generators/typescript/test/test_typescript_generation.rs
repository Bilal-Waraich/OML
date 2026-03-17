use std::fs;
use std::path::Path;

use crate::core::generate::Generate;
use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::generators::typescript::oml_typescript::TypescriptGenerator;

const TEST_RESULTS_DIR: &str = "test_results";

fn ensure_test_results_dir() {
    fs::create_dir_all(TEST_RESULTS_DIR).expect("Failed to create test_results directory");
}

fn generate_and_write(oml_path: &str, file_name: &str) -> String {
    ensure_test_results_dir();

    let generator = TypescriptGenerator;

    let path = Path::new(oml_path);
    let oml_objects = OmlObject::get_from_file(path)
        .expect(&format!("Failed to parse OML file: {}", oml_path));

    let ts_output = generator.generate(&oml_objects, file_name)
        .expect(&format!("Failed to generate TypeScript for: {}", file_name));

    let output_path = format!("{}/{}.{}", TEST_RESULTS_DIR, file_name, generator.extension());
    fs::write(&output_path, &ts_output)
        .expect(&format!("Failed to write output file: {}", output_path));

    ts_output
}

#[test]
fn test_person_class_generates_ts_file() {
    let output = generate_and_write(
        "src/generators/typescript/test_oml_files/person.oml",
        "Person",
    );

    // Header comment
    assert!(output.contains("// This file has been generated from Person.oml"));

    // Class declaration
    assert!(output.contains("export class Person {"));

    // Fields
    assert!(output.contains("name: string;"));
    assert!(output.contains("age: number;"));
    // optional field uses nullable union syntax
    assert!(output.contains("nickname?: string | null;"));

    // Constructor — required params before optional
    assert!(output.contains("constructor("));
    assert!(output.contains("name: string"));
    assert!(output.contains("age: number"));
    let name_pos  = output.find("name: string,").unwrap();
    let nick_pos  = output.find("nickname: string | null = null").unwrap();
    assert!(name_pos < nick_pos, "required params must precede optional params");

    // Assignment in constructor body
    assert!(output.contains("this.name = name;"));
    assert!(output.contains("this.age = age;"));
    assert!(output.contains("this.nickname = nickname;"));
}

#[test]
fn test_point_struct_generates_ts_file() {
    let output = generate_and_write(
        "src/generators/typescript/test_oml_files/point.oml",
        "Point",
    );

    // Structs map to classes in TypeScript
    assert!(output.contains("export class Point {"));
    assert!(output.contains("public x: number;"));
    assert!(output.contains("public y: number;"));

    // Constructor is generated
    assert!(output.contains("constructor("));
    assert!(output.contains("this.x = x;"));
    assert!(output.contains("this.y = y;"));
}

#[test]
fn test_color_enum_generates_ts_file() {
    let output = generate_and_write(
        "src/generators/typescript/test_oml_files/color.oml",
        "Color",
    );

    // Enum declaration
    assert!(output.contains("export enum Color {"));
    assert!(output.contains("RED = \"RED\","));
    assert!(output.contains("GREEN = \"GREEN\","));
    assert!(output.contains("BLUE = \"BLUE\","));
    // Last variant has no trailing comma
    assert!(output.contains("YELLOW = \"YELLOW\""));
    assert!(!output.contains("YELLOW = \"YELLOW\","));

    // Enums should not emit a constructor
    assert!(!output.contains("constructor"));
}

#[test]
fn test_game_entity_arrays_generate_ts_file() {
    let output = generate_and_write(
        "src/generators/typescript/test_oml_files/game_entity.oml",
        "GameEntity",
    );

    assert!(output.contains("export class GameEntity {"));

    // Static array: float[3] => number[] /* [3] */
    assert!(output.contains("position: number[] /* [3] */;"));

    // Dynamic list: list string => string[]
    assert!(output.contains("tags: string[];"));

    // Scalar fields
    assert!(output.contains("name: string;"));
    assert!(output.contains("health: number;"));
    assert!(output.contains("active: boolean;"));
}

// ── Inline unit tests ──────────────────────────────────────────────────────────

#[test]
fn test_enum_single_variant_no_trailing_comma() {
    let oml_object = OmlObject {
        oml_type: ObjectType::ENUM,
        name: "Single".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), array_kind: ArrayKind::None, name: "Only".to_string() },
        ],
    };

    let output = TypescriptGenerator.generate(std::slice::from_ref(&oml_object), "Single").unwrap();
    assert!(output.contains("\tONLY = \"ONLY\""));
    assert!(!output.contains("ONLY = \"ONLY\","));
}

#[test]
fn test_empty_class_no_constructor() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Empty".to_string(),
        variables: vec![],
    };

    let output = TypescriptGenerator.generate(std::slice::from_ref(&oml_object), "Empty").unwrap();
    assert!(output.contains("export class Empty {"));
    assert!(!output.contains("constructor"));
}

#[test]
fn test_const_field_generates_readonly() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Config".to_string(),
        variables: vec![
            Variable { var_mod: vec![VariableModifier::CONST], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "version".to_string() },
        ],
    };

    let output = TypescriptGenerator.generate(std::slice::from_ref(&oml_object), "Config").unwrap();
    assert!(output.contains("public readonly version: string;"));
}

#[test]
fn test_static_field_not_in_constructor() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Counter".to_string(),
        variables: vec![
            Variable { var_mod: vec![VariableModifier::STATIC], visibility: VariableVisibility::PUBLIC, var_type: "int32".to_string(), array_kind: ArrayKind::None, name: "count".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PRIVATE, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "name".to_string() },
        ],
    };

    let output = TypescriptGenerator.generate(std::slice::from_ref(&oml_object), "Counter").unwrap();
    assert!(output.contains("public static count: number;"));
    assert!(!output.contains("this.count"));
}

#[test]
fn test_visibility_modifiers_emitted() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Vis".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC,    var_type: "int32".to_string(), array_kind: ArrayKind::None, name: "pub_val".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PROTECTED, var_type: "int32".to_string(), array_kind: ArrayKind::None, name: "prot_val".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PRIVATE,   var_type: "int32".to_string(), array_kind: ArrayKind::None, name: "priv_val".to_string() },
        ],
    };

    let output = TypescriptGenerator.generate(std::slice::from_ref(&oml_object), "Vis").unwrap();
    assert!(output.contains("public pub_val: number;"));
    assert!(output.contains("protected prot_val: number;"));
    assert!(output.contains("private priv_val: number;"));
}

#[test]
fn test_all_builtin_types_convert_to_ts() {
    let vars: Vec<(&str, &str)> = vec![
        ("int8",   "number"), ("int16",  "number"), ("int32",  "number"), ("int64",  "number"),
        ("uint8",  "number"), ("uint16", "number"), ("uint32", "number"), ("uint64", "number"),
        ("float",  "number"), ("double", "number"),
        ("bool",   "boolean"),
        ("string", "string"), ("char",   "string"),
    ];

    let variables: Vec<Variable> = vars.iter().enumerate().map(|(i, (oml_type, _))| Variable {
        var_mod: vec![],
        visibility: VariableVisibility::PUBLIC,
        var_type: oml_type.to_string(),
        array_kind: ArrayKind::None,
        name: format!("field_{}", i),
    }).collect();

    let oml_object = OmlObject { oml_type: ObjectType::CLASS, name: "AllTypes".to_string(), variables };
    let output = TypescriptGenerator.generate(std::slice::from_ref(&oml_object), "AllTypes").unwrap();

    for (i, (_, expected)) in vars.iter().enumerate() {
        let expected_field = format!("field_{}: {};", i, expected);
        assert!(output.contains(&expected_field), "Missing: {}", expected_field);
    }
}

#[test]
fn test_undecided_object_type_returns_error() {
    let oml_object = OmlObject { oml_type: ObjectType::UNDECIDED, name: "Bad".to_string(), variables: vec![] };
    assert!(TypescriptGenerator.generate(std::slice::from_ref(&oml_object), "Bad").is_err());
}

#[test]
fn test_extension_is_ts() {
    assert_eq!(TypescriptGenerator.extension(), "ts");
}
