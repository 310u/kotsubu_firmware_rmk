pub const USB_VENDOR_ID: u16 = 0x4c4b;
pub const USB_PRODUCT_ID: u16 = 0x4b32;
pub const USB_MANUFACTURER: &str = "310u";
pub const USB_PRODUCT_NAME: &str = "Kotsubu RMK";
pub const USB_SERIAL_NUMBER: &str = "vial:kotsubu:000001";

pub const STORAGE_START_ADDR: usize = 0xA0000;
pub const STORAGE_NUM_SECTORS: u8 = 6;

pub const VIAL_UNLOCK_KEYS: &[(u8, u8)] = &[(0, 0), (0, 1)];
