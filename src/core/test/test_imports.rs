use std::collections::HashSet;
use std::path::Path;

use crate::core::import_resolver::resolve_all;
use crate::core::oml_object::{OmlObject, OmlFile};
use crate::core::dir_parser::parse_path;

// ── scan_file_with_imports ────────────────────────────────────────────────────

#[test]
fn test_scan_file_strips_import_lines() {
    let content = r#"
import "engine.oml";

class Car {
    string name;
}
"#;
    let (objects, imports) = OmlObject::scan_file_with_imports(content.to_string()).unwrap();
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].name, "Car");
    assert_eq!(imports, vec!["engine.oml"]);
}

#[test]
fn test_scan_file_multiple_imports() {
    let content = r#"
import "a.oml";
import "b.oml";

class Foo {
    string x;
}
"#;
    let (_objects, imports) = OmlObject::scan_file_with_imports(content.to_string()).unwrap();
    assert_eq!(imports.len(), 2);
    assert!(imports.contains(&"a.oml".to_string()));
    assert!(imports.contains(&"b.oml".to_string()));
}

#[test]
fn test_scan_file_no_imports_is_empty_vec() {
    let content = r#"
class Plain {
    string name;
}
"#;
    let (objects, imports) = OmlObject::scan_file_with_imports(content.to_string()).unwrap();
    assert_eq!(objects.len(), 1);
    assert!(imports.is_empty());
}

// ── validate_custom_types with imports ───────────────────────────────────────

#[test]
fn test_validate_accepts_locally_defined_custom_type() {
    let content = r#"
class Engine {
    string model;
}
class Car {
    Engine engine;
}
"#;
    let (objects, _) = OmlObject::scan_file_with_imports(content.to_string()).unwrap();
    assert!(OmlObject::validate_custom_types(&objects, &HashSet::new()).is_ok());
}

#[test]
fn test_validate_accepts_imported_custom_type() {
    let content = r#"
class Car {
    Engine engine;
}
"#;
    let (objects, _) = OmlObject::scan_file_with_imports(content.to_string()).unwrap();

    let mut imported = HashSet::new();
    imported.insert("Engine".to_string());

    assert!(OmlObject::validate_custom_types(&objects, &imported).is_ok());
}

#[test]
fn test_validate_rejects_unknown_custom_type() {
    let content = r#"
class Car {
    Engine engine;
}
"#;
    let (objects, _) = OmlObject::scan_file_with_imports(content.to_string()).unwrap();
    assert!(OmlObject::validate_custom_types(&objects, &HashSet::new()).is_err());
}

// ── resolve_all with real files ───────────────────────────────────────────────

#[test]
fn test_resolve_imports_loads_imported_file() {
    let path = Path::new("src/core/test/oml_files/car.oml");
    let files = parse_path(path, 3).expect("Failed to parse car.oml");
    let (all_files, names_map) = resolve_all(files).expect("Failed to resolve imports");

    // Should have both car.oml and the imported engine.oml
    assert_eq!(all_files.len(), 2);

    let car_file = all_files.iter().find(|f| f.file_name == "car").unwrap();
    let engine_names = names_map.get(&car_file.path).unwrap();
    assert!(engine_names.contains("Engine"), "Engine should be in imported names");
}

#[test]
fn test_resolve_validates_imported_type() {
    let path = Path::new("src/core/test/oml_files/car.oml");
    let files = parse_path(path, 3).expect("Failed to parse car.oml");
    let (all_files, names_map) = resolve_all(files).expect("Failed to resolve imports");

    for oml_file in &all_files {
        let extra = names_map.get(&oml_file.path).cloned().unwrap_or_default();
        assert!(
            OmlObject::validate_custom_types(&oml_file.objects, &extra).is_ok(),
            "Validation failed for {}",
            oml_file.file_name
        );
    }
}

#[test]
fn test_resolve_detects_circular_imports() {
    let path = Path::new("src/core/test/oml_files/cycle_a.oml");
    let files = parse_path(path, 3).expect("Failed to parse cycle_a.oml");
    let result = resolve_all(files);
    assert!(result.is_err(), "Circular import should be detected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Circular import") || msg.contains("cycle"), "Got: {}", msg);
}

#[test]
fn test_resolve_missing_import_is_error() {
    let content = r#"import "nonexistent_file.oml";

class Foo {
    string bar;
}
"#;
    let (objects, imports) = OmlObject::scan_file_with_imports(content.to_string()).unwrap();

    // Build a fake OmlFile pointing at the current directory so the import
    // path can be resolved relative to it.
    let fake_path = std::env::current_dir()
        .unwrap()
        .join("src/core/test/oml_files/fake.oml");

    let oml_file = OmlFile {
        file_name: "fake".to_string(),
        path: fake_path,
        objects,
        imports,
    };

    let result = resolve_all(vec![oml_file]);
    assert!(result.is_err(), "Missing import should fail");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("nonexistent_file.oml"), "Got: {}", msg);
}
