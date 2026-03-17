use crate::core::oml_object::{
    OmlObject, ObjectType, VariableModifier, VariableVisibility, ArrayKind
};
use crate::core::generate::Generate;
use std::error::Error;
use std::fmt::Write;

/// Generator that outputs OML syntax from OmlObjects.
/// Implements the Generate trait so it can be used in the same pipeline
/// as the language generators.
pub struct OmlGenerator;

impl Generate for OmlGenerator {
    fn generate(&self, oml_objects: &[OmlObject], _file_name: &str) -> Result<String, Box<dyn Error>> {
        let mut oml_file = String::new();

        for (i, obj) in oml_objects.iter().enumerate() {
            match &obj.oml_type {
                ObjectType::ENUM => generate_enum(obj, &mut oml_file)?,
                ObjectType::CLASS => generate_class(obj, &mut oml_file)?,
                ObjectType::STRUCT => generate_struct(obj, &mut oml_file)?,
                ObjectType::UNDECIDED => return Err("Cannot generate OML for UNDECIDED object type".into()),
            }
            if i < oml_objects.len() - 1 {
                writeln!(oml_file)?;
            }
        }

        Ok(oml_file)
    }

    fn extension(&self) -> &str {
        "oml"
    }
}

fn generate_enum(obj: &OmlObject, out: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(out, "enum {} {{", obj.name)?;
    for var in &obj.variables {
        writeln!(out, "    {} {};", var.var_type, var.name)?;
    }
    writeln!(out, "}}")?;
    Ok(())
}

fn generate_class(obj: &OmlObject, out: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(out, "class {} {{", obj.name)?;
    write_variables(obj, out)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn generate_struct(obj: &OmlObject, out: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(out, "struct {} {{", obj.name)?;
    write_variables(obj, out)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_variables(obj: &OmlObject, out: &mut String) -> Result<(), std::fmt::Error> {
    for var in &obj.variables {
        write!(out, "    ")?;

        // Visibility (private is default, omit it)
        match var.visibility {
            VariableVisibility::PUBLIC => write!(out, "public ")?,
            VariableVisibility::PROTECTED => write!(out, "protected ")?,
            VariableVisibility::PRIVATE => {},
        }

        // Modifiers
        for m in &var.var_mod {
            match m {
                VariableModifier::CONST => write!(out, "const ")?,
                VariableModifier::MUT => write!(out, "mut ")?,
                VariableModifier::STATIC => write!(out, "static ")?,
                VariableModifier::OPTIONAL => write!(out, "optional ")?,
            }
        }

        // Type with array kind
        match &var.array_kind {
            ArrayKind::None => write!(out, "{}", var.var_type)?,
            ArrayKind::Static(n) => write!(out, "{}[{}]", var.var_type, n)?,
            ArrayKind::Dynamic => write!(out, "list {}", var.var_type)?,
        }

        writeln!(out, " {};", var.name)?;
    }
    Ok(())
}
