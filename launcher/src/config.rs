use figment::{
    Figment,
    providers::{Format, Toml},
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main configuration structure for the Obake launcher application.
///
/// This structure represents the complete configuration loaded from the TOML file,
/// containing all application settings including audio interface configurations
/// and data directory settings.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Audio configuration section containing interface settings
    pub audio: AudioConfig,
    /// Data configuration section containing directory paths
    pub data: DataConfig,
}

/// Audio configuration containing default interface and available interfaces.
///
/// This structure holds all audio-related configuration including the default
/// interface to use and a map of all available audio interfaces with their
/// specific configurations.
///
/// # Example
///
/// ```rust
/// use crate::config::Config;
///
/// let config = Config::load()?;
///
/// // Get the default interface name
/// println!("Default interface: {}", config.audio.default_interface);
///
/// // List all available interfaces
/// for interface_name in config.list_audio_interfaces() {
///     println!("Available interface: {}", interface_name);
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct AudioConfig {
    /// The name of the default audio interface to use
    ///
    /// This should correspond to a key in the `interfaces` HashMap.
    #[serde(rename = "default-interface")]
    pub default_interface: String,

    /// Map of available audio interfaces by name
    pub interfaces: HashMap<String, AudioInterface>,
}

/// Configuration for a single audio interface.
///
/// This structure defines the properties of an audio interface including
/// its type and the systemd unit that manages it.
///
/// # Example
///
/// ```rust
/// use crate::config::Config;
///
/// let config = Config::load()?;
///
/// // Get a specific interface configuration
/// if let Some(interface) = config.get_audio_interface("mixpre") {
///     println!("Interface type: {}", interface.interface_type);
///     println!("Systemd unit: {}", interface.unit);
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct AudioInterface {
    /// The type of audio interface (e.g., "jack", "alsa", "pulse")
    ///
    /// This determines how the interface is handled by the audio system.
    #[serde(rename = "type")]
    pub interface_type: String,

    /// The systemd unit name that manages this audio interface (if applicable)
    ///
    /// This unit will be started/stopped when the interface is activated/deactivated.
    /// If not specified, the interface may not have an associated systemd unit.
    ///
    /// # Example
    ///
    /// ```toml
    /// # With systemd unit
    /// "mixpre" = { type = "jack", unit = "jack@mixpre.service" }
    ///
    /// # Without systemd unit
    /// "alsa" = { type = "alsa" }
    /// ```
    pub unit: Option<String>,
}

/// Data configuration containing directory paths for the application.
///
/// This structure holds all data-related configuration including paths to
/// directories where images, setups, and data files are stored.
///
/// # Example
///
/// ```rust
/// use crate::config::Config;
///
/// let config = Config::load()?;
///
/// // Get data directory paths
/// println!("Images directory: {}", config.data.images_dir);
/// println!("Setups directory: {}", config.data.setups_dir);
/// println!("Data directory: {}", config.data.data_dir);
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct DataConfig {
    /// Directory path where shape images are stored
    ///
    /// This directory contains container images and other files
    /// related to shape definitions.
    ///
    /// # Example
    ///
    /// ```toml
    /// [data]
    /// images-dir = "/home/obake/shapes"
    /// ```
    #[serde(rename = "images-dir")]
    pub images_dir: String,

    /// Directory path where setup configurations are stored
    ///
    /// This directory contains setup configuration files
    /// that define how shapes should be configured.
    ///
    /// # Example
    ///
    /// ```toml
    /// [data]
    /// setups-dir = "/home/obake/setups"
    /// ```
    #[serde(rename = "setups-dir")]
    pub setups_dir: String,

    /// Directory path where application data is stored
    ///
    /// This directory contains runtime data, logs, and other
    /// application-specific files.
    ///
    /// # Example
    ///
    /// ```toml
    /// [data]
    /// data-dir = "/home/obake/data"
    /// ```
    #[serde(rename = "data-dir")]
    pub data_dir: String,
}

