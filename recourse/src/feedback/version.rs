//! Semver version helpers for the feedback module.
//!
//! The current shipped version is read from `<data_dir>/feedback/version.toml`.
//! If absent, the baseline is 0.0.0.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn zero() -> Self {
        Version {
            major: 0,
            minor: 0,
            patch: 0,
        }
    }

    /// Parse "major.minor.patch".
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("invalid semver '{}'; expected major.minor.patch", s));
        }
        let major = parts[0]
            .parse::<u64>()
            .map_err(|e| format!("invalid major '{}': {e}", parts[0]))?;
        let minor = parts[1]
            .parse::<u64>()
            .map_err(|e| format!("invalid minor '{}': {e}", parts[1]))?;
        let patch = parts[2]
            .parse::<u64>()
            .map_err(|e| format!("invalid patch '{}': {e}", parts[2]))?;
        Ok(Version {
            major,
            minor,
            patch,
        })
    }

    pub fn to_string_ver(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Return the next minor bump (patch → 0).
    pub fn next_minor(&self) -> Self {
        Version {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    /// Returns true iff self is strictly greater than other.
    pub fn is_greater_than(&self, other: &Version) -> bool {
        (self.major, self.minor, self.patch) > (other.major, other.minor, other.patch)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionFile {
    version: String,
}

/// Read the current shipped version from `<data_dir>/feedback/version.toml`.
/// Returns Version::zero() if the file does not exist.
pub fn read_current(data_dir: &Path) -> Result<Version, Box<dyn std::error::Error>> {
    let path = data_dir.join("feedback").join("version.toml");
    if !path.exists() {
        return Ok(Version::zero());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let vf: VersionFile = toml::from_str(&content)
        .map_err(|e| format!("cannot parse {}: {e}", path.display()))?;
    Version::parse(&vf.version)
        .map_err(|e| format!("invalid version in {}: {e}", path.display()))
        .map_err(Box::<dyn std::error::Error>::from)
}

/// Write the current shipped version to `<data_dir>/feedback/version.toml`.
pub fn write_current(data_dir: &Path, v: &Version) -> Result<(), Box<dyn std::error::Error>> {
    let dir = data_dir.join("feedback");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("version.toml");
    let vf = VersionFile {
        version: v.to_string_ver(),
    };
    let content = toml::to_string_pretty(&vf)?;
    std::fs::write(&path, content)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.to_string_ver(), "1.2.3");
    }

    #[test]
    fn next_minor_bumps_correctly() {
        let v = Version::parse("1.2.3").unwrap();
        let next = v.next_minor();
        assert_eq!(next.to_string_ver(), "1.3.0");
    }

    #[test]
    fn is_greater_than() {
        let a = Version::parse("1.3.0").unwrap();
        let b = Version::parse("1.2.3").unwrap();
        assert!(a.is_greater_than(&b));
        assert!(!b.is_greater_than(&a));
    }

    #[test]
    fn zero_baseline() {
        let z = Version::zero();
        let v = Version::parse("0.1.0").unwrap();
        assert!(v.is_greater_than(&z));
    }

    #[test]
    fn parse_invalid() {
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("1.2.x").is_err());
    }
}
