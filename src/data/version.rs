// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaVersion {
    major: u8,
    minor: u8,
    migration: u16,
}

impl SchemaVersion {
    /// Creates a new schema version with the given major, minor, and migration numbers.
    #[must_use]
    pub const fn new(major: u8, minor: u8, migration: u16) -> Self {
        Self {
            major,
            minor,
            migration,
        }
    }

    #[must_use]
    fn from_i32(raw: i32) -> Self {
        let raw = raw.cast_unsigned(); // Convert to u32 to avoid sign issues
        let major = ((raw >> 24) & 0xFF) as u8;
        let minor = ((raw >> 16) & 0xFF) as u8;
        let migration = (raw & 0xFFFF) as u16;
        Self {
            major,
            minor,
            migration,
        }
    }

    #[must_use]
    fn as_i32(self) -> i32 {
        let raw = ((u32::from(self.major)) << 24)
            | ((u32::from(self.minor)) << 16)
            | (u32::from(self.migration));
        raw.cast_signed()
    }

    /// Returns the major version number.
    #[must_use]
    pub fn major(&self) -> u8 {
        self.major
    }

    /// Returns the minor version number.
    #[must_use]
    pub fn minor(&self) -> u8 {
        self.minor
    }

    /// Returns the migration version number.
    #[must_use]
    pub fn migration(&self) -> u16 {
        self.migration
    }
}

impl FromStr for SchemaVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version string: {s}"));
        }

        let major = parts[0]
            .parse::<u8>()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u8>()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let migration = parts[2]
            .parse::<u16>()
            .map_err(|_| format!("Invalid migration version: {}", parts[2]))?;

        Ok(SchemaVersion::new(major, minor, migration))
    }
}

impl From<i32> for SchemaVersion {
    fn from(raw: i32) -> Self {
        Self::from_i32(raw)
    }
}

impl From<SchemaVersion> for i32 {
    fn from(version: SchemaVersion) -> Self {
        version.as_i32()
    }
}

impl Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.migration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_conversion() {
        let version = SchemaVersion::new(1, 2, 3);
        let raw = version.as_i32();
        let converted_version = SchemaVersion::from_i32(raw);
        assert_eq!(version, converted_version);

        let maximum = SchemaVersion::new(u8::MAX, u8::MAX, u16::MAX);
        assert_eq!(SchemaVersion::from_i32(maximum.as_i32()), maximum);
    }

    #[test]
    fn test_signed_encodings_use_the_same_bytes() {
        let version = SchemaVersion::new(0x80, 0x12, 0x3456);
        let expected = [0x80, 0x12, 0x34, 0x56];

        assert_eq!(version.as_i32().to_be_bytes(), expected);
    }

    #[test]
    fn test_version_ordering() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 0, 1);
        let v3 = SchemaVersion::new(1, 1, 0);
        let v4 = SchemaVersion::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
    }

    #[test]
    fn test_version_equality() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 0, 0);
        let v3 = SchemaVersion::new(1, 0, 1);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_string_conversions() {
        let version = SchemaVersion::new(1, 2, 345);

        assert_eq!(version.to_string(), "1.2.345");
        assert_eq!("1.2.345".parse(), Ok(version));
    }

    #[test]
    fn test_invalid_version_strings() {
        for input in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "major.2.3",
            "1.minor.3",
            "1.2.migration",
            "256.2.3",
            "1.256.3",
            "1.2.65536",
        ] {
            assert!(
                input.parse::<SchemaVersion>().is_err(),
                "{input} should be rejected"
            );
        }
    }
}
