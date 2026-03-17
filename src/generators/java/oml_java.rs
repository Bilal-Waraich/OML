use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::core::generate::Generate;
use std::error::Error;
use std::fmt::Write;

pub struct JavaGenerator;

impl Generate for JavaGenerator {
    fn generate(&self, oml_objects: &[OmlObject], file_name: &str) -> Result<String, Box<dyn Error>> {
        let mut java_file = String::new();

        writeln!(java_file, "// This file has been generated from {}.oml", file_name)?;
        writeln!(java_file)?;

        // Collect imports needed across all objects
        let imports = collect_imports(oml_objects);
        if !imports.is_empty() {
            for import in &imports {
                writeln!(java_file, "{}", import)?;
            }
            writeln!(java_file)?;
        }

        for (i, oml_object) in oml_objects.iter().enumerate() {
            match &oml_object.oml_type {
                ObjectType::ENUM => generate_enum(oml_object, &mut java_file)?,
                ObjectType::CLASS | ObjectType::STRUCT => generate_class(oml_object, &mut java_file)?,
                ObjectType::UNDECIDED => return Err("Cannot generate code for UNDECIDED object type".into()),
            }
            if i < oml_objects.len() - 1 {
                writeln!(java_file)?;
            }
        }

        Ok(java_file)
    }

    fn extension(&self) -> &str {
        "java"
    }
}

fn collect_imports(oml_objects: &[OmlObject]) -> Vec<String> {
    let mut imports: Vec<String> = Vec::new();

    let needs_list = oml_objects.iter().any(|o|
        o.oml_type != ObjectType::ENUM &&
        o.variables.iter().any(|v| v.array_kind == ArrayKind::Dynamic)
    );

    if needs_list {
        imports.push("import java.util.List;".to_string());
        imports.push("import java.util.ArrayList;".to_string());
    }

    imports
}

fn generate_enum(oml_object: &OmlObject, java_file: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(java_file, "public enum {} {{", oml_object.name)?;
    let length = oml_object.variables.len();

    for (index, var) in oml_object.variables.iter().enumerate() {
        write!(java_file, "\t{}", var.name.to_uppercase())?;
        if index == length - 1 {
            writeln!(java_file, ";")?;
        } else {
            writeln!(java_file, ",")?;
        }
    }

    writeln!(java_file, "}}")?;

    Ok(())
}

fn generate_class(
    oml_object: &OmlObject,
    java_file: &mut String,
) -> Result<(), std::fmt::Error> {
    writeln!(java_file, "public class {} {{", oml_object.name)?;

    if oml_object.variables.is_empty() {
        writeln!(java_file, "}}")?;
        return Ok(());
    }

    // Emit field declarations
    for var in &oml_object.variables {
        write_field(var, java_file)?;
    }

    writeln!(java_file)?;

    // Constructor (only instance — non-static — vars)
    let instance_vars: Vec<&Variable> = oml_object.variables
        .iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::STATIC))
        .collect();

    if !instance_vars.is_empty() {
        // Required params first, then optional
        let required: Vec<&&Variable> = instance_vars
            .iter()
            .filter(|v| !v.var_mod.contains(&VariableModifier::OPTIONAL))
            .collect();
        let optional: Vec<&&Variable> = instance_vars
            .iter()
            .filter(|v| v.var_mod.contains(&VariableModifier::OPTIONAL))
            .collect();

        let total = required.len() + optional.len();
        let mut index = 0;

        writeln!(java_file, "\tpublic {}(", oml_object.name)?;
        for var in &required {
            let java_type = type_annotation(&var.var_type, &var.array_kind, false);
            write!(java_file, "\t\t{} {}", java_type, var.name)?;
            index += 1;
            if index < total { writeln!(java_file, ",")?; } else { writeln!(java_file)?; }
        }
        for var in &optional {
            let java_type = type_annotation(&var.var_type, &var.array_kind, false);
            write!(java_file, "\t\t{} {}", java_type, var.name)?;
            index += 1;
            if index < total { writeln!(java_file, ",")?; } else { writeln!(java_file)?; }
        }

        writeln!(java_file, "\t) {{")?;

        for var in required.iter().chain(optional.iter()) {
            writeln!(java_file, "\t\tthis.{} = {};", var.name, var.name)?;
        }

        writeln!(java_file, "\t}}")?;
    }

    // Getters and setters for non-static, non-const fields
    let has_accessors = oml_object.variables.iter().any(|v|
        !v.var_mod.contains(&VariableModifier::STATIC)
    );

    if has_accessors {
        writeln!(java_file)?;
        for var in &oml_object.variables {
            if var.var_mod.contains(&VariableModifier::STATIC) {
                continue;
            }
            write_getter(var, java_file)?;
            // No setter for const (final) fields
            if !var.var_mod.contains(&VariableModifier::CONST)
                || var.var_mod.contains(&VariableModifier::MUT)
            {
                write_setter(var, java_file)?;
            }
        }
    }

    writeln!(java_file, "}}")?;

    Ok(())
}

