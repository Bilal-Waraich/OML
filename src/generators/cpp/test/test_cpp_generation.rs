use std::fs;
use std::path::Path;

use crate::core::generate::Generate;
use crate::core::oml_object::{
    OmlObject, ObjectType, Variable, VariableVisibility, VariableModifier
};
use crate::generators::cpp::oml_cpp::CppGenerator;

const TEST_RESULTS_DIR: &str = "test_results";

fn ensure_test_results_dir() {
    fs::create_dir_all(TEST_RESULTS_DIR).expect("Failed to create test_results directory");
}

fn generate_and_write(oml_path: &str, file_name: &str) -> String {
    ensure_test_results_dir();

    let generator = CppGenerator;

    let path = Path::new(oml_path);
    let oml_objects = OmlObject::get_from_file(path)
        .expect(&format!("Failed to parse OML file: {}", oml_path));

    let cpp_output = generator.generate(&oml_objects, file_name)
        .expect(&format!("Failed to generate C++ for: {}", file_name));

    let output_path = format!("{}/{}.{}", TEST_RESULTS_DIR, file_name, generator.extension());
    fs::write(&output_path, &cpp_output)
        .expect(&format!("Failed to write output file: {}", output_path));

    cpp_output
}

#[test]
fn test_person_class_generates_cpp_file() {
    let output = generate_and_write("src/generators/cpp/test_oml_files/person.oml", "Person");

    // Header guards
    assert!(output.contains("#ifndef PERSON_H"));
    assert!(output.contains("#define PERSON_H"));
    assert!(output.contains("#endif // PERSON_H"));

    // Class declaration
    assert!(output.contains("class Person {"));

    // Private members (default visibility)
    assert!(output.contains("private:"));
    assert!(output.contains("std::string name;"));
    assert!(output.contains("int32_t age;"));
    assert!(output.contains("std::optional<std::string> nickname;"));

    // Default constructor
    assert!(output.contains("Person() = default;"));

    // Constructor with required params only (name, age)
    assert!(output.contains("explicit Person(std::string name, int32_t age)"));

    // Constructor with all params (name, age, nickname)
    assert!(output.contains("Person(std::string name, int32_t age, std::optional<std::string> nickname)"));

    // Copy/move constructors
    assert!(output.contains("Person(const Person& other) = default;"));
    assert!(output.contains("Person(Person&& other) noexcept = default;"));

    // Assignment operators
    assert!(output.contains("Person& operator=(const Person& other) = default;"));
    assert!(output.contains("Person& operator=(Person&& other) noexcept = default;"));

    // Destructor
    assert!(output.contains("~Person() = default;"));

    // Getters
    assert!(output.contains("std::string getName() const { return name; }"));
    assert!(output.contains("int32_t getAge() const { return age; }"));
    assert!(output.contains("std::optional<std::string> getNickname() const { return nickname; }"));

    // Setters
    assert!(output.contains("void setName(const std::string& value) { name = value; }"));
    assert!(output.contains("void setAge(const int32_t& value) { age = value; }"));
    assert!(output.contains("void setNickname(const std::optional<std::string>& value) { nickname = value; }"));
}

#[test]
fn test_vehicle_class_generates_cpp_file() {
    let output = generate_and_write("src/generators/cpp/test_oml_files/vehicle.oml", "Vehicle");

    // Class structure
    assert!(output.contains("class Vehicle {"));

    // Required-only constructor should have make, model, year
    assert!(output.contains("explicit Vehicle(std::string make, std::string model, int32_t year)"));

    // Full constructor should have all 5 params
    assert!(output.contains("Vehicle(std::string make, std::string model, int32_t year, std::optional<double> mileage, std::optional<std::string> color)"));

    // Getters for all private vars
    assert!(output.contains("getMake()"));
    assert!(output.contains("getModel()"));
    assert!(output.contains("getYear()"));
    assert!(output.contains("getMileage()"));
    assert!(output.contains("getColor()"));

    // Setters for all private vars
    assert!(output.contains("setMake("));
    assert!(output.contains("setModel("));
    assert!(output.contains("setYear("));
    assert!(output.contains("setMileage("));
    assert!(output.contains("setColor("));
}

#[test]
fn test_point_struct_generates_cpp_file() {
    let output = generate_and_write("src/generators/cpp/test_oml_files/point.oml", "Point");

    // Struct declaration
    assert!(output.contains("struct Point {"));

    // Public members
    assert!(output.contains("public:"));
    assert!(output.contains("double x;"));
    assert!(output.contains("double y;"));

    // Constructors still generated
    assert!(output.contains("Point() = default;"));
    assert!(output.contains("Point(double x, double y)"));

    // No getters/setters for public members
    assert!(!output.contains("getX()"));
    assert!(!output.contains("setX("));
}

#[test]
fn test_color_enum_generates_cpp_file() {
    ensure_test_results_dir();

    let oml_object = OmlObject {
        oml_type: ObjectType::ENUM,
        name: "Color".to_string(),
        variables: vec![
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), name: "Red".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), name: "Green".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), name: "Blue".to_string() },
            Variable { var_mod: vec![], visibility: VariableVisibility::PUBLIC, var_type: "".to_string(), name: "Yellow".to_string() },
        ],
    };

    let generator = CppGenerator;
    let output = generator.generate(std::slice::from_ref(&oml_object), "Color").unwrap();

    let output_path = format!("{}/Color.h", TEST_RESULTS_DIR);
    fs::write(&output_path, &output).expect("Failed to write Color.h");

    // Enum declaration
    assert!(output.contains("enum class Color {"));
    assert!(output.contains("RED,"));
    assert!(output.contains("GREEN,"));
    assert!(output.contains("BLUE,"));
    assert!(output.contains("YELLOW"));
    // Last variant should NOT have a trailing comma
    assert!(!output.contains("YELLOW,"));

    // Enums should NOT have constructors or getters/setters
    assert!(!output.contains("Color() = default"));
    assert!(!output.contains("get"));
    assert!(!output.contains("set"));
}

#[test]
fn test_hello_class_from_core_test_files() {
    let output = generate_and_write("src/core/test/oml_files/hello.oml", "Hello");

    assert!(output.contains("class Hello {"));

    // const int64 meow => should have getter but NO setter (it's const)
    assert!(output.contains("const int64_t meow;"));
    assert!(output.contains("getMeow()"));
    assert!(!output.contains("setMeow("));

    // string hello => getter + setter
    assert!(output.contains("std::string hello;"));
    assert!(output.contains("getHello()"));
    assert!(output.contains("setHello("));

    // bool isTrue => getter + setter
    assert!(output.contains("bool isTrue;"));
    assert!(output.contains("getIsTrue()"));
    assert!(output.contains("setIsTrue("));
}
