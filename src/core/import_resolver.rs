use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::core::oml_object::{OmlFile, OmlObject};

/// Resolves all transitive imports for the given root files.
/// Returns all discovered files and a map from each file's path to the set of
/// object names imported into it.  Errors on missing files or circular imports.
pub fn resolve_all(
    root_files: Vec<OmlFile>,
) -> Result<(Vec<OmlFile>, HashMap<PathBuf, HashSet<String>>), Box<dyn std::error::Error>> {
    let mut all_files: HashMap<PathBuf, OmlFile> = HashMap::new();

    for f in root_files {
        all_files.insert(f.path.clone(), f);
    }

    // BFS: discover and parse every imported file not yet seen.
    let mut queue: Vec<PathBuf> = all_files.keys().cloned().collect();

    while let Some(current) = queue.pop() {
        let imports = all_files[&current].imports.clone();
        let parent = current
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        for import_str in imports {
            let raw_path = parent.join(&import_str);
            let canonical = raw_path.canonicalize().map_err(|_| {
                format!(
                    "Import '{}' not found (imported in '{}')",
                    import_str,
                    current.display()
                )
            })?;

            if all_files.contains_key(&canonical) {
                continue;
            }

            let (objects, sub_imports) = OmlObject::get_from_file(&raw_path).map_err(|e| {
                format!(
                    "Failed to parse imported file '{}': {}",
                    raw_path.display(),
                    e
                )
            })?;

            let file_name = raw_path
                .file_stem()
                .ok_or_else(|| format!("Invalid import path '{}'", import_str))?
                .to_string_lossy()
                .to_string();

            let oml_file = OmlFile {
                file_name,
                path: canonical.clone(),
                objects,
                imports: sub_imports,
            };

            all_files.insert(canonical.clone(), oml_file);
            queue.push(canonical);
        }
    }

    let mut adj: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for (path, file) in &all_files {
        let parent = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        let deps: Vec<PathBuf> = file
            .imports
            .iter()
            .filter_map(|imp| parent.join(imp).canonicalize().ok())
            .collect();

        adj.insert(path.clone(), deps);
    }

    // Cycle detection: 0 = unvisited, 1 = in stack, 2 = done.
    let mut state: HashMap<PathBuf, u8> = HashMap::new();
    for path in all_files.keys().cloned().collect::<Vec<_>>() {
        if state.get(&path).copied().unwrap_or(0) == 0 {
            dfs_detect_cycle(&path, &adj, &mut state)?;
        }
    }

    let mut names_cache: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    for path in all_files.keys().cloned().collect::<Vec<_>>() {
        let names = collect_imported_names(&path, &adj, &all_files, &mut names_cache);
        names_cache.insert(path, names);
    }

    let files: Vec<OmlFile> = all_files.into_values().collect();
    Ok((files, names_cache))
}

/// DFS helper that errors on back-edges (cycles).
fn dfs_detect_cycle(
    node: &PathBuf,
    adj: &HashMap<PathBuf, Vec<PathBuf>>,
    state: &mut HashMap<PathBuf, u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    state.insert(node.clone(), 1);

    if let Some(deps) = adj.get(node) {
        for dep in deps {
            match state.get(dep).copied().unwrap_or(0) {
                1 => {
                    return Err(format!(
                        "Circular import detected: '{}' is part of an import cycle",
                        dep.display()
                    )
                    .into());
                }
                0 => dfs_detect_cycle(dep, adj, state)?,
                _ => {}
            }
        }
    }

    state.insert(node.clone(), 2);
    Ok(())
}

/// Returns the set of object names transitively available in `path` via imports.
fn collect_imported_names(
    path: &PathBuf,
    adj: &HashMap<PathBuf, Vec<PathBuf>>,
    all_files: &HashMap<PathBuf, OmlFile>,
    cache: &mut HashMap<PathBuf, HashSet<String>>,
) -> HashSet<String> {
    if let Some(cached) = cache.get(path) {
        return cached.clone();
    }

    let mut names: HashSet<String> = HashSet::new();

    if let Some(deps) = adj.get(path) {
        for dep in deps {
            if let Some(dep_file) = all_files.get(dep) {
                for obj in &dep_file.objects {
                    names.insert(obj.name.clone());
                }
            }
            let transitive = collect_imported_names(dep, adj, all_files, cache);
            names.extend(transitive);
        }
    }

    cache.insert(path.clone(), names.clone());
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::oml_object::OmlFile;

    fn empty_file(name: &str, path: &str) -> OmlFile {
        OmlFile {
            file_name: name.to_string(),
            path: PathBuf::from(path),
            objects: vec![],
            imports: vec![],
        }
    }

    #[test]
    fn test_no_imports_returns_same_files() {
        let files = vec![empty_file("a", "/fake/a.oml"), empty_file("b", "/fake/b.oml")];
        let (all, names) = resolve_all(files).unwrap();
        assert_eq!(all.len(), 2);
        for (_, set) in &names {
            assert!(set.is_empty());
        }
    }

    #[test]
    fn test_cycle_detection() {
        let a = PathBuf::from("/fake/a.oml");
        let b = PathBuf::from("/fake/b.oml");

        let mut adj: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        adj.insert(a.clone(), vec![b.clone()]);
        adj.insert(b.clone(), vec![a.clone()]);

        let mut state = HashMap::new();
        let result = dfs_detect_cycle(&a, &adj, &mut state);
        assert!(result.is_err(), "Cycle should be detected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Circular import"), "Got: {}", msg);
    }

    #[test]
    fn test_collect_imported_names_no_deps() {
        let path = PathBuf::from("/fake/a.oml");
        let adj: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        let all_files: HashMap<PathBuf, OmlFile> = HashMap::new();
        let mut cache = HashMap::new();

        let names = collect_imported_names(&path, &adj, &all_files, &mut cache);
        assert!(names.is_empty());
    }
}
