use figment::{
    Figment,
    providers::{Format, Toml},
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Setup configuration structure for the Obake launcher application.
///
/// This structure represents the setup configuration loaded from TOML files,
/// containing interface settings and shape configurations.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct SetupConfig {
    /// Main setup configuration section
    pub setup: SetupSection,
    /// Map of shape configurations by name
    pub shapes: HashMap<String, ShapeConfig>,
}

/// Main setup configuration section.
///
/// This structure holds the primary setup settings including the default
/// interface and list of shapes to be managed.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct SetupSection {
    /// The name of the interface to use for setup
    pub interface: String,
    /// List of shape names to be configured
    pub shapes: Vec<String>,
}

/// Configuration for a single shape.
///
/// This structure defines the properties of a shape including its container
/// image and environment variables.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ShapeConfig {
    /// The container image to use for this shape (optional)
    ///
    /// If not specified, the shape may not have an associated container image.
    pub image: Option<String>,
    /// Environment variables for the shape (optional)
    pub env: Option<HashMap<String, String>>,
}

impl SetupConfig {
    /// Load setup configuration from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the TOML configuration file to load
    ///
    /// # Returns
    ///
    /// - `Ok(SetupConfig)`: Successfully loaded and parsed configuration
    /// - `Err(Box<dyn std::error::Error>)`: Error loading or parsing the configuration file
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The specified file doesn't exist
    /// - The file cannot be read
    /// - The TOML content is invalid
    /// - The configuration structure doesn't match the expected format
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::setup::SetupConfig;
    ///
    /// // Load test setup configuration
    /// let test_config = SetupConfig::load_from_path("test_setup.toml")?;
    /// ```
    pub fn load_from_path(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        debug!("Loading setup configuration from path: {}", path);
        let config: SetupConfig = Figment::new().merge(Toml::file(path)).extract()?;

        debug!("Setup configuration loaded from path: {}", path);
        Ok(config)
    }

    /// Get the configuration for a specific shape by name.
    ///
    /// This method allows you to retrieve the configuration for any shape
    /// by specifying its name. This is useful when you need to work with
    /// a specific shape configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the shape to retrieve
    ///
    /// # Returns
    ///
    /// - `Some(&ShapeConfig)`: Configuration for the specified shape
    /// - `None`: If the shape name is not found in the configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::setup::SetupConfig;
    ///
    /// let config = SetupConfig::load()?;
    ///
    /// // Get a specific shape configuration
    /// if let Some(shape) = config.get_shape("serialosc") {
    ///     // Check if the shape has an image
    ///     if let Some(image) = &shape.image {
    ///         println!("Shape image: {}", image);
    ///     } else {
    ///         println!("Shape has no container image");
    ///     }
    ///     
    ///     // Check if the shape has environment variables
    ///     if let Some(env) = &shape.env {
    ///         for (key, value) in env {
    ///             println!("Environment variable {} = {}", key, value);
    ///         }
    ///     }
    /// } else {
    ///     println!("Shape 'serialosc' not configured");
    /// }
    /// ```
    pub fn get_shape(&self, name: &str) -> Option<&ShapeConfig> {
        self.shapes.get(name)
    }

    /// List all available shape names.
    ///
    /// This method returns a vector containing references to all the shape
    /// names defined in the configuration. This is useful for displaying
    /// available options to users or for iterating over all configured shapes.
    ///
    /// # Returns
    ///
    /// A `Vec<&String>` containing references to all shape names in the configuration.
    /// The order of the shapes is not guaranteed as it depends on the HashMap iteration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::setup::SetupConfig;
    ///
    /// let config = SetupConfig::load()?;
    ///
    /// // List all available shapes
    /// println!("Available shapes:");
    /// for shape_name in config.list_shapes() {
    ///     println!("  - {}", shape_name);
    /// }
    ///
    /// // Check if a specific shape exists
    /// let available_shapes = config.list_shapes();
    /// if available_shapes.contains(&&"serialosc".to_string()) {
    ///     println!("SerialOSC shape is available");
    /// }
    ///
    /// // Count available shapes
    /// println!("Total shapes configured: {}", available_shapes.len());
    /// ```
    pub fn list_shapes(&self) -> Vec<&String> {
        self.shapes.keys().collect()
    }

