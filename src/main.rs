use anyhow::{Context, Result};
use rusb;
use rusb::{DeviceHandle, Language, UsbContext};
use std::path::{Path, PathBuf};

fn utf16_to_be_string(s: &str) -> String {
    s.encode_utf16()
        .map(|b| format!("{:04x}", b.to_be()))
        .collect()
}

fn string_descriptor(
    handle: &DeviceHandle<rusb::Context>,
    lang: &Language,
    index: u8,
) -> rusb::Result<String> {
    let timeout = std::time::Duration::from_millis(100);
    handle.read_string_descriptor(*lang, index, timeout)
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
    let s = string_descriptor(&handle, &lang, 201)?;
    println!("HUION_FIRMWARE_ID={s}");

    // Get the pen input parameters, see uclogic_params_pen_init_v2()
    // This retrieves magic configuration parameters but more importantly
    // switches the tablet to send events on the 0x8 Report ID (88 bits of Vendor Usage in
    // Usage Page 0x00FF).
    match string_descriptor(&handle, &lang, 200) {
        Ok(s) => {
            if s.as_bytes().len() >= 18 {
                let bytes = utf16_to_be_string(&s);
                println!("HUION_MAGIC_BYTES={bytes}");
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
    match string_descriptor(&handle, &lang, 100) {
        Ok(s) => {
            if s.as_bytes().len() == 12 {
                let bytes = utf16_to_be_string(&s);
                println!("HUION_MAGIC_BYTES={bytes}");
                // switch the buttons into raw mode
                let s = string_descriptor(&handle, &lang, 123)?;
                println!("HUION_PAD_MODE={s}");
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
    let strs: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
    let rc = match &strs[..] {
        [path] => search_udev(path),
        _ => send_usb_to_all(),
    };
    if let Err(e) = rc {
        eprintln!("Error: {e}");
    }
}