impl Config {
    /// Load configuration from `config.toml` file in standard locations.
    ///
    /// This method first checks if the `OBAKE_CONFIG_FILE` environment variable is set.
    /// If it is, the file specified by that environment variable will be loaded directly.
    /// Otherwise, it searches for a file named `config.toml` in the following locations
    /// in order of preference:
    /// 1. `$HOME/.config/obake/config.toml` (user-specific configuration)
    /// 2. `/etc/obake/config.toml` (system-wide configuration)
    ///
    /// The first file found will be loaded and parsed as a TOML configuration file.
    /// The configuration is validated against the expected structure and returned as
    /// a `Config` instance.
    ///
    /// # Returns
    ///
    /// - `Ok(Config)`: Successfully loaded and parsed configuration
    /// - `Err(Box<dyn std::error::Error>)`: Error loading or parsing the configuration file
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The `OBAKE_CONFIG_FILE` environment variable is set but the specified file doesn't exist
    /// - No `config.toml` file is found in any of the search paths (when env var is not set)
    /// - The file cannot be read
    /// - The TOML content is invalid
    /// - The configuration structure doesn't match the expected format
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::config::Config;
    ///
    /// // Load configuration from standard locations or environment variable
    /// match Config::load() {
    ///     Ok(config) => {
    ///         println!("Configuration loaded successfully");
    ///         println!("Default interface: {}", config.audio.default_interface);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to load configuration: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Environment Variable
    ///
    /// Set the `OBAKE_CONFIG_FILE` environment variable to specify a custom configuration file:
    ///
    /// ```bash
    /// export OBAKE_CONFIG_FILE="/path/to/custom/config.toml"
    /// ```
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Check if OBAKE_CONFIG_FILE environment variable is set
        if let Ok(config_file) = std::env::var("OBAKE_CONFIG_FILE") {
            debug!(
                "OBAKE_CONFIG_FILE environment variable set to: {}",
                config_file
            );
            return Self::load_from_path(&config_file);
        }

        // Search paths in order of preference
        let search_paths = vec![
            // User-specific configuration
            format!(
                "{}/.config/obake/config.toml",
                std::env::var("HOME").unwrap_or_default()
            ),
            // System-wide configuration
            "/etc/obake/config.toml".to_string(),
        ];

        // Find the first existing configuration file
        for path in &search_paths {
            debug!("checking config path: {}", path);
            if Path::new(path).exists() {
                return Self::load_from_path(path);
            }
        }