    /// Get the list of shapes from the setup section.
    ///
    /// This method returns the list of shape names specified in the setup
    /// section, which represents the shapes that should be managed.
    ///
    /// # Returns
    ///
    /// A reference to the vector of shape names from the setup section.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::setup::SetupConfig;
    ///
    /// let config = SetupConfig::load()?;
    ///
    /// // Get the list of shapes to manage
    /// let shapes_to_manage = config.get_managed_shapes();
    /// println!("Shapes to manage: {:?}", shapes_to_manage);
    ///
    /// // Iterate over shapes to manage
    /// for shape_name in shapes_to_manage {
    ///     println!("Managing shape: {}", shape_name);
    /// }
    /// ```
    pub fn get_managed_shapes(&self) -> &Vec<String> {
        &self.setup.shapes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_config_loading() {
        // Test loading from a specific path (for backward compatibility with existing setup.toml)
        if std::path::Path::new("setup.toml").exists() {
            let config = SetupConfig::load_from_path("setup.toml");
            assert!(config.is_ok());

            let config = config.unwrap();
            assert_eq!(config.setup.interface, "mixpre");
            assert!(config.setup.shapes.contains(&"serialosc".to_string()));
            assert!(config.setup.shapes.contains(&"siren".to_string()));

            // Test that shapes are parsed correctly
            if let Some(serialosc_shape) = config.shapes.get("serialosc") {
                assert!(serialosc_shape.image.is_some());
                assert_eq!(serialosc_shape.image.as_ref().unwrap(), "serialosc.sif");
                assert!(serialosc_shape.env.is_some());

                if let Some(env) = &serialosc_shape.env {
                    assert!(env.contains_key("SINGULARITY_BIND"));
                    assert_eq!(env.get("SINGULARITY_BIND").unwrap(), "/run/udev:/run/udev");
                }
            }
        }
    }

    #[test]
    fn test_setup_config_with_tempfile() {
        // Test setup configuration loading with temporary file
        let test_config = r#"
[setup]
interface = "test_interface"
shapes = ["shape1", "shape2"]

[shapes.shape1]
image = "shape1.sif"

[shapes.shape2]
image = "shape2.sif"

[shapes.shape2.env]
TEST_VAR = "test_value"
"#;

        // Create a temporary file
        let temp_file = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
        let config_path = temp_file.path().to_str().unwrap();

        // Write config content to temporary file
        std::fs::write(config_path, test_config).unwrap();

        // Test loading from the temporary path
        let config = SetupConfig::load_from_path(config_path);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.setup.interface, "test_interface");
        assert_eq!(config.setup.shapes.len(), 2);
        assert!(config.setup.shapes.contains(&"shape1".to_string()));
        assert!(config.setup.shapes.contains(&"shape2".to_string()));

        // Test shape configurations
        if let Some(shape1) = config.get_shape("shape1") {
            assert!(shape1.image.is_some());
            assert_eq!(shape1.image.as_ref().unwrap(), "shape1.sif");
            assert!(shape1.env.is_none());
        }

        if let Some(shape2) = config.get_shape("shape2") {
            assert!(shape2.image.is_some());
            assert_eq!(shape2.image.as_ref().unwrap(), "shape2.sif");
            assert!(shape2.env.is_some());

            if let Some(env) = &shape2.env {
                assert!(env.contains_key("TEST_VAR"));
                assert_eq!(env.get("TEST_VAR").unwrap(), "test_value");
            }
        }

        // Test helper methods
        let managed_shapes = config.get_managed_shapes();
        assert_eq!(managed_shapes.len(), 2);

        let available_shapes = config.list_shapes();
        assert_eq!(available_shapes.len(), 2);

        // Clean up is automatic when temp_file goes out of scope
    }

    #[test]
    fn test_optional_image_field() {
        // Test configuration with optional image field
        let test_config = r#"
[setup]
interface = "test_interface"
shapes = ["shape_with_image", "shape_without_image"]

[shapes.shape_with_image]
image = "shape_with_image.sif"

[shapes.shape_without_image]
# No image field specified
"#;

        // Create a temporary file
        let temp_file = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
        let config_path = temp_file.path().to_str().unwrap();

        // Write config content to temporary file
        std::fs::write(config_path, test_config).unwrap();

        // Test loading from the temporary path
        let config = SetupConfig::load_from_path(config_path);
        assert!(config.is_ok());

        let config = config.unwrap();

        // Test shape with image
        if let Some(shape_with_image) = config.get_shape("shape_with_image") {
            assert!(shape_with_image.image.is_some());
            assert_eq!(
                shape_with_image.image.as_ref().unwrap(),
                "shape_with_image.sif"
            );
        }

        // Test shape without image
        if let Some(shape_without_image) = config.get_shape("shape_without_image") {
            assert!(shape_without_image.image.is_none());
        }

        // Clean up is automatic when temp_file goes out of scope
    }
}
