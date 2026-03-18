use std::fs;
use std::path::Path;
use crate::core::errors::ParseError;
use crate::core::oml_object::{OmlFile, OmlObject};

pub fn parse_path(
    path: &Path,
    max_depth: usize
) -> Result<Vec<OmlFile>, ParseError> {
    if max_depth == 0 {
        return Err(ParseError::MaxDepthExceeded);
    }

    let mut result = vec![];

    let metadata = fs::symlink_metadata(path)?;

    if metadata.file_type().is_symlink() {
        return Err(ParseError::InvalidPath);
    }

    if path.is_file() {
        if let Some(extension) = path.extension() {
            if extension.to_string_lossy() != "oml" {
                return Err(ParseError::InvalidPath);
            }
        } else {
            return Err(ParseError::InvalidPath);
        }

        let file_name = path.file_stem()
            .ok_or(ParseError::InvalidPath)?
            .to_string_lossy()
            .to_string();

        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        return match OmlObject::get_from_file(path) {
            Ok((objects, imports)) => {
                result.push(OmlFile { file_name, path: canonical, objects, imports });
                Ok(result)
            },
            Err(_) => {
                eprintln!("could not obtain oml objects from file {}", path.display());
                Err(ParseError::InvalidPath)
            }
        }
    }

    if !path.is_dir() {
        return Err(ParseError::InvalidPath);
    }

    let dir = fs::read_dir(path)?;

    for entry in dir {
        let entry = entry?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)?;

        if metadata.file_type().is_symlink() {
            eprintln!("Warning: Skipping symlink: {}", entry_path.display());
            continue;
        }

        if entry_path.is_file() {
            if let Some(extension) = entry_path.extension() {
                if extension.to_string_lossy() != "oml" {
                    eprintln!("Warning: Skipping non-oml file: {}", entry_path.display());
                    continue;
                }
            } else {
                continue;
            }

            let file_name = entry_path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let canonical = entry_path.canonicalize().unwrap_or_else(|_| entry_path.clone());
            match OmlObject::get_from_file(&entry_path) {
                Ok((objects, imports)) => result.push(OmlFile { file_name, path: canonical, objects, imports }),
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", entry_path.display(), e);
                }
            }
            continue;
        }

        if entry_path.is_dir() {
            let mut sub_files = parse_path(&entry_path, max_depth - 1)?;
            result.append(&mut sub_files);
        }
    }

    Ok(result)
}

pub fn parse_dir_from_string(
    path_str: String,
    max_depth: usize
) -> Result<Vec<OmlFile>, ParseError> {
    let path = Path::new(&path_str);

    if !path.exists() {
        return Err(ParseError::InvalidPath);
    }

    parse_path(path, max_depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_path_works() {
        let _result = parse_dir_from_string("./src".to_string(), 10);
    }

    #[test]
    fn test_absolute_path_works() {
        let _result = parse_dir_from_string("/home/user/project".to_string(), 10);
    }

    #[test]
    fn test_max_depth_prevents_overflow() {
        let _result = parse_dir_from_string("./".to_string(), 5);
    }
}
