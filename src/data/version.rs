use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaVersion {
    major: u8,
    minor: u8,
    migration: u16,
}

impl SchemaVersion {
    #[must_use]
    pub const fn new(major: u8, minor: u8, migration: u16) -> Self {
        Self {
            major,
            minor,
            migration,
        }
    }

    #[must_use]
    pub fn from_i32(raw: i32) -> Self {
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
    pub fn as_i32(&self) -> i32 {
        let raw = ((u32::from(self.major)) << 24)
            | ((u32::from(self.minor)) << 16)
            | (u32::from(self.migration));
        raw.cast_signed()
    }

    #[must_use]
    pub fn as_u32(&self) -> u32 {
        ((u32::from(self.major)) << 24)
            | ((u32::from(self.minor)) << 16)
            | (u32::from(self.migration))
    }

    #[must_use]
    pub fn major(&self) -> u8 {
        self.major
    }

    #[must_use]
    pub fn minor(&self) -> u8 {
        self.minor
    }

    #[must_use]
    pub fn migration(&self) -> u16 {
        self.migration
    }

    #[must_use]
    pub fn triple(&self) -> (u8, u8, u16) {
        (self.major, self.minor, self.migration)
    }
}

impl From<SchemaVersion> for String {
    fn from(version: SchemaVersion) -> Self {
        format!("{}.{}.{}", version.major, version.minor, version.migration)
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

impl From<(u8, u8, u16)> for SchemaVersion {
    fn from(triple: (u8, u8, u16)) -> Self {
        Self::new(triple.0, triple.1, triple.2)
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

impl From<SchemaVersion> for u32 {
    fn from(version: SchemaVersion) -> Self {
        version.as_u32()
    }
}

impl Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.migration)
    }
}

fn check<T>()
where
    T: std::fmt::Debug
        + std::fmt::Display
        + std::cmp::PartialEq
        + std::cmp::PartialOrd
        + std::clone::Clone
        + std::marker::Copy
        + From<i32>
        + Into<i32>
        + Into<u32>,
{
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
    fn test_signed_and_unsigned_encodings_use_the_same_bytes() {
        let version = SchemaVersion::new(0x80, 0x12, 0x3456);
        let expected = [0x80, 0x12, 0x34, 0x56];

        assert_eq!(version.as_i32().to_be_bytes(), expected);
        assert_eq!(version.as_u32().to_be_bytes(), expected);
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
    fn test_string_and_tuple_conversions() {
        let version = SchemaVersion::new(1, 2, 345);

        assert_eq!(version.to_string(), "1.2.345");
        assert_eq!(String::from(version), "1.2.345");
        assert_eq!("1.2.345".parse(), Ok(version));
        assert_eq!(SchemaVersion::from((1, 2, 345)), version);
        assert_eq!(version.triple(), (1, 2, 345));
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

    #[test]
    fn test_version_traits() {
        check::<SchemaVersion>();
    }
}
