use clap::{Parser, CommandFactory, Subcommand};
use crate::core::errors;
use crate::core::dir_parser::parse_dir_from_string;
use crate::core::generate::{Generate, BackwardsGenerate};
use crate::core::oml_object::OmlFile;

use crate::generators::{
    cpp::oml_cpp::CppGenerator,
    java::oml_java::JavaGenerator,
    kotlin::oml_kotlin::KotlinGenerator,
    python::oml_python::PythonGenerator,
    rust::oml_rust::RustGenerator,
    sql::oml_sql::SqlGenerator,
    typescript::oml_typescript::TypescriptGenerator,
};

#[derive(Parser)]
#[command(name = "oml")]
#[command(about = "Parse OML files and generate code from .oml definitions", long_about = None)]
pub struct OmlCli {

    #[command(subcommand)]
    pub command: Option<Commands>,

    // names of the directories / oml files
    inputs: Option<Vec<String>>,

    #[arg(short, long, default_value = "./oml_output")]
    pub output: String,

    // if oml should check files within folders recursively
    #[arg(short, long)]
    recursive: bool,

    #[arg(short, long, default_value_t = 3)]
    depth: usize,

    #[arg(long)]
    use_data_class: bool,

    // language conversions

    #[arg(long)]
    cpp: bool,

    #[arg(long)]
    python: bool,

    #[arg(long)]
    java: bool,

    #[arg(long)]
    kotlin: bool,

    #[arg(long)]
    rust: bool,

    #[arg(long)]
    typescript: bool,

    #[arg(long)]
    sql: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Revert generated files back to OML format
    Revert {
        /// Files to convert back to OML (e.g. file.cpp, file.kt, file.py)
        files: Vec<String>,

        /// Output directory for the generated .oml files
        #[arg(short, long, default_value = "./oml_output")]
        output: String,
    },
}

impl OmlCli {
    pub fn has_inputs(&self) -> bool {
        self.inputs.is_some()
    }

    pub fn print_help() {
        Self::command().print_help().unwrap();
        println!();
    }

    pub fn get_files(&self) -> Result<Vec<OmlFile>, errors::ParseError> {
        let input_files = match &self.inputs {
            Some(inputs) => inputs,
            None => {
                return Err(errors::ParseError::InvalidPath);
            }
        };

        let mut files = Vec::new();

        for file_name in input_files {
            let mut parsed = parse_dir_from_string(file_name.clone(), self.depth)?;
            files.append(&mut parsed);
        }

        Ok(files)
    }

    pub fn get_generators(&self) -> Vec<Box<dyn Generate>> {
        let mut generators: Vec<Box<dyn Generate>> = Vec::new();

        if self.cpp {
            generators.push(Box::new(CppGenerator));
        }

        if self.python {
            generators.push(Box::new(PythonGenerator::new(self.use_data_class)));
        }
        if self.kotlin {
            generators.push(Box::new(KotlinGenerator::new(self.use_data_class)));
        }


        if self.java {
            generators.push(Box::new(JavaGenerator));
        }
        if self.rust {
            generators.push(Box::new(RustGenerator));
        }
        if self.typescript {
            generators.push(Box::new(TypescriptGenerator));
        }
        if self.sql {
            generators.push(Box::new(SqlGenerator));
        }

        generators
    }
}

/// Returns the appropriate backwards generator for a file based on its extension.
pub fn get_backwards_generator(extension: &str) -> Option<Box<dyn BackwardsGenerate>> {
    match extension {
        "rs" => Some(Box::new(RustGenerator)),
        "kt" => Some(Box::new(KotlinGenerator::new(false))),
        "cpp" | "h" => Some(Box::new(CppGenerator)),
        "py" => Some(Box::new(PythonGenerator::new(false))),
        "java" => Some(Box::new(JavaGenerator)),
        "ts" => Some(Box::new(TypescriptGenerator)),
        "sql" => Some(Box::new(SqlGenerator)),
        _ => None,
    }
}
