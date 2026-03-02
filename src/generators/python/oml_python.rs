use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableModifier, ArrayKind
};
use crate::core::generate::Generate;
use std::error::Error;
use std::fmt::Write;

pub struct PythonGenerator {
    pub use_data_class: bool,
}
impl PythonGenerator {
    pub fn new(use_data_class: bool) -> Self {
        Self { use_data_class }
    }
}

impl Generate for PythonGenerator {
    fn generate(&self, oml_objects: &[OmlObject], file_name: &str) -> Result<String, Box<dyn Error>> {
        let mut py_file = String::new();

        writeln!(py_file, "# This file has been generated from {}.oml", file_name)?;
        writeln!(py_file)?;

        // Collect imports needed across all objects
        let imports = collect_imports(oml_objects, self.use_data_class);
        if !imports.is_empty() {
            for import in &imports {
                writeln!(py_file, "{}", import)?;
            }
            writeln!(py_file)?;
        }

        for (i, oml_object) in oml_objects.iter().enumerate() {
            match &oml_object.oml_type {
                ObjectType::ENUM => generate_enum(oml_object, &mut py_file)?,
                ObjectType::CLASS => generate_class(oml_object, &mut py_file, self.use_data_class)?,
                ObjectType::STRUCT => generate_class(oml_object, &mut py_file, true)?,
                ObjectType::UNDECIDED => return Err("Cannot generate code for UNDECIDED object type".into()),
            }
            if i < oml_objects.len() - 1 {
                writeln!(py_file)?;
            }
        }

        Ok(py_file)
    }

    fn extension(&self) -> &str { "py" }
}

fn collect_imports(oml_objects: &[OmlObject], use_data_class: bool) -> Vec<String> {
    let mut imports: Vec<String> = Vec::new();

    let has_enum = oml_objects.iter().any(|o| o.oml_type == ObjectType::ENUM);
    let has_struct = oml_objects.iter().any(|o| o.oml_type == ObjectType::STRUCT);
    let has_class_dataclass = use_data_class && oml_objects.iter().any(|o| o.oml_type == ObjectType::CLASS);
    let needs_dataclass = has_struct || has_class_dataclass;

    // For dataclass objects, check if any have static vars
    let needs_classvar = oml_objects.iter().any(|o| {
        let is_dc = o.oml_type == ObjectType::STRUCT || (use_data_class && o.oml_type == ObjectType::CLASS);
        is_dc && o.variables.iter().any(|v| v.var_mod.contains(&VariableModifier::STATIC))
    });

    let needs_optional = oml_objects.iter().any(|o|
        o.oml_type != ObjectType::ENUM &&
        o.variables.iter().any(|v| v.var_mod.contains(&VariableModifier::OPTIONAL))
    );

    if has_enum {
        imports.push("from enum import Enum".to_string());
    }
    if needs_dataclass {
        imports.push("from dataclasses import dataclass, field".to_string());
    }

    let mut typing_imports: Vec<&str> = Vec::new();
    if needs_classvar {
        typing_imports.push("ClassVar");
    }
    if needs_optional {
        typing_imports.push("Optional");
    }
    if !typing_imports.is_empty() {
        imports.push(format!("from typing import {}", typing_imports.join(", ")));
    }

    imports
}

fn generate_enum(oml_object: &OmlObject, py_file: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(py_file, "class {}(Enum):", oml_object.name)?;

    if oml_object.variables.is_empty() {
        writeln!(py_file, "\tpass")?;
    } else {
        for (index, var) in oml_object.variables.iter().enumerate() {
            writeln!(py_file, "\t{} = {}", var.name.to_uppercase(), index)?;
        }
    }

    Ok(())
}

fn generate_class(
    oml_object: &OmlObject,
    py_file: &mut String,
    use_data_class: bool,
) -> Result<(), std::fmt::Error> {
    if use_data_class {
        generate_data_class(oml_object, py_file)
    } else {
        generate_regular_class(oml_object, py_file)
    }
}

// ── dataclass ────────────────────────────────────────────────────────────────

