use std::sync::Once;

use bitflags::bitflags;
use wasapi::{DeviceCollection, Direction};

static INIT: Once = Once::new();

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Device {
    pub name: String,
    pub id: String,
}

#[derive(Debug)]
pub struct OutputDeviceConfig {
    pub sample_rates: Vec<u32>,
    pub block_sizes: Option<BlockSizeRange>,
    pub num_out_channels: u16,
}

/// The range of possible block sizes for an audio device.
#[derive(Debug)]
pub struct BlockSizeRange {
    /// The minimum buffer/block size that can be used (inclusive)
    pub min: u32,

    /// The maximum buffer/block size that can be used (inclusive)
    pub max: u32,

    /// The default buffer/block size for this device
    pub default: u32,
}

// Defined at https://docs.microsoft.com/en-us/windows/win32/coreaudio/device-state-xxx-constants
bitflags! {
    struct DeviceState: u32 {
        const ACTIVE = 0b00000001;
        const DISABLED = 0b00000010;
        const NOTPRESENT = 0b00000100;
        const UNPLUGGED = 0b00001000;
        const ALL = 0b00001111;
    }
}

pub fn check_init() {
    // Initialize this only once.
    INIT.call_once(|| wasapi::initialize_mta().unwrap());
}

pub fn enumerate_audio_backend() -> Vec<Device> {
    eprintln!("Enumerating WASAPI server...");

    check_init();

    let coll = match DeviceCollection::new(&Direction::Render) {
        Ok(coll) => coll,
        Err(e) => {
            panic!("Failed to get WASAPI device collection: {}", e);
        }
    };

    let num_devices = match coll.get_nbr_devices() {
        Ok(num_devices) => num_devices,
        Err(e) => {
            panic!("Failed to get number of WASAPI devices: {}", e);
        }
    };

    let mut devices: Vec<Device> = Vec::new();

    for i in 0..num_devices {
        match coll.get_device_at_index(i) {
            Ok(device) => {
                match device.get_id() {
                    Ok(device_id) => {
                        let device_name = match device.get_friendlyname() {
                            Ok(name) => name,
                            Err(e) => {
                                eprintln!(
                                    "Failed to get name of WASAPI device with ID {}: {}",
                                    &device_id, e
                                );
                                String::from("unkown device")
                            }
                        };

                        match device.get_state() {
                            Ok(state) => {
                                match DeviceState::from_bits(state) {
                                    Some(state) => {
                                        // What a weird API of using bit flags for each of the different
                                        // states the device can be in.
                                        if state.contains(DeviceState::DISABLED) {
                                            eprintln!("The WASAPI device {} has been disabled by the user", &device_name);
                                        } else if state.contains(DeviceState::NOTPRESENT) {
                                            eprintln!(
                                                "The WASAPI device {} is not present",
                                                &device_name
                                            );
                                        } else {
                                            devices.push(Device {
                                                name: device_name,
                                                id: device_id,
                                            })
                                        }
                                    }
                                    None => {
                                        eprintln!(
                                            "Got invalid state {} for WASAPI device {}",
                                            state, &device_name
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to get state of WASAPI device {}: {}",
                                    &device_name, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get ID of WASAPI device at index {}: {}", i, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get info of WASAPI device at index {}: {}", i, e);
            }
        }
    }

    if devices.is_empty() {
        panic!("Empty");
    } else {
        devices
    }
}

pub fn enumerate_audio_device(device: &Device) -> Result<OutputDeviceConfig, String> {
    eprintln!("Enumerating WASAPI device {} ...", &device.name);

    check_init();

    let (id, wdevice, _) = match find_device(device) {
        Some((id, device, jack_unpopulated)) => (id, device, jack_unpopulated),
        None => return Err(String::from("Failed to find device.")),
    };

    let audio_client = match wdevice.get_iaudioclient() {
        Ok(audio_client) => audio_client,
        Err(e) => {
            return Err(format!(
                "Failed to get audio client from WASAPI device {}: {}",
                &id.name, e
            ));
        }
    };

    // Get the default format for this device.
    let default_format = match audio_client.get_mixformat() {
        Ok(format) => format,
        Err(e) => {
            return Err(format!(
                "Failed to get default wave format of WASAPI device {}: {}",
                &id.name, e
            ));
        }
    };

    // We only care about channels and sample rate, and not the sample type.
    // We will always convert to/from `f32` buffers  anyway.
    // let default_sample_type = match default_format.get_subformat() {
    //     Ok(s) => s,
    //     Err(e) => {
    //         eprintln!(
    //             "Failed to get default wave format of WASAPI device {}: {}",
    //             &id.name,
    //             e
    //         );
    //         return Err(());
    //     }
    // };
    // let default_bps = default_format.get_bitspersample();
    // let default_vbps = default_format.get_validbitspersample();

    let default_sample_rate = default_format.get_samplespersec();
    let default_num_channels = default_format.get_nchannels();
    let default_buffer_size = match audio_client.get_bufferframecount() {
        Ok(b) => Some(BlockSizeRange {
            min: b,
            max: b,
            default: b,
        }),
        Err(e) => {
            eprintln!(
                "Could not get default buffer size of WASAPI device {}: {}",
                &id.name, e
            );
            None
        }
    };

    // We must use the default config when running in shared mode.
    Ok(OutputDeviceConfig {
        sample_rates: vec![default_sample_rate],
        block_sizes: default_buffer_size,
        num_out_channels: default_num_channels,
    })
}

pub fn find_device(device: &Device) -> Option<(Device, wasapi::Device, bool)> {
    eprintln!("Finding WASAPI device {} ...", &device.name);

    let coll = match DeviceCollection::new(&Direction::Render) {
        Ok(coll) => coll,
        Err(e) => {
            eprintln!("Failed to get WASAPI device collection: {}", e);
            return None;
        }
    };

    let num_devices = match coll.get_nbr_devices() {
        Ok(num_devices) => num_devices,
        Err(e) => {
            eprintln!("Failed to get number of WASAPI devices: {}", e);
            return None;
        }
    };

    for i in 0..num_devices {
        match coll.get_device_at_index(i) {
            Ok(d) => {
                match d.get_id() {
                    Ok(device_id) => {
                        let device_name = match d.get_friendlyname() {
                            Ok(name) => name,
                            Err(e) => {
                                eprintln!(
                                    "Failed to get name of WASAPI device with ID {}: {}",
                                    &device_id, e
                                );
                                String::from("unkown device")
                            }
                        };

                        match d.get_state() {
                            Ok(state) => {
                                match DeviceState::from_bits(state) {
                                    Some(state) => {
                                        // What a weird API of using bit flags for each of the different
                                        // states the device can be in.
                                        if state.contains(DeviceState::DISABLED) {
                                            eprintln!("The WASAPI device {} has been disabled by the user", &device_name);
                                        } else if state.contains(DeviceState::NOTPRESENT) {
                                            eprintln!(
                                                "The WASAPI device {} is not present",
                                                &device_name
                                            );
                                        } else {
                                            let id = Device {
                                                name: device_name,
                                                id: device_id,
                                            };

                                            if &id == device {
                                                let jack_unpopulated =
                                                    state.contains(DeviceState::UNPLUGGED);

                                                return Some((id, d, jack_unpopulated));
                                            }
                                        }
                                    }
                                    None => {
                                        eprintln!(
                                            "Got invalid state {} for WASAPI device {}",
                                            state, &device_name
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to get state of WASAPI device {}: {}",
                                    &device_name, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get ID of WASAPI device at index {}: {}", i, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get info of WASAPI device at index {}: {}", i, e);
            }
        }
    }

    None
}
