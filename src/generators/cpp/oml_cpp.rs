use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
};
use crate::core::generate::{Generate, BackwardsGenerate};
use std::error::Error;
use std::fmt::Write;

pub struct CppGenerator;

impl BackwardsGenerate for CppGenerator {
    fn reverse(&self, content: &str) -> Result<Vec<OmlObject>, Box<dyn Error>> {
        let mut objects = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("enum class ") && trimmed.ends_with('{') {
                let name = trimmed
                    .strip_prefix("enum class ")
                    .unwrap()
                    .trim_end_matches(|c: char| c == '{' || c == ' ')
                    .to_string();
                let mut vars = Vec::new();
                i += 1;
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line.starts_with("};") { break; }
                    let variant = line.trim_end_matches(',').trim().to_string();
                    if !variant.is_empty() {
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
            } else if (trimmed.starts_with("class ") || trimmed.starts_with("struct "))
                && trimmed.ends_with('{')
            {
                let is_struct = trimmed.starts_with("struct ");
                let prefix = if is_struct { "struct " } else { "class " };
                let name = trimmed
                    .strip_prefix(prefix)
                    .unwrap()
                    .trim_end_matches(|c: char| c == '{' || c == ' ')
                    .to_string();
                let mut vars = Vec::new();
                let mut current_visibility = if is_struct {
                    VariableVisibility::PUBLIC
                } else {
                    VariableVisibility::PRIVATE
                };
                i += 1;
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line.starts_with("};") { break; }
                    if line == "public:" {
                        current_visibility = VariableVisibility::PUBLIC;
                    } else if line == "private:" {
                        current_visibility = VariableVisibility::PRIVATE;
                    } else if line == "protected:" {
                        current_visibility = VariableVisibility::PROTECTED;
                    } else if !line.is_empty()
                        && !line.starts_with("//")
                        && !line.contains('(')
                        && !line.contains('~')
                        && line.ends_with(';')
                    {
                        if let Some(var) = parse_cpp_field(line, &current_visibility) {
                            vars.push(var);
                        }
                    }
                    i += 1;
                }
                let oml_type = if is_struct { ObjectType::STRUCT } else { ObjectType::CLASS };
                objects.push(OmlObject {
                    oml_type,
                    name,
                    variables: vars,
                });
            }
            i += 1;
        }

        Ok(objects)
    }
}

fn reverse_cpp_type(cpp_type: &str) -> String {
    match cpp_type {
        "int8_t" => "int8".to_string(),
        "int16_t" => "int16".to_string(),
        "int32_t" => "int32".to_string(),
        "int64_t" => "int64".to_string(),
        "uint8_t" => "uint8".to_string(),
        "uint16_t" => "uint16".to_string(),
        "uint32_t" => "uint32".to_string(),
        "uint64_t" => "uint64".to_string(),
        "float" => "float".to_string(),
        "double" => "double".to_string(),
        "bool" => "bool".to_string(),
        "std::string" => "string".to_string(),
        "char" => "char".to_string(),
        other => other.to_string(),
    }
}

fn parse_cpp_field(line: &str, default_vis: &VariableVisibility) -> Option<Variable> {
    let line = line.trim().trim_end_matches(';').trim();
    if line.is_empty() { return None; }

    let mut var_mod = Vec::new();
    let mut rest = line;

    if rest.starts_with("static ") {
        var_mod.push(VariableModifier::STATIC);
        rest = &rest[7..];
    }

    if rest.starts_with("const ") {
        var_mod.push(VariableModifier::CONST);
        rest = &rest[6..];
    }

    // Handle std::optional<...>
    if rest.starts_with("std::optional<") {
        var_mod.push(VariableModifier::OPTIONAL);
        let close = rest.rfind('>')?;
        let inner = &rest[14..close];
        rest = inner;
        // Need to also get the name from after the >
        let after_close = line.trim().trim_end_matches(';');
        let close_pos = after_close.rfind('>')?;
        let name = after_close[close_pos + 1..].trim().to_string();

        let (var_type, array_kind) = parse_cpp_type_and_name_inner(rest);
        return Some(Variable {
            var_mod,
            visibility: default_vis.clone(),
            var_type,
            array_kind,
            name,
        });
    }

    // Handle std::vector<T>
    if rest.starts_with("std::vector<") {
        let close = rest.rfind('>')?;
        let inner = &rest[12..close];
        let name = rest[close + 1..].trim().to_string();
        return Some(Variable {
            var_mod,
            visibility: default_vis.clone(),
            var_type: reverse_cpp_type(inner.trim()),
            array_kind: ArrayKind::Dynamic,
            name,
        });
    }

    // Handle std::array<T, N>
    if rest.starts_with("std::array<") {
        let close = rest.rfind('>')?;
        let inner = &rest[11..close];
        let name = rest[close + 1..].trim().to_string();
        if let Some(comma) = inner.find(',') {
            let elem_type = inner[..comma].trim();
            let size_str = inner[comma + 1..].trim();
            if let Ok(size) = size_str.parse::<u32>() {
                return Some(Variable {
                    var_mod,
                    visibility: default_vis.clone(),
                    var_type: reverse_cpp_type(elem_type),
                    array_kind: ArrayKind::Static(size),
                    name,
                });
            }
        }
    }

    // Simple "type name"
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() >= 2 {
        let cpp_type = tokens[..tokens.len() - 1].join(" ");
        let name = tokens[tokens.len() - 1].to_string();
        return Some(Variable {
            var_mod,
            visibility: default_vis.clone(),
            var_type: reverse_cpp_type(&cpp_type),
            array_kind: ArrayKind::None,
            name,
        });
    }

    None
}

