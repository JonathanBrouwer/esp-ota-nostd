use crate::error::OtaInternalError;
use crate::ota_data_structs::EspOTAData;
use crate::partitions::find_partition_type;
use crate::SECTOR_SIZE;
use embedded_storage::Storage;
use esp_partition_table::{DataPartitionType, PartitionType, StorageOpError};

/// Read from ota data partition
pub fn read_ota_data<S: Storage>(storage: &mut S) -> Result<EspOTAData, OtaInternalError<S>> {
    let ota_data_part = find_partition_type(storage, PartitionType::Data(DataPartitionType::Ota))?;
    let mut buffer = [0; 32];

    storage
        .read(ota_data_part.offset, &mut buffer)
        .map_err(|e| StorageOpError::StorageError(e))?;
    if let Ok(data) = EspOTAData::try_from(buffer) {
        return Ok(data);
    }

    storage
        .read(ota_data_part.offset + SECTOR_SIZE as u32, &mut buffer)
        .map_err(|e| StorageOpError::StorageError(e))?;
    EspOTAData::try_from(buffer).map_err(|_| OtaInternalError::OtaDataCorrupt)
}

/// Write to ota data partition
pub fn write_ota_data<S: Storage>(storage: &mut S, data: EspOTAData) -> Result<(), OtaInternalError<S>> {
    let ota_data_part = find_partition_type(storage, PartitionType::Data(DataPartitionType::Ota))?;
    let buffer: [u8; 32] = data.into();

    storage.write(ota_data_part.offset, &buffer).map_err(|e| StorageOpError::StorageError(e))?;
    storage.write(ota_data_part.offset + SECTOR_SIZE as u32, &buffer).map_err(|e| StorageOpError::StorageError(e))?;

    Ok(())
}