        // If no configuration file found, return an error
        Err(format!(
            "No configuration file found. Searched in: {}",
            search_paths.join(", ")
        )
        .into())
    }

    /// Load configuration from a custom file path.
    ///
    /// This method allows you to specify a custom path to a TOML configuration file
    /// instead of using the standard search paths. This is useful for testing,
    /// different environments, or when you need to load multiple configuration files.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the TOML configuration file to load
    ///
    /// # Returns
    ///
    /// - `Ok(Config)`: Successfully loaded and parsed configuration
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
    /// use crate::config::Config;
    ///
    /// // Load configuration from a custom path
    /// let config = Config::load_from_path("/etc/obake/config.toml")?;
    ///
    /// // Load test configuration
    /// let test_config = Config::load_from_path("test_config.toml")?;
    ///
    /// // Load user-specific configuration
    /// let user_config = Config::load_from_path("~/.config/obake.toml")?;
    /// ```
    pub fn load_from_path(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Config = Figment::new().merge(Toml::file(path)).extract()?;
        debug!("loaded config from path: {}", path);
        Ok(config)
    }

    /// Get the configuration for the default audio interface.
    ///
    /// This method returns the `AudioInterface` configuration for the interface
    /// specified as the default in the configuration file. If the default interface
    /// name doesn't exist in the interfaces map, `None` is returned.
    ///
    /// # Returns
    ///
    /// - `Some(&AudioInterface)`: Configuration for the default audio interface
    /// - `None`: If the default interface name is not found in the interfaces map
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::config::Config;
    ///
    /// let config = Config::load()?;
    ///
    /// // Get the default interface configuration
    /// if let Some(default_interface) = config.get_default_audio_interface() {
    ///     println!("Default interface type: {}", default_interface.interface_type);
    ///     
    ///     // Check if the interface has a systemd unit
    ///     if let Some(unit) = &default_interface.unit {
    ///         println!("Default interface unit: {}", unit);
    ///         // Start the default interface's systemd unit
    ///         // systemctl start {}", unit);
    ///     } else {
    ///         println!("Default interface has no systemd unit");
    ///     }
    /// } else {
    ///     eprintln!("Default interface '{}' not found in configuration",
    ///               config.audio.default_interface);
    /// }
    /// ```
    pub fn get_default_audio_interface(&self) -> Option<&AudioInterface> {
        self.audio.interfaces.get(&self.audio.default_interface)
    }

    /// Get the configuration for a specific audio interface by name.
    ///
    /// This method allows you to retrieve the configuration for any audio interface
    /// by specifying its name. This is useful when you need to work with a specific
    /// interface that may not be the default.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the audio interface to retrieve
    ///
    /// # Returns
    ///
    /// - `Some(&AudioInterface)`: Configuration for the specified audio interface
    /// - `None`: If the interface name is not found in the configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::config::Config;
    ///
    /// let config = Config::load()?;
    ///
    /// // Get a specific interface configuration
    /// if let Some(interface) = config.get_audio_interface("aes67") {
    ///     println!("AES67 interface type: {}", interface.interface_type);
    ///     
    ///     // Check if the interface has a systemd unit
    ///     if let Some(unit) = &interface.unit {
    ///         println!("AES67 systemd unit: {}", unit);
    ///     } else {
    ///         println!("AES67 interface has no systemd unit");
    ///     }
    ///     
    ///     // Check if this is a JACK interface
    ///     if interface.interface_type == "jack" {
    ///         println!("This is a JACK interface");
    ///     }
    /// } else {
    ///     println!("AES67 interface not configured");
    /// }
    ///
    /// // Try to get a non-existent interface
    /// match config.get_audio_interface("nonexistent") {
    ///     Some(_) => println!("Interface found"),
    ///     None => println!("Interface not found"),
    /// }
    /// ```
    pub fn get_audio_interface(&self, name: &str) -> Option<&AudioInterface> {
        self.audio.interfaces.get(name)
    }

    /// List all available audio interface names.
    ///
    /// This method returns a vector containing references to all the audio interface
    /// names defined in the configuration. This is useful for displaying available
    /// options to users or for iterating over all configured interfaces.
    ///
    /// # Returns
    ///
    /// A `Vec<&String>` containing references to all interface names in the configuration.
    /// The order of the interfaces is not guaranteed as it depends on the HashMap iteration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::config::Config;
    ///
    /// let config = Config::load()?;
    ///
    /// // List all available interfaces
    /// println!("Available audio interfaces:");
    /// for interface_name in config.list_audio_interfaces() {
    ///     println!("  - {}", interface_name);
    /// }
    ///
    /// // Check if a specific interface exists
    /// let available_interfaces = config.list_audio_interfaces();
    /// if available_interfaces.contains(&&"mixpre".to_string()) {
    ///     println!("MixPre interface is available");
    /// }
    ///
    /// // Count available interfaces
    /// println!("Total interfaces configured: {}", available_interfaces.len());
    ///
    /// // Validate that the default interface is in the list
    /// if available_interfaces.contains(&&config.audio.default_interface) {
    ///     println!("Default interface '{}' is properly configured",
    ///              config.audio.default_interface);
    /// } else {
    ///     eprintln!("Warning: Default interface '{}' not found in available interfaces",
    ///               config.audio.default_interface);
    /// }
    /// ```
    pub fn list_audio_interfaces(&self) -> Vec<&String> {
        self.audio.interfaces.keys().collect()
    }

    /// Get the images directory path.
    ///
    /// This method returns the path to the directory where shape images
    /// and container files are stored.
    ///
    /// # Returns
    ///
    /// A reference to the images directory path string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::config::Config;
    ///
    /// let config = Config::load()?;
    ///
    /// // Get the images directory path
    /// let images_dir = config.get_images_dir();
    /// println!("Images directory: {}", images_dir);
    ///
    /// // Use the path to load an image
    /// let image_path = format!("{}/my_shape.sif", images_dir);
    /// ```
    pub fn get_images_dir(&self) -> &String {
        &self.data.images_dir
    }

    /// Get the setups directory path.
    ///
    /// This method returns the path to the directory where setup
    /// configuration files are stored.
    ///
    /// # Returns
    ///
    /// A reference to the setups directory path string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::config::Config;
    ///
    /// let config = Config::load()?;
    ///
    /// // Get the setups directory path
    /// let setups_dir = config.get_setups_dir();
    /// println!("Setups directory: {}", setups_dir);
    ///
    /// // Use the path to load a setup file
    /// let setup_path = format!("{}/my_setup.toml", setups_dir);
    /// ```
    pub fn get_setups_dir(&self) -> &String {
        &self.data.setups_dir
    }

    /// Get the data directory path.
    ///
    /// This method returns the path to the directory where application
    /// data, logs, and runtime files are stored.
    ///
    /// # Returns
    ///
    /// A reference to the data directory path string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::config::Config;
    ///
    /// let config = Config::load()?;
    ///
    /// // Get the data directory path
    /// let data_dir = config.get_data_dir();
    /// println!("Data directory: {}", data_dir);
    ///
    /// // Use the path to store application data
    /// let log_path = format!("{}/application.log", data_dir);
    /// ```
    pub fn get_data_dir(&self) -> &String {
        &self.data.data_dir
    }

    /// Get all data directory paths.
    ///
    /// This method returns a tuple containing all three data directory paths
    /// for convenient access to all data-related paths at once.
    ///
    /// # Returns
    ///
    /// A tuple containing references to (images_dir, setups_dir, data_dir).
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::config::Config;
    ///
    /// let config = Config::load()?;
    ///
    /// // Get all data directory paths
    /// let (images_dir, setups_dir, data_dir) = config.get_data_dirs();
    /// println!("Images: {}, Setups: {}, Data: {}", images_dir, setups_dir, data_dir);
    ///
    /// // Use the paths for various operations
    /// let image_path = format!("{}/shape.sif", images_dir);
    /// let setup_path = format!("{}/setup.toml", setups_dir);
    /// let log_path = format!("{}/app.log", data_dir);
    /// ```
    pub fn get_data_dirs(&self) -> (&String, &String, &String) {
        (
            &self.data.images_dir,
            &self.data.setups_dir,
            &self.data.data_dir,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CONFIG_PATH: &str = "obake.toml";

    #[test]
    fn test_config_loading() {
        // Test loading from a specific path (for backward compatibility with existing obake.toml)
        if std::path::Path::new(TEST_CONFIG_PATH).exists() {
            let config = Config::load_from_path(TEST_CONFIG_PATH);
            assert!(config.is_ok());

            let config = config.unwrap();
            assert_eq!(config.audio.default_interface, "mixpre");
            assert!(config.audio.interfaces.contains_key("mixpre"));

            // Test that interfaces with unit fields are parsed correctly
            if let Some(mixpre_interface) = config.audio.interfaces.get("mixpre") {
                assert_eq!(mixpre_interface.interface_type, "jack");
                assert!(mixpre_interface.unit.is_some());
                assert_eq!(
                    mixpre_interface.unit.as_ref().unwrap(),
                    "jack@mixpre.service"
                );
            }
        }
    }

    #[test]
    fn test_config_load_search_paths() {
        // Test the new search behavior by creating a temporary config file
        let test_config = r#"
[audio]
default-interface = "test"

[audio.interfaces]
"test" = { type = "test", unit = "test.service" }

[data]
images-dir = "/test/images"
setups-dir = "/test/setups"
data-dir = "/test/data"
"#;

        // Create a temporary file
        let temp_file = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
        let config_path = temp_file.path().to_str().unwrap();

        // Write config content to temporary file
        std::fs::write(config_path, test_config).unwrap();

        // Test loading from the temporary path
        let config = Config::load_from_path(config_path);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.audio.default_interface, "test");
        assert!(config.audio.interfaces.contains_key("test"));

        // Test data configuration
        assert_eq!(config.data.images_dir, "/test/images");
        assert_eq!(config.data.setups_dir, "/test/setups");
        assert_eq!(config.data.data_dir, "/test/data");

        // Test data helper methods
        assert_eq!(config.get_images_dir(), "/test/images");
        assert_eq!(config.get_setups_dir(), "/test/setups");
        assert_eq!(config.get_data_dir(), "/test/data");

        let (images_dir, setups_dir, data_dir) = config.get_data_dirs();
        assert_eq!(images_dir, "/test/images");
        assert_eq!(setups_dir, "/test/setups");
        assert_eq!(data_dir, "/test/data");

        // Clean up is automatic when temp_file goes out of scope
    }

    #[test]
    fn test_optional_unit_field() {
        // Test configuration with optional unit field
        let test_config = r#"
[audio]
default-interface = "alsa"

[audio.interfaces]
"alsa" = { type = "alsa" }
"jack" = { type = "jack", unit = "jack.service" }

[data]
images-dir = "/test/images"
setups-dir = "/test/setups"
data-dir = "/test/data"
"#;

        // Create a temporary file
        let temp_file = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
        let test_file = temp_file.path().to_str().unwrap();

        // Write config content to temporary file
        std::fs::write(test_file, test_config).unwrap();

        // Load and verify
        let config = Config::load_from_path(test_file);
        assert!(config.is_ok());

        let config = config.unwrap();

        // Test interface without unit
        if let Some(alsa_interface) = config.audio.interfaces.get("alsa") {
            assert_eq!(alsa_interface.interface_type, "alsa");
            assert!(alsa_interface.unit.is_none());
        }

        // Test interface with unit
        if let Some(jack_interface) = config.audio.interfaces.get("jack") {
            assert_eq!(jack_interface.interface_type, "jack");
            assert!(jack_interface.unit.is_some());
            assert_eq!(jack_interface.unit.as_ref().unwrap(), "jack.service");
        }

        // Clean up is automatic when temp_file goes out of scope
    }

    #[test]
    fn test_data_configuration() {
        // Test data configuration loading and access
        let test_config = r#"
[audio]
default-interface = "test"

[audio.interfaces]
"test" = { type = "test" }

[data]
images-dir = "/custom/images"
setups-dir = "/custom/setups"
data-dir = "/custom/data"
"#;

        // Create a temporary file
        let temp_file = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
        let config_path = temp_file.path().to_str().unwrap();

        // Write config content to temporary file
        std::fs::write(config_path, test_config).unwrap();

        // Test loading from the temporary path
        let config = Config::load_from_path(config_path);
        assert!(config.is_ok());

        let config = config.unwrap();

        // Test direct access to data fields
        assert_eq!(config.data.images_dir, "/custom/images");
        assert_eq!(config.data.setups_dir, "/custom/setups");
        assert_eq!(config.data.data_dir, "/custom/data");

        // Test individual getter methods
        assert_eq!(config.get_images_dir(), "/custom/images");
        assert_eq!(config.get_setups_dir(), "/custom/setups");
        assert_eq!(config.get_data_dir(), "/custom/data");

        // Test tuple getter method
        let (images_dir, setups_dir, data_dir) = config.get_data_dirs();
        assert_eq!(images_dir, "/custom/images");
        assert_eq!(setups_dir, "/custom/setups");
        assert_eq!(data_dir, "/custom/data");

        // Test that the paths can be used to construct file paths
        let image_path = format!("{}/shape.sif", config.get_images_dir());
        let setup_path = format!("{}/setup.toml", config.get_setups_dir());
        let log_path = format!("{}/app.log", config.get_data_dir());

        assert_eq!(image_path, "/custom/images/shape.sif");
        assert_eq!(setup_path, "/custom/setups/setup.toml");
        assert_eq!(log_path, "/custom/data/app.log");

        // Clean up is automatic when temp_file goes out of scope
    }
}