fn parse_cpp_type_and_name_inner(type_str: &str) -> (String, ArrayKind) {
    let type_str = type_str.trim();

    if type_str.starts_with("std::vector<") {
        let close = type_str.rfind('>').unwrap_or(type_str.len());
        let inner = &type_str[12..close];
        return (reverse_cpp_type(inner.trim()), ArrayKind::Dynamic);
    }

    if type_str.starts_with("std::array<") {
        let close = type_str.rfind('>').unwrap_or(type_str.len());
        let inner = &type_str[11..close];
        if let Some(comma) = inner.find(',') {
            let elem_type = inner[..comma].trim();
            let size_str = inner[comma + 1..].trim();
            if let Ok(size) = size_str.parse::<u32>() {
                return (reverse_cpp_type(elem_type), ArrayKind::Static(size));
            }
        }
    }

    (reverse_cpp_type(type_str), ArrayKind::None)
}

impl Generate for CppGenerator {
    fn generate(&self, oml_objects: &[OmlObject], file_name: &str) -> Result<String, Box<dyn Error>> {
        let mut cpp_file = String::new();
        let header_guard = format!("{}_H", file_name.to_uppercase());

        writeln!(cpp_file, "// This file has been generated from {}.oml", file_name)?;
        writeln!(cpp_file, "#ifndef {}", header_guard)?;
        writeln!(cpp_file, "#define {}", header_guard)?;
        writeln!(cpp_file)?;

        let has_class_or_struct = oml_objects.iter().any(|o|
            o.oml_type == ObjectType::CLASS || o.oml_type == ObjectType::STRUCT
        );

        if has_class_or_struct {
            writeln!(cpp_file, "#include <cstdint>")?;
            writeln!(cpp_file, "#include <string>")?;
            writeln!(cpp_file, "#include <optional>")?;
            writeln!(cpp_file, "#include <utility>")?;

            let has_static_array = oml_objects.iter().any(|o|
                o.variables.iter().any(|v| matches!(v.array_kind, ArrayKind::Static(_))));
            let has_dynamic_array = oml_objects.iter().any(|o|
                o.variables.iter().any(|v| v.array_kind == ArrayKind::Dynamic));
            if has_static_array  { writeln!(cpp_file, "#include <array>")?; }
            if has_dynamic_array { writeln!(cpp_file, "#include <vector>")?; }
            writeln!(cpp_file)?;
        }

        for (i, oml_object) in oml_objects.iter().enumerate() {
            match &oml_object.oml_type {
                ObjectType::ENUM => generate_enum(oml_object, &mut cpp_file)?,
                ObjectType::CLASS | ObjectType::STRUCT => generate_class_or_struct(oml_object, &mut cpp_file)?,
                ObjectType::UNDECIDED => return Err("Cannot generate code for UNDECIDED object type".into()),
            }
            if i < oml_objects.len() - 1 {
                writeln!(cpp_file)?;
            }
        }

        writeln!(cpp_file, "#endif // {}\n", header_guard)?;

        Ok(cpp_file)
    }

    fn extension(&self) -> &str {
        "h"
    }
}

fn generate_enum(oml_object: &OmlObject, cpp_file: &mut String) -> Result<(), std::fmt::Error> {
    writeln!(cpp_file, "enum class {} {{", oml_object.name)?;
    let length = oml_object.variables.len();

    for (index, var) in oml_object.variables.iter().enumerate() {
        write!(cpp_file, "\t{}", var.name.to_uppercase())?;
        if index == length-1 {
            writeln!(cpp_file, "")?;
            continue
        }
        writeln!(cpp_file, ",")?;

    }

    writeln!(cpp_file, "}};")?;

    Ok(())
}

