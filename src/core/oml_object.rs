use std::cmp::PartialEq;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::core::errors;

#[derive(Debug, PartialEq)]
pub enum ObjectType {
    ENUM,
    CLASS,
    STRUCT,
    UNDECIDED
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariableModifier {
    CONST,
    MUT,
    STATIC,
    OPTIONAL,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariableVisibility {
    PRIVATE,
    PUBLIC,
    PROTECTED
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayKind {
    None,
    Static(u32),  // type[N] — N > 0 required
    Dynamic,       // list type
}

#[derive(Debug, PartialEq)]
pub struct Variable {
    pub var_mod: Vec<VariableModifier>,
    pub visibility: VariableVisibility,
    pub var_type: String,
    pub array_kind: ArrayKind,
    pub name: String,
}

#[derive(Debug)]
pub struct OmlObject {
    pub oml_type: ObjectType,
    pub name: String,
    pub variables: Vec<Variable>
}

/// Groups all OML objects parsed from a single file.
#[derive(Debug)]
pub struct OmlFile {
    pub file_name: String,
    pub objects: Vec<OmlObject>,
}

impl OmlObject {
    const CLASS_NAME: &'static str = "class";
    const ENUM_NAME: &'static str = "enum";
    const STRUCT_NAME: &'static str = "struct";

    pub const BUILTIN_TYPES: &'static [&'static str] = &[
        "int8", "int16", "int32", "int64",
        "uint8", "uint16", "uint32", "uint64",
        "float", "double", "bool", "string", "char",
    ];

    pub fn is_builtin_type(var_type: &str) -> bool {
        Self::BUILTIN_TYPES.contains(&var_type)
    }

    /// Validates that any non-built-in type used as a variable type in these
    /// objects actually corresponds to another object defined in the same set.
    pub fn validate_custom_types(objects: &[Self]) -> Result<(), Box<dyn std::error::Error>> {
        let object_names: HashSet<&str> = objects.iter().map(|o| o.name.as_str()).collect();

        for obj in objects {
            // Enums don't have typed variables
            if obj.oml_type == ObjectType::ENUM {
                continue;
            }
            for var in &obj.variables {
                if !var.var_type.is_empty()
                    && !Self::is_builtin_type(&var.var_type)
                    && !object_names.contains(var.var_type.as_str())
                {
                    return Err(format!(
                        "Type '{}' used in object '{}' is not a built-in type and no object with that name is defined in the same file",
                        var.var_type, obj.name
                    ).into());
                }
            }
        }

        Ok(())
    }

    pub fn get_from_file(path: &Path) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        Self::scan_file(content)
    }

    pub fn scan_file(content: String) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut results: Vec<Self> = Vec::new();

        let mut current: Option<Self> = None;
        let mut inside_body = false;
        let mut commenting = false;
        let mut body_lines: Vec<String> = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            #[allow(unused_assignments)]
            let mut processed_line: String = String::new();
            let mut line_ref: &str = trimmed;

            if commenting {
                if let Some(pos) = line_ref.find("*/") {
                    commenting = false;
                    line_ref = line_ref[pos + 2..].trim_start();
                    if line_ref.is_empty() {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if let Some(pos) = line_ref.find("//") {
                line_ref = line_ref[..pos].trim_end();
                if line_ref.is_empty() {
                    continue;
                }
            }

            if let Some(pos) = line_ref.find("/*") {
                let before_comment = line_ref[..pos].trim_end();

                if let Some(end_pos) = line_ref[pos..].find("*/") {
                    let after_comment = line_ref[pos + end_pos + 2..].trim_start();
                    processed_line = format!("{} {}", before_comment, after_comment);
                    line_ref = processed_line.trim();
                } else {
                    commenting = true;
                    line_ref = before_comment;
                }

                if line_ref.is_empty() {
                    continue;
                }
            }

            if !inside_body {
                let tokens: Vec<&str> = line_ref.split_whitespace().collect();
                if tokens.is_empty() {
                    continue;
                }

                let obj_type = match tokens[0] {
                    Self::CLASS_NAME => Some(ObjectType::CLASS),
                    Self::ENUM_NAME => Some(ObjectType::ENUM),
                    Self::STRUCT_NAME => Some(ObjectType::STRUCT),
                    _ => None,
                };

                if let Some(oml_type) = obj_type {
                    let mut obj = Self {
                        oml_type,
                        name: String::from("Nothing"),
                        variables: vec![],
                    };
                    if tokens.len() > 1 {
                        obj.assign_obj_name(tokens[1])?;
                    }
                    current = Some(obj);
                }

                if line_ref.contains('{') {
                    inside_body = true;
                }
                continue;
            }

            if line_ref.contains('}') {
                // finish the current object
                if let Some(mut obj) = current.take() {
                    if !body_lines.is_empty() {
                        obj.variables = Self::extract_object_variables(body_lines.drain(..).collect())?;
                    }
                    results.push(obj);
                }
                body_lines.clear();
                inside_body = false;
                continue;
            }

            if !line_ref.is_empty() {
                let tokens: Vec<&str> = line_ref.split_whitespace().collect();
                let has_type_and_name = tokens.iter().any(|&t| Self::is_type(t))
                    && tokens.len() >= 2;

                if has_type_and_name || line_ref.ends_with(';') {
                    body_lines.push(line_ref.to_string());
                }
            }
        }

        Ok(results)
    }

    fn assign_obj_name(&mut self, name: &str) -> Result<(), errors::NameError> {
        match Self::is_valid_name(name) {
            true => self.name = name.to_string(),
            false => {
                let message = format!("{} is not a valid obj name.", name);
                return Err(errors::NameError::new(message));
            }
        }

        Ok(())
    }

    #[inline]
    fn parse_visibility(token: &str) -> Option<VariableVisibility> {
        match token {
            "private" => Some(VariableVisibility::PRIVATE),
            "public" => Some(VariableVisibility::PUBLIC),
            "protected" => Some(VariableVisibility::PROTECTED),
            _ => None
        }
    }

    #[inline]
    fn parse_modifier(token: &str) -> Option<VariableModifier> {
        match token {
            "const" => Some(VariableModifier::CONST),
            "mut" => Some(VariableModifier::MUT),
            "static" => Some(VariableModifier::STATIC),
            "optional" => Some(VariableModifier::OPTIONAL),
            _ => None,
        }
    }

    #[inline]
    fn is_type(token: &str) -> bool {
        // Recognise static array tokens like "int32[4]"
        if let Some(bracket_pos) = token.find('[') {
            if token.ends_with(']') {
                let base  = &token[..bracket_pos];
                let inner = &token[bracket_pos + 1..token.len() - 1];
                if inner.parse::<u32>().map(|n| n > 0).unwrap_or(false) {
                    return Self::is_builtin_type(base) || Self::is_valid_name(base);
                }
            }
            return false; // malformed bracket → not a type
        }
        matches!(token,
            "int8" | "int16" | "int32" | "int64" |
            "uint8" | "uint16" | "uint32" | "uint64" |
            "float" | "double" | "bool" | "string" | "char"
        ) || Self::is_valid_name(token)
    }

    /// Parses a `type[N]` token into `(base_type, N)`.  Returns `None` if the
    /// token does not match the pattern or if N is zero.
    fn parse_array_type(token: &str) -> Option<(String, u32)> {
        let bp = token.find('[')?;
        if !token.ends_with(']') {
            return None;
        }
        let base  = &token[..bp];
        let inner = &token[bp + 1..token.len() - 1];
        let size: u32 = inner.parse().ok().filter(|&n| n > 0)?;
        if Self::is_builtin_type(base) || Self::is_valid_name(base) {
            Some((base.to_string(), size))
        } else {
            None
        }
    }

    fn extract_object_variables(lines: Vec<String>) -> Result<Vec<Variable>, Box<dyn std::error::Error>> {
        let mut vars: Vec<Variable> = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let cleaned = trimmed.trim_end_matches(|c| c == ';' || c == '\n').trim();

            match Self::parse_variable_declaration(cleaned) {
                Ok(var) => vars.push(var),
                Err(e) => {
                    return Err(format!("Error parsing line '{}': {}", line, e).into());
                }
            }
        }

        Ok(vars)
    }

    fn parse_variable_declaration(line: &str) -> Result<Variable, String> {
        let tokens: Vec<&str> = line.split_whitespace().collect();

        if tokens.is_empty() {
            return Err("Empty line".to_string());
        }

        let mut visibility: Option<VariableVisibility> = None;
        let mut modifiers: Vec<VariableModifier> = Vec::new();
        let mut var_type: Option<String> = None;
        let mut var_name: Option<String> = None;
        let mut array_kind = ArrayKind::None;
        let mut type_seen = false;

        for token in &tokens {
            if let Some(vis) = Self::parse_visibility(token) {
                if type_seen {
                    return Err(format!(
                        "Visibility modifier '{}' cannot appear after type",
                        token
                    ));
                }

                if visibility.is_some() {
                    return Err("Multiple visibility modifiers found".to_string());
                }
                visibility = Some(vis);
                continue;
            }

            if let Some(modifier) = Self::parse_modifier(token) {
                if type_seen {
                    return Err(format!(
                        "Modifier '{}' cannot appear after type",
                        token
                    ));
                }
                modifiers.push(modifier);
                continue;
            }

            // "list" keyword → dynamic array; the next token will be the element type
            if *token == "list" && !type_seen {
                if array_kind != ArrayKind::None {
                    return Err("Multiple array kind specifiers".to_string());
                }
                array_kind = ArrayKind::Dynamic;
                continue;
            }

            // Detect bare "[]" and give a helpful error
            if token.contains('[') && token.ends_with(']') && !type_seen {
                let bp = token.find('[').unwrap();
                let inner = &token[bp + 1..token.len() - 1];
                if inner.is_empty() {
                    let base = &token[..bp];
                    return Err(format!(
                        "Static arrays require a size: use '{base}[N]' (N > 0), or 'list {base}' for a dynamic array"
                    ));
                }
                if inner.parse::<u32>().map(|n| n == 0).unwrap_or(false) {
                    return Err(format!("Array size must be greater than 0 in '{}'", token));
                }
            }

            // "type[N]" → static array
            if var_type.is_none() && !type_seen {
                if let Some((base_type, size)) = Self::parse_array_type(token) {
                    if array_kind == ArrayKind::Dynamic {
                        return Err("Cannot combine 'list' with static array syntax 'type[N]'".to_string());
                    }
                    var_type = Some(base_type);
                    array_kind = ArrayKind::Static(size);
                    type_seen = true;
                    continue;
                }
            }

            if Self::is_type(token) && var_type.is_none() {
                var_type = Some(token.to_string());
                type_seen = true;
                continue;
            }

            if var_type.is_some() && var_name.is_none() {
                var_name = Some(token.to_string());
                break;
            }

            return Err(format!("Unexpected token: {}", token));
        }

        let final_type = var_type.ok_or("No type specified")?;
        let final_name = var_name.ok_or("No variable name specified")?;
        let final_visibility = visibility.unwrap_or(VariableVisibility::PRIVATE);

        if modifiers.contains(&VariableModifier::CONST) && modifiers.contains(&VariableModifier::MUT) {
            return Err(format!("Const Error: variable {} cannot be const and mut simultaneously!", final_name));
        }

        Ok(Variable {
            var_mod: modifiers,
            visibility: final_visibility,
            var_type: final_type,
            array_kind,
            name: final_name,
        })
    }

    #[inline]
    fn is_valid_name(name: &str) -> bool {
        let re = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_.-]*$").unwrap();
        re.is_match(name)
    }
}


