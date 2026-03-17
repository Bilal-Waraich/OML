use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::core::generate::Generate;
use std::error::Error;
use std::fmt::Write;

pub struct RustGenerator;

impl Generate for RustGenerator {
    fn generate(&self, oml_objects: &[OmlObject], file_name: &str) -> Result<String, Box<dyn Error>> {
        let mut rs_file = String::new();

        writeln!(rs_file, "// This file has been generated from {}.oml", file_name)?;
        writeln!(rs_file)?;

        // Emit `#[allow(dead_code)]` once at the top to suppress unused-field warnings
        // on generated code that the user may not have wired up yet.
        writeln!(rs_file, "#[allow(dead_code)]")?;
        writeln!(rs_file)?;

        for (i, oml_object) in oml_objects.iter().enumerate() {
            match &oml_object.oml_type {
                ObjectType::ENUM => generate_enum(oml_object, &mut rs_file)?,
                ObjectType::CLASS | ObjectType::STRUCT => generate_struct(oml_object, &mut rs_file)?,
                ObjectType::UNDECIDED => return Err("Cannot generate code for UNDECIDED object type".into()),
            }
            if i < oml_objects.len() - 1 {
                writeln!(rs_file)?;
            }
        }

        Ok(rs_file)
    }

    fn extension(&self) -> &str {
        "rs"
    }
}

fn generate_enum(oml_object: &OmlObject, rs_file: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(rs_file, "#[derive(Debug, Clone, PartialEq)]")?;
    writeln!(rs_file, "pub enum {} {{", oml_object.name)?;

    for var in &oml_object.variables {
        // Capitalise first letter to match Rust enum variant convention
        let name = capitalise(&var.name);
        writeln!(rs_file, "\t{},", name)?;
    }

    writeln!(rs_file, "}}")?;

    Ok(())
}

fn generate_struct(
    oml_object: &OmlObject,
    rs_file: &mut String,
) -> Result<(), std::fmt::Error> {
    // Separate static (associated-const) vars from regular fields
    let static_vars: Vec<&Variable> = oml_object.variables
        .iter()
        .filter(|v| v.var_mod.contains(&VariableModifier::STATIC))
        .collect();

    let field_vars: Vec<&Variable> = oml_object.variables
        .iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::STATIC))
        .collect();

    writeln!(rs_file, "#[derive(Debug, Clone)]")?;
    writeln!(rs_file, "pub struct {} {{", oml_object.name)?;

    for var in &field_vars {
        write_field(var, rs_file)?;
    }

    writeln!(rs_file, "}}")?;

    // Emit associated constants in an impl block for static vars
    if !static_vars.is_empty() {
        writeln!(rs_file)?;
        writeln!(rs_file, "impl {} {{", oml_object.name)?;
        for var in &static_vars {
            write_associated_const(var, rs_file)?;
        }
        writeln!(rs_file, "}}")?;
    }

    Ok(())
}

/// Writes a single struct field.
fn write_field(var: &Variable, rs_file: &mut String) -> Result<(), std::fmt::Error> {
    write!(rs_file, "\t")?;

    // In Rust, `pub` / `pub(crate)` / (private) map to PUBLIC / PROTECTED / PRIVATE
    match var.visibility {
        VariableVisibility::PUBLIC => write!(rs_file, "pub ")?,
        VariableVisibility::PROTECTED => write!(rs_file, "pub(crate) ")?,
        VariableVisibility::PRIVATE => {},
    }

    let rs_type = type_annotation(&var.var_type, &var.array_kind, var.var_mod.contains(&VariableModifier::OPTIONAL));

    writeln!(rs_file, "{}: {},", var.name, rs_type)?;

    Ok(())
}

/// Emits a static variable as an associated constant in an `impl` block.
fn write_associated_const(var: &Variable, rs_file: &mut String) -> Result<(), std::fmt::Error> {
    let vis = match var.visibility {
        VariableVisibility::PUBLIC => "pub ",
        VariableVisibility::PROTECTED => "pub(crate) ",
        VariableVisibility::PRIVATE => "",
    };

    let rs_type = type_annotation(&var.var_type, &var.array_kind, var.var_mod.contains(&VariableModifier::OPTIONAL));

    // Const fields use `const`, mutable statics use `static mut` (unsafe in Rust).
    // We default to a placeholder comment when the value is unknown.
    if var.var_mod.contains(&VariableModifier::CONST) && !var.var_mod.contains(&VariableModifier::MUT) {
        writeln!(rs_file, "\t{}const {}: {} = todo!();", vis, var.name.to_uppercase(), rs_type)?;
    } else {
        // Static mutable fields are inherently unsafe in Rust; emit a warning comment.
        writeln!(rs_file, "\t// SAFETY: mutable static — initialise before use")?;
        writeln!(rs_file, "\t{}static mut {}: {} = todo!();", vis, var.name.to_uppercase(), rs_type)?;
    }

    Ok(())
}

#[inline]
fn convert_type(var_type: &str) -> String {
    match var_type {
        "int8" => "i8".to_string(),
        "int16" => "i16".to_string(),
        "int32" => "i32".to_string(),
        "int64" => "i64".to_string(),
        "uint8" => "u8".to_string(),
        "uint16" => "u16".to_string(),
        "uint32" => "u32".to_string(),
        "uint64" => "u64".to_string(),
        "float" => "f32".to_string(),
        "double" => "f64".to_string(),
        "bool" => "bool".to_string(),
        "string" => "String".to_string(),
        "char" => "char".to_string(),
        other => other.to_string(),
    }
}

fn type_annotation(var_type: &str, array_kind: &ArrayKind, is_optional: bool) -> String {
    let base = convert_type(var_type);
    let with_array = match array_kind {
        ArrayKind::None => base,
        ArrayKind::Static(n) => format!("[{}; {}]", base, n),
        ArrayKind::Dynamic => format!("Vec<{}>", base),
    };

    if is_optional {
        format!("Option<{}>", with_array)
    } else {
        with_array
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
