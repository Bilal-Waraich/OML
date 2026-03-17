use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::core::generate::{Generate, BackwardsGenerate};
use std::error::Error;
use std::fmt::Write;

pub struct KotlinGenerator {
    pub use_data_class: bool,
}

impl BackwardsGenerate for KotlinGenerator {
    fn reverse(&self, content: &str) -> Result<Vec<OmlObject>, Box<dyn Error>> {
        let mut objects = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("enum class ") && trimmed.ends_with('{') {
                let name = trimmed
                    .strip_prefix("enum class ")
                    .unwrap()
                    .trim_end_matches(|c: char| c == '{' || c == ' ')
                    .to_string();
                let mut vars = Vec::new();
                i += 1;
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line == "}" { break; }
                    let variant = line.trim_end_matches(',').trim().to_string();
                    if !variant.is_empty() {
                        vars.push(Variable {
                            var_mod: vec![],
                            visibility: VariableVisibility::PUBLIC,
                            var_type: "string".to_string(),
                            array_kind: ArrayKind::None,
                            name: variant,
                        });
                    }
                    i += 1;
                }
                objects.push(OmlObject {
                    oml_type: ObjectType::ENUM,
                    name,
                    variables: vars,
                });
            } else if (trimmed.starts_with("data class ") || trimmed.starts_with("class "))
                && (trimmed.contains('(') || trimmed.ends_with('{'))
            {
                let is_data = trimmed.starts_with("data class ");
                let prefix = if is_data { "data class " } else { "class " };
                let after = trimmed.strip_prefix(prefix).unwrap();
                let name_end = after.find(|c: char| c == '(' || c == '{' || c == ' ').unwrap_or(after.len());
                let name = after[..name_end].trim().to_string();

                let mut vars = Vec::new();
                let mut in_companion = false;

                // Collect all lines until the class closes
                if trimmed.contains('(') {
                    // Constructor params
                    i += 1;
                    while i < lines.len() {
                        let line = lines[i].trim();
                        if line.starts_with(')') { break; }
                        if let Some(var) = parse_kotlin_param(line) {
                            vars.push(var);
                        }
                        i += 1;
                    }
                }

                // Look for companion object or closing brace
                i += 1;
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line == "}" && !in_companion { break; }
                    if line == "}" && in_companion {
                        in_companion = false;
                        i += 1;
                        continue;
                    }
                    if line.starts_with("companion object {") {
                        in_companion = true;
                        i += 1;
                        continue;
                    }
                    if in_companion {
                        if let Some(var) = parse_kotlin_companion_var(line) {
                            vars.push(var);
                        }
                    }
                    i += 1;
                }

                let oml_type = if is_data { ObjectType::CLASS } else { ObjectType::CLASS };
                objects.push(OmlObject {
                    oml_type,
                    name,
                    variables: vars,
                });
            }
            i += 1;
        }

        Ok(objects)
    }
}

fn reverse_kotlin_type(kt_type: &str) -> String {
    match kt_type {
        "Int" => "int32".to_string(),
        "Long" => "int64".to_string(),
        "UInt" => "uint32".to_string(),
        "ULong" => "uint64".to_string(),
        "Float" => "float".to_string(),
        "Double" => "double".to_string(),
        "Boolean" => "bool".to_string(),
        "String" => "string".to_string(),
        "Char" => "char".to_string(),
        other => other.to_string(),
    }
}

fn parse_kotlin_type_annotation(type_str: &str) -> (String, ArrayKind, bool) {
    let type_str = type_str.trim();

    // Optional: "Type? = null" or "Type?"
    let (type_str, is_optional) = if type_str.ends_with("? = null") {
        (&type_str[..type_str.len() - 8], true)
    } else if type_str.ends_with('?') {
        (&type_str[..type_str.len() - 1], true)
    } else {
        (type_str, false)
    };

    // Array<T>
    if type_str.starts_with("Array<") && type_str.ends_with('>') {
        let inner = &type_str[6..type_str.len() - 1];
        return (reverse_kotlin_type(inner), ArrayKind::Static(0), is_optional);
    }

    // MutableList<T>
    if type_str.starts_with("MutableList<") && type_str.ends_with('>') {
        let inner = &type_str[12..type_str.len() - 1];
        return (reverse_kotlin_type(inner), ArrayKind::Dynamic, is_optional);
    }

    (reverse_kotlin_type(type_str), ArrayKind::None, is_optional)
}

