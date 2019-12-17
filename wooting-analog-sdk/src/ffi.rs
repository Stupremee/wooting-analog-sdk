use crate::sdk::*;
use wooting_analog_common::enum_primitive::FromPrimitive;
//use ffi_support::FfiStr;
use std::os::raw::{c_float, c_int, c_uint, c_ushort};
use std::slice;
use wooting_analog_common::*;

/// Initialises the Analog SDK, this needs to be successfully called before any other functions
/// of the SDK can be called
///
/// # Expected Returns
/// * `ret>=0`: Meaning the SDK initialised successfully and the number indicates the number of devices that were found on plugin initialisation
/// * `NoPlugins`: Meaning that either no plugins were found or some were found but none were successfully initialised
#[no_mangle]
pub extern "C" fn wooting_analog_initialise() -> i32 {
    env_logger::init();
    ANALOG_SDK.lock().unwrap().initialise().into()
}

/// Returns a bool indicating if the Analog SDK has been initialised
#[no_mangle]
pub extern "C" fn wooting_analog_is_initialised() -> bool {
    ANALOG_SDK.lock().unwrap().initialised
}

/// Uninitialises the SDK, returning it to an empty state, similar to how it would be before first initialisation
/// # Expected Returns
/// * `Ok`: Indicates that the SDK was successfully uninitialised
#[no_mangle]
pub extern "C" fn wooting_analog_uninitialise() -> WootingAnalogResult {
    ANALOG_SDK.lock().unwrap().unload();
    WootingAnalogResult::Ok
}

/// Sets the type of Keycodes the Analog SDK will receive (in `read_analog`) and output (in `read_full_buffer`).
///
/// By default, the mode is set to HID
///
/// # Notes
/// * `VirtualKey` and `VirtualKeyTranslate` are only available on Windows
/// * With all modes except `VirtualKeyTranslate`, the key identifier will point to the physical key on the standard layout. i.e. if you ask for the Q key, it will be the key right to tab regardless of the layout you have selected
/// * With `VirtualKeyTranslate`, if you request Q, it will be the key that inputs Q on the current layout, not the key that is Q on the standard layout.
///
/// # Expected Returns
/// * `Ok`: The Keycode mode was changed successfully
/// * `InvalidArgument`: The given `KeycodeType` is not one supported by the SDK
/// * `NotAvailable`: The given `KeycodeType` is present, but not supported on the current platform
/// * `UnInitialized`: The SDK is not initialised
#[no_mangle]
pub extern "C" fn wooting_analog_set_keycode_mode(mode: u32) -> WootingAnalogResult {
    if !ANALOG_SDK.lock().unwrap().initialised {
        return WootingAnalogResult::UnInitialized;
    }

    //TODO: Make it return invalid argument when attempting to use virutal keys on platforms other than win
    if let Some(key_mode) = KeycodeType::from_u32(mode) {
        #[cfg(not(windows))]
        {
            if key_mode == KeycodeType::VirtualKey || key_mode == KeycodeType::VirtualKeyTranslate {
                return WootingAnalogResult::NotAvailable;
            }
        }
        ANALOG_SDK.lock().unwrap().keycode_mode = key_mode;
        WootingAnalogResult::Ok
    } else {
        WootingAnalogResult::InvalidArgument
    }
}

/// Reads the Analog value of the key with identifier `code` from any connected device. The set of key identifiers that is used
/// depends on the Keycode mode set using `wooting_analog_set_mode`.
///
/// # Examples
/// ```ignore
/// wooting_analog_set_mode(KeycodeType::ScanCode1);
/// wooting_analog_read_analog(0x10); //This will get you the value for the key which is Q in the standard US layout (The key just right to tab)
///
/// wooting_analog_set_mode(KeycodeType::VirtualKey); //This will only work on Windows
/// wooting_analog_read_analog(0x51); //This will get you the value for the key that is Q on the standard layout
///
/// wooting_analog_set_mode(KeycodeType::VirtualKeyTranslate);
/// wooting_analog_read_analog(0x51); //This will get you the value for the key that inputs Q on the current layout
/// ```
///
/// # Expected Returns
/// The float return value can be either a 0->1 analog value, or (if <0) is part of the WootingAnalogResult enum, which is how errors are given back on this call.
/// So if the value is below 0, you should cast it as WootingAnalogResult to see what the error is.
/// * `0.0f - 1.0f`: The Analog value of the key with the given id `code`
/// * `WootingAnalogResult::NoMapping`: No keycode mapping was found from the selected mode (set by wooting_analog_set_mode) and HID.
/// * `WootingAnalogResult::UnInitialized`: The SDK is not initialised
/// * `WootingAnalogResult::NoDevices`: There are no connected devices
#[no_mangle]
pub extern "C" fn wooting_analog_read_analog(code: c_ushort) -> f32 {
    wooting_analog_read_analog_device(code, 0)
}

