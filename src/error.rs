use embedded_storage::ReadStorage;
use esp_partition_table::StorageOpError;

/// Errors that may occur during an OTA update
pub enum OtaUpdateError<S: ReadStorage, R> {
    /// The image that was booted hasn't been verified as working yet,
    /// so it may not start an update before being verified.
    /// See `ota_accept`
    PendingVerify,
    /// Not enough space in partition
    OutOfSpace,
    /// Another update is already in progress
    AlreadyUpdating,
    /// Read error
    ReadError(R),
    /// Internal error while working with the ota partitions
    InternalError(OtaInternalError<S>),
}

impl<S: ReadStorage, R> From<OtaInternalError<S>> for OtaUpdateError<S, R> {
    fn from(value: OtaInternalError<S>) -> Self {
        OtaUpdateError::InternalError(value)
    }
}

pub enum OtaInternalError<S: ReadStorage> {
    OtaDataCorrupt,
    StorageOpError(StorageOpError<S>),
    PartitionNotFound,
    PartitionFoundTwice,
}

impl<S: ReadStorage> From<StorageOpError<S>> for OtaInternalError<S> {
    fn from(value: StorageOpError<S>) -> Self {
        OtaInternalError::StorageOpError(value)
    }
}