fn parse_kotlin_param(line: &str) -> Option<Variable> {
    let line = line.trim().trim_end_matches(',');
    if line.is_empty() { return None; }

    let mut visibility = VariableVisibility::PUBLIC;
    let mut rest = line;

    if rest.starts_with("private ") {
        visibility = VariableVisibility::PRIVATE;
        rest = &rest[8..];
    } else if rest.starts_with("protected ") {
        visibility = VariableVisibility::PROTECTED;
        rest = &rest[10..];
    }

    let mut var_mod = Vec::new();
    let is_val;
    if rest.starts_with("val ") {
        var_mod.push(VariableModifier::CONST);
        rest = &rest[4..];
        is_val = true;
    } else if rest.starts_with("var ") {
        rest = &rest[4..];
        is_val = false;
    } else {
        return None;
    }
    let _ = is_val;

    // "name: Type" or "name: Type? = null"
    let colon_pos = rest.find(':')?;
    let name = rest[..colon_pos].trim().to_string();
    let type_str = rest[colon_pos + 1..].trim();

    let (var_type, array_kind, is_optional) = parse_kotlin_type_annotation(type_str);
    if is_optional {
        var_mod.push(VariableModifier::OPTIONAL);
    }

    Some(Variable {
        var_mod,
        visibility,
        var_type,
        array_kind,
        name,
    })
}

fn parse_kotlin_companion_var(line: &str) -> Option<Variable> {
    let line = line.trim();
    if line.is_empty() { return None; }

    let mut var_mod = vec![VariableModifier::STATIC];
    let mut rest = line;

    if rest.starts_with("val ") {
        var_mod.push(VariableModifier::CONST);
        rest = &rest[4..];
    } else if rest.starts_with("var ") {
        rest = &rest[4..];
    } else {
        return None;
    }

    let colon_pos = rest.find(':')?;
    let name = rest[..colon_pos].trim().to_string();
    let type_str = rest[colon_pos + 1..].trim();

    let (var_type, array_kind, is_optional) = parse_kotlin_type_annotation(type_str);
    if is_optional {
        var_mod.push(VariableModifier::OPTIONAL);
    }

    Some(Variable {
        var_mod,
        visibility: VariableVisibility::PRIVATE,
        var_type,
        array_kind,
        name,
    })
}

impl KotlinGenerator {
    pub fn new(use_data_class: bool) -> Self {
        Self { use_data_class }
    }
}

impl Generate for KotlinGenerator {
    fn generate(&self, oml_objects: &[OmlObject], file_name: &str) -> Result<String, Box<dyn Error>> {
        let mut kt_file = String::new();

        writeln!(kt_file, "// This file has been generated from {}.oml", file_name)?;
        writeln!(kt_file)?;

        for (i, oml_object) in oml_objects.iter().enumerate() {
            match &oml_object.oml_type {
                ObjectType::ENUM => generate_enum(oml_object, &mut kt_file)?,
                ObjectType::CLASS => generate_class(oml_object, &mut kt_file, self.use_data_class)?,
                ObjectType::STRUCT => generate_class(oml_object, &mut kt_file, true)?,
                ObjectType::UNDECIDED => return Err("Cannot generate code for UNDECIDED object type".into()),
            }
            if i < oml_objects.len() - 1 {
                writeln!(kt_file)?;
            }
        }

        Ok(kt_file)
    }

    fn extension(&self) -> &str {
        "kt"
    }
}

fn generate_enum(oml_object: &OmlObject, kt_file: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(kt_file, "enum class {} {{", oml_object.name)?;
    let length = oml_object.variables.len();

    for (index, var) in oml_object.variables.iter().enumerate() {
        write!(kt_file, "\t{}", var.name.to_uppercase())?;
        if index == length - 1 {
            writeln!(kt_file)?;
        } else {
            writeln!(kt_file, ",")?;
        }
    }

    writeln!(kt_file, "}}")?;

    Ok(())
}