fn generate_data_class(oml_object: &OmlObject, py_file: &mut String) -> Result<(), std::fmt::Error> {
    let vars = &oml_object.variables;

    let static_vars: Vec<&Variable> = vars.iter()
        .filter(|v| v.var_mod.contains(&VariableModifier::STATIC))
        .collect();

    let instance_vars: Vec<&Variable> = vars.iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::STATIC))
        .collect();

    let all_const = !instance_vars.is_empty() && instance_vars.iter()
        .all(|v| v.var_mod.contains(&VariableModifier::CONST));

    if all_const {
        writeln!(py_file, "@dataclass(frozen=True)")?;
    } else {
        writeln!(py_file, "@dataclass")?;
    }
    writeln!(py_file, "class {}:", oml_object.name)?;

    if vars.is_empty() {
        writeln!(py_file, "\tpass")?;
        return Ok(());
    }

    // Static (ClassVar) fields first
    for var in &static_vars {
        let py_type = type_annotation(&var.var_type, &var.array_kind);
        writeln!(py_file, "\t{}: ClassVar[{}]", var.name, py_type)?;
    }

    // Required instance fields (non-optional, non-static) — required first
    let required: Vec<&&Variable> = instance_vars.iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::OPTIONAL))
        .collect();

    let optional: Vec<&&Variable> = instance_vars.iter()
        .filter(|v| v.var_mod.contains(&VariableModifier::OPTIONAL))
        .collect();

    for var in &required {
        let py_type = type_annotation(&var.var_type, &var.array_kind);
        writeln!(py_file, "\t{}: {}", var.name, py_type)?;
    }

    for var in &optional {
        let py_type = type_annotation(&var.var_type, &var.array_kind);
        writeln!(py_file, "\t{}: Optional[{}] = None", var.name, py_type)?;
    }

    Ok(())
}

// ── regular class ─────────────────────────────────────────────────────────────

fn generate_regular_class(oml_object: &OmlObject, py_file: &mut String) -> Result<(), std::fmt::Error> {
    let vars = &oml_object.variables;

    let static_vars: Vec<&Variable> = vars.iter()
        .filter(|v| v.var_mod.contains(&VariableModifier::STATIC))
        .collect();

    let instance_vars: Vec<&Variable> = vars.iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::STATIC))
        .collect();

    writeln!(py_file, "class {}:", oml_object.name)?;

    if vars.is_empty() {
        writeln!(py_file, "\tpass")?;
        return Ok(());
    }

    // Class-level static variables
    for var in &static_vars {
        let py_type = type_annotation(&var.var_type, &var.array_kind);
        if var.var_mod.contains(&VariableModifier::CONST) {
            writeln!(py_file, "\t{}: {} = ...", var.name, py_type)?;
        } else {
            writeln!(py_file, "\t{}: {}", var.name, py_type)?;
        }
    }

    if !static_vars.is_empty() {
        writeln!(py_file)?;
    }

    // __slots__
    if !instance_vars.is_empty() {
        write!(py_file, "\t__slots__ = (")?;
        for var in &instance_vars {
            write!(py_file, "'_{}', ", var.name)?;
        }
        writeln!(py_file, ")")?;
        writeln!(py_file)?;
    }

    // __init__ — required params before optional
    let required: Vec<&&Variable> = instance_vars.iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::OPTIONAL))
        .collect();

    let optional: Vec<&&Variable> = instance_vars.iter()
        .filter(|v| v.var_mod.contains(&VariableModifier::OPTIONAL))
        .collect();

    if !instance_vars.is_empty() {
        write!(py_file, "\tdef __init__(self")?;
        for var in &required {
            let py_type = type_annotation(&var.var_type, &var.array_kind);
            write!(py_file, ", {}: {}", var.name, py_type)?;
        }
        for var in &optional {
            let py_type = type_annotation(&var.var_type, &var.array_kind);
            write!(py_file, ", {}: Optional[{}] = None", var.name, py_type)?;
        }
        writeln!(py_file, "):")?;

        for var in &instance_vars {
            writeln!(py_file, "\t\tself._{} = {}", var.name, var.name)?;
        }
        writeln!(py_file)?;
    }

    // Properties (getters + setters)
    for var in &instance_vars {
        let py_type = type_annotation(&var.var_type, &var.array_kind);
        let is_const = var.var_mod.contains(&VariableModifier::CONST);
        let is_optional = var.var_mod.contains(&VariableModifier::OPTIONAL);

        let return_type = if is_optional {
            format!("Optional[{}]", py_type)
        } else {
            py_type.clone()
        };

        // getter
        writeln!(py_file, "\t@property")?;
        writeln!(py_file, "\tdef {}(self) -> {}:", var.name, return_type)?;
        writeln!(py_file, "\t\treturn self._{}", var.name)?;

        // setter — only for non-const
        if !is_const {
            writeln!(py_file, "\t@{}.setter", var.name)?;
            writeln!(py_file, "\tdef {}(self, value: {}):", var.name, return_type)?;
            writeln!(py_file, "\t\tself._{} = value", var.name)?;
        }

        writeln!(py_file)?;
    }

    Ok(())
}

#[inline]
fn convert_type(var_type: &str) -> String {
    match var_type {
        "int8" | "int16" | "int32" | "int64" => "int",
        "uint8" | "uint16" | "uint32" | "uint64" => "int",
        "float" | "double" => "float",
        "bool" => "bool",
        "string" | "char" => "str",
        _ => var_type,
    }.to_string()
}