#[cfg(test)]
mod test {
    use super::*;

    const VALID_NAMES: [&str; 8] = [
        "myfile.txt",
        "variable_name",
        "Config",
        "test123",
        "my-file-name",
        "file.tar.gz",
        "a",
        "MyClass"
    ];

    const INVALID_NAMES: [&str; 8] = [
        "123file",
        "_private",
        "-file",
        "my file",
        "file@name",
        "my$var",
        "file/path",
        "",
    ];

    #[test]
    fn test_name_validity() {
        for valid_name in VALID_NAMES {
            assert_eq!(OmlObject::is_valid_name(valid_name), true);
        }

        for valid_name in INVALID_NAMES {
            assert_eq!(OmlObject::is_valid_name(valid_name), false);
        }
    }

    #[test]
    fn test_assign_name() {
        let mut oml_obj = OmlObject {
            oml_type: ObjectType::UNDECIDED,
            name: String::new(),
            variables: vec![],
        };

        for valid_name in VALID_NAMES {
            oml_obj.assign_obj_name(valid_name).expect("this should not happen");
            assert_eq!(oml_obj.name, valid_name);
        }

        for invalid_name in INVALID_NAMES {
            let error = oml_obj.assign_obj_name(invalid_name).unwrap_err();
            let message = format!("{} is not a valid obj name.", invalid_name);
            assert_eq!(error.message, message);
        }
    }