/// Reads the Analog value of the key with identifier `code` from the device with id `device_id`. The set of key identifiers that is used
/// depends on the Keycode mode set using `wooting_analog_set_mode`.
///
/// The `device_id` can be found through calling `wooting_analog_device_info` and getting the DeviceID from one of the DeviceInfo structs
///
/// # Expected Returns
/// The float return value can be either a 0->1 analog value, or (if <0) is part of the WootingAnalogResult enum, which is how errors are given back on this call.
/// So if the value is below 0, you should cast it as WootingAnalogResult to see what the error is.
/// * `0.0f - 1.0f`: The Analog value of the key with the given id `code` from device with id `device_id`
/// * `WootingAnalogResult::NoMapping`: No keycode mapping was found from the selected mode (set by wooting_analog_set_mode) and HID.
/// * `WootingAnalogResult::UnInitialized`: The SDK is not initialised
/// * `WootingAnalogResult::NoDevices`: There are no connected devices with id `device_id`
#[no_mangle]
pub extern "C" fn wooting_analog_read_analog_device(code: c_ushort, device_id: DeviceID) -> f32 {
    ANALOG_SDK
        .lock()
        .unwrap()
        .read_analog(code, device_id)
        .into()
}

/// Set the callback which is called when there is a DeviceEvent. Currently these events can either be Disconnected or Connected(Currently not properly implemented).
/// The callback gets given the type of event `DeviceEventType` and a pointer to the DeviceInfo struct that the event applies to
///
/// # Notes
/// * There's no guarentee to the lifetime of the DeviceInfo pointer given during the callback, if it's a Disconnected event, it's likely the memory will be freed immediately after the callback, so it's best to copy any data you wish to use.
/// * The execution of the callback is performed in a separate thread so it is fine to put time consuming code and further SDK calls inside your callback
///
/// # Expected Returns
/// * `Ok`: The callback was set successfully
/// * `UnInitialized`: The SDK is not initialised
#[no_mangle]
pub extern "C" fn wooting_analog_set_device_event_cb(
    cb: extern "C" fn(DeviceEventType, DeviceInfoPointer),
) -> WootingAnalogResult {
    ANALOG_SDK.lock().unwrap().set_device_event_cb(move |event, device| cb(event, device)).into()
}

/// Clears the device event callback that has been set
///
/// # Expected Returns
/// * `Ok`: The callback was cleared successfully
/// * `UnInitialized`: The SDK is not initialised
#[no_mangle]
pub extern "C" fn wooting_analog_clear_device_event_cb() -> WootingAnalogResult {
    ANALOG_SDK.lock().unwrap().clear_device_event_cb().into()
}

/// Fills up the given `buffer`(that has length `len`) with pointers to the DeviceInfo structs for all connected devices (as many that can fit in the buffer)
///
/// # Notes
/// There is no guarenteed lifetime of the DeviceInfo structs given back, so if you wish to use any data from them, please copy it.
///
/// # Expected Returns
/// Similar to wooting_analog_read_analog, the errors and returns are encoded into one type. Values >=0 indicate the number of items filled into the buffer, with `<0` being of type WootingAnalogResult
/// * `ret>=0`: The number of connected devices that have been filled into the buffer
/// * `WootingAnalogResult::UnInitialized`: Indicates that the AnalogSDK hasn't been initialised
#[no_mangle]
pub unsafe extern "C" fn wooting_analog_get_connected_devices_info(
    buffer: *mut DeviceInfoPointer,
    len: c_uint,
) -> c_int {


    match ANALOG_SDK.lock().unwrap().get_device_info().0 {
        Ok(mut devices) => {
            let device_no = (len as usize).min(devices.len());

            let buff = {
                assert!(!buffer.is_null());

                slice::from_raw_parts_mut(buffer, device_no)
            };

            devices.truncate(device_no);
            buff.swap_with_slice(devices.as_mut());

            device_no as i32
        },
        Err(e) => {
            e.into()
        }

    }
}