fn generate_class(
    oml_object: &OmlObject,
    kt_file: &mut String,
    use_data_class: bool,
) -> Result<(), std::fmt::Error> {
    let class_keyword = if use_data_class { "data class" } else { "class" };

    let all_vars: Vec<&Variable> = oml_object.variables.iter().collect();

    if all_vars.is_empty() {
        writeln!(kt_file, "{} {}", class_keyword, oml_object.name)?;
        return Ok(());
    }

    // Separate static vars from instance vars
    let static_vars: Vec<&Variable> = all_vars
        .iter()
        .filter(|v| v.var_mod.contains(&VariableModifier::STATIC))
        .copied()
        .collect();

    let instance_vars: Vec<&Variable> = all_vars
        .iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::STATIC))
        .copied()
        .collect();

    if instance_vars.is_empty() && !static_vars.is_empty() {
        // Only static vars, no primary constructor params
        writeln!(kt_file, "{} {} {{", class_keyword, oml_object.name)?;
    } else {
        // Write class header with primary constructor
        writeln!(kt_file, "{}{}(", class_keyword, format!(" {}", oml_object.name))?;
        write_constructor_params(&instance_vars, kt_file)?;
        write!(kt_file, ")")?;

        if static_vars.is_empty() {
            writeln!(kt_file)?;
        } else {
            writeln!(kt_file, " {{")?;
        }
    }

    // Companion object for static vars
    if !static_vars.is_empty() {
        writeln!(kt_file, "\tcompanion object {{")?;
        for var in &static_vars {
            write_static_property(var, kt_file)?;
        }
        writeln!(kt_file, "\t}}")?;
        writeln!(kt_file, "}}")?;
    }

    Ok(())
}

fn write_constructor_params(
    vars: &[&Variable],
    kt_file: &mut String,
) -> Result<(), std::fmt::Error> {
    let required_vars: Vec<&&Variable> = vars
        .iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::OPTIONAL))
        .collect();

    let optional_vars: Vec<&&Variable> = vars
        .iter()
        .filter(|v| v.var_mod.contains(&VariableModifier::OPTIONAL))
        .collect();

    // Required params first, then optional params (with defaults)
    let total = required_vars.len() + optional_vars.len();
    let mut index = 0;

    for var in &required_vars {
        write_property_param(var, kt_file, false)?;
        index += 1;
        if index < total {
            writeln!(kt_file, ",")?;
        } else {
            writeln!(kt_file)?;
        }
    }

    for var in &optional_vars {
        write_property_param(var, kt_file, true)?;
        index += 1;
        if index < total {
            writeln!(kt_file, ",")?;
        } else {
            writeln!(kt_file)?;
        }
    }

    Ok(())
}

fn write_property_param(
    var: &Variable,
    kt_file: &mut String,
    is_optional: bool,
) -> Result<(), std::fmt::Error> {
    write!(kt_file, "\t")?;

    // Visibility modifier (public is default, so we omit it)
    match var.visibility {
        VariableVisibility::PRIVATE => write!(kt_file, "private ")?,
        VariableVisibility::PROTECTED => write!(kt_file, "protected ")?,
        VariableVisibility::PUBLIC => {},
    }

    // val for const, var for mutable
    if var.var_mod.contains(&VariableModifier::CONST)
        && !var.var_mod.contains(&VariableModifier::MUT) {
        write!(kt_file, "val ")?;
    } else {
        write!(kt_file, "var ")?;
    }

    let kt_type = type_annotation(&var.var_type, &var.array_kind);

    write!(kt_file, "{}: ", var.name)?;

    if is_optional {
        write!(kt_file, "{}? = null", kt_type)?;
    } else {
        write!(kt_file, "{}", kt_type)?;
    }

    Ok(())
}

fn write_static_property(
    var: &Variable,
    kt_file: &mut String,
) -> Result<(), std::fmt::Error> {
    write!(kt_file, "\t\t")?;

    // Static const → const val, static mutable → var
    if var.var_mod.contains(&VariableModifier::CONST)
        && !var.var_mod.contains(&VariableModifier::MUT) {
        write!(kt_file, "val ")?;
    } else {
        write!(kt_file, "var ")?;
    }

    let kt_type = type_annotation(&var.var_type, &var.array_kind);

    if var.var_mod.contains(&VariableModifier::OPTIONAL) {
        writeln!(kt_file, "{}: {}? = null", var.name, kt_type)?;
    } else {
        writeln!(kt_file, "{}: {}", var.name, kt_type)?;
    }

    Ok(())
}

#[inline]
fn convert_type(var_type: &str) -> String {
    match var_type {
        "int8" | "int16" | "int32" => "Int".to_string(),
        "int64" => "Long".to_string(),
        "uint8" | "uint16" | "uint32" => "UInt".to_string(),
        "uint64" => "ULong".to_string(),
        "float" => "Float".to_string(),
        "double" => "Double".to_string(),
        "bool" => "Boolean".to_string(),
        "string" => "String".to_string(),
        "char" => "Char".to_string(),
        other => other.to_string(),
    }
}

