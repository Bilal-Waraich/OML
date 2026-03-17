use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::core::generate::{Generate, BackwardsGenerate};
use std::error::Error;
use std::fmt::Write;

pub struct SqlGenerator;

impl BackwardsGenerate for SqlGenerator {
    fn reverse(&self, content: &str) -> Result<Vec<OmlObject>, Box<dyn Error>> {
        let mut objects = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("CREATE TABLE ") {
                let name = trimmed
                    .strip_prefix("CREATE TABLE ")
                    .unwrap()
                    .trim_end_matches(|c: char| c == '(' || c == ' ')
                    .to_string();

                // Check if this is an enum (lookup table) by looking for INSERT with name values
                let mut vars = Vec::new();
                let mut is_enum = false;
                i += 1;

                // Collect column definitions
                let mut columns = Vec::new();
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line.starts_with(");") { break; }
                    if !line.starts_with("CONSTRAINT") && !line.is_empty() {
                        columns.push(line.to_string());
                    }
                    i += 1;
                }

                // Check for INSERT INTO (enum pattern)
                i += 1;
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line.is_empty() { i += 1; continue; }
                    if line.starts_with(&format!("INSERT INTO {} (name) VALUES", name)) {
                        is_enum = true;
                        // Parse enum values
                        let values_start = line.find("VALUES").unwrap() + 6;
                        let values_str = &line[values_start..].trim_end_matches(';');
                        for val in values_str.split("),(") {
                            let clean = val.trim().trim_matches(|c: char| c == '(' || c == ')' || c == '\'').trim();
                            if !clean.is_empty() {
                                vars.push(Variable {
                                    var_mod: vec![],
                                    visibility: VariableVisibility::PUBLIC,
                                    var_type: "string".to_string(),
                                    array_kind: ArrayKind::None,
                                    name: clean.to_string(),
                                });
                            }
                        }
                    }
                    break;
                }

                if is_enum {
                    objects.push(OmlObject {
                        oml_type: ObjectType::ENUM,
                        name,
                        variables: vars,
                    });
                } else {
                    // Parse as struct from columns
                    for col in &columns {
                        if let Some(var) = parse_sql_column(col) {
                            vars.push(var);
                        }
                    }
                    objects.push(OmlObject {
                        oml_type: ObjectType::STRUCT,
                        name,
                        variables: vars,
                    });
                }
                continue;
            }
            i += 1;
        }

        Ok(objects)
    }
}

fn reverse_sql_type(sql_type: &str) -> String {
    match sql_type {
        "TINYINT" => "int8".to_string(),
        "SMALLINT" => "int16".to_string(),
        "INT" => "int32".to_string(),
        "BIGINT" => "int64".to_string(),
        "TINYINT UNSIGNED" => "uint8".to_string(),
        "SMALLINT UNSIGNED" => "uint16".to_string(),
        "INT UNSIGNED" => "uint32".to_string(),
        "BIGINT UNSIGNED" => "uint64".to_string(),
        "FLOAT" => "float".to_string(),
        "DOUBLE" => "double".to_string(),
        "BOOLEAN" => "bool".to_string(),
        "TEXT" => "string".to_string(),
        "CHAR(1)" => "char".to_string(),
        "VARCHAR(255)" => "string".to_string(),
        _ => "int32".to_string(),
    }
}

fn parse_sql_column(line: &str) -> Option<Variable> {
    let line = line.trim().trim_end_matches(',');
    if line.is_empty() { return None; }
    // Skip id column
    if line.starts_with("id ") { return None; }

    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 2 { return None; }

    let name = tokens[0].to_string();
    // Reconstruct the SQL type (might be multi-word like "TINYINT UNSIGNED")
    let mut type_end = 1;
    if tokens.len() > 2 && tokens[2] != "NOT" && tokens[2] != "NULL" {
        // Check for "UNSIGNED"
        if tokens[type_end] == "UNSIGNED" || (tokens.len() > type_end + 1 && tokens[type_end + 1] == "UNSIGNED") {
            // multi-word type
        }
    }

    let sql_type_str;
    if tokens.len() > 2 && tokens[2] == "UNSIGNED" {
        sql_type_str = format!("{} UNSIGNED", tokens[1]);
        type_end = 3;
    } else {
        sql_type_str = tokens[1].to_string();
        type_end = 2;
    }

    let is_optional = tokens[type_end..].contains(&"NULL") && !tokens[type_end..].contains(&"NOT");
    let mut var_mod = Vec::new();
    if is_optional {
        var_mod.push(VariableModifier::OPTIONAL);
    }

    Some(Variable {
        var_mod,
        visibility: VariableVisibility::PRIVATE,
        var_type: reverse_sql_type(&sql_type_str),
        array_kind: ArrayKind::None,
        name,
    })
}

impl Generate for SqlGenerator {
    fn generate(&self, oml_objects: &[OmlObject], file_name: &str) -> Result<String, Box<dyn Error>> {
        let mut sql_file = String::new();

        writeln!(sql_file, "-- This file has been generated from {}.oml", file_name)?;
        writeln!(sql_file)?;

        for (i, oml_object) in oml_objects.iter().enumerate() {
            match &oml_object.oml_type {
                // ENUMs become lookup tables with a single value column
                ObjectType::ENUM => generate_enum_table(oml_object, &mut sql_file)?,
                ObjectType::CLASS | ObjectType::STRUCT => generate_table(oml_object, &mut sql_file)?,
                ObjectType::UNDECIDED => return Err("Cannot generate SQL for UNDECIDED object type".into()),
            }
            if i < oml_objects.len() - 1 {
                writeln!(sql_file)?;
            }
        }

        Ok(sql_file)
    }

