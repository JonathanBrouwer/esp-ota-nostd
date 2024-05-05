use embedded_storage::nor_flash::NorFlash;
use crate::error::OtaInternalError;
use crate::error::OtaInternalError::{PartitionFoundTwice, PartitionNotFound, NorFlashOpError};
use esp_partition_table::{PartitionEntry, PartitionTable, PartitionType};

/// Find partition entry by type
pub fn find_partition_type<S: NorFlash>(
    storage: &mut S,
    typ: PartitionType,
) -> Result<PartitionEntry, OtaInternalError<S>> {
    let table = PartitionTable::default();
    let mut found_partition = None;

    for entry in table.iter_nor_flash(storage, false) {
        let entry = entry.map_err(NorFlashOpError)?;
        if entry.type_ == typ {
            if found_partition.is_none() {
                found_partition = Some(entry);
            } else {
                return Err(PartitionFoundTwice);
            }
        }
    }

    found_partition.ok_or(PartitionNotFound)
}
