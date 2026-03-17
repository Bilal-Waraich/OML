use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::core::generate::{Generate, BackwardsGenerate};
use std::error::Error;
use std::fmt::Write;

pub struct RustGenerator;

impl BackwardsGenerate for RustGenerator {
    fn reverse(&self, content: &str) -> Result<Vec<OmlObject>, Box<dyn Error>> {
        let mut objects = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("pub enum ") && trimmed.ends_with('{') {
                let name = trimmed
                    .strip_prefix("pub enum ")
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
            } else if trimmed.starts_with("pub struct ") && trimmed.ends_with('{') {
                let name = trimmed
                    .strip_prefix("pub struct ")
                    .unwrap()
                    .trim_end_matches(|c: char| c == '{' || c == ' ')
                    .to_string();
                let mut vars = Vec::new();
                i += 1;
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line == "}" { break; }
                    if let Some(var) = parse_rust_field(line) {
                        vars.push(var);
                    }
                    i += 1;
                }
                // Check for impl block with associated constants
                let impl_prefix = format!("impl {} {{", name);
                let mut j = i + 1;
                while j < lines.len() {
                    let line = lines[j].trim();
                    if line.starts_with(&impl_prefix) {
                        j += 1;
                        while j < lines.len() {
                            let impl_line = lines[j].trim();
                            if impl_line == "}" { break; }
                            if let Some(var) = parse_rust_associated_const(impl_line) {
                                vars.push(var);
                            }
                            j += 1;
                        }
                        break;
                    }
                    j += 1;
                }
                objects.push(OmlObject {
                    oml_type: ObjectType::STRUCT,
                    name,
                    variables: vars,
                });
            }
            i += 1;
        }

        Ok(objects)
    }
}

fn reverse_rust_type(rs_type: &str) -> String {
    match rs_type {
        "i8" => "int8".to_string(),
        "i16" => "int16".to_string(),
        "i32" => "int32".to_string(),
        "i64" => "int64".to_string(),
        "u8" => "uint8".to_string(),
        "u16" => "uint16".to_string(),
        "u32" => "uint32".to_string(),
        "u64" => "uint64".to_string(),
        "f32" => "float".to_string(),
        "f64" => "double".to_string(),
        "bool" => "bool".to_string(),
        "String" => "string".to_string(),
        "char" => "char".to_string(),
        other => other.to_string(),
    }
}

fn parse_rust_field(line: &str) -> Option<Variable> {
    let line = line.trim().trim_end_matches(',');
    if line.is_empty() { return None; }

    let mut visibility = VariableVisibility::PRIVATE;
    let mut rest = line;

    if rest.starts_with("pub(crate) ") {
        visibility = VariableVisibility::PROTECTED;
        rest = &rest[11..];
    } else if rest.starts_with("pub ") {
        visibility = VariableVisibility::PUBLIC;
        rest = &rest[4..];
    }

    // format: "name: Type"
    let colon_pos = rest.find(':')?;
    let name = rest[..colon_pos].trim().to_string();
    let type_str = rest[colon_pos + 1..].trim();

    let (var_type, array_kind, is_optional) = parse_rust_type_annotation(type_str);

    let mut var_mod = Vec::new();
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

fn parse_rust_type_annotation(type_str: &str) -> (String, ArrayKind, bool) {
    let type_str = type_str.trim();

    // Option<...>
    if type_str.starts_with("Option<") && type_str.ends_with('>') {
        let inner = &type_str[7..type_str.len() - 1];
        let (var_type, array_kind, _) = parse_rust_type_annotation(inner);
        return (var_type, array_kind, true);
    }

    // Vec<T>
    if type_str.starts_with("Vec<") && type_str.ends_with('>') {
        let inner = &type_str[4..type_str.len() - 1];
        return (reverse_rust_type(inner), ArrayKind::Dynamic, false);
    }

    // [T; N]
    if type_str.starts_with('[') && type_str.ends_with(']') {
        let inner = &type_str[1..type_str.len() - 1];
        if let Some(semi_pos) = inner.find(';') {
            let elem_type = inner[..semi_pos].trim();
            let size_str = inner[semi_pos + 1..].trim();
            if let Ok(size) = size_str.parse::<u32>() {
                return (reverse_rust_type(elem_type), ArrayKind::Static(size), false);
            }
        }
    }

    (reverse_rust_type(type_str), ArrayKind::None, false)
}

fn parse_rust_associated_const(line: &str) -> Option<Variable> {
    let line = line.trim();
    if line.starts_with("//") { return None; }

    let mut visibility = VariableVisibility::PRIVATE;
    let mut rest = line;
    let mut var_mod = Vec::new();

    if rest.starts_with("pub(crate) ") {
        visibility = VariableVisibility::PROTECTED;
        rest = &rest[11..];
    } else if rest.starts_with("pub ") {
        visibility = VariableVisibility::PUBLIC;
        rest = &rest[4..];
    }

    if rest.starts_with("const ") {
        var_mod.push(VariableModifier::STATIC);
        var_mod.push(VariableModifier::CONST);
        rest = &rest[6..];
    } else if rest.starts_with("static mut ") {
        var_mod.push(VariableModifier::STATIC);
        var_mod.push(VariableModifier::MUT);
        rest = &rest[11..];
    } else {
        return None;
    }

    // "NAME: Type = todo!();"
    let colon_pos = rest.find(':')?;
    let name = rest[..colon_pos].trim().to_lowercase();
    let after_colon = rest[colon_pos + 1..].trim();
    let eq_pos = after_colon.find('=')?;
    let type_str = after_colon[..eq_pos].trim();

    let (var_type, array_kind, is_optional) = parse_rust_type_annotation(type_str);
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