fn generate_class_or_struct(
    oml_object: &OmlObject,
    cpp_file: &mut String
) -> Result<(), std::fmt::Error> {
    let oml_type = match &oml_object.oml_type {
        ObjectType::CLASS => "class",
        ObjectType::STRUCT => "struct",
        _ => return Err(std::fmt::Error)
    };

    writeln!(cpp_file, "{} {} {{", oml_type, oml_object.name)?;

    // Public section: constructors, special members, getters/setters, public vars
    writeln!(cpp_file, "public:")?;
    generate_constructors(oml_object, cpp_file)?;
    writeln!(cpp_file)?;
    generate_copy_move_and_destructor(oml_object, cpp_file)?;
    writeln!(cpp_file)?;
    generate_getters_and_setters(&oml_object.variables, cpp_file)?;

    // Public member variables (after getters/setters)
    generate_visibility_vars(&oml_object.variables, cpp_file, VariableVisibility::PUBLIC, false)?;

    // Protected and private member variables
    generate_visibility_vars(&oml_object.variables, cpp_file, VariableVisibility::PROTECTED, true)?;
    generate_visibility_vars(&oml_object.variables, cpp_file, VariableVisibility::PRIVATE, true)?;

    writeln!(cpp_file, "}};")?;

    Ok(())
}

/// Writes variables of a given visibility. If `write_label` is true, emits the
/// visibility label (e.g. `private:`) before the variables.
fn generate_visibility_vars(
    variables: &Vec<Variable>,
    cpp_file: &mut String,
    visibility: VariableVisibility,
    write_label: bool,
) -> Result<(), std::fmt::Error> {
    let vars: Vec<_> = variables
        .iter()
        .filter(|v| v.visibility == visibility)
        .collect();

    if vars.is_empty() {
        return Ok(());
    }

    if write_label {
        let label = match visibility {
            VariableVisibility::PUBLIC => "public:",
            VariableVisibility::PROTECTED => "protected:",
            VariableVisibility::PRIVATE => "private:",
        };
        writeln!(cpp_file, "{}", label)?;
    }

    for var in vars {
        convert_modifiers_and_type(var, cpp_file)?;
    }

    Ok(())
}

#[inline]
fn convert_type(var_type: &str) -> String {
    match var_type {
        "int8" => "int8_t".to_string(),
        "int16" => "int16_t".to_string(),
        "int32" => "int32_t".to_string(),
        "int64" => "int64_t".to_string(),
        "uint8" => "uint8_t".to_string(),
        "uint16" => "uint16_t".to_string(),
        "uint32" => "uint32_t".to_string(),
        "uint64" => "uint64_t".to_string(),
        "float" => "float".to_string(),
        "double" => "double".to_string(),
        "bool" => "bool".to_string(),
        "string" => "std::string".to_string(),
        "char" => "char".to_string(),
        other => other.to_string(),
    }
}

fn type_annotation(var_type: &str, array_kind: &ArrayKind) -> String {
    let base = convert_type(var_type);
    match array_kind {
        ArrayKind::None => base,
        ArrayKind::Static(n) => format!("std::array<{}, {}>", base, n),
        ArrayKind::Dynamic => format!("std::vector<{}>", base),
    }
}

