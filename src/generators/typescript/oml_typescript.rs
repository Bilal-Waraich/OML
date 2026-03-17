use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::core::generate::Generate;
use std::error::Error;
use std::fmt::Write;

pub struct TypescriptGenerator;

impl Generate for TypescriptGenerator {
    fn generate(&self, oml_objects: &[OmlObject], file_name: &str) -> Result<String, Box<dyn Error>> {
        let mut ts_file = String::new();

        writeln!(ts_file, "// This file has been generated from {}.oml", file_name)?;
        writeln!(ts_file)?;

        for (i, oml_object) in oml_objects.iter().enumerate() {
            match &oml_object.oml_type {
                ObjectType::ENUM => generate_enum(oml_object, &mut ts_file)?,
                ObjectType::CLASS => generate_class(oml_object, &mut ts_file)?,
                // TypeScript has no struct keyword; structs map to classes
                ObjectType::STRUCT => generate_class(oml_object, &mut ts_file)?,
                ObjectType::UNDECIDED => return Err("Cannot generate code for UNDECIDED object type".into()),
            }
            if i < oml_objects.len() - 1 {
                writeln!(ts_file)?;
            }
        }

        Ok(ts_file)
    }

    fn extension(&self) -> &str {
        "ts"
    }
}

fn generate_enum(oml_object: &OmlObject, ts_file: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(ts_file, "export enum {} {{", oml_object.name)?;
    let length = oml_object.variables.len();

    for (index, var) in oml_object.variables.iter().enumerate() {
        let name = var.name.to_uppercase();
        write!(ts_file, "\t{} = \"{}\"", name, name)?;
        if index == length - 1 {
            writeln!(ts_file)?;
        } else {
            writeln!(ts_file, ",")?;
        }
    }

    writeln!(ts_file, "}}")?;

    Ok(())
}

fn generate_class(
    oml_object: &OmlObject,
    ts_file: &mut String,
) -> Result<(), std::fmt::Error> {
    writeln!(ts_file, "export class {} {{", oml_object.name)?;

    if oml_object.variables.is_empty() {
        writeln!(ts_file, "}}")?;
        return Ok(());
    }

    // Emit field declarations
    for var in &oml_object.variables {
        write_field(var, ts_file)?;
    }

    writeln!(ts_file)?;

    // Emit constructor
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

        write!(ts_file, "\tconstructor(")?;
        for var in &required {
            let ts_type = type_annotation(&var.var_type, &var.array_kind);
            write!(ts_file, "{}: {}", var.name, ts_type)?;
            index += 1;
            if index < total { write!(ts_file, ", ")?; }
        }
        for var in &optional {
            let ts_type = type_annotation(&var.var_type, &var.array_kind);
            write!(ts_file, "{}: {} | null = null", var.name, ts_type)?;
            index += 1;
            if index < total { write!(ts_file, ", ")?; }
        }
        writeln!(ts_file, ") {{")?;

        for var in &required {
            writeln!(ts_file, "\t\tthis.{} = {};", var.name, var.name)?;
        }
        for var in &optional {
            writeln!(ts_file, "\t\tthis.{} = {};", var.name, var.name)?;
        }

        writeln!(ts_file, "\t}}")?;
    }

    writeln!(ts_file, "}}")?;

    Ok(())
}

/// Writes a single class field declaration.
fn write_field(var: &Variable, ts_file: &mut String) -> Result<(), std::fmt::Error> {
    write!(ts_file, "\t")?;

    // Visibility
    match var.visibility {
        VariableVisibility::PRIVATE => write!(ts_file, "private ")?,
        VariableVisibility::PROTECTED => write!(ts_file, "protected ")?,
        VariableVisibility::PUBLIC => write!(ts_file, "public ")?,
    }

    // static modifier
    if var.var_mod.contains(&VariableModifier::STATIC) {
        write!(ts_file, "static ")?;
    }

    // readonly for const (without mut override)
    if var.var_mod.contains(&VariableModifier::CONST)
        && !var.var_mod.contains(&VariableModifier::MUT)
    {
        write!(ts_file, "readonly ")?;
    }

    let ts_type = type_annotation(&var.var_type, &var.array_kind);

    if var.var_mod.contains(&VariableModifier::OPTIONAL) {
        writeln!(ts_file, "{}?: {} | null;", var.name, ts_type)?;
    } else {
        writeln!(ts_file, "{}: {};", var.name, ts_type)?;
    }

    Ok(())
}

#[inline]
fn convert_type(var_type: &str) -> String {
    match var_type {
        "int8" | "int16" | "int32" | "int64"
        | "uint8" | "uint16" | "uint32" | "uint64"
        | "float" | "double" => "number".to_string(),
        "bool" => "boolean".to_string(),
        "string" | "char" => "string".to_string(),
        other => other.to_string(),
    }
}

fn type_annotation(var_type: &str, array_kind: &ArrayKind) -> String {
    let base = convert_type(var_type);
    match array_kind {
        ArrayKind::None => base,
        // TypeScript has no fixed-size array type; use a tuple-like annotation with a comment
        ArrayKind::Static(n) => format!("{0}[] /* [{1}] */", base, n),
        ArrayKind::Dynamic => format!("{}[]", base),
    }
}