/// Reads all the analog values for pressed keys for all devices and combines their values, filling up `code_buffer` with the
/// keycode identifying the pressed key and fills up `analog_buffer` with the corresponding float analog values. i.e. The analog
/// value for they key at index 0 of code_buffer, is at index 0 of analog_buffer.
///
/// # Notes
/// * `len` is the length of code_buffer & analog_buffer, if the buffers are of unequal length, then pass the lower of the two, as it is the max amount of
/// key & analog value pairs that can be filled in.
/// * The codes that are filled into the `code_buffer` are of the KeycodeType set with wooting_analog_set_mode
/// * If two devices have the same key pressed, the greater value will be given
/// * When a key is released it will be returned with an analog value of 0.0f in the first read_full_buffer call after the key has been released
///
/// # Expected Returns
/// Similar to other functions like `wooting_analog_device_info`, the return value encodes both errors and the return value we want.
/// Where >=0 is the actual return, and <0 should be cast as WootingAnalogResult to find the error.
/// * `>=0` means the value indicates how many keys & analog values have been read into the buffers
/// * `WootingAnalogResult::UnInitialized`: Indicates that the AnalogSDK hasn't been initialised
/// * `WootingAnalogResult::NoDevices`: Indicates no devices are connected
#[no_mangle]
pub unsafe extern "C" fn wooting_analog_read_full_buffer(
    code_buffer: *mut c_ushort,
    analog_buffer: *mut c_float,
    len: c_uint,
) -> c_int {
    wooting_analog_read_full_buffer_device(code_buffer, analog_buffer, len, 0)
}

/// Reads all the analog values for pressed keys for the device with id `device_id`, filling up `code_buffer` with the
/// keycode identifying the pressed key and fills up `analog_buffer` with the corresponding float analog values. i.e. The analog
/// value for they key at index 0 of code_buffer, is at index 0 of analog_buffer.
///
/// # Notes
/// * `len` is the length of code_buffer & analog_buffer, if the buffers are of unequal length, then pass the lower of the two, as it is the max amount of
/// key & analog value pairs that can be filled in.
/// * The codes that are filled into the `code_buffer` are of the KeycodeType set with wooting_analog_set_mode
/// * When a key is released it will be returned with an analog value of 0.0f in the first read_full_buffer call after the key has been released
///
/// # Expected Returns
/// Similar to other functions like `wooting_analog_device_info`, the return value encodes both errors and the return value we want.
/// Where >=0 is the actual return, and <0 should be cast as WootingAnalogResult to find the error.
/// * `>=0` means the value indicates how many keys & analog values have been read into the buffers
/// * `WootingAnalogResult::UnInitialized`: Indicates that the AnalogSDK hasn't been initialised
/// * `WootingAnalogResult::NoDevices`: Indicates the device with id `device_id` is not connected
#[no_mangle]
pub unsafe extern "C" fn wooting_analog_read_full_buffer_device(
    code_buffer: *mut c_ushort,
    analog_buffer: *mut c_float,
    len: c_uint,
    device_id: DeviceID,
) -> c_int {
    let codes = {
        assert!(!code_buffer.is_null());

        slice::from_raw_parts_mut(code_buffer, len as usize)
    };

    let analog = {
        assert!(!analog_buffer.is_null());

        slice::from_raw_parts_mut(analog_buffer, len as usize)
    };

    match ANALOG_SDK.lock().unwrap().read_full_buffer(len as usize, device_id).0 {
        Ok(analog_data) => {
            //Fill up given slices
            let mut count: usize = 0;
            for (code, val) in analog_data.iter() {
                if count >= codes.len() {
                    break;
                }

                codes[count] = *code;
                analog[count] = *val;
                count += 1;
            }
            (count as c_int)
        },
        Err(e) => {
            e as c_int
        }
    }
}
