#[repr(u8)]
pub enum HidSubclass {
    NoBoot = 0,
    Boot = 1,
}

#[repr(u8)]
pub enum HidDevice {
    Keyboard = 1,
    Mouse = 2,
}

#[repr(u8)]
pub enum HidProtocol {
    Boot = 0,
    Report = 1,
}