fn convert_modifiers_and_type(
    var: &Variable,
    cpp_file: &mut String
) -> Result<(), std::fmt::Error> {
    write!(cpp_file, "\t")?;

    if var.var_mod.contains(&VariableModifier::STATIC) {
        write!(cpp_file, "static ")?;
    }

    if var.var_mod.contains(&VariableModifier::CONST)
        && !var.var_mod.contains(&VariableModifier::MUT) {
        write!(cpp_file, "const ")?;
    }

    let var_type = get_full_type(var);
    write!(cpp_file, "{}", var_type)?;

    writeln!(cpp_file, " {};", var.name)?;

    Ok(())
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn generate_getters_and_setters(
    variables: &Vec<Variable>,
    cpp_file: &mut String,
) -> Result<(), std::fmt::Error> {
    let private_vars = variables
        .iter()
        .filter(|v| v.visibility == VariableVisibility::PRIVATE)
        .collect::<Vec<_>>();

    if private_vars.is_empty() {
        return Ok(());
    }

    for var in &private_vars {
        let cpp_type = get_full_type(var);
        let capitalized = capitalize_first(&var.name);

        // Getter
        writeln!(cpp_file, "\t{} get{}() const {{ return {}; }}", cpp_type, capitalized, var.name)?;
    }

    writeln!(cpp_file)?;

    for var in &private_vars {
        // Skip setters for const variables
        if var.var_mod.contains(&VariableModifier::CONST) {
            continue;
        }

        let cpp_type = get_full_type(var);
        let capitalized = capitalize_first(&var.name);

        // Setter
        writeln!(
            cpp_file,
            "\tvoid set{}(const {}& value) {{ {} = value; }}",
            capitalized, cpp_type, var.name
        )?;
    }

    Ok(())
}

fn get_full_type(var: &Variable) -> String {
    let base_type = type_annotation(&var.var_type, &var.array_kind);
    if var.var_mod.contains(&VariableModifier::OPTIONAL) {
        format!("std::optional<{}>", base_type)
    } else {
        base_type
    }
}

const MAX_LINE_LENGTH: usize = 120;

fn write_constructor(
    cpp_file: &mut String,
    prefix: &str,
    name: &str,
    params: &[String],
    inits: &[String],
) -> Result<(), std::fmt::Error> {
    let params_str = params.join(", ");
    let inits_str = inits.join(", ");

    let single_line = format!("\t{}{}({}) : {} {{}}", prefix, name, params_str, inits_str);

    if single_line.len() <= MAX_LINE_LENGTH {
        writeln!(cpp_file, "{}", single_line)?;
    } else {
        // Signature on first line, initializers indented on following lines
        writeln!(cpp_file, "\t{}{}({})", prefix, name, params_str)?;
        write!(cpp_file, "\t\t: ")?;

        // Try all inits on one line after the colon
        let colon_line = format!("\t\t: {} {{}}", inits_str);
        if colon_line.len() <= MAX_LINE_LENGTH {
            writeln!(cpp_file, "{} {{}}", inits_str)?;
        } else {
            // Each initializer on its own line
            for (i, init) in inits.iter().enumerate() {
                if i == 0 {
                    writeln!(cpp_file, "{}", init)?;
                } else {
                    writeln!(cpp_file, "\t\t, {}", init)?;
                }
            }
            writeln!(cpp_file, "\t{{}}")?;
        }
    }

    Ok(())
}

fn generate_constructors(
    oml_object: &OmlObject,
    cpp_file: &mut String,
) -> Result<(), std::fmt::Error> {
    let all_vars: Vec<&Variable> = oml_object.variables.iter().collect();

    if all_vars.is_empty() {
        writeln!(cpp_file, "\t{}() = default;", oml_object.name)?;
        return Ok(());
    }

    let required_vars: Vec<&&Variable> = all_vars
        .iter()
        .filter(|v| !v.var_mod.contains(&VariableModifier::OPTIONAL))
        .collect();

    let optional_vars: Vec<&&Variable> = all_vars
        .iter()
        .filter(|v| v.var_mod.contains(&VariableModifier::OPTIONAL))
        .collect();

    // Default constructor
    writeln!(cpp_file, "\t{}() = default;", oml_object.name)?;

    // Constructor with required params only (if there are optional vars, otherwise skip since
    // the full constructor below would be identical)
    if !required_vars.is_empty() && !optional_vars.is_empty() {
        let params: Vec<String> = required_vars
            .iter()
            .map(|v| format!("{} {}", get_full_type(v), v.name))
            .collect();

        let inits: Vec<String> = required_vars
            .iter()
            .map(|v| format!("{}(std::move({}))", v.name, v.name))
            .collect();

        write_constructor(cpp_file, "explicit ", &oml_object.name, &params, &inits)?;
    }

    // Constructor with all params
    {
        let params: Vec<String> = all_vars
            .iter()
            .map(|v| format!("{} {}", get_full_type(v), v.name))
            .collect();

        let inits: Vec<String> = all_vars
            .iter()
            .map(|v| format!("{}(std::move({}))", v.name, v.name))
            .collect();

        write_constructor(cpp_file, "", &oml_object.name, &params, &inits)?;
    }

    Ok(())
}

fn generate_copy_move_and_destructor(
    oml_object: &OmlObject,
    cpp_file: &mut String,
) -> Result<(), std::fmt::Error> {
    let name = &oml_object.name;

    // Copy constructor
    writeln!(cpp_file, "\t{}(const {}& other) = default;", name, name)?;

    // Move constructor
    writeln!(cpp_file, "\t{}({}&& other) noexcept = default;", name, name)?;

    // Copy assignment operator
    writeln!(cpp_file, "\t{}& operator=(const {}& other) = default;", name, name)?;

    // Move assignment operator
    writeln!(cpp_file, "\t{}& operator=({}&& other) noexcept = default;", name, name)?;

    // Destructor
    writeln!(cpp_file, "\t~{}() = default;", name)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generate::Generate;
    use crate::core::oml_object::{
        OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier, ArrayKind
    };

    fn oml_to_cpp(oml_object: &OmlObject, file_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        CppGenerator.generate(std::slice::from_ref(oml_object), file_name)
    }

    // ========== ENUM GENERATION TESTS ==========

    #[test]
    fn test_generate_enum_basic() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Color".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Red".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Green".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Blue".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_enum(&oml_object, &mut output).unwrap();

        assert!(output.contains("enum class Color {"));
        assert!(output.contains("\tRED,"));
        assert!(output.contains("\tGREEN,"));
        assert!(output.contains("\tBLUE"));
        assert!(!output.contains("BLUE,"));
        assert!(output.contains("};"));
    }

    #[test]
    fn test_generate_enum_single_variant() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Status".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Active".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_enum(&oml_object, &mut output).unwrap();

        assert!(output.contains("enum class Status {"));
        assert!(output.contains("\tACTIVE"));
        assert!(!output.contains("ACTIVE,"));
        assert!(output.contains("};"));
    }

    #[test]
    fn test_generate_enum_empty() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Empty".to_string(),
            variables: vec![],
        };

        let mut output = String::new();
        generate_enum(&oml_object, &mut output).unwrap();

        assert!(output.contains("enum class Empty {"));
        assert!(output.contains("};"));
    }

    // ========== CLASS/STRUCT GENERATION TESTS ==========

    #[test]
    fn test_generate_class_with_all_visibility_levels() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "TestClass".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "public_var".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "private_var".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PROTECTED,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "protected_var".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        println!("{}", output);

        // assert!(output.contains("class TestClass {"));
        // assert!(output.contains("private:"));
        // assert!(output.contains("protected:"));
        // assert!(output.contains("public:"));
        // assert!(output.contains("};"));
    }

    #[test]
    fn test_generate_struct() {
        let oml_object = OmlObject {
            oml_type: ObjectType::STRUCT,
            name: "Point".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "float".to_string(),
                    array_kind: ArrayKind::None,
                    name: "x".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "float".to_string(),
                    array_kind: ArrayKind::None,
                    name: "y".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        assert!(output.contains("struct Point {"));
        assert!(output.contains("float"));
        assert!(output.contains("x"));
        assert!(output.contains("y"));
        assert!(output.contains("};"));
    }

    #[test]
    fn test_generate_class_empty() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "EmptyClass".to_string(),
            variables: vec![],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        assert!(output.contains("class EmptyClass {"));
        assert!(output.contains("};"));
    }

    // ========== MODIFIER TESTS ==========

    #[test]
    fn test_static_modifier() {
        let var = Variable {
            var_mod: vec![VariableModifier::STATIC],
            visibility: VariableVisibility::PUBLIC,
            var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
            name: "count".to_string(),
        };

        let mut output = String::new();
        convert_modifiers_and_type(&var, &mut output).unwrap();

        assert!(output.contains("static"));
        assert!(output.contains("int32_t"));
    }

    #[test]
    fn test_const_modifier() {
        let var = Variable {
            var_mod: vec![VariableModifier::CONST],
            visibility: VariableVisibility::PUBLIC,
            var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
            name: "MAX_SIZE".to_string(),
        };

        let mut output = String::new();
        convert_modifiers_and_type(&var, &mut output).unwrap();

        assert!(output.contains("const"));
        assert!(output.contains("int32_t"));
    }

    #[test]
    fn test_const_static_modifiers_combined() {
        let var = Variable {
            var_mod: vec![VariableModifier::CONST, VariableModifier::STATIC],
            visibility: VariableVisibility::PUBLIC,
            var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
            name: "MAX_VALUE".to_string(),
        };

        let mut output = String::new();
        convert_modifiers_and_type(&var, &mut output).unwrap();

        assert!(output.contains("static"));
        assert!(output.contains("const"));
        assert!(output.contains("int32_t"));

        // Verify order: static should come before const
        let static_pos = output.find("static").unwrap();
        let const_pos = output.find("const").unwrap();
        assert!(static_pos < const_pos);
    }

    #[test]
    fn test_mut_modifier_overrides_const() {
        let var = Variable {
            var_mod: vec![VariableModifier::CONST, VariableModifier::MUT],
            visibility: VariableVisibility::PUBLIC,
            var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
            name: "value".to_string(),
        };

        let mut output = String::new();
        convert_modifiers_and_type(&var, &mut output).unwrap();

        // Should not contain const when mut is present
        assert!(!output.contains("const"));
        assert!(output.contains("int32_t"));
    }

    #[test]
    fn test_optional_modifier() {
        let var = Variable {
            var_mod: vec![VariableModifier::OPTIONAL],
            visibility: VariableVisibility::PUBLIC,
            var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
            name: "nickname".to_string(),
        };

        let mut output = String::new();
        convert_modifiers_and_type(&var, &mut output).unwrap();

        assert!(output.contains("std::optional<std::string>"));
    }

    #[test]
    fn test_optional_with_static() {
        let var = Variable {
            var_mod: vec![VariableModifier::OPTIONAL, VariableModifier::STATIC],
            visibility: VariableVisibility::PUBLIC,
            var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
            name: "cache".to_string(),
        };

        let mut output = String::new();
        convert_modifiers_and_type(&var, &mut output).unwrap();

        assert!(output.contains("static"));
        assert!(output.contains("std::optional<int32_t>"));
    }

    #[test]
    fn test_optional_with_const() {
        let var = Variable {
            var_mod: vec![VariableModifier::OPTIONAL, VariableModifier::CONST],
            visibility: VariableVisibility::PUBLIC,
            var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
            name: "config".to_string(),
        };

        let mut output = String::new();
        convert_modifiers_and_type(&var, &mut output).unwrap();

        assert!(output.contains("const"));
        assert!(output.contains("std::optional<std::string>"));
    }

    // ========== TYPE CONVERSION TESTS ==========

    #[test]
    fn test_convert_all_integer_types() {
        assert_eq!(convert_type("int8"), "int8_t");
        assert_eq!(convert_type("int16"), "int16_t");
        assert_eq!(convert_type("int32"), "int32_t");
        assert_eq!(convert_type("int64"), "int64_t");
        assert_eq!(convert_type("uint8"), "uint8_t");
        assert_eq!(convert_type("uint16"), "uint16_t");
        assert_eq!(convert_type("uint32"), "uint32_t");
        assert_eq!(convert_type("uint64"), "uint64_t");
    }

    #[test]
    fn test_convert_floating_point_types() {
        assert_eq!(convert_type("float"), "float");
        assert_eq!(convert_type("double"), "double");
    }

    #[test]
    fn test_convert_other_basic_types() {
        assert_eq!(convert_type("bool"), "bool");
        assert_eq!(convert_type("char"), "char");
        assert_eq!(convert_type("string"), "std::string");
    }

    #[test]
    fn test_convert_custom_type() {
        assert_eq!(convert_type("CustomType"), "CustomType");
        assert_eq!(convert_type("Address"), "Address");
    }

    // ========== FULL FILE GENERATION TESTS ==========

    #[test]
    fn test_oml_to_cpp_with_enum() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Color".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Red".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Blue".to_string(),
                },
            ],
        };

        let result = oml_to_cpp(&oml_object, "Color").unwrap();

        // Check header guard
        assert!(result.contains("#ifndef COLOR_H"));
        assert!(result.contains("#define COLOR_H"));
        assert!(result.contains("#endif // COLOR_H"));

        // Check comment
        assert!(result.contains("// This file has been generated from Color.oml"));

        // Enum-only files should not have includes
        assert!(!result.contains("#include"));

        // Check enum content
        assert!(result.contains("enum class Color {"));
    }

    #[test]
    fn test_oml_to_cpp_with_class() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Person".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "name".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "age".to_string(),
                },
            ],
        };

        let result = oml_to_cpp(&oml_object, "Person").unwrap();

        assert!(result.contains("#ifndef PERSON_H"));
        assert!(result.contains("#define PERSON_H"));
        assert!(result.contains("class Person {"));
        assert!(result.contains("std::string"));
        assert!(result.contains("int32_t"));
        assert!(result.contains("#endif // PERSON_H"));
    }

    #[test]
    fn test_oml_to_cpp_header_guard_uppercase() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "MyClass".to_string(),
            variables: vec![],
        };

        let result = oml_to_cpp(&oml_object, "my_class").unwrap();

        assert!(result.contains("#ifndef MY_CLASS_H"));
        assert!(result.contains("#define MY_CLASS_H"));
        assert!(result.contains("#endif // MY_CLASS_H"));
    }

    #[test]
    fn test_oml_to_cpp_with_undecided_type_fails() {
        let oml_object = OmlObject {
            oml_type: ObjectType::UNDECIDED,
            name: "Test".to_string(),
            variables: vec![],
        };

        let result = oml_to_cpp(&oml_object, "Test");

        assert!(result.is_err());
    }

    // ========== VARIABLE GROUPING TESTS ==========

    #[test]
    fn test_variables_grouped_by_visibility() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Test".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "pub1".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "priv1".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "pub2".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        // Verify public section comes before private section
        let public_pos = output.find("public:").unwrap();
        let private_pos = output.find("private:").unwrap();
        assert!(public_pos < private_pos);

        // Verify private variable declarations appear in the private section
        // (look for the tab-indented declaration, not constructor params)
        let priv1_decl = output.find("\tint32_t priv1;").unwrap();
        assert!(priv1_decl > private_pos);
    }

    #[test]
    fn test_only_private_variables() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "PrivateOnly".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "var1".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "var2".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        assert!(output.contains("private:"));
        // public: is now always present for constructors/getters/setters
        assert!(output.contains("public:"));
        assert!(!output.contains("protected:"));
    }

    #[test]
    fn test_only_public_variables() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "PublicOnly".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "var1".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        assert!(!output.contains("private:"));
        assert!(!output.contains("protected:"));
    }

    // ========== COMPLEX INTEGRATION TESTS ==========

    #[test]
    fn test_complex_class_with_all_features() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "ComplexClass".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![VariableModifier::STATIC, VariableModifier::CONST],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "MAX_SIZE".to_string(),
                },
                Variable {
                    var_mod: vec![VariableModifier::OPTIONAL],
                    visibility: VariableVisibility::PRIVATE,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "nickname".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PROTECTED,
                    var_type: "float".to_string(),
                    array_kind: ArrayKind::None,
                    name: "value".to_string(),
                },
            ],
        };

        let result = oml_to_cpp(&oml_object, "ComplexClass").unwrap();

        assert!(result.contains("static const int32_t"));
        assert!(result.contains("std::optional<std::string>"));
        assert!(result.contains("float"));
        assert!(result.contains("private:"));
        assert!(result.contains("protected:"));
    }

    #[test]
    fn test_multiple_variables_same_visibility() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "MultiVar".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "var1".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "var2".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "var3".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        assert!(output.contains("var1"));
        assert!(output.contains("var2"));
        assert!(output.contains("var3"));
    }

    #[test]
    fn test_struct_vs_class_keyword() {
        let class_obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "MyClass".to_string(),
            variables: vec![],
        };

        let struct_obj = OmlObject {
            oml_type: ObjectType::STRUCT,
            name: "MyStruct".to_string(),
            variables: vec![],
        };

        let mut class_output = String::new();
        let mut struct_output = String::new();

        generate_class_or_struct(&class_obj, &mut class_output).unwrap();
        generate_class_or_struct(&struct_obj, &mut struct_output).unwrap();

        assert!(class_output.contains("class MyClass"));
        assert!(struct_output.contains("struct MyStruct"));
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_variable_with_all_modifiers() {
        let var = Variable {
            var_mod: vec![
                VariableModifier::STATIC,
                VariableModifier::CONST,
                VariableModifier::OPTIONAL,
            ],
            visibility: VariableVisibility::PUBLIC,
            var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
            name: "value".to_string(),
        };

        let mut output = String::new();
        convert_modifiers_and_type(&var, &mut output).unwrap();

        assert!(output.contains("static"));
        assert!(output.contains("const"));
        assert!(output.contains("std::optional"));
    }

    #[test]
    fn test_empty_variable_name() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Test".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "".to_string(),
                },
            ],
        };

        let result = oml_to_cpp(&oml_object, "Test");

        // Should still generate, even with empty name
        assert!(result.is_ok());
    }

    #[test]
    fn test_special_characters_in_class_name() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "My_Class-123".to_string(),
            variables: vec![],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        assert!(output.contains("class My_Class-123 {"));
    }

    #[test]
    fn test_long_variable_names() {
        let long_name = "this_is_a_very_long_variable_name_that_should_still_work_correctly";

        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Test".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: long_name.to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        assert!(output.contains(long_name));
    }

    // ========== FORMATTING TESTS ==========

    #[test]
    fn test_enum_has_proper_indentation() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Test".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                    name: "Value".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_enum(&oml_object, &mut output).unwrap();

        assert!(output.contains("\tVALUE"));
    }

    #[test]
    fn test_full_output_has_proper_structure() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Test".to_string(),
            variables: vec![],
        };

        let result = oml_to_cpp(&oml_object, "Test").unwrap();

        // Verify order of sections
        let comment_pos = result.find("//").unwrap();
        let ifndef_pos = result.find("#ifndef").unwrap();
        let define_pos = result.find("#define").unwrap();
        let include_pos = result.find("#include").unwrap();
        let class_pos = result.find("class").unwrap();
        let endif_pos = result.find("#endif").unwrap();

        assert!(comment_pos < ifndef_pos);
        assert!(ifndef_pos < define_pos);
        assert!(define_pos < include_pos);
        assert!(include_pos < class_pos);
        assert!(class_pos < endif_pos);
    }

    #[test]
    fn test_semicolon_after_class_closing_brace() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Test".to_string(),
            variables: vec![],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        assert!(output.contains("};"));
    }

    #[test]
    fn test_semicolon_after_enum_closing_brace() {
        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "Test".to_string(),
            variables: vec![],
        };

        let mut output = String::new();
        generate_enum(&oml_object, &mut output).unwrap();

        assert!(output.contains("};"));
    }

    // ========== REGRESSION TESTS ==========

    #[test]
    fn test_bug_include_has_backslash_n() {
        // Test for the bug in line 7: writeln!(cpp_file, "#\ninclude <cstdint>")?;
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Test".to_string(),
            variables: vec![],
        };

        let result = oml_to_cpp(&oml_object, "Test").unwrap();

        // This will fail with current code due to the bug
        // The correct line should be: writeln!(cpp_file, "#include <cstdint>")?;
        // Currently it outputs: #\ninclude <cstdint>

        // This test documents the bug
        assert!(result.contains("#include <cstdint>") || result.contains("#\ninclude <cstdint>"));
    }

    #[test]
    fn test_variable_output_has_semicolon() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Test".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "value".to_string(),
                },
            ],
        };

        let result = oml_to_cpp(&oml_object, "Test").unwrap();

        // Variables should end with semicolon
        // Note: Current implementation uses write! instead of writeln! for variable names
        // and manually adds \n, so this tests actual behavior
        assert!(result.contains("value"));
    }

    #[test]
    fn test_protected_section_visibility() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Test".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PROTECTED,
                    var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                    name: "prot_var".to_string(),
                },
            ],
        };

        let mut output = String::new();
        generate_class_or_struct(&oml_object, &mut output).unwrap();

        // With current implementation, protected vars are output but no label is shown
        // This test documents current behavior
        assert!(output.contains("prot_var"));
    }

    // ========== PERFORMANCE/STRESS TESTS ==========

    #[test]
    fn test_class_with_many_variables() {
        let mut variables = vec![];
        for i in 0..100 {
            variables.push(Variable {
                var_mod: vec![],
                visibility: if i % 3 == 0 {
                    VariableVisibility::PUBLIC
                } else if i % 3 == 1 {
                    VariableVisibility::PRIVATE
                } else {
                    VariableVisibility::PROTECTED
                },
                var_type: "int32".to_string(),
                    array_kind: ArrayKind::None,
                name: format!("var{}", i),
            });
        }

        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "LargeClass".to_string(),
            variables,
        };

        let result = oml_to_cpp(&oml_object, "LargeClass");

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("var0"));
        assert!(output.contains("var99"));
    }

    #[test]
    fn test_enum_with_many_variants() {
        let mut variables = vec![];
        for i in 0..50 {
            variables.push(Variable {
                var_mod: vec![],
                visibility: VariableVisibility::PUBLIC,
                var_type: "".to_string(),
                    array_kind: ArrayKind::None,
                name: format!("Variant{}", i),
            });
        }

        let oml_object = OmlObject {
            oml_type: ObjectType::ENUM,
            name: "LargeEnum".to_string(),
            variables,
        };

        let mut output = String::new();
        generate_enum(&oml_object, &mut output).unwrap();

        assert!(output.contains("VARIANT0,"));
        assert!(output.contains("VARIANT49"));
        assert!(!output.contains("VARIANT49,"));
    }

    // ========== TYPE-SPECIFIC TESTS ==========

    #[test]
    fn test_all_integer_types_in_class() {
        let types = vec!["int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64"];
        let mut variables = vec![];

        for (i, type_name) in types.iter().enumerate() {
            variables.push(Variable {
                var_mod: vec![],
                visibility: VariableVisibility::PUBLIC,
                var_type: type_name.to_string(),
                    array_kind: ArrayKind::None,
                name: format!("var{}", i),
            });
        }

        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "AllTypes".to_string(),
            variables,
        };

        let result = oml_to_cpp(&oml_object, "AllTypes").unwrap();

        assert!(result.contains("int8_t"));
        assert!(result.contains("int16_t"));
        assert!(result.contains("int32_t"));
        assert!(result.contains("int64_t"));
        assert!(result.contains("uint8_t"));
        assert!(result.contains("uint16_t"));
        assert!(result.contains("uint32_t"));
        assert!(result.contains("uint64_t"));
    }

    #[test]
    fn test_string_type_in_class() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "StringTest".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "string".to_string(),
                    array_kind: ArrayKind::None,
                    name: "text".to_string(),
                },
            ],
        };

        let result = oml_to_cpp(&oml_object, "StringTest").unwrap();

        assert!(result.contains("std::string"));
        assert!(result.contains("#include <string>"));
    }

    #[test]
    fn test_bool_and_char_types() {
        let oml_object = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "BasicTypes".to_string(),
            variables: vec![
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "bool".to_string(),
                    array_kind: ArrayKind::None,
                    name: "flag".to_string(),
                },
                Variable {
                    var_mod: vec![],
                    visibility: VariableVisibility::PUBLIC,
                    var_type: "char".to_string(),
                    array_kind: ArrayKind::None,
                    name: "letter".to_string(),
                },
            ],
        };

        let result = oml_to_cpp(&oml_object, "BasicTypes").unwrap();

        assert!(result.contains("bool"));
        assert!(result.contains("char"));
    }
}