    #[test]
    fn test_parse_variable_declaration() {
        // Valid declarations
        let valid_cases = vec![
            ("public const int64 myVar", "myVar", "int64", 1, VariableVisibility::PUBLIC),
            ("private mut string name", "name", "string", 1, VariableVisibility::PRIVATE),
            ("protected static int32 count", "count", "int32", 1, VariableVisibility::PROTECTED),
            ("int64 simpleVar", "simpleVar", "int64", 0, VariableVisibility::PRIVATE),
            ("const int64 meow", "meow", "int64", 1, VariableVisibility::PRIVATE),
            ("string hello", "hello", "string", 0, VariableVisibility::PRIVATE),
            ("bool isTrue", "isTrue", "bool", 0, VariableVisibility::PRIVATE),
        ];

        for (input, expected_name, expected_type, expected_mod_count, _expected_vis) in valid_cases {
            let result = OmlObject::parse_variable_declaration(input);
            assert!(result.is_ok(), "Failed to parse: {}", input);
            let var = result.unwrap();
            assert_eq!(var.name, expected_name);
            assert_eq!(var.var_type, expected_type);
            assert_eq!(var.var_mod.len(), expected_mod_count);
            assert!(matches!(var.visibility, _expected_vis));
        }
        let invalid_cases = vec![
            "int64 private myVar",
            "int32 public x",
            "string private name",
            "int64 const x",
        ];

        for input in invalid_cases {
            let result = OmlObject::parse_variable_declaration(input);
            assert!(result.is_err(), "Should have failed: {}", input);
        }
    }

