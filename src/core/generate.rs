use std::error::Error;
use crate::core::oml_object::OmlObject;

/// Trait that should be used to convert OML to a programming language.
/// This is a must as the OML CLI uses the functions from this trait.
pub trait Generate {
    /// Generate the code in the respective language given the OML objects and file name.
    /// All objects from the same .oml file are passed together so they can be
    /// emitted into a single output file.
    /// If there is an error, it will be returned as a Conversion Error
    fn generate(&self, oml_objects: &[OmlObject], file_name: &str) -> Result<String, Box<dyn Error>>;

    /// Gives the file extension so that it can be saved correctly.
    fn extension(&self) -> &str;
}

/// Trait for converting generated code back into OML objects.
/// Implementors parse language-specific source code and reconstruct the
/// original OML representation.
pub trait BackwardsGenerate {
    /// Parse the given source content back into a list of OML objects.
    fn reverse(&self, content: &str) -> Result<Vec<OmlObject>, Box<dyn Error>>;
}
