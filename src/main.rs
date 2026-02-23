mod core;
mod cli;
mod generators;

use std::fs;
use std::path::Path;

use clap::Parser;
use cli::oml::OmlCli;
use crate::core::oml_object::OmlObject;

fn main() {
    let cli = OmlCli::parse();

    if !cli.has_inputs() {
        OmlCli::print_help();
        return;
    }

    let oml_files = match cli.get_files() {
        Ok(files) => files,
        Err(e) => {
            eprintln!("An error was encountered when parsing the input files: {:?}", e);
            return;
        }
    };

    if oml_files.is_empty() {
        eprintln!("No .oml files found");
        return;
    }

    // Validate custom/nested types across each file's objects
    for oml_file in &oml_files {
        if let Err(e) = OmlObject::validate_custom_types(&oml_file.objects) {
            eprintln!("Type error in {}.oml: {}", oml_file.file_name, e);
            return;
        }
    }

    let generators = cli.get_generators();

    if generators.is_empty() {
        eprintln!("No language flag specified (e.g. --cpp)");
        return;
    }

    let output_dir = Path::new(&cli.output);

    if let Err(e) = fs::create_dir_all(output_dir) {
        eprintln!("Failed to create output directory '{}': {}", cli.output, e);
        return;
    }

    for oml_file in &oml_files {
        for generator in &generators {
            match generator.generate(&oml_file.objects, &oml_file.file_name) {
                Ok(content) => {
                    let output_path = output_dir.join(
                        format!("{}.{}", oml_file.file_name, generator.extension())
                    );
                    if let Err(e) = fs::write(&output_path, &content) {
                        eprintln!("Failed to write {}: {}", output_path.display(), e);
                    } else {
                        println!("Generated {}", output_path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Failed to generate {} for {}: {}", generator.extension(), oml_file.file_name, e);
                }
            }
        }
    }
}