    #[test]
    fn test_parse_class_from_string() {
        let content = r#"
            class Hello {
                const int64 meow;
                string hello;
                bool isTrue;
            }
        "#;

        let result = OmlObject::scan_file(content.to_string());

        assert!(result.is_ok());
        let objects = result.unwrap();
        assert_eq!(objects.len(), 1);
        let oml_obj = &objects[0];
        assert_eq!(oml_obj.name, "Hello");
        assert!(matches!(oml_obj.oml_type, ObjectType::CLASS));
        assert_eq!(oml_obj.variables.len(), 3);

        // check first variable
        assert_eq!(oml_obj.variables[0].name, "meow");
        assert_eq!(oml_obj.variables[0].var_type, "int64");
        assert_eq!(oml_obj.variables[0].var_mod.len(), 1);
        assert!(matches!(oml_obj.variables[0].var_mod[0], VariableModifier::CONST));

        // check second variable
        assert_eq!(oml_obj.variables[1].name, "hello");
        assert_eq!(oml_obj.variables[1].var_type, "string");
        assert_eq!(oml_obj.variables[1].var_mod.len(), 0);

        // check third variable
        assert_eq!(oml_obj.variables[2].name, "isTrue");
        assert_eq!(oml_obj.variables[2].var_type, "bool");
    }

    #[test]
    fn test_parse_multiple_objects_from_string() {
        let content = r#"
            class Hello {
                string name;
            }

            enum Color {
                string Red;
                string Green;
                string Blue;
            }

            struct Point {
                int32 x;
                int32 y;
            }
        "#;

        let result = OmlObject::scan_file(content.to_string());
        assert!(result.is_ok());
        let objects = result.unwrap();
        assert_eq!(objects.len(), 3);

        assert_eq!(objects[0].name, "Hello");
        assert!(matches!(objects[0].oml_type, ObjectType::CLASS));

        assert_eq!(objects[1].name, "Color");
        assert!(matches!(objects[1].oml_type, ObjectType::ENUM));

        assert_eq!(objects[2].name, "Point");
        assert!(matches!(objects[2].oml_type, ObjectType::STRUCT));
    }

