use std::path::{Path, PathBuf};

use oxc_diagnostics::OxcDiagnostic;

/// A supported configuration file discovered on disk.
///
/// The variant identifies which config source matched, while the contained path
/// points to the concrete file. Consumers can use [`DiscoveredConfigFile::path`]
/// when they only need the filesystem location.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum DiscoveredConfigFile {
    Json(PathBuf),
    Jsonc(PathBuf),
    Js(PathBuf),
    Vite(PathBuf),
}

impl DiscoveredConfigFile {
    pub fn path(&self) -> &Path {
        match self {
            Self::Json(path) | Self::Jsonc(path) | Self::Js(path) | Self::Vite(path) => path,
        }
    }
}

/// Multiple supported config files were found in the same directory.
///
/// Consumers should surface this as a user-facing configuration error. Use the
/// [`From<ConfigConflict>`] implementation to convert it into an [`OxcDiagnostic`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigConflict {
    /// Directory containing the conflicting config files.
    dir: PathBuf,
    /// Config files discovered in `dir`.
    configs: Vec<DiscoveredConfigFile>,
}

impl ConfigConflict {
    pub fn new(dir: PathBuf, configs: Vec<DiscoveredConfigFile>) -> Self {
        debug_assert!(
            configs.len() > 1,
            "ConfigConflict should only be created when multiple configs are found"
        );
        Self { dir, configs }
    }
}

impl ConfigConflict {
    fn message(&self) -> String {
        let config_names = self.config_names();

        if config_names.is_empty() {
            return String::new();
        }

        let config_list = format_conflicting_config_names(&config_names);
        if config_names.len() == 2 {
            format!("Both {config_list} found in {}.", self.dir.display())
        } else {
            format!("Multiple config files found in {}: {config_list}.", self.dir.display())
        }
    }

    fn config_names(&self) -> Vec<String> {
        self.configs
            .iter()
            .filter_map(|config| {
                config.path().file_name().map(|name| name.to_string_lossy().into_owned())
            })
            .collect()
    }
}

impl From<ConfigConflict> for OxcDiagnostic {
    fn from(conflict: ConfigConflict) -> Self {
        OxcDiagnostic::error(conflict.message())
            .with_note("Only one of `.oxlintrc.json`, `.oxlintrc.jsonc`, or `oxlint.config.ts` is allowed per directory.")
            .with_help("Delete one of the configuration files.")
    }
}

fn format_conflicting_config_names(config_names: &[String]) -> String {
    debug_assert!(config_names.len() > 1);

    let mut quoted_names = config_names.iter().map(|name| format!("'{name}'")).collect::<Vec<_>>();
    if quoted_names.len() == 2 {
        return format!("{} and {}", quoted_names[0], quoted_names[1]);
    }

    let last = quoted_names.pop().unwrap();
    format!("{}, and {last}", quoted_names.join(", "))
}
