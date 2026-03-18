mod core;
mod cli;
mod generators;

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use clap::Parser;
use cli::oml::{OmlCli, Commands, get_backwards_generator, get_generators_from_flags};
use crate::core::import_resolver::resolve_all;
use crate::core::oml_object::OmlObject;
use crate::core::backwards_converting::OmlGenerator;
use crate::core::generate::Generate;

fn main() {
    let cli = OmlCli::parse();

    // Handle subcommands
    if let Some(command) = &cli.command {
        match command {
            Commands::Revert { files, output } => {
                handle_revert(files, output);
                return;
            }
            Commands::Translate { files, output, cpp, python, java, kotlin, rust, typescript, sql, use_data_class } => {
                handle_translate(files, output, *cpp, *python, *java, *kotlin, *rust, *typescript, *sql, *use_data_class);
                return;
            }
        }
    }

    if !cli.has_inputs() {
        OmlCli::print_help();
        return;
    }

    let root_files = match cli.get_files() {
        Ok(files) => files,
        Err(e) => {
            eprintln!("An error was encountered when parsing the input files: {:?}", e);
            return;
        }
    };

    if root_files.is_empty() {
        eprintln!("No .oml files found");
        return;
    }

    // Remember which paths are "root" files so we only generate code for them,
    // not for files that were pulled in transitively via imports.
    let root_paths: HashSet<_> = root_files.iter().map(|f| f.path.clone()).collect();

    // Resolve all imports (parses imported files, detects cycles).
    let (all_files, imported_names) = match resolve_all(root_files) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Import error: {}", e);
            return;
        }
    };

    // Validate custom/nested types for every file, taking imports into account.
    for oml_file in &all_files {
        let extra = imported_names
            .get(&oml_file.path)
            .cloned()
            .unwrap_or_default();
        if let Err(e) = OmlObject::validate_custom_types(&oml_file.objects, &extra) {
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

    // Only generate code for the files the user explicitly passed in.
    for oml_file in all_files.iter().filter(|f| root_paths.contains(&f.path)) {
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

fn handle_translate(
    files: &[String], output: &str,
    cpp: bool, python: bool, java: bool, kotlin: bool,
    rust: bool, typescript: bool, sql: bool, use_data_class: bool,
) {
    if files.is_empty() {
        eprintln!("No files specified for translate");
        return;
    }

    let generators = get_generators_from_flags(cpp, python, java, kotlin, rust, typescript, sql, use_data_class);
    if generators.is_empty() {
        eprintln!("No target language specified (e.g. --java)");
        return;
    }

    let output_dir = Path::new(output);
    if let Err(e) = fs::create_dir_all(output_dir) {
        eprintln!("Failed to create output directory '{}': {}", output, e);
        return;
    }

    // Resolve inputs: expand directories into individual supported files
    let resolved = resolve_translate_inputs(files);
    if resolved.is_empty() {
        eprintln!("No supported source files found");
        return;
    }

    for file_path in &resolved {
        let path = Path::new(file_path);

        let extension = path.extension().and_then(|e| e.to_str()).unwrap();

        let backwards_gen = match get_backwards_generator(extension) {
            Some(g) => g,
            None => {
                eprintln!("Unsupported source file type '.{}' for translate", extension);
                continue;
            }
        };

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read '{}': {}", file_path, e);
                continue;
            }
        };

        let oml_objects: Vec<OmlObject> = match backwards_gen.reverse(&content) {
            Ok(objects) => objects,
            Err(e) => {
                eprintln!("Failed to parse '{}': {}", file_path, e);
                continue;
            }
        };

        if oml_objects.is_empty() {
            eprintln!("No objects found in '{}'", file_path);
            continue;
        }

        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        for generator in &generators {
            match generator.generate(&oml_objects, file_stem) {
                Ok(generated) => {
                    let output_path = output_dir.join(
                        format!("{}.{}", file_stem, generator.extension())
                    );
                    if let Err(e) = fs::write(&output_path, &generated) {
                        eprintln!("Failed to write {}: {}", output_path.display(), e);
                    } else {
                        println!("Translated {} -> {}", file_path, output_path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Failed to generate {} for '{}': {}", generator.extension(), file_path, e);
                }
            }
        }
    }
}

const SUPPORTED_EXTENSIONS: &[&str] = &["rs", "kt", "cpp", "h", "py", "java", "ts", "sql"];

fn is_supported_source(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

fn resolve_translate_inputs(inputs: &[String]) -> Vec<String> {
    let mut resolved = Vec::new();
    for input in inputs {
        let path = Path::new(input);
        if path.is_file() {
            if is_supported_source(path) {
                resolved.push(input.clone());
            } else {
                eprintln!("Skipping unsupported file: {}", input);
            }
        } else if path.is_dir() {
            collect_supported_files(path, &mut resolved);
        } else {
            eprintln!("Path not found: {}", input);
        }
    }
    resolved
}

fn collect_supported_files(dir: &Path, out: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to read directory '{}': {}", dir.display(), e);
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_file() && is_supported_source(&path) {
            out.push(path.to_string_lossy().to_string());
        } else if path.is_dir() {
            collect_supported_files(&path, out);
        }
    }
}

fn handle_revert(files: &[String], output: &str) {
    if files.is_empty() {
        eprintln!("No files specified for revert");
        return;
    }

    let output_dir = Path::new(output);
    if let Err(e) = fs::create_dir_all(output_dir) {
        eprintln!("Failed to create output directory '{}': {}", output, e);
        return;
    }

    let oml_generator = OmlGenerator;

    for file_path in files {
        let path = Path::new(file_path);

        let extension = match path.extension().and_then(|e| e.to_str()) {
            Some(ext) => ext,
            None => {
                eprintln!("Cannot determine file type for '{}': no extension", file_path);
                continue;
            }
        };

        let backwards_gen = match get_backwards_generator(extension) {
            Some(g) => g,
            None => {
                eprintln!("Unsupported file type '.{}' for revert", extension);
                continue;
            }
        };

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read '{}': {}", file_path, e);
                continue;
            }
        };

        let oml_objects: Vec<OmlObject> = match backwards_gen.reverse(&content) {
            Ok(objects) => objects,
            Err(e) => {
                eprintln!("Failed to parse '{}' back to OML: {}", file_path, e);
                continue;
            }
        };

        if oml_objects.is_empty() {
            eprintln!("No OML objects found in '{}'", file_path);
            continue;
        }

        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        match oml_generator.generate(&oml_objects, file_stem) {
            Ok(oml_content) => {
                let output_path = output_dir.join(format!("{}.oml", file_stem));
                if let Err(e) = fs::write(&output_path, &oml_content) {
                    eprintln!("Failed to write {}: {}", output_path.display(), e);
                } else {
                    println!("Reverted {} -> {}", file_path, output_path.display());
                }
            }
            Err(e) => {
                eprintln!("Failed to generate OML for '{}': {}", file_path, e);
            }
        }
    }
}