    #[test]
    fn test_modifier_ordering_rule() {
        let content = r#"
            class Test {
                int64 private x;
            }
        "#;

        let result = OmlObject::scan_file(content.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_const() {
        let content = r#"
            class Test {
                const mut int64 x;
            }
        "#;

        let content_swaped = r#"
            class Test {
                const mut int64 x;
            }
        "#;

        let result = OmlObject::scan_file(content.to_string());
        assert!(result.is_err());

        let result = OmlObject::scan_file(content_swaped.to_string());
        assert!(result.is_err());
    }

    // ── array / list tests ───────────────────────────────────────────────────

    #[test]
    fn test_parse_static_array() {
        let result = OmlObject::parse_variable_declaration("uint16[4] scores");
        assert!(result.is_ok(), "Failed: {:?}", result);
        let var = result.unwrap();
        assert_eq!(var.var_type, "uint16");
        assert_eq!(var.name, "scores");
        assert_eq!(var.array_kind, ArrayKind::Static(4));
    }

    #[test]
    fn test_parse_dynamic_list() {
        let result = OmlObject::parse_variable_declaration("list string tags");
        assert!(result.is_ok(), "Failed: {:?}", result);
        let var = result.unwrap();
        assert_eq!(var.var_type, "string");
        assert_eq!(var.name, "tags");
        assert_eq!(var.array_kind, ArrayKind::Dynamic);
    }

    #[test]
    fn test_parse_static_array_with_modifiers() {
        let result = OmlObject::parse_variable_declaration("public const int32[10] ids");
        assert!(result.is_ok(), "Failed: {:?}", result);
        let var = result.unwrap();
        assert_eq!(var.var_type, "int32");
        assert_eq!(var.array_kind, ArrayKind::Static(10));
        assert!(var.var_mod.contains(&VariableModifier::CONST));
        assert!(matches!(var.visibility, VariableVisibility::PUBLIC));
    }

    #[test]
    fn test_parse_dynamic_list_with_modifiers() {
        let result = OmlObject::parse_variable_declaration("private list int64 values");
        assert!(result.is_ok(), "Failed: {:?}", result);
        let var = result.unwrap();
        assert_eq!(var.var_type, "int64");
        assert_eq!(var.array_kind, ArrayKind::Dynamic);
    }

    #[test]
    fn test_parse_bare_brackets_error() {
        let result = OmlObject::parse_variable_declaration("uint16[] x");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Static arrays require a size"), "Got: {}", msg);
    }

    #[test]
    fn test_parse_zero_size_array_error() {
        let result = OmlObject::parse_variable_declaration("int32[0] x");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_list_with_array_notation_error() {
        let result = OmlObject::parse_variable_declaration("list uint16[4] x");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Cannot combine"), "Got: {}", msg);
    }

    #[test]
    fn test_scan_file_with_arrays() {
        let content = r#"
            class ExampleArrays {
                uint16[4]   static_array;
                list uint16 dynamic_array;
                list string tags;
            }
        "#;

        let result = OmlObject::scan_file(content.to_string());
        assert!(result.is_ok());
        let objects = result.unwrap();
        assert_eq!(objects.len(), 1);
        let vars = &objects[0].variables;
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0].var_type, "uint16");
        assert_eq!(vars[0].array_kind, ArrayKind::Static(4));
        assert_eq!(vars[1].var_type, "uint16");
        assert_eq!(vars[1].array_kind, ArrayKind::Dynamic);
        assert_eq!(vars[2].var_type, "string");
        assert_eq!(vars[2].array_kind, ArrayKind::Dynamic);
    }

    #[cfg(test)]
    mod comment_tests {
        use super::*;

        #[test]
        fn test_single_comments() {
            let content = r#"
               // ignore me
               class Test {
                const int64 x;
                const int64 y; // this is good
                // const int64 z;
               }
            "#;

            let content2 = r#"
                // ignore me
                //class Test {
                // const int64 x;
                //const int64 y; // this is good
                //  const int64 z;
               //}
            "#;

            let vars = vec![
                Variable { var_mod: vec![VariableModifier::CONST], visibility: VariableVisibility::PRIVATE, var_type: String::from("int64"), array_kind: ArrayKind::None, name: String::from("x") },
                Variable { var_mod: vec![VariableModifier::CONST], visibility: VariableVisibility::PRIVATE, var_type: String::from("int64"), array_kind: ArrayKind::None, name: String::from("y") },
            ];

            let result = OmlObject::scan_file(content.to_string());
            assert!(result.is_ok());
            let objects = result.unwrap();
            assert_eq!(objects.len(), 1);
            assert_eq!(objects[0].variables, vars);

            let result2 = OmlObject::scan_file(content2.to_string());
            assert!(result2.is_ok());
            assert_eq!(result2.unwrap().len(), 0);
        }

        #[test]
        fn test_multi_lined_comments() {
            let content = r#"
               /* ignore me

               */
               class Test {
                const int64 x;
                const int64 y; /* hello world */
                const /* this should cause no issue */
               }
            "#;

            let content2 = r#"
                // ignore me
                //class Test {
                // const int64 x;
                //const int64 y; // this is good
                //  const int64 z;
               //}
            "#;

            let vars = vec![
                Variable { var_mod: vec![VariableModifier::CONST], visibility: VariableVisibility::PRIVATE, var_type: String::from("int64"), array_kind: ArrayKind::None, name: String::from("x") },
                Variable { var_mod: vec![VariableModifier::CONST], visibility: VariableVisibility::PRIVATE, var_type: String::from("int64"), array_kind: ArrayKind::None, name: String::from("y") },
            ];

            let result = OmlObject::scan_file(content.to_string());
            assert!(result.is_ok());
            let objects = result.unwrap();
            assert_eq!(objects.len(), 1);
            assert_eq!(objects[0].variables, vars);

            let result2 = OmlObject::scan_file(content2.to_string());
            assert!(result2.is_ok());
            assert_eq!(result2.unwrap().len(), 0);
        }
    }
}