    fn extension(&self) -> &str {
        "sql"
    }
}

/// Generates a simple lookup table for an OML enum.
///
/// Example output:
/// ```sql
/// CREATE TABLE Color (
///     id   INT          NOT NULL AUTO_INCREMENT PRIMARY KEY,
///     name VARCHAR(255) NOT NULL
/// );
/// INSERT INTO Color (name) VALUES ('RED'), ('GREEN'), ('BLUE');
/// ```
fn generate_enum_table(oml_object: &OmlObject, sql_file: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(sql_file, "CREATE TABLE {} (", oml_object.name)?;
    writeln!(sql_file, "\tid   INT          NOT NULL AUTO_INCREMENT PRIMARY KEY,")?;
    writeln!(sql_file, "\tname VARCHAR(255) NOT NULL")?;
    writeln!(sql_file, ");")?;

    if !oml_object.variables.is_empty() {
        writeln!(sql_file)?;
        write!(sql_file, "INSERT INTO {} (name) VALUES", oml_object.name)?;
        let length = oml_object.variables.len();
        for (index, var) in oml_object.variables.iter().enumerate() {
            write!(sql_file, " ('{}')", var.name.to_uppercase())?;
            if index < length - 1 {
                write!(sql_file, ",")?;
            }
        }
        writeln!(sql_file, ";")?;
    }

    Ok(())
}

/// Generates a `CREATE TABLE` statement for an OML class or struct.
///
/// Dynamic arrays (`list T`) produce a separate junction table.
/// Static arrays (`T[N]`) produce N individual columns (e.g. `col_0`, `col_1`, …).
fn generate_table(
    oml_object: &OmlObject,
    sql_file: &mut String,
) -> Result<(), std::fmt::Error> {
    // Collect inline columns (non-dynamic-array fields)
    let inline_vars: Vec<&Variable> = oml_object.variables
        .iter()
        .filter(|v| v.array_kind != ArrayKind::Dynamic)
        .collect();

    // Collect dynamic-array fields that need a junction table
    let list_vars: Vec<&Variable> = oml_object.variables
        .iter()
        .filter(|v| v.array_kind == ArrayKind::Dynamic)
        .collect();

    writeln!(sql_file, "CREATE TABLE {} (", oml_object.name)?;
    writeln!(sql_file, "\tid INT NOT NULL AUTO_INCREMENT PRIMARY KEY,")?;

    for var in &inline_vars {
        match &var.array_kind {
            ArrayKind::Static(n) => {
                // Expand static arrays into N individual columns
                for i in 0..*n {
                    let is_optional = var.var_mod.contains(&VariableModifier::OPTIONAL);
                    let null_str = if is_optional { "NULL" } else { "NOT NULL" };
                    writeln!(
                        sql_file,
                        "\t{}_{} {} {},",
                        var.name, i,
                        convert_type(&var.var_type),
                        null_str
                    )?;
                }
            }
            ArrayKind::None => {
                let is_optional = var.var_mod.contains(&VariableModifier::OPTIONAL);
                let null_str = if is_optional { "NULL" } else { "NOT NULL" };
                writeln!(
                    sql_file,
                    "\t{} {} {},",
                    var.name,
                    convert_type(&var.var_type),
                    null_str
                )?;
            }
            ArrayKind::Dynamic => unreachable!(),
        }
    }

    // Remove the trailing comma from the last column line if we need to close cleanly.
    // Approach: always emit trailing commas above, then add a closing line without one.
    // The PRIMARY KEY line above serves as the last "guaranteed" line; the field lines
    // all get a trailing comma which is fine because at minimum `id` is always present.
    writeln!(sql_file, "\tCONSTRAINT pk_{} PRIMARY KEY (id)", oml_object.name)?;
    writeln!(sql_file, ");")?;

    // Junction tables for dynamic-array fields
    for var in &list_vars {
        writeln!(sql_file)?;
        let junction_name = format!("{}_{}", oml_object.name, var.name);
        writeln!(sql_file, "-- Junction table for {}.{} (list {})", oml_object.name, var.name, var.var_type)?;
        writeln!(sql_file, "CREATE TABLE {} (", junction_name)?;
        writeln!(sql_file, "\tid         INT NOT NULL AUTO_INCREMENT PRIMARY KEY,")?;
        writeln!(sql_file, "\tparent_id  INT NOT NULL,")?;
        writeln!(sql_file, "\tvalue      {} NOT NULL,", convert_type(&var.var_type))?;
        writeln!(sql_file, "\tCONSTRAINT fk_{}_{} FOREIGN KEY (parent_id) REFERENCES {}(id)", junction_name, oml_object.name, oml_object.name)?;
        writeln!(sql_file, ");")?;
    }

    Ok(())
}

#[inline]
fn convert_type(var_type: &str) -> String {
    match var_type {
        "int8" => "TINYINT".to_string(),
        "int16" => "SMALLINT".to_string(),
        "int32" => "INT".to_string(),
        "int64" => "BIGINT".to_string(),
        "uint8" => "TINYINT UNSIGNED".to_string(),
        "uint16" => "SMALLINT UNSIGNED".to_string(),
        "uint32" => "INT UNSIGNED".to_string(),
        "uint64" => "BIGINT UNSIGNED".to_string(),
        "float" => "FLOAT".to_string(),
        "double" => "DOUBLE".to_string(),
        "bool" => "BOOLEAN".to_string(),
        "string" => "TEXT".to_string(),
        "char" => "CHAR(1)".to_string(),
        // Custom types: store as a foreign-key reference (INT)
        _ => "INT".to_string(),
    }
}
