//! USB class constants and structs
//! Used by descriptor parser and drivers
pub mod audio;
pub mod hid;

#[derive(Clone, Copy, Debug, PartialEq, strum_macros::FromRepr)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum DeviceClass {
    FromInterface = 0x0,
    Audio = 0x01,
    Cdc = 0x02,
    Hid = 0x03,
    Physical = 0x05,
    Imaging = 0x06,
    Printer = 0x07,
    MassStorage = 0x08,
    Hub = 0x09,
    CdcData = 0x0A,
    SmartCard = 0x0B,
    ContentSecurity = 0x0D,
    Video = 0x0E,
    PersonalHealthcare = 0x0F,
    AudioVideo = 0x10,
    Billboard = 0x11,
    UsbTypeCBridge = 0x12,
    I3C = 0x30,
    Diagnostic = 0xDC,
    WirelessController = 0xE0,
    Misc = 0xEF,
    ApplicationSpecific = 0xFE,
    VendorSpecific = 0xFF,
}

pub type DeviceSubclass = u8;
