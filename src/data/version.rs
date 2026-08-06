use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub(super) release: u8,
    pub(super) schema: u8,
    pub(super) migration: u16,
}

impl Version {
    #[must_use]
    pub fn new(release: u8, schema: u8, migration: u16) -> Self {
        Self { release, schema, migration }
    }

    #[must_use]
    pub fn from_i32(raw: i32) -> Self {
        let raw = raw.cast_unsigned(); // Convert to u32 to avoid sign issues
        let release = ((raw >> 24) & 0xFF) as u8;
        let schema = ((raw >> 16) & 0xFF) as u8;
        let migration = (raw & 0xFFFF) as u16;
        Self { release, schema, migration }
    }

    #[must_use]
    pub fn as_i32(&self) -> i32 {
        let raw = ((u32::from(self.release)) << 24)
            | ((u32::from(self.schema)) << 16)
            | (u32::from(self.migration));
        raw.cast_signed()
    }

    #[must_use]
    pub fn as_u32(&self) -> u32 {
        ((u32::from(self.release)) << 24)
            | ((u32::from(self.schema)) << 16)
            | (u32::from(self.migration))
    }

    #[must_use]
    pub fn release(&self) -> u8 {
        self.release
    }

    #[must_use]
    pub fn schema(&self) -> u8 {
        self.schema
    }

    #[must_use]
    pub fn migration(&self) -> u16 {
        self.migration
    }
}

impl From<i32> for Version {
    fn from(raw: i32) -> Self {
        Self::from_i32(raw)
    }
}

impl From<Version> for i32 {
    fn from(version: Version) -> Self {
        version.as_i32()
    }
}

impl From<Version> for u32 {
    fn from(version: Version) -> Self {
        version.as_u32()
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.release, self.schema, self.migration)
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
        let version = Version::new(1, 2, 3);
        let raw = version.as_i32();
        let converted_version = Version::from_i32(raw);
        assert_eq!(version, converted_version);
    }

    #[test]
    fn test_version_ordering() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 0, 1);
        let v3 = Version::new(1, 1, 0);
        let v4 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
    }

    #[test]
    fn test_version_equality() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 0, 0);
        let v3 = Version::new(1, 0, 1);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    // Compile-time check for trait implementations
    fn test_version_traits() {
        check::<Version>();
    }
}