fn type_annotation(var_type: &str, array_kind: &ArrayKind) -> String {
    let base = convert_type(var_type);
    match array_kind {
        ArrayKind::None => base,
        ArrayKind::Static(_) => format!("Array<{}>", base),
        ArrayKind::Dynamic => format!("MutableList<{}>", base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generate::Generate;
    use crate::core::oml_object::{
        OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
    };

    fn oml_to_kotlin(oml_object: &OmlObject, file_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        KotlinGenerator::new(true).generate(std::slice::from_ref(oml_object), file_name)
    }

    fn oml_to_kotlin_no_data(oml_object: &OmlObject, file_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        KotlinGenerator::new(false).generate(std::slice::from_ref(oml_object), file_name)
    }

    // ========== ENUM GENERATION TESTS ==========

    #[test]
    fn test_generate_enum_basic() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Color".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Red".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Green".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Blue".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Color").unwrap();
        assert!(output.contains("enum class Color {"));
        assert!(output.contains("\tRED,"));
        assert!(output.contains("\tGREEN,"));
        assert!(output.contains("\tBLUE"));
        // Last variant should NOT have trailing comma
        assert!(!output.contains("BLUE,"));
    }

    #[test]
    fn test_generate_enum_single_variant() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Single".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Only".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Single").unwrap();
        assert!(output.contains("enum class Single {"));
        assert!(output.contains("\tONLY"));
        assert!(!output.contains("ONLY,"));
    }

    #[test]
    fn test_generate_enum_empty() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Empty".to_string(),
            variables: vec![],
        };

        let output = oml_to_kotlin(&oml_object, "Empty").unwrap();
        assert!(output.contains("enum class Empty {"));
        assert!(output.contains("}"));
    }

    // ========== DATA CLASS GENERATION TESTS ==========

    #[test]
    fn test_data_class_basic() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Person".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "name".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "age".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Person").unwrap();
        assert!(output.contains("data class Person("));
        assert!(output.contains("private var name: String"));
        assert!(output.contains("private var age: Int"));
    }

    #[test]
    fn test_regular_class_basic() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Person".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "name".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "age".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin_no_data(&oml_object, "Person").unwrap();
        assert!(output.contains("class Person("));
        assert!(!output.contains("data class"));
        assert!(output.contains("private var name: String"));
        assert!(output.contains("private var age: Int"));
    }

    #[test]
    fn test_struct_always_data_class() {
        let oml_object = OmlObject {
            oml_type: ObjectType::STRUCT,
            name: "Point".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "double".to_string(),
                    array_kind: ArrayKind::None,
                    name: "x".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "double".to_string(),
                    array_kind: ArrayKind::None,
                    name: "y".to_string(),
                },
            ],
        };

        // Even with no-data-class, structs should be data class
        let output = oml_to_kotlin_no_data(&oml_object, "Point").unwrap();
        assert!(output.contains("data class Point("));
    }

    #[test]
    fn test_data_class_empty() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Empty".to_string(),
            variables: vec![],
        };

        let output = oml_to_kotlin(&oml_object, "Empty").unwrap();
        assert!(output.contains("data class Empty"));
        assert!(!output.contains("("));
    }

    #[test]
    fn test_class_with_optional_fields() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "User".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "name".to_string(),
                },
                Variable {
                    var_mod: vec![VariableModifier::OPTIONAL],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "email".to_string(),
                },
                Variable {
                    var_mod: vec![VariableModifier::OPTIONAL],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "age".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "User").unwrap();
        assert!(output.contains("private var name: String"));
        assert!(output.contains("private var email: String? = null"));
        assert!(output.contains("private var age: Int? = null"));
    }

    #[test]
    fn test_class_optional_params_come_after_required() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Mixed".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::OPTIONAL],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "optional_first".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "required".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Mixed").unwrap();
        // Required should appear before optional in the constructor
        let required_pos = output.find("required: Int").unwrap();
        let optional_pos = output.find("optional_first: String? = null").unwrap();
        assert!(required_pos < optional_pos, "Required params should come before optional params");
    }

    #[test]
    fn test_const_modifier_generates_val() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Config".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::CONST],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "name".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Config").unwrap();
        assert!(output.contains("private val name: String"));
        assert!(!output.contains("var name"));
    }

    #[test]
    fn test_mut_modifier_generates_var() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Config".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::MUT],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "name".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Config").unwrap();
        assert!(output.contains("private var name: String"));
    }

    #[test]
    fn test_mut_overrides_const() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Config".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::CONST, VariableModifier::MUT],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "value".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Config").unwrap();
        assert!(output.contains("var value: Int"));
        assert!(!output.contains("val value"));
    }

    #[test]
    fn test_static_modifier_companion_object() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Config".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "name".to_string(),
                },
                Variable {
                    var_mod: vec![VariableModifier::STATIC],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "count".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Config").unwrap();
        assert!(output.contains("companion object {"));
        assert!(output.contains("\t\tvar count: Int"));
    }

    #[test]
    fn test_static_const_in_companion() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Constants".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::STATIC, VariableModifier::CONST],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "MAX".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Constants").unwrap();
        assert!(output.contains("companion object {"));
        assert!(output.contains("\t\tval MAX: Int"));
    }

    #[test]
    fn test_optional_with_static() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Config".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::STATIC, VariableModifier::OPTIONAL],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "instance".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Config").unwrap();
        assert!(output.contains("companion object {"));
        assert!(output.contains("var instance: String? = null"));
    }

    // ========== VISIBILITY TESTS ==========

    #[test]
    fn test_public_visibility_omitted() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Foo".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "x".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Foo").unwrap();
        // Public is default in Kotlin, should not appear
        assert!(output.contains("\tvar x: Int"));
        assert!(!output.contains("public "));
    }

    #[test]
    fn test_private_visibility() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Foo".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "x".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Foo").unwrap();
        assert!(output.contains("\tprivate var x: Int"));
    }

    #[test]
    fn test_protected_visibility() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Foo".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PROTECTED,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "x".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Foo").unwrap();
        assert!(output.contains("\tprotected var x: Int"));
    }

    #[test]
    fn test_all_visibility_levels() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Mixed".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "pub_val".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PROTECTED,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "prot_val".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "priv_val".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Mixed").unwrap();
        assert!(output.contains("\tvar pub_val: Int"));
        assert!(output.contains("\tprotected var prot_val: Int"));
        assert!(output.contains("\tprivate var priv_val: Int"));
    }

    // ========== TYPE CONVERSION TESTS ==========

    #[test]
    fn test_convert_all_integer_types() {
        assert_eq!(convert_type("int8"), "Int");
        assert_eq!(convert_type("int16"), "Int");
        assert_eq!(convert_type("int32"), "Int");
        assert_eq!(convert_type("int64"), "Long");
    }

    #[test]
    fn test_convert_unsigned_integer_types() {
        assert_eq!(convert_type("uint8"), "UInt");
        assert_eq!(convert_type("uint16"), "UInt");
        assert_eq!(convert_type("uint32"), "UInt");
        assert_eq!(convert_type("uint64"), "ULong");
    }

    #[test]
    fn test_convert_floating_point_types() {
        assert_eq!(convert_type("float"), "Float");
        assert_eq!(convert_type("double"), "Double");
    }

    #[test]
    fn test_convert_other_basic_types() {
        assert_eq!(convert_type("bool"), "Boolean");
        assert_eq!(convert_type("string"), "String");
        assert_eq!(convert_type("char"), "Char");
    }

    #[test]
    fn test_convert_custom_type() {
        assert_eq!(convert_type("Address"), "Address");
        assert_eq!(convert_type("Person"), "Person");
    }

    // ========== FULL OUTPUT TESTS ==========

    #[test]
    fn test_oml_to_kotlin_with_enum() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Direction".to_string(),
            variables: vec![
                Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), array_kind: ArrayKind::None, name: "North".to_string() },
                Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), array_kind: ArrayKind::None, name: "South".to_string() },
                Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), array_kind: ArrayKind::None, name: "East".to_string() },
                Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), array_kind: ArrayKind::None, name: "West".to_string() },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Direction").unwrap();
        assert!(output.starts_with("// This file has been generated from Direction.oml"));
        assert!(output.contains("enum class Direction {"));
        assert!(output.contains("\tNORTH,"));
        assert!(output.contains("\tSOUTH,"));
        assert!(output.contains("\tEAST,"));
        assert!(output.contains("\tWEST"));
        assert!(!output.contains("WEST,"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_oml_to_kotlin_with_class() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Foo".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "bar".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Foo").unwrap();
        assert!(output.starts_with("// This file has been generated from Foo.oml"));
        assert!(output.contains("data class Foo("));
        assert!(output.contains("\tprivate var bar: Int"));
        assert!(output.contains(")"));
    }

    #[test]
    fn test_oml_to_kotlin_with_undecided_type_fails() {
        let oml_object = OmlObject {
            oml_type: ObjectType::UNDECIDED,
            name: "Bad".to_string(),
            variables: vec![],
        };

        let result = oml_to_kotlin(&oml_object, "Bad");
        assert!(result.is_err());
    }

    #[test]
    fn test_full_output_has_proper_structure() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Example".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::CONST],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "id".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "count".to_string(),
                },
                Variable {
                    var_mod: vec![VariableModifier::OPTIONAL],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "description".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Example").unwrap();
        assert!(output.contains("// This file has been generated from Example.oml"));
        assert!(output.contains("data class Example("));
        assert!(output.contains("private val id: String"));
        assert!(output.contains("var count: Int"));
        assert!(output.contains("private var description: String? = null"));
    }

    #[test]
    fn test_extension_is_kt() {
        let _gen = KotlinGenerator::new(true);
        assert_eq!(_gen.extension(), "kt");
    }

    #[test]
    fn test_class_with_many_variables() {
        let mut variables = Vec::new();
        for i in 0..20 {
            variables.push(Variable {
                var_mod: if i % 3 == 0 { vec![VariableModifier::OPTIONAL] } else { vec![] },
                visibility: VariableVisibility::PRIVATE,
                var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                name: format!("var_{}", i),
            });
        }

        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "ManyVars".to_string(),
            variables,
        };

        let output = oml_to_kotlin(&oml_object, "ManyVars").unwrap();
        assert!(output.contains("data class ManyVars("));
        // Check some required vars come before optional ones
        let first_required = output.find("var_1: Int").unwrap();
        let first_optional = output.find("var_0: Int? = null").unwrap();
        assert!(first_required < first_optional);
    }

    #[test]
    fn test_enum_with_many_variants() {
        let variables: Vec<Variable> = (0..50).map(|i| Variable {
            var_mod: vec![],
            visibility: VariableVisibility::PUBLIC,
            var_type: "".to_string(),
                    array_kind: ArrayKind::None,
            name: format!("Variant{}", i),
        }).collect();

        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "BigEnum".to_string(),
            variables,
        };

        let output = oml_to_kotlin(&oml_object, "BigEnum").unwrap();
        assert!(output.contains("enum class BigEnum {"));
        assert!(output.contains("VARIANT0,"));
        assert!(output.contains("VARIANT49"));
        assert!(!output.contains("VARIANT49,"));
    }

    #[test]
    fn test_all_types_in_class() {
        let types_and_expected = vec![
            ("int8", "Int"), ("int16", "Int"), ("int32", "Int"), ("int64", "Long"),
            ("uint8", "UInt"), ("uint16", "UInt"), ("uint32", "UInt"), ("uint64", "ULong"),
            ("float", "Float"), ("double", "Double"),
            ("bool", "Boolean"), ("string", "String"), ("char", "Char"),
        ];

        let variables: Vec<Variable> = types_and_expected.iter().enumerate().map(|(i, (oml_type, _))| {
            Variable {
                var_mod: vec![],
                visibility: VariableVisibility::PUBLIC,
                var_type: oml_type.to_string(),
                    array_kind: ArrayKind::None,
                name: format!("field_{}", i),
            }
        }).collect();

        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "AllTypes".to_string(),
            variables,
        };

        let output = oml_to_kotlin(&oml_object, "AllTypes").unwrap();

        for (i, (_, expected_kt)) in types_and_expected.iter().enumerate() {
            let expected = format!("field_{}: {}", i, expected_kt);
            assert!(output.contains(&expected), "Missing: {} in output:\n{}", expected, output);
        }
    }

    #[test]
    fn test_optional_with_const() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Foo".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::CONST, VariableModifier::OPTIONAL],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "value".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Foo").unwrap();
        assert!(output.contains("val value: String? = null"));
    }

    #[test]
    fn test_variable_with_all_modifiers() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Full".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::STATIC, VariableModifier::CONST, VariableModifier::OPTIONAL],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "everything".to_string(),
                },
            ],
        };

        let output = oml_to_kotlin(&oml_object, "Full").unwrap();
        assert!(output.contains("companion object {"));
        assert!(output.contains("val everything: Int? = null"));
    }
}
