use anyhow::{anyhow, Context, Result};
use rusb;
use rusb::{DeviceHandle, Direction, Language, Recipient, RequestType, UsbContext};
use std::path::{Path, PathBuf};

fn bytestring(s: &[u8]) -> String {
    s.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<String>>()
        .join("")
}

/// The hid-uclogic driver queries the bytes for this buffer
/// and returns the *full* buffer, including the len + DT prefix bytes.
/// To be compatible with that code, let's do the same here.
fn huion_string_descriptor(
    handle: &DeviceHandle<rusb::Context>,
    lang: &Language,
    index: u8,
) -> rusb::Result<Vec<u8>> {
    let timeout = std::time::Duration::from_millis(100);

    // This matches rusb::DeviceHandle::read_string_descriptor() but
    // that function enforces a utf16 string with even lengths.
    // But we don't have a string here, we just have random bytes,
    // and on the Kamvas 12 our return buffer is length 19 which
    // always results in Error::BadDescriptor.
    let mut buf = [0u8; 256];
    let len = handle.read_control(
        rusb::request_type(Direction::In, RequestType::Standard, Recipient::Device),
        rusb::constants::LIBUSB_REQUEST_GET_DESCRIPTOR,
        u16::from(rusb::constants::LIBUSB_DT_STRING) << 8 | u16::from(index),
        lang.lang_id(),
        &mut buf,
        timeout,
    )?;
    // buf[0] is length of the buffer
    // buf[1] is the descriptor type (LIBUSB_DT_STRING == 0x3)
    if buf[0] != len as u8 {
        return Err(rusb::Error::BadDescriptor);
    }

    Ok(buf[..len].to_vec())
}

fn send_usb_request(device: &rusb::Device<rusb::Context>) -> Result<()> {
    let timeout = std::time::Duration::from_millis(100);
    let handle = device.open()?;
    // See the uclogic driver
    const MAGIC_LANGUAGE_ID: u16 = 0x409;
    let Some(lang) = handle
        .read_languages(timeout)
        .unwrap()
        .into_iter()
        .find(|l| l.lang_id() == MAGIC_LANGUAGE_ID)
    else {
        return Ok(());
    };

    // Firmware call for Huion devices
    // Note: yes, this is a normal read_string_descriptor, see hid-uclogic
    let s = handle.read_string_descriptor(lang, 201, timeout)?;
    let s = s.trim_end_matches('\0');
    println!("HUION_FIRMWARE_ID={s:?}");

    // Get the pen input parameters, see uclogic_params_pen_init_v2()
    // This retrieves magic configuration parameters but more importantly
    // switches the tablet to send events on the 0x8 Report ID (88 bits of Vendor Usage in
    // Usage Page 0x00FF).
    match huion_string_descriptor(&handle, &lang, 200) {
        Ok(bytes) => {
            if bytes.len() >= 18 {
                let bytes = bytestring(&bytes);
                println!("HUION_MAGIC_BYTES={bytes:?}");
                return Ok(());
            }
        }
        Err(rusb::Error::Pipe) => {}
        Err(e) => Err(e).context(format!(
            "Failed reading string descriptor for lang 0x{:x} index 200",
            lang.lang_id()
        ))?,
    };

    // We got a short string descriptor above, try for older tablets, see
    // uclogic_params_pen_init_v2()
    match huion_string_descriptor(&handle, &lang, 100) {
        Ok(bytes) => {
            if bytes.len() >= 12 {
                let bytes = bytestring(&bytes);
                println!("HUION_MAGIC_BYTES={bytes:?}");
                // switch the buttons into raw mode
                // Note: yes, this is a normal read_string_descriptor, see hid-uclogic
                let s = handle.read_string_descriptor(lang, 123, timeout)?;
                println!("HUION_PAD_MODE={s:?}");
            }
        }
        Err(rusb::Error::Pipe) => {}
        Err(e) => Err(e).context(format!(
            "Failed reading string descriptor for lang 0x{:x} index 100",
            lang.lang_id()
        ))?,
    }
    Ok(())
}

fn send_usb_to_all() -> Result<()> {
    let ctx = rusb::Context::new().unwrap();

    const HUION_VENDOR_ID: u16 = 0x256C;

    let rc = ctx
        .devices()
        .unwrap()
        .iter()
        .filter(|d| {
            if let Ok(desc) = d.device_descriptor() {
                desc.vendor_id() == HUION_VENDOR_ID
            } else {
                false
            }
        })
        .try_for_each(|device| send_usb_request(&device));

    rc
}

fn send_usb_to_device(path: &Path) -> Result<()> {
    let device = udev::Device::from_syspath(path)?;

    let usbdev = if device.devtype().unwrap_or_default() == "usb_device" {
        device
    } else {
        device
            .parent_with_subsystem_devtype("usb", "usb_device")?
            .context("No parent device")?
    };

    let busnum = usbdev
        .property_value("BUSNUM")
        .context("Failed to find BUSNUM")?;
    let devnum = usbdev
        .property_value("DEVNUM")
        .context("Failed to find DEVNUM")?;

    let bus = str::parse(&busnum.to_string_lossy())?;
    let addr = str::parse(&devnum.to_string_lossy())?;

    let ctx = rusb::Context::new().unwrap();
    let rc = ctx
        .devices()
        .unwrap()
        .iter()
        .filter(|d| d.bus_number() == bus && d.address() == addr)
        .try_for_each(|device| send_usb_request(&device));

    rc
}

fn search_udev(path: &str) -> Result<()> {
    let path = PathBuf::from(path);
    let mut device = udev::Device::from_syspath(&path)?;
    let properties: Vec<&str> = vec!["HUION_FIRMWARE_ID", "HUION_MAGIC_BYTES", "HUION_PAD_MODE"];

    loop {
        // We expect all properties to be set on the same device
        if properties
            .iter()
            .map(|p| {
                let v = device.property_value(p);
                if let Some(v) = v {
                    println!("{p}={}", v.to_string_lossy());
                }
                v.is_some()
            })
            .fold(false, |acc, x| acc || x)
        {
            break;
        }

        if let Some(parent) = device.parent() {
            device = parent;
        } else {
            // we're out of udev parents so no device has the
            // property set yet. Which means we should send the USB request.
            send_usb_to_device(&path)?;
            break;
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage_string = r#"Usage: huion-switcher --all|<PATH>

Switch a Huion tablet device to vendor reporting mode. If a sysfs PATH
is given, that device is switched. Otherwise if --all is given,
all connected Huion tablets are switched to vendor reporting mode."#;

    if args.iter().any(|s| s == "--help") {
        eprintln!("{usage_string}");
        return ();
    }
    if args.iter().any(|s| s == "--version") {
        println!(env!("CARGO_PKG_VERSION"));
        return ();
    }

    let strs: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
    let rc = match &strs[..] {
        ["--all"] => send_usb_to_all(),
        [path] => search_udev(path),
        _ => Err(anyhow!(format!(
            "Invalid or missing argument\n\n{usage_string}"
        ))),
    };
    if let Err(e) = rc {
        eprintln!("Error: {e}");
    }
}
