use crate::crc::esp_crc32;
use core::fmt::{Display, Formatter};

/// Copied from esp-idf
/// -`New`: Monitor the first boot. In bootloader this state is changed to PendingVerify.
/// -`PendingVerify`: First boot for this app was. If while the second boot this state is then it will be changed to Aborted.
/// -`Valid`: App was confirmed as workable. App can boot and work without limits.
/// -`Invalid`: App was confirmed as non-workable. This app will not be selected to boot at all.
/// -`Aborted`: App could not confirm the workable or non-workable. In bootloader IMG_PENDING_VERIFY state will be changed to IMG_ABORTED. This app will not be selected to boot at all.
/// -`Undefined`: Undefined. App can boot and work without limits.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EspOTAState {
    New,
    PendingVerify,
    Valid,
    Invalid,
    Aborted,
    Undefined,
}

/// Weak form of conversion, will return an error if unknown
impl TryFrom<u32> for EspOTAState {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::New),
            1 => Ok(Self::PendingVerify),
            2 => Ok(Self::Valid),
            3 => Ok(Self::Invalid),
            4 => Ok(Self::Aborted),
            u32::MAX => Ok(Self::Undefined),
            _ => Err(()),
        }
    }
}

impl From<EspOTAState> for u32 {
    fn from(value: EspOTAState) -> Self {
        match value {
            EspOTAState::New => 0,
            EspOTAState::PendingVerify => 1,
            EspOTAState::Valid => 2,
            EspOTAState::Invalid => 3,
            EspOTAState::Aborted => 4,
            EspOTAState::Undefined => u32::MAX,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EspOTAData {
    pub(crate) seq: u32,
    pub(crate) label: [u8; 20],
    pub(crate) state: EspOTAState,
    pub(crate) crc: u32,
}

impl EspOTAData {
    pub(crate) fn new(seq: u32, label: [u8; 20]) -> Self {
        let state = EspOTAState::New;
        let crc = esp_crc32(&seq.to_le_bytes());
        Self {
            seq,
            label,
            state,
            crc,
        }
    }

    /// Returns true if this OTA update has been accepted, i.e. with `ota_accept`
    pub fn is_valid(&self) -> bool {
        self.state == EspOTAState::Valid || self.state == EspOTAState::Undefined
    }
}

impl Display for EspOTAData {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "EspOTAData {{ seq: {}, label: {:x?}, state: {:?}, crc: 0x{:08x} }}",
            self.seq, self.label, self.state, self.crc
        )
    }
}

impl TryFrom<[u8; 32]> for EspOTAData {
    type Error = ();
    fn try_from(value: [u8; 32]) -> Result<Self, Self::Error> {
        let seq = u32::from_le_bytes(value[0..4].try_into().unwrap());
        let label = value[4..24].try_into().unwrap();
        let state =
            EspOTAState::try_from(u32::from_le_bytes(value[24..28].try_into().unwrap())).unwrap();
        let crc = u32::from_le_bytes(value[28..32].try_into().unwrap());
        if crc == esp_crc32(&seq.to_le_bytes()) {
            Ok(Self {
                seq,
                label,
                state,
                crc,
            })
        } else {
            Err(())
        }
    }
}

impl From<EspOTAData> for [u8; 32] {
    fn from(value: EspOTAData) -> Self {
        let mut ret = [0; 32];
        ret[0..4].copy_from_slice(&value.seq.to_le_bytes());
        ret[4..24].copy_from_slice(&value.label);
        ret[24..28].copy_from_slice(&u32::to_le_bytes(value.state.into()));
        let crc = esp_crc32(&value.seq.to_le_bytes());
        ret[28..32].copy_from_slice(&crc.to_le_bytes());
        ret
    }
}
