#![no_std]

mod crc;
mod error;
mod ota_data;
mod ota_data_structs;
mod partitions;

use crate::error::{OtaInternalError, OtaUpdateError};
use crate::ota_data::{read_ota_data, write_ota_data};
use crate::partitions::find_partition_type;
use core::sync::atomic::Ordering;
use embedded_io_async::Read;
use embedded_storage::Storage;
use esp_partition_table::{AppPartitionType, PartitionType, StorageOpError};
use portable_atomic::AtomicBool;
use crate::ota_data_structs::EspOTAData;

/// Size of a flash sector
const SECTOR_SIZE: usize = 0x1000;

static IS_UPDATING: AtomicBool = AtomicBool::new(false);

/// Starts a new OTA update.
/// - The `binary` is the data that should be written to the ota partition.
/// - This function returns an error if multiple ota updates are attempted concurrently.
/// - If the update was successful, the caller should reboot to activate the new firmware.
/// - The `progress_fn` is called periodically with the total amount of bytes written so far.
pub async fn ota_begin<S: Storage, R: Read>(
    storage: &mut S,
    mut binary: R,
    mut progress_fn: impl FnMut(usize),
) -> Result<(), OtaUpdateError<S, R::Error>> {
    // Check if there is already an update happening
    if IS_UPDATING.swap(true, Ordering::SeqCst) {
        return Err(OtaUpdateError::AlreadyUpdating);
    }

    // Check if we're in a valid state
    let ota_data = read_ota_data(storage)?;
    if !ota_data.is_valid() {
        return Err(OtaUpdateError::PendingVerify);
    }

    // Find partition to write to
    let booted_seq = ota_data.seq - 1;
    let new_seq = ota_data.seq + 1;
    let new_part = ((new_seq - 1) % 2) as u8;
    log::info!("Starting OTA update. Current sequence is {booted_seq}, updating to sequence {new_seq} (partition {new_part}).");
    let ota_app = find_partition_type(
        storage,
        PartitionType::App(AppPartitionType::Ota(new_part)),
    )?;

    // Write ota data to flash
    let mut data_written = 0;
    loop {
        let mut data_buffer = [0; SECTOR_SIZE];
        let mut read_len = 0;

        let mut is_done = false;
        while read_len < SECTOR_SIZE {
            let read = binary
                .read(&mut data_buffer[read_len..])
                .await
                .map_err(|e| {
                    OtaUpdateError::ReadError(e)
                })?;
            if read == 0 {
                is_done = true;
                break;
            }
            read_len += read;
        }

        if data_written + read_len > ota_app.size {
            return Err(OtaUpdateError::OutOfSpace);
        }

        storage.write(
            ota_app.offset + data_written as u32,
            &data_buffer[0..read_len],
        ).map_err(|e| OtaInternalError::StorageOpError(StorageOpError::StorageError(e)))?;

        data_written += read_len;
        progress_fn(data_written);

        if is_done {
            break;
        }
    }

    // Write new OTA data boot entry
    let data = EspOTAData::new(new_seq, [0xFF; 20]);
    write_ota_data(storage, data)?;

    Ok(())
}
