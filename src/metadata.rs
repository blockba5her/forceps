use crate::{ForcepError, Result};
use std::io;
use std::path;
use std::time;

/// Type definition for an array of bytes that make up an `md5` hash.
pub type Md5Bytes = [u8; 16];

/// Metadata information about a certain entry in the cache
///
/// This metadata contains information about when the entry was last modified, the size (in bytes)
/// of the entry, the `md5` integrity of the entry, etc.
///
/// # Examples
///
/// ```rust
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// use forceps::CacheBuilder;
///
/// let cache = CacheBuilder::new("./cache")
///     .build()
///     .await
///     .unwrap();
///
/// cache.write(&b"MY_KEY", &b"Hello World").await.unwrap();
///
/// let metadata = cache.read_metadata(&b"MY_KEY").unwrap();
/// # }
/// ```
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
    /// Size in bytes of the corresponding entry
    size: u64,
    /// Last time this entry was modified, milliseconds since epoch
    last_modified: u64,
    /// Last time since this entry was accessed, milliseconds since epoch
    last_accessed: u64,
    /// Md5 hash of the underlying data
    integrity: Md5Bytes,
}

/// Database for cache entry metadata
#[derive(Debug)]
pub(crate) struct MetaDb {
    db: sled::Db,
}

/// Milliseconds from epoch to now
fn now_since_epoch() -> u64 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .map(|x| x.as_millis() as u64)
        .unwrap_or(0)
}

impl Metadata {
    /// Creates a new instance of [`Metadata`] from the given `data`
    pub(crate) fn new(data: &[u8]) -> Self {
        Self {
            size: data.len() as u64,
            last_modified: now_since_epoch(),
            last_accessed: now_since_epoch(),
            integrity: md5::compute(data).into(),
        }
    }

    /// Serializes the metadata into bytes
    pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
        let document = bson::to_document(self).map_err(ForcepError::MetaSer)?;

        // write document contents to memory stream
        let mut buf = Vec::<u8>::new();
        let mut writer = io::Cursor::new(&mut buf);
        document
            .to_writer(&mut writer)
            .map_err(ForcepError::MetaSer)?;

        Ok(buf)
    }

    /// Deserializes a slice of bytes into metadata
    pub(crate) fn deserialize(buf: &[u8]) -> Result<Self> {
        // create a reader so we can convert the document
        let mut cursor = io::Cursor::new(buf);
        let document = bson::Document::from_reader(&mut cursor).map_err(ForcepError::MetaDe)?;

        bson::from_document(document).map_err(ForcepError::MetaDe)
    }

    /// The size in bytes of the corresponding cache entry.
    #[inline]
    pub fn get_size(&self) -> u64 {
        self.size
    }

    /// Retrives the last time this entry was modified.
    pub fn get_last_modified(&self) -> Option<time::SystemTime> {
        match self.last_modified {
            0 => None,
            millis => Some(time::UNIX_EPOCH + time::Duration::from_millis(millis)),
        }
    }
    /// Retrieves the raw `last_modified` time, which is the milliseconds since
    /// [`time::UNIX_EPOCH`]. If the returned result is `0`, that means there is no `last_modified`
    /// time.
    #[inline]
    pub fn get_last_modified_raw(&self) -> u64 {
        self.last_modified
    }

    /// Retrieves the internal [`Md5Bytes`] integrity of the corresponding metadata entry.
    #[inline]
    pub fn get_integrity(&self) -> &Md5Bytes {
        &self.integrity
    }

    /// Verifies that the metadata integrity matches the integrity of the data provided.
    #[inline]
    pub fn check_integrity_of(&self, data: &[u8]) -> bool {
        let other_integrity: Md5Bytes = md5::compute(data).into();
        &other_integrity == &self.integrity
    }
}

impl MetaDb {
    /// Initializes a new metadata database with sled.
    pub fn new(path: &path::Path) -> Result<Self> {
        sled::open(path)
            .map_err(ForcepError::MetaDb)
            .map(|db| Self { db })
    }

    /// Retrieves an entry in the metadata database with the corresponding key.
    pub fn get_metadata(&self, key: &[u8]) -> Result<Metadata> {
        let data = match self.db.get(key) {
            Ok(Some(data)) => data,
            Ok(None) => Err(ForcepError::NotFound)?,
            Err(e) => Err(ForcepError::MetaDb(e))?,
        };
        Metadata::deserialize(&data)
    }

    /// Inserts a new entry into the metadata database for the associated key and data.
    ///
    /// If a previous entry exists, it is simply overwritten.
    pub fn insert_metadata_for(&self, key: &[u8], data: &[u8]) -> Result<Metadata> {
        let meta = Metadata::new(data);
        let bytes = Metadata::serialize(&meta)?;
        self.db
            .insert(key, &bytes[..])
            .map_err(ForcepError::MetaDb)?;
        Ok(meta)
    }

    /// Iterator over the entire metadata database
    pub fn metadata_iter(&self) -> impl Iterator<Item = Result<(Vec<u8>, Metadata)>> {
        self.db.iter().map(|x| match x {
            Ok((key, data)) => Metadata::deserialize(&data[..]).map(|m| (key.to_vec(), m)),
            Err(e) => Err(ForcepError::MetaDb(e)),
        })
    }
}