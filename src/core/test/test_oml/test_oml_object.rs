use std::path::Path;
use crate::core::oml_object::OmlObject;

#[test]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("src/core/test/oml_files/hello.oml");
    let (objects, _imports) = OmlObject::get_from_file(path)?;
    println!("{:?}", objects);

    Ok(())
}