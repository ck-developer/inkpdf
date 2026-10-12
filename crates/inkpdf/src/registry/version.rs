//! Template versions and version selectors (research R9 of feature 005).

use std::fmt;
use std::str::FromStr;

/// A template version, `MAJOR.MINOR.PATCH`: three numbers, ordered numerically
/// (`1.10.0` > `1.9.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Text that is not a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected MAJOR.MINOR.PATCH")]
pub struct NotAVersion;

impl FromStr for Version {
    type Err = NotAVersion;

    /// Three dot-separated numbers, digits only, without leading zeros: `1.0.0`, not `v1.0.0`,
    /// `1.2` or `01.0.0`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = text.split('.').map(number);
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(Some(major)), Some(Some(minor)), Some(Some(patch)), None) => Ok(Self {
                major,
                minor,
                patch,
            }),
            _ => Err(NotAVersion),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A version number: digits only, no leading zero, within `u32`.
fn number(text: &str) -> Option<u32> {
    let digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    if !digits || (text.len() > 1 && text.starts_with('0')) {
        return None;
    }
    text.parse().ok()
}

/// The `version` a caller asks for: a major number (`1`, the highest valid `1.x.y`) or an
/// exact version (`1.0.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionSelector {
    Major(u32),
    Exact(Version),
}

impl VersionSelector {
    /// Whether `version` is one this selector designates.
    pub fn matches(self, version: Version) -> bool {
        match self {
            Self::Major(major) => version.major == major,
            Self::Exact(exact) => version == exact,
        }
    }
}

/// Text that is not a version selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected a major version (`2`) or an exact version (`2.0.0`)")]
pub struct NotASelector;

impl FromStr for VersionSelector {
    type Err = NotASelector;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if let Some(major) = number(text) {
            return Ok(Self::Major(major));
        }
        text.parse()
            .map(Self::Exact)
            .map_err(|NotAVersion| NotASelector)
    }
}

impl fmt::Display for VersionSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Major(major) => write!(f, "{major}"),
            Self::Exact(version) => write!(f, "{version}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        text.parse().unwrap()
    }

    #[test]
    fn versions_are_three_plain_numbers() {
        assert_eq!(
            version("1.20.3"),
            Version {
                major: 1,
                minor: 20,
                patch: 3
            }
        );
        assert_eq!(version("0.0.0").to_string(), "0.0.0");
        for text in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "v1.0.0",
            "01.0.0",
            "1.00.0",
            "1.0.0-beta",
            "1.0.0+b",
            " 1.0.0",
            "1..0",
            "-1.0.0",
            "+1.0.0",
            "1.0.x",
            "4294967296.0.0",
        ] {
            assert_eq!(text.parse::<Version>(), Err(NotAVersion), "{text:?}");
        }
    }

    #[test]
    fn versions_are_ordered_numerically() {
        assert!(version("1.10.0") > version("1.9.0"));
        assert!(version("2.0.0") > version("1.99.99"));
        assert!(version("1.0.10") > version("1.0.9"));
    }

    #[test]
    fn selectors_are_a_major_or_an_exact_version() {
        assert_eq!("1".parse(), Ok(VersionSelector::Major(1)));
        assert_eq!("0".parse(), Ok(VersionSelector::Major(0)));
        assert_eq!(
            "1.0.0".parse(),
            Ok(VersionSelector::Exact(version("1.0.0")))
        );
        for text in [
            "", "latest", "v2", "1.2", "01", "../1.0.0", "1.0.0/..", "1.0.0/", " 1", "1 ",
        ] {
            assert_eq!(
                text.parse::<VersionSelector>(),
                Err(NotASelector),
                "{text:?}"
            );
        }
    }

    #[test]
    fn selectors_match_their_versions() {
        let major = VersionSelector::Major(1);
        assert!(major.matches(version("1.0.0")));
        assert!(major.matches(version("1.9.3")));
        assert!(!major.matches(version("2.0.0")));
        let exact = VersionSelector::Exact(version("1.1.0"));
        assert!(exact.matches(version("1.1.0")));
        assert!(!exact.matches(version("1.1.1")));
    }
}