/// Writes a single class field declaration.
fn write_field(var: &Variable, java_file: &mut String) -> Result<(), std::fmt::Error> {
    write!(java_file, "\t")?;

    // Visibility
    match var.visibility {
        VariableVisibility::PRIVATE => write!(java_file, "private ")?,
        VariableVisibility::PROTECTED => write!(java_file, "protected ")?,
        VariableVisibility::PUBLIC => write!(java_file, "public ")?,
    }

    // static modifier
    if var.var_mod.contains(&VariableModifier::STATIC) {
        write!(java_file, "static ")?;
    }

    // final for const (without mut override)
    if var.var_mod.contains(&VariableModifier::CONST)
        && !var.var_mod.contains(&VariableModifier::MUT)
    {
        write!(java_file, "final ")?;
    }

    let java_type = type_annotation(&var.var_type, &var.array_kind, var.var_mod.contains(&VariableModifier::OPTIONAL));

    writeln!(java_file, "{} {};", java_type, var.name)?;

    Ok(())
}

fn write_getter(var: &Variable, java_file: &mut String) -> Result<(), std::fmt::Error> {
    let java_type = type_annotation(&var.var_type, &var.array_kind, var.var_mod.contains(&VariableModifier::OPTIONAL));
    let getter_name = format!("get{}", capitalise(&var.name));
    writeln!(java_file, "\tpublic {} {}() {{ return {}; }}", java_type, getter_name, var.name)?;
    Ok(())
}

fn write_setter(var: &Variable, java_file: &mut String) -> Result<(), std::fmt::Error> {
    let java_type = type_annotation(&var.var_type, &var.array_kind, var.var_mod.contains(&VariableModifier::OPTIONAL));
    let setter_name = format!("set{}", capitalise(&var.name));
    writeln!(java_file, "\tpublic void {}({} value) {{ this.{} = value; }}", setter_name, java_type, var.name)?;
    Ok(())
}

#[inline]
fn convert_type(var_type: &str) -> String {
    match var_type {
        "int8" => "byte".to_string(),
        "int16" => "short".to_string(),
        "int32" => "int".to_string(),
        "int64" => "long".to_string(),
        // Java has no unsigned primitives; map to the next-larger signed type
        "uint8" => "short".to_string(),
        "uint16" => "int".to_string(),
        "uint32" => "long".to_string(),
        "uint64" => "long".to_string(),
        "float" => "float".to_string(),
        "double" => "double".to_string(),
        "bool" => "boolean".to_string(),
        "string" => "String".to_string(),
        "char" => "char".to_string(),
        other => other.to_string(),
    }
}

/// Returns the boxed type string for use inside generics (e.g. `List<Integer>`).
fn boxed_type(var_type: &str) -> String {
    match var_type {
        "int8" => "Byte".to_string(),
        "int16" => "Short".to_string(),
        "int32" => "Integer".to_string(),
        "int64" => "Long".to_string(),
        "uint8" => "Short".to_string(),
        "uint16" => "Integer".to_string(),
        "uint32" => "Long".to_string(),
        "uint64" => "Long".to_string(),
        "float" => "Float".to_string(),
        "double" => "Double".to_string(),
        "bool" => "Boolean".to_string(),
        "string" => "String".to_string(),
        "char" => "Character".to_string(),
        other => other.to_string(),
    }
}

fn type_annotation(var_type: &str, array_kind: &ArrayKind, _is_optional: bool) -> String {
    match array_kind {
        ArrayKind::None => convert_type(var_type),
        // Java arrays have no compile-time size; the [N] constraint is a comment
        ArrayKind::Static(n) => format!("{}[] /* [{}] */", convert_type(var_type), n),
        ArrayKind::Dynamic => format!("List<{}>", boxed_type(var_type)),
    }
}

/// Capitalises the first character of a string, leaving the rest unchanged.
fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
