use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::core::generate::{Generate, BackwardsGenerate};
use std::error::Error;
use std::fmt::Write;

pub struct TypescriptGenerator;

impl BackwardsGenerate for TypescriptGenerator {
    fn reverse(&self, content: &str) -> Result<Vec<OmlObject>, Box<dyn Error>> {
        let mut objects = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("export enum ") && trimmed.ends_with('{') {
                let name = trimmed
                    .strip_prefix("export enum ")
                    .unwrap()
                    .trim_end_matches(|c: char| c == '{' || c == ' ')
                    .to_string();
                let mut vars = Vec::new();
                i += 1;
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line == "}" { break; }
                    // NAME = "NAME"
                    if let Some(eq_pos) = line.find(" = ") {
                        let variant = line[..eq_pos].trim().to_string();
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
            } else if trimmed.starts_with("export class ") && trimmed.ends_with('{') {
                let name = trimmed
                    .strip_prefix("export class ")
                    .unwrap()
                    .trim_end_matches(|c: char| c == '{' || c == ' ')
                    .to_string();
                let mut vars = Vec::new();
                i += 1;
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line == "}" { break; }
                    // Stop at constructor
                    if line.starts_with("constructor(") { break; }
                    if line.ends_with(';') && !line.contains('(') {
                        if let Some(var) = parse_ts_field(line) {
                            vars.push(var);
                        }
                    }
                    i += 1;
                }
                // Skip to end of class
                let mut brace_depth = 1;
                while i < lines.len() && brace_depth > 0 {
                    let line = lines[i].trim();
                    brace_depth += line.matches('{').count();
                    brace_depth -= line.matches('}').count();
                    i += 1;
                }
                objects.push(OmlObject {
                    oml_type: ObjectType::CLASS,
                    name,
                    variables: vars,
                });
                continue;
            }
            i += 1;
        }

        Ok(objects)
    }
}

fn reverse_ts_type(ts_type: &str) -> String {
    match ts_type {
        "number" => "int32".to_string(),
        "boolean" => "bool".to_string(),
        "string" => "string".to_string(),
        other => other.to_string(),
    }
}

fn parse_ts_field(line: &str) -> Option<Variable> {
    let line = line.trim().trim_end_matches(';').trim();
    if line.is_empty() { return None; }

    let mut rest = line;
    let mut visibility = VariableVisibility::PRIVATE;
    let mut var_mod = Vec::new();

    if rest.starts_with("private ") {
        visibility = VariableVisibility::PRIVATE;
        rest = &rest[8..];
    } else if rest.starts_with("protected ") {
        visibility = VariableVisibility::PROTECTED;
        rest = &rest[10..];
    } else if rest.starts_with("public ") {
        visibility = VariableVisibility::PUBLIC;
        rest = &rest[7..];
    }

    if rest.starts_with("static ") {
        var_mod.push(VariableModifier::STATIC);
        rest = &rest[7..];
    }

    if rest.starts_with("readonly ") {
        var_mod.push(VariableModifier::CONST);
        rest = &rest[9..];
    }

    // Handle optional: "name?: Type | null"
    let is_optional;
    let colon_pos;
    if let Some(qpos) = rest.find("?: ") {
        is_optional = true;
        colon_pos = qpos;
    } else if let Some(cpos) = rest.find(": ") {
        is_optional = false;
        colon_pos = cpos;
    } else {
        return None;
    }

    let name = rest[..colon_pos].trim().to_string();
    let type_part = if is_optional {
        rest[colon_pos + 3..].trim()
    } else {
        rest[colon_pos + 2..].trim()
    };

    // Remove "| null" suffix
    let type_part = type_part.trim_end_matches("| null").trim();

    // Handle array: "type[]" or "type[] /* [N] */"
    let (var_type, array_kind) = if type_part.contains("[]") {
        let base = type_part.split("[]").next().unwrap().trim();
        // Check for static size comment
        if let Some(start) = type_part.find("/* [") {
            let end = type_part[start..].find("] */");
            if let Some(e) = end {
                if let Ok(n) = type_part[start + 4..start + e].parse::<u32>() {
                    (reverse_ts_type(base), ArrayKind::Static(n))
                } else {
                    (reverse_ts_type(base), ArrayKind::Dynamic)
                }
            } else {
                (reverse_ts_type(base), ArrayKind::Dynamic)
            }
        } else {
            (reverse_ts_type(base), ArrayKind::Dynamic)
        }
    } else {
        (reverse_ts_type(type_part), ArrayKind::None)
    };

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
