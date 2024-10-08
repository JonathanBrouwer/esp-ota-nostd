use crate::error::OtaInternalError;
use crate::ota_data_structs::EspOTAData;
use crate::partitions::find_partition_by_type;
use crate::SECTOR_SIZE;
use embedded_storage::nor_flash::NorFlash;
use esp_partition_table::{DataPartitionType, NorFlashOpError, PartitionType};

/// Read from ota data partition
pub fn read_ota_data<S: NorFlash>(storage: &mut S) -> Result<EspOTAData, OtaInternalError<S>> {
    let ota_data_part =
        find_partition_by_type(storage, PartitionType::Data(DataPartitionType::Ota))?;
    let mut buffer = [0; 32];

    storage
        .read(ota_data_part.offset, &mut buffer)
        .map_err(|e| NorFlashOpError::StorageError(e))?;
    if let Ok(data) = EspOTAData::try_from(buffer) {
        return Ok(data);
    }

    storage
        .read(ota_data_part.offset + SECTOR_SIZE as u32, &mut buffer)
        .map_err(|e| NorFlashOpError::StorageError(e))?;
    EspOTAData::try_from(buffer).map_err(|_| OtaInternalError::OtaDataCorrupt)
}

/// Write to ota data partition
pub fn write_ota_data<S: NorFlash>(
    storage: &mut S,
    data: EspOTAData,
) -> Result<(), OtaInternalError<S>> {
    let ota_data_part =
        find_partition_by_type(storage, PartitionType::Data(DataPartitionType::Ota))?;
    let buffer: [u8; 32] = data.into();

    // Write sector A
    storage
        .erase(ota_data_part.offset, ota_data_part.offset + SECTOR_SIZE as u32)
        .map_err(|e| NorFlashOpError::StorageError(e))?;
    storage
        .write(ota_data_part.offset, &buffer)
        .map_err(|e| NorFlashOpError::StorageError(e))?;

    // Write sector B
    storage
        .erase(
            ota_data_part.offset + SECTOR_SIZE as u32,
            ota_data_part.offset + 2 * SECTOR_SIZE as u32,
        )
        .map_err(|e| NorFlashOpError::StorageError(e))?;
    storage
        .write(ota_data_part.offset + SECTOR_SIZE as u32, &buffer)
        .map_err(|e| NorFlashOpError::StorageError(e))?;

    Ok(())
}