fn type_annotation(var_type: &str, array_kind: &ArrayKind) -> String {
    let base = convert_type(var_type);
    match array_kind {
        ArrayKind::None => base,
        ArrayKind::Static(_) | ArrayKind::Dynamic => format!("list[{}]", base),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::oml_object::{ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind};

    fn to_python(oml_object: &OmlObject, use_data_class: bool) -> String {
        PythonGenerator::new(use_data_class)
            .generate(std::slice::from_ref(oml_object), "test")
            .unwrap()
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    fn var(name: &str, ty: &str, mods: Vec<VariableModifier>) -> Variable {
        Variable {
            var_mod: mods,
            visibility: VariableVisibility::PRIVATE,
            var_type: ty.to_string(),
            array_kind: ArrayKind::None,
            name: name.to_string(),
        }
    }

    // ── enum ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_enum_basic() {
        let obj = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Color".to_string(),
            variables: vec![
                var("Red", "", vec![]),
                var("Green", "", vec![]),
                var("Blue", "", vec![]),
            ],
        };
        let out = to_python(&obj, false);
        assert!(out.contains("from enum import Enum"));
        assert!(out.contains("class Color(Enum):"));
        assert!(out.contains("\tRED = 0"));
        assert!(out.contains("\tGREEN = 1"));
        assert!(out.contains("\tBLUE = 2"));
    }

    #[test]
    fn test_enum_empty() {
        let obj = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Empty".to_string(),
            variables: vec![],
        };
        let out = to_python(&obj, false);
        assert!(out.contains("class Empty(Enum):"));
        assert!(out.contains("\tpass"));
    }

    // ── regular class ─────────────────────────────────────────────────────────

    #[test]
    fn test_regular_class_basic() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Person".to_string(),
            variables: vec![
                var("name", "string", vec![]),
                var("age", "int32", vec![]),
            ],
        };
        let out = to_python(&obj, false);
        assert!(out.contains("class Person:"));
        assert!(out.contains("def __init__(self, name: str, age: int):"));
        assert!(out.contains("self._name = name"));
        assert!(out.contains("self._age = age"));
        assert!(out.contains("def name(self) -> str:"));
        assert!(out.contains("def age(self) -> int:"));
        // both mutable, so setters present
        assert!(out.contains("@name.setter"));
        assert!(out.contains("@age.setter"));
    }

    #[test]
    fn test_regular_class_const_no_setter() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Config".to_string(),
            variables: vec![
                var("max_size", "int64", vec![VariableModifier::CONST]),
            ],
        };
        let out = to_python(&obj, false);
        assert!(out.contains("def max_size(self) -> int:"));
        assert!(!out.contains("@max_size.setter"));
    }

    #[test]
    fn test_regular_class_optional_field() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "User".to_string(),
            variables: vec![
                var("name", "string", vec![]),
                var("nickname", "string", vec![VariableModifier::OPTIONAL]),
            ],
        };
        let out = to_python(&obj, false);
        assert!(out.contains("from typing import Optional"));
        // required before optional in __init__
        assert!(out.contains("def __init__(self, name: str, nickname: Optional[str] = None):"));
        assert!(out.contains("def nickname(self) -> Optional[str]:"));
    }

    #[test]
    fn test_regular_class_static_field() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Counter".to_string(),
            variables: vec![
                var("count", "int32", vec![VariableModifier::STATIC]),
                var("name", "string", vec![]),
            ],
        };
        let out = to_python(&obj, false);
        // static goes at class level
        assert!(out.contains("\tcount: int"));
        // instance var gets __init__ and property
        assert!(out.contains("def __init__(self, name: str):"));
    }

    #[test]
    fn test_regular_class_empty() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Empty".to_string(),
            variables: vec![],
        };
        let out = to_python(&obj, false);
        assert!(out.contains("class Empty:"));
        assert!(out.contains("\tpass"));
        assert!(!out.contains("__init__"));
    }

    // ── dataclass ─────────────────────────────────────────────────────────────

    #[test]
    fn test_dataclass_basic() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Person".to_string(),
            variables: vec![
                var("name", "string", vec![]),
                var("age", "int32", vec![]),
            ],
        };
        let out = to_python(&obj, true);
        assert!(out.contains("from dataclasses import dataclass, field"));
        assert!(out.contains("@dataclass"));
        assert!(!out.contains("frozen=True"));
        assert!(out.contains("class Person:"));
        assert!(out.contains("\tname: str"));
        assert!(out.contains("\tage: int"));
    }

    #[test]
    fn test_dataclass_all_const_is_frozen() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Point".to_string(),
            variables: vec![
                var("x", "float", vec![VariableModifier::CONST]),
                var("y", "float", vec![VariableModifier::CONST]),
            ],
        };
        let out = to_python(&obj, true);
        assert!(out.contains("@dataclass(frozen=True)"));
    }

    #[test]
    fn test_dataclass_optional_field() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "User".to_string(),
            variables: vec![
                var("name", "string", vec![]),
                var("email", "string", vec![VariableModifier::OPTIONAL]),
            ],
        };
        let out = to_python(&obj, true);
        assert!(out.contains("from typing import Optional"));
        assert!(out.contains("\tname: str"));
        assert!(out.contains("\temail: Optional[str] = None"));
        // required field must appear before optional
        let name_pos = out.find("\tname: str").unwrap();
        let email_pos = out.find("\temail: Optional").unwrap();
        assert!(name_pos < email_pos);
    }

    #[test]
    fn test_dataclass_static_classvar() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Registry".to_string(),
            variables: vec![
                var("count", "int32", vec![VariableModifier::STATIC]),
                var("name", "string", vec![]),
            ],
        };
        let out = to_python(&obj, true);
        assert!(out.contains("from typing import ClassVar"));
        assert!(out.contains("\tcount: ClassVar[int]"));
        assert!(out.contains("\tname: str"));
    }

    #[test]
    fn test_dataclass_empty() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Empty".to_string(),
            variables: vec![],
        };
        let out = to_python(&obj, true);
        assert!(out.contains("@dataclass"));
        assert!(out.contains("class Empty:"));
        assert!(out.contains("\tpass"));
    }

    #[test]
    fn test_struct_always_dataclass() {
        let obj = OmlObject {
            oml_type: ObjectType::STRUCT,
            name: "Point".to_string(),
            variables: vec![
                var("x", "double", vec![]),
                var("y", "double", vec![]),
            ],
        };
        // even with use_data_class=false, STRUCT → dataclass
        let out = to_python(&obj, false);
        assert!(out.contains("@dataclass"));
        assert!(out.contains("class Point:"));
    }

    #[test]
    fn test_type_conversion() {
        assert_eq!(convert_type("int8"), "int");
        assert_eq!(convert_type("int32"), "int");
        assert_eq!(convert_type("uint64"), "int");
        assert_eq!(convert_type("float"), "float");
        assert_eq!(convert_type("double"), "float");
        assert_eq!(convert_type("bool"), "bool");
        assert_eq!(convert_type("string"), "str");
        assert_eq!(convert_type("char"), "str");
        assert_eq!(convert_type("MyType"), "MyType");
    }

    #[test]
    fn test_undecided_returns_error() {
        let obj = OmlObject {
            oml_type: ObjectType::UNDECIDED,
            name: "Bad".to_string(),
            variables: vec![],
        };
        let result = PythonGenerator::new(false).generate(std::slice::from_ref(&obj), "test");
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod array_tests {
    use super::*;
    use crate::core::oml_object::{OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind};

    fn to_python(oml_object: &OmlObject, use_data_class: bool) -> String {
        PythonGenerator::new(use_data_class)
            .generate(std::slice::from_ref(oml_object), "test")
            .unwrap()
    }

    fn array_var(name: &str, ty: &str, kind: ArrayKind) -> Variable {
        Variable {
            var_mod: vec![],
            visibility: VariableVisibility::PRIVATE,
            var_type: ty.to_string(),
            array_kind: kind,
            name: name.to_string(),
        }
    }

    #[test]
    fn test_static_array_dataclass() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Arr".to_string(),
            variables: vec![array_var("scores", "uint16", ArrayKind::Static(4))],
        };
        let out = to_python(&obj, true);
        assert!(out.contains("scores: list[int]"), "Got: {}", out);
    }

    #[test]
    fn test_dynamic_list_dataclass() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Lst".to_string(),
            variables: vec![array_var("tags", "string", ArrayKind::Dynamic)],
        };
        let out = to_python(&obj, true);
        assert!(out.contains("tags: list[str]"), "Got: {}", out);
    }

    #[test]
    fn test_static_array_regular_class() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Arr".to_string(),
            variables: vec![array_var("ids", "int32", ArrayKind::Static(10))],
        };
        let out = to_python(&obj, false);
        assert!(out.contains("ids: list[int]"), "Got: {}", out);
    }

    #[test]
    fn test_optional_dynamic_list() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Opt".to_string(),
            variables: vec![Variable {
                var_mod: vec![VariableModifier::OPTIONAL],
                visibility: VariableVisibility::PRIVATE,
                var_type: "string".to_string(),
                array_kind: ArrayKind::Dynamic,
                name: "tags".to_string(),
            }],
        };
        let out = to_python(&obj, true);
        assert!(out.contains("tags: Optional[list[str]] = None"), "Got: {}", out);
    }
}
