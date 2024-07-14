use embedded_storage::nor_flash::NorFlash;
use esp_partition_table::NorFlashOpError;

/// Errors that may occur during an OTA update
#[derive(Debug)]
pub enum OtaUpdateError<S: NorFlash, R> {
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

impl<S: NorFlash, R> From<OtaInternalError<S>> for OtaUpdateError<S, R> {
    fn from(value: OtaInternalError<S>) -> Self {
        OtaUpdateError::InternalError(value)
    }
}

#[derive(Debug)]
pub enum OtaInternalError<S: NorFlash> {
    OtaDataCorrupt,
    NorFlashOpError(NorFlashOpError<S>),
    PartitionNotFound,
    PartitionFoundTwice,
}

impl<S: NorFlash> From<NorFlashOpError<S>> for OtaInternalError<S> {
    fn from(value: NorFlashOpError<S>) -> Self {
        OtaInternalError::NorFlashOpError(value)
    }
}
