use std::fs;
use std::path::Path;

use crate::core::generate::Generate;
use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::generators::java::oml_java::JavaGenerator;

const TEST_RESULTS_DIR: &str = "test_results";

fn ensure_test_results_dir() {
    fs::create_dir_all(TEST_RESULTS_DIR).expect("Failed to create test_results directory");
}

fn generate_and_write(oml_path: &str, file_name: &str) -> String {
    ensure_test_results_dir();

    let generator = JavaGenerator;

    let path = Path::new(oml_path);
    let oml_objects = OmlObject::get_from_file(path)
        .expect(&format!("Failed to parse OML file: {}", oml_path));

    let java_output = generator.generate(&oml_objects, file_name)
        .expect(&format!("Failed to generate Java for: {}", file_name));

    let output_path = format!("{}/{}.{}", TEST_RESULTS_DIR, file_name, generator.extension());
    fs::write(&output_path, &java_output)
        .expect(&format!("Failed to write output file: {}", output_path));

    java_output
}

#[test]
fn test_person_class_generates_java_file() {
    let output = generate_and_write(
        "src/generators/java/test_oml_files/person.oml",
        "Person",
    );

    // Header comment
    assert!(output.contains("// This file has been generated from Person.oml"));

    // Class declaration
    assert!(output.contains("public class Person {"));

    // Fields — person.oml uses default (private) visibility
    assert!(output.contains("String name;"));
    assert!(output.contains("int age;"));
    // optional field is still the same Java type (nullable reference)
    assert!(output.contains("String nickname;"));

    // Constructor — required params before optional
    assert!(output.contains("public Person("));
    assert!(output.contains("String name"));
    assert!(output.contains("int age"));
    let name_pos = output.find("String name").unwrap();
    let nick_pos = output.find("String nickname").unwrap();
    assert!(name_pos < nick_pos, "required params must precede optional params");

    // Assignment in constructor body
    assert!(output.contains("this.name = name;"));
    assert!(output.contains("this.age = age;"));
    assert!(output.contains("this.nickname = nickname;"));

    // Getters and setters
    assert!(output.contains("getName()"));
    assert!(output.contains("getAge()"));
    assert!(output.contains("getNickname()"));
    assert!(output.contains("setName("));
    assert!(output.contains("setAge("));
    assert!(output.contains("setNickname("));
}

#[test]
fn test_point_struct_generates_java_file() {
    let output = generate_and_write(
        "src/generators/java/test_oml_files/point.oml",
        "Point",
    );

    // Structs map to classes in Java
    assert!(output.contains("public class Point {"));
    assert!(output.contains("public double x;"));
    assert!(output.contains("public double y;"));

    // Constructor
    assert!(output.contains("public Point("));
    assert!(output.contains("this.x = x;"));
    assert!(output.contains("this.y = y;"));

    // Getters and setters generated for all fields
    assert!(output.contains("getX()"));
    assert!(output.contains("getY()"));
    assert!(output.contains("setX("));
    assert!(output.contains("setY("));
}

#[test]
fn test_color_enum_generates_java_file() {
    let output = generate_and_write(
        "src/generators/java/test_oml_files/color.oml",
        "Color",
    );

    assert!(output.contains("public enum Color {"));
    assert!(output.contains("RED,"));
    assert!(output.contains("GREEN,"));
    assert!(output.contains("BLUE,"));
    // Last variant ends with a semicolon
    assert!(output.contains("YELLOW;"));

    // Enums should not have constructors or getters/setters
    assert!(!output.contains("public Color("));
    assert!(!output.contains("get"));
    assert!(!output.contains("set"));
}

#[test]
fn test_game_entity_arrays_generate_java_file() {
    let output = generate_and_write(
        "src/generators/java/test_oml_files/game_entity.oml",
        "GameEntity",
    );

    assert!(output.contains("public class GameEntity {"));

    // List import must be present for dynamic arrays
    assert!(output.contains("import java.util.List;"));
    assert!(output.contains("import java.util.ArrayList;"));

    // Static array: float[3] => float[] /* [3] */
    assert!(output.contains("float[] /* [3] */ position;"));

    // Dynamic list: list string => List<String>
    assert!(output.contains("List<String> tags;"));

    // Scalar fields
    assert!(output.contains("String name;"));
    assert!(output.contains("int health;"));
    assert!(output.contains("boolean active;"));
}

// ── Inline unit tests ──────────────────────────────────────────────────────────

#[test]
fn test_enum_single_variant_ends_with_semicolon() {
    let oml_object = OmlObject {
        oml_type: ObjectType::ENUM,
        name: "Single".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), array_kind: ArrayKind::None, name: "Only".to_string() },
        ],
    };

    let output = JavaGenerator.generate(std::slice::from_ref(&oml_object), "Single").unwrap();
    assert!(output.contains("\tONLY;"));
    assert!(!output.contains("ONLY,"));
}

