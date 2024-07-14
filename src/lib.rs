#![no_std]

mod crc;
mod error;
mod ota_data;
mod ota_data_structs;
pub mod partitions;

use crate::error::{OtaInternalError, OtaUpdateError};
use crate::ota_data::{read_ota_data, write_ota_data};
use crate::ota_data_structs::{EspOTAData, EspOTAState};
use core::sync::atomic::Ordering;
use embedded_io_async::Read;
use embedded_storage::nor_flash::NorFlash;
use esp_partition_table::{AppPartitionType, NorFlashOpError, PartitionEntry, PartitionType};
use portable_atomic::AtomicBool;
use crate::partitions::find_partition_by_type;

/// Size of a flash sector
const SECTOR_SIZE: usize = 0x1000;

static IS_UPDATING: AtomicBool = AtomicBool::new(false);

/// Starts a new OTA update.
/// - The `binary` is the data that should be written to the ota partition.
/// - This function returns an error if multiple ota updates are attempted concurrently.
/// - If the update was successful, the caller should reboot to activate the new firmware.
/// - The `progress_fn` is called periodically with the total amount of bytes written so far.
pub async fn ota_begin<S: NorFlash, R: Read>(
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
    let booted_seq = ota_data.seq;
    let new_seq = ota_data.seq + 1;
    let new_part = ((new_seq - 1) % 2) as u8;
    let ota_app =
        find_partition_by_type(storage, PartitionType::App(AppPartitionType::Ota(new_part)))?;
    log::info!("Starting OTA update. Current sequence is {booted_seq}, updating to sequence {new_seq} (partition {}).", ota_app.name());

    // Erase partition
    storage
        .erase(ota_app.offset, ota_app.offset + ota_app.size as u32)
        .map_err(|e| OtaInternalError::NorFlashOpError(NorFlashOpError::StorageError(e)))?;

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
                .map_err(|e| OtaUpdateError::ReadError(e))?;
            if read == 0 {
                is_done = true;
                break;
            }
            read_len += read;
        }

        if data_written + read_len > ota_app.size {
            return Err(OtaUpdateError::OutOfSpace);
        }

        storage
            .write(
                ota_app.offset + data_written as u32,
                &data_buffer[0..read_len],
            )
            .map_err(|e| OtaInternalError::NorFlashOpError(NorFlashOpError::StorageError(e)))?;

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

/// Mark OTA update as valid.
/// Must be called after an OTA update and reboot to confirm the new firmware works.
/// May also be called after a reboot without OTA update.
/// If the system reboots before an OTA update is accepted
/// the update will be marked as aborted and will not be booted again.
pub fn ota_accept<S: NorFlash>(storage: &mut S) -> Result<(), OtaInternalError<S>> {
    let mut ota_data = read_ota_data(storage)?;
    match ota_data.state {
        EspOTAState::PendingVerify => {
            log::info!("Accepted pending OTA update");
            ota_data.state = EspOTAState::Valid;
            write_ota_data(storage, ota_data)?;
        },
        EspOTAState::New | EspOTAState::Undefined => {
            log::warn!("Accepted OTA update from {:?} state", ota_data.state);
            ota_data.state = EspOTAState::Valid;
            write_ota_data(storage, ota_data)?;
        },
        EspOTAState::Invalid | EspOTAState::Aborted => {
            log::warn!("Detected rollback that was not processed by bootloader, rolling back manually.");
            ota_data.state = EspOTAState::Valid;
            ota_data.seq -= 1;
            write_ota_data(storage, ota_data)?;
        }
        EspOTAState::Valid => {},
    }
    Ok(())
}

/// Explicitly mark an OTA update as invalid.
/// May be called after an OTA update failed, but is not required.
/// If the system reboots before an OTA update is confirmed as valid
/// the update will be marked as aborted and will not be booted again.
pub fn ota_reject<S: NorFlash>(storage: &mut S) -> Result<(), OtaInternalError<S>> {
    let mut ota_data = read_ota_data(storage)?;
    match ota_data.state {
        EspOTAState::PendingVerify => {
            log::info!("Rejected pending OTA update");
            ota_data.state = EspOTAState::Invalid;
            write_ota_data(storage, ota_data)?;
        }
        EspOTAState::New | EspOTAState::Undefined => {
            log::warn!("Rejected OTA update from {:?} state", ota_data.state);
            ota_data.state = EspOTAState::Invalid;
            write_ota_data(storage, ota_data)?;
        }
        EspOTAState::Valid => {
            log::error!("Tried to reject OTA update that has already been accepted, ignoring request.");
        }
        EspOTAState::Invalid => {
            log::warn!("Tried to reject OTA update that has already been rejected, ignoring request.");
        }
        EspOTAState::Aborted => {
            log::warn!("Tried to reject OTA update from aborted state, ignoring request.");
        }
    }
    Ok(())
}

// /// This function rolls back the app if the previous boot did not 
// pub fn ota_rollback<S: NorFlash>(storage: &mut S) -> Result<(), OtaInternalError<S>> {
//     
// }

/// Returns true if this OTA update has been accepted, i.e. with `ota_accept`
pub fn ota_is_valid<S: NorFlash>(storage: &mut S) -> Result<bool, OtaInternalError<S>> {
    Ok(read_ota_data(storage)?.is_valid())
}

/// Find the ota partition we're currently running on
pub fn get_booted_partition<S: NorFlash>(storage: &mut S) -> Result<PartitionEntry, OtaInternalError<S>> {
    let ota_data = read_ota_data(storage)?;
    let booted_seq = ota_data.seq;
    let new_part = ((booted_seq - 1) % 2) as u8;
    find_partition_by_type(storage, PartitionType::App(AppPartitionType::Ota(new_part)))
}