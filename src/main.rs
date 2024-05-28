use libusb::*;

fn send_usb_request(device: &Device) {
    let timeout = std::time::Duration::from_millis(100);
    let handle = device.open().unwrap();
    // See the uclogic driver
    const MAGIC_LANGUAGE_ID: u16 = 0x409;
    handle
        .read_languages(timeout)
        .unwrap()
        .iter()
        .filter(|l| l.lang_id() == MAGIC_LANGUAGE_ID)
        .for_each(|lang| {
            // Firmware call for Huion devices
            let s = handle.read_string_descriptor(*lang, 201, timeout).unwrap();
            println!("HUION_FIRMWARE_ID={s}");
            // Get the pen input parameters, see uclogic_params_pen_init_v2()
            // This retrieves magic configuratino parameters but more importantly
            // switches the tablet to send events on the 0x8 Report ID (88 bits of Vendor Usage in
            // Usage Page 0x00FF).
            let s = handle.read_string_descriptor(*lang, 200, timeout).unwrap();
            if s.as_bytes().len() >= 18 {
                let bytes: Vec<String> = s.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
                println!("HUION_MAGIC_BYTES={}", bytes.join(""));
            } else {
                let s = handle.read_string_descriptor(*lang, 100, timeout).unwrap();
                let bytes: Vec<String> = s.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
                println!("HUION_MAGIC_BYTES={}", bytes.join(""));
                // switch the buttons into raw mode
                let s = handle.read_string_descriptor(*lang, 123, timeout).unwrap();
                println!("HUION_PAD_MODE={s}");
            }
        });
}

fn main() {
    let ctx = Context::new().unwrap();

    const HUION_VENDOR_ID: u16 = 0x256C;

    ctx.devices()
        .unwrap()
        .iter()
        .filter(|d| {
            if let Ok(desc) = d.device_descriptor() {
                desc.vendor_id() == HUION_VENDOR_ID
            } else {
                false
            }
        })
        .for_each(|d| {
            send_usb_request(&d);
        });
}