#[test]
fn test_const_field_generates_final_no_setter() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Config".to_string(),
        variables: vec![
            Variable { var_mod: vec![VariableModifier::CONST], visibility: VariableVisibility::PRIVATE, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "version".to_string() },
        ],
    };

    let output = JavaGenerator.generate(std::slice::from_ref(&oml_object), "Config").unwrap();
    assert!(output.contains("private final String version;"));
    // No setter for final fields
    assert!(!output.contains("setVersion("));
    // But getter is still generated
    assert!(output.contains("getVersion()"));
}

#[test]
fn test_static_field_not_in_constructor() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Counter".to_string(),
        variables: vec![
            Variable { var_mod: vec![VariableModifier::STATIC], visibility: VariableVisibility::PRIVATE, var_type: "int32".to_string(), array_kind: ArrayKind::None, name: "count".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PRIVATE, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "name".to_string() },
        ],
    };

    let output = JavaGenerator.generate(std::slice::from_ref(&oml_object), "Counter").unwrap();
    assert!(output.contains("private static int count;"));
    assert!(!output.contains("this.count"));
}

#[test]
fn test_optional_params_come_after_required_in_constructor() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Mixed".to_string(),
        variables: vec![
            Variable { var_mod: vec![VariableModifier::OPTIONAL], visibility: VariableVisibility::PRIVATE, var_type: "string".to_string(), array_kind: ArrayKind::None, name: "opt_first".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PRIVATE, var_type: "int32".to_string(), array_kind: ArrayKind::None, name: "required".to_string() },
        ],
    };

    let output = JavaGenerator.generate(std::slice::from_ref(&oml_object), "Mixed").unwrap();
    // Search within the constructor block to avoid matching the field declarations above it
    let constructor_start = output.find("public Mixed(").unwrap();
    let constructor_region = &output[constructor_start..];
    let req_pos = constructor_region.find("int required").unwrap();
    let opt_pos = constructor_region.find("String opt_first").unwrap();
    assert!(req_pos < opt_pos, "required params must precede optional params");
}

#[test]
fn test_dynamic_list_generates_list_type_and_import() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Container".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "string".to_string(), array_kind: ArrayKind::Dynamic, name: "tags".to_string() },
        ],
    };

    let output = JavaGenerator.generate(std::slice::from_ref(&oml_object), "Container").unwrap();
    assert!(output.contains("import java.util.List;"));
    assert!(output.contains("public List<String> tags;"));
}

#[test]
fn test_static_array_expands_with_size_comment() {
    let oml_object = OmlObject {
        oml_type: ObjectType::CLASS,
        name: "Matrix".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "float".to_string(), array_kind: ArrayKind::Static(4), name: "data".to_string() },
        ],
    };

    let output = JavaGenerator.generate(std::slice::from_ref(&oml_object), "Matrix").unwrap();
    assert!(output.contains("public float[] /* [4] */ data;"));
}

#[test]
fn test_all_builtin_types_convert_to_java() {
    let pairs: Vec<(&str, &str)> = vec![
        ("int8", "byte"), ("int16", "short"), ("int32", "int"), ("int64", "long"),
        ("uint8", "short"), ("uint16", "int"), ("uint32", "long"), ("uint64", "long"),
        ("float", "float"), ("double", "double"),
        ("bool", "boolean"), ("string", "String"), ("char", "char"),
    ];

    let variables: Vec<Variable> = pairs.iter().enumerate().map(|(i, (oml_type, _))| Variable {
        var_mod: vec![],
        visibility: VariableVisibility::PUBLIC,
        var_type: oml_type.to_string(),
        array_kind: ArrayKind::None,
        name: format!("field_{}", i),
    }).collect();

    let oml_object = OmlObject { oml_type: ObjectType::CLASS, name: "AllTypes".to_string(), variables };
    let output = JavaGenerator.generate(std::slice::from_ref(&oml_object), "AllTypes").unwrap();

    for (i, (_, expected)) in pairs.iter().enumerate() {
        let expected_field = format!("{} field_{};", expected, i);
        assert!(output.contains(&expected_field), "Missing: {}", expected_field);
    }
}

#[test]
fn test_undecided_object_type_returns_error() {
    let oml_object = OmlObject { oml_type: ObjectType::UNDECIDED, name: "Bad".to_string(), variables: vec![] };
    assert!(JavaGenerator.generate(std::slice::from_ref(&oml_object), "Bad").is_err());
}

#[test]
fn test_extension_is_java() {
    assert_eq!(JavaGenerator.extension(), "java");
}
