use tokio_rusqlite::ToSql;
use tokio_rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef};

pub mod prelude {
    pub use super::SqlU64;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlU64(pub u64);

impl ToSql for SqlU64 {
    fn to_sql(&self) -> tokio_rusqlite::rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Blob(
            self.0.to_be_bytes().to_vec(),
        )))
    }
}

impl FromSql for SqlU64 {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let blob = value.as_blob()?;
        let bytes = blob.try_into().map_err(|_| FromSqlError::InvalidBlobSize {
            expected_size: size_of::<u64>(),
            blob_size: blob.len(),
        })?;
        Ok(Self(u64::from_be_bytes(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_u64_to_sql_output() {
        assert_eq!(
            SqlU64(42).to_sql().unwrap(),
            ToSqlOutput::Owned(Value::Blob(vec![0, 0, 0, 0, 0, 0, 0, 42]))
        );
    }

    #[test]
    fn sql_u64_from_sql_output() {
        assert_eq!(
            SqlU64::column_result(ValueRef::Blob(&[0, 0, 0, 0, 0, 0, 0, 42])).unwrap(),
            SqlU64(42)
        );
    }

    #[test]
    fn sql_u64_max_to_sql_output() {
        assert_eq!(
            SqlU64(u64::MAX).to_sql().unwrap(),
            ToSqlOutput::Owned(Value::Blob(vec![0xff; 8]))
        );
    }

    #[test]
    fn sql_u64_max_from_sql_output() {
        assert_eq!(
            SqlU64::column_result(ValueRef::Blob(&[0xff; 8])).unwrap(),
            SqlU64(u64::MAX)
        );
    }

    #[test]
    fn sql_u64_rejects_wrong_blob_size() {
        assert!(matches!(
            SqlU64::column_result(ValueRef::Blob(&[0; 7])),
            Err(FromSqlError::InvalidBlobSize {
                expected_size: 8,
                blob_size: 7
            })
        ));
    }

    #[test]
    fn sql_u64_accepts_correct_blob_size() {
        assert!(SqlU64::column_result(ValueRef::Blob(&[0; 8])).is_ok());
    }
}