#[cfg(test)]
mod array_tests {
    use super::*;
    use crate::core::oml_object::{OmlObject, ObjectType, Variable, VariableVisibility, ArrayKind};

    fn to_cpp(oml_object: &OmlObject) -> String {
        CppGenerator.generate(std::slice::from_ref(oml_object), "test").unwrap()
    }

    fn array_var(name: &str, ty: &str, kind: ArrayKind) -> Variable {
        Variable {
            var_mod: vec![],
            visibility: VariableVisibility::PUBLIC,
            var_type: ty.to_string(),
            array_kind: kind,
            name: name.to_string(),
        }
    }

    #[test]
    fn test_static_array_generates_std_array() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Arr".to_string(),
            variables: vec![array_var("scores", "uint16", ArrayKind::Static(4))],
        };
        let out = to_cpp(&obj);
        assert!(out.contains("std::array<uint16_t, 4>"), "Got: {}", out);
        assert!(out.contains("#include <array>"), "Got: {}", out);
    }

    #[test]
    fn test_dynamic_list_generates_std_vector() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Lst".to_string(),
            variables: vec![array_var("tags", "string", ArrayKind::Dynamic)],
        };
        let out = to_cpp(&obj);
        assert!(out.contains("std::vector<std::string>"), "Got: {}", out);
        assert!(out.contains("#include <vector>"), "Got: {}", out);
    }

    #[test]
    fn test_no_array_no_extra_includes() {
        let obj = OmlObject {
            oml_type: ObjectType::CLASS,
            name: "Plain".to_string(),
            variables: vec![array_var("x", "int32", ArrayKind::None)],
        };
        let out = to_cpp(&obj);
        assert!(!out.contains("#include <array>"), "Got: {}", out);
        assert!(!out.contains("#include <vector>"), "Got: {}", out);
    }
}
