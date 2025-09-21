use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration structure for the Obake launcher application.
///
/// This structure represents the complete configuration loaded from the TOML file,
/// containing all application settings including audio interface configurations.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Audio configuration section containing interface settings
    pub audio: AudioConfig,
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

impl Config {
    /// Load configuration from the default `obake.toml` file in the current directory.
    ///
    /// This method looks for a file named `obake.toml` in the current working directory
    /// and attempts to parse it as a TOML configuration file. The configuration is
    /// validated against the expected structure and returned as a `Config` instance.
    ///
    /// # Returns
    ///
    /// - `Ok(Config)`: Successfully loaded and parsed configuration
    /// - `Err(Box<dyn std::error::Error>)`: Error loading or parsing the configuration file
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The `obake.toml` file doesn't exist
    /// - The file cannot be read
    /// - The TOML content is invalid
    /// - The configuration structure doesn't match the expected format
    ///
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config: Config = Figment::new().merge(Toml::file("obake.toml")).extract()?;

        Ok(config)
    }

    /// Load configuration from a custom file path.
    ///
    /// This method allows you to specify a custom path to a TOML configuration file
    /// instead of using the default `obake.toml` file. This is useful for testing,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CONFIG_PATH: &str = "obake.toml";

    #[test]
    fn test_config_loading() {
        // This test would require the obake.toml file to be present
        // In a real test environment, you might want to create a test config file
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
    fn test_optional_unit_field() {
        // Test configuration with optional unit field
        let test_config = r#"
[audio]
default-interface = "alsa"

[audio.interfaces]
"alsa" = { type = "alsa" }
"jack" = { type = "jack", unit = "jack.service" }
"#;

        // Write test config to temporary file
        use std::fs;
        let test_file = "test_config_optional_unit.toml";
        fs::write(test_file, test_config).unwrap();

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

        // Clean up test file
        fs::remove_file(test_file).unwrap();
    }
}
