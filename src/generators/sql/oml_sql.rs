use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableModifier, ArrayKind
};
use crate::core::generate::Generate;
use std::error::Error;
use std::fmt::Write;

pub struct SqlGenerator;

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
