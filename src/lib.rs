#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod sys {
    #![allow(non_camel_case_types, non_upper_case_globals, unused, non_snake_case)]
    include!("sys.rs");
}

macro_rules! rtlsdr_result {
    ($expr:expr) => {{
        let result = |ret: i32| -> Result<i32> {
            if ret < 0 {
                Err(RtlsdrError::from(ret))
            } else {
                Ok(ret)
            }
        };
        result(unsafe { $expr })
    }};
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// General error type for rtlsdr operations, which can be either a libusb error or
/// some other unspecified error
pub enum RtlsdrError {
    LibusbError(LibusbError),
    Unspecified(i32),
}

impl From<i32> for RtlsdrError {
    fn from(err: i32) -> Self {
        match LibusbError::try_from(err) {
            Ok(e) => Self::LibusbError(e),
            Err(e) => Self::Unspecified(e),
        }
    }
}

pub type Result<T> = std::result::Result<T, RtlsdrError>;

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LibusbError {
    /// Input/output error
    IoError = -1,
    /// Invalid parameter
    InvalidParam = -2,
    /// Access denied (insufficient permissions)
    AccessDenied = -3,
    /// No such device (it may have been disconnected)
    NoDevice = -4,
    /// Entity not found
    NoEntity = -5,
    /// Resource busy
    Busy = -6,
    /// Operation timed out
    Timeout = -7,
    /// Overflow
    Overflow = -8,
    /// Pipe error
    Pipe = -9,
    /// System call interrupted (perhaps due to signal)
    Interrupted = -10,
    /// Insufficient memory
    InsufficientMemory = -11,
    /// Operation not supported or unimplemented on this platform
    NotSupported = -12,
    /// Other error
    Other = -99,
}

impl TryFrom<i32> for LibusbError {
    type Error = i32;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::IoError),
            -2 => Ok(Self::InvalidParam),
            -3 => Ok(Self::AccessDenied),
            -4 => Ok(Self::NoDevice),
            -5 => Ok(Self::NoEntity),
            -6 => Ok(Self::Busy),
            -7 => Ok(Self::Timeout),
            -8 => Ok(Self::Overflow),
            -9 => Ok(Self::Pipe),
            -10 => Ok(Self::Interrupted),
            -11 => Ok(Self::InsufficientMemory),
            -12 => Ok(Self::NotSupported),
            -99 => Ok(Self::Other),
            e => Err(e),
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TunerType {
    /// Unknown tuner type
    Unknown = 0,
    /// Elonics E4000 tuner
    E4000 = 1,
    /// FC0012 tuner
    FC0012 = 2,
    /// FC0013 tuner
    FC0013 = 3,
    /// FC2580 tuner
    FC2580 = 4,
    /// Realtek 820T tuner
    R820T = 5,
    /// Realtek 828D tuner
    R828D = 6,
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sideband {
    Lower = 0,
    Upper = 1,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DirectSampling {
    Disabled = 0,
    I = 1,
    Q = 2,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DirectSamplingThreshold {
    Disabled = 0,
    I = 1,
    Q = 2,
    IBelow = 3,
    QBelow = 4,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Device struct
pub struct Device {
    index: u32,
    dev: *mut sys::rtlsdr_dev,
}

impl Device {
    /// Open a device
    /// # Errors
    /// This will return an error if the device cannot be opened by librtlsdr (e.g. libusb failure or device is already open)
    pub fn open(&mut self) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_open(&raw mut self.dev, self.index))?;

        Ok(())
    }

    /// Close device
    /// # Errors
    /// This will return an error if the device cannot be closed by librtlsdr (e.g. libusb failure or device is not open)
    pub fn close(&mut self) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_close(self.dev))?;

        self.dev = std::ptr::null_mut();

        Ok(())
    }

    /// Get crystal oscillator frequencies used for the RTL2832 and the tuner IC
    /// Usually both ICs use the same clock.
    /// # Errors
    /// This will return an error if the frequencies cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    pub fn get_xtal_freq(&self) -> Result<(u32, u32)> {
        let mut rtl_freq = 0;
        let mut tuner_freq = 0;

        rtlsdr_result!(sys::rtlsdr_get_xtal_freq(
            self.dev,
            &raw mut rtl_freq,
            &raw mut tuner_freq
        ))?;

        Ok((rtl_freq, tuner_freq))
    }

    /// Set crystal oscillator frequencies used for the RTL2832 and the tuner IC.
    /// Usually both ICs use the same clock.
    /// Changing the clock may make sense if you are applying an external clock to the tuner
    /// or to compensate the frequency (and samplerate) error caused by the original (cheap) crystal.
    ///
    /// NOTE: Call this function only if you fully understand the implications.
    /// # Errors
    /// This will return an error if the frequencies cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_xtal_freq(&mut self, rtl_freq: u32, tuner_freq: u32) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_xtal_freq(self.dev, rtl_freq, tuner_freq))?;
        Ok(())
    }

    /// Get USB device strings.
    /// # Returns
    /// (manufacturer, product, serial) strings
    /// # Panics
    /// This will panic if the retrieved strings from librtlsdr are not valid UTF-8 or if they are not null-terminated.
    /// # Errors
    /// This will return an error if the strings cannot be retrieved by librtlsdr (e
    pub fn get_usb_device_strings(&self) -> Result<(String, String, String)> {
        let mut manufact = [0u8; 256];
        let mut product = [0u8; 256];
        let mut serial = [0u8; 256];

        rtlsdr_result!(sys::rtlsdr_get_usb_strings(
            self.dev,
            manufact.as_mut_ptr().cast::<i8>(),
            product.as_mut_ptr().cast::<i8>(),
            serial.as_mut_ptr().cast::<i8>()
        ))?;

        let mps = [&manufact, &product, &serial].map(|s| {
            std::ffi::CStr::from_bytes_until_nul(s)
                .expect("usb device string is not null-terminated")
                .to_str()
                .expect("Failed to convert usb device string to str")
                .to_string()
        });

        Ok(mps.into())
    }

    /// Read the device EEPROM
    /// # Errors
    /// This will return an error if the EEPROM cannot be read by librtlsdr (e.g. libusb failure or device is not open)
    pub fn read_eeprom(&self, offset: u8, len: u16) -> Result<Box<[u8]>> {
        let mut buf = vec![0u8; len as usize];

        let n = rtlsdr_result!(sys::rtlsdr_read_eeprom(
            self.dev,
            buf.as_mut_ptr(),
            offset,
            len
        ))?;

        #[allow(clippy::cast_sign_loss)]
        // negative values are already handled by the error case above
        buf.truncate(n as usize);

        Ok(buf.into_boxed_slice())
    }

    /// Write the device EEPROM
    /// # Errors
    /// This will return an error if the EEPROM cannot be written by librtlsdr (e.g. libusb failure or device is not open)
    /// # Panics
    /// This will panic if the buffer length exceeds the maximum EEPROM size (65535 bytes)
    pub fn write_eeprom(&mut self, offset: u8, buf: &mut [u8]) -> Result<()> {
        let len = u16::try_from(buf.len()).expect("Buffer length exceeds maximum EEPROM size");

        rtlsdr_result!(sys::rtlsdr_write_eeprom(
            self.dev,
            buf.as_mut_ptr(),
            offset,
            len
        ))?;

        Ok(())
    }

    /// Get actual frequency the device is tuned to in Hz
    /// # Errors
    /// This will return an error if the frequency cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    pub fn get_center_freq(&self) -> Result<u32> {
        match unsafe { sys::rtlsdr_get_center_freq(self.dev) } {
            0 => Err(RtlsdrError::Unspecified(0)),
            freq => Ok(freq),
        }
    }

    /// Set the frequency the device is tuned to in Hz
    /// # Errors
    /// This will return an error if the frequency cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_center_freq(&mut self, freq: u32) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_center_freq(self.dev, freq))?;
        Ok(())
    }

    /// Get actual frequency correction value of the device.
    /// Returns correction value in parts per million (ppm)
    #[must_use]
    pub fn get_freq_correction(&self) -> i32 {
        unsafe { sys::rtlsdr_get_freq_correction(self.dev) }
    }

    /// Set frequency correction value for the device in parts per million (ppm)
    /// # Errors
    /// This will return an error if the frequency cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_freq_correction(&mut self, ppm: i32) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_freq_correction(self.dev, ppm))?;
        Ok(())
    }

    /// Get the tuner type
    #[must_use]
    pub fn get_tuner_type(&self) -> TunerType {
        let tuner_type = unsafe { sys::rtlsdr_get_tuner_type(self.dev) };

        match tuner_type {
            1 => TunerType::E4000,
            2 => TunerType::FC0012,
            3 => TunerType::FC0013,
            4 => TunerType::FC2580,
            5 => TunerType::R820T,
            6 => TunerType::R828D,
            _ => TunerType::Unknown,
        }
    }

    /// Get a list of gains supported by the tuner.
    /// Gain values in tenths of a dB, 115 means 11.5 dB
    /// # Errors
    /// This will return an error if the frequency cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    #[allow(clippy::cast_sign_loss)]
    pub fn get_tuner_gains(&self) -> Result<Vec<i32>> {
        let n = rtlsdr_result!(sys::rtlsdr_get_tuner_gains(self.dev, std::ptr::null_mut()))?;

        let mut gains = vec![0i32; n as usize];

        let n = rtlsdr_result!(sys::rtlsdr_get_tuner_gains(self.dev, gains.as_mut_ptr()))?;
        gains.truncate(n as usize);

        Ok(gains)
    }

    /// Get actual (RF / HF) gain the device is configured to - excluding the IF gain.
    /// Gain in tenths of a dB, 115 means 11.5 dB.
    /// # Errors
    /// This will return an error if the gain cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    pub fn get_tuner_gain(&self) -> Result<i32> {
        match unsafe { sys::rtlsdr_get_tuner_gain(self.dev) } {
            0 => Err(RtlsdrError::Unspecified(0)),
            gain => Ok(gain),
        }
    }

    /// Set the gain for the device.
    /// Manual gain mode must be enabled for this to work.
    /// Valid gain values may be queried with [`Device::get_tuner_gains`] function.
    /// Gain in tenths of a dB, 115 means 11.5 dB
    /// # Errors
    /// This will return an error if the gain cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_tuner_gain(&mut self, gain: i32) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_tuner_gain(self.dev, gain))?;
        Ok(())
    }

    /// Set the bandwidth for the device in Hz. Zero means automatic BW selection.
    /// # Errors
    /// This will return an error if the bandwidth cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_tuner_bandwidth(&mut self, bw: u32) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_tuner_bandwidth(self.dev, bw))?;
        Ok(())
    }

    /// Set the intermediate frequency gain for the device.
    /// - `stage` intermediate frequency gain stage number (1 to 6 for E4000)
    /// - `gain` in tenths of a dB, -30 means -3.0 dB.
    /// # Errors
    /// This will return an error if the IF gain cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_tuner_if_gain(&mut self, stage: i32, gain: i32) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_tuner_if_gain(self.dev, stage, gain))?;
        Ok(())
    }

    /// Set the gain mode (automatic/manual) for the device.
    /// Manual gain mode must be enabled for the gain setter function to work.
    /// # Errors
    /// This will return an error if the gain mode cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_tuner_gain_mode(&mut self, manual: bool) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_tuner_gain_mode(self.dev, i32::from(manual)))?;
        Ok(())
    }

    /// Get actual sample rate the device is configured to in Hz
    /// # Errors
    /// This will return an error if the sample rate cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    pub fn get_sample_rate(&self) -> Result<u32> {
        match unsafe { sys::rtlsdr_get_sample_rate(self.dev) } {
            0 => Err(RtlsdrError::Unspecified(0)),
            rate => Ok(rate),
        }
    }

    /// Set the sample rate for the device in Hz
    /// # Errors
    /// This will return an error if the sample rate cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_sample_rate(&mut self, rate: u32) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_sample_rate(self.dev, rate))?;
        Ok(())
    }

    /// Enable test mode that returns an 8 bit counter instead of the samples.
    /// The counter is generated inside the RTL2832.
    /// # Errors
    /// This will return an error if the test mode cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_test_mode(&mut self, test_mode: bool) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_testmode(self.dev, i32::from(test_mode)))?;
        Ok(())
    }

    /// Enable or disable the internal digital AGC of the RTL2832.
    /// # Errors
    /// This will return an error if the AGC mode cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_agc_mode(&mut self, enabled: bool) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_agc_mode(self.dev, i32::from(enabled)))?;
        Ok(())
    }

    /// Get state of the direct sampling mode
    /// # Errors
    /// This will return an error if the direct sampling mode cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    pub fn get_direct_sampling(&self) -> Result<DirectSampling> {
        rtlsdr_result!(sys::rtlsdr_get_direct_sampling(self.dev)).map(|mode| match mode {
            1 => DirectSampling::I,
            2 => DirectSampling::Q,
            _ => DirectSampling::Disabled,
        })
    }

    /// Enable or disable the direct sampling mode.
    /// When enabled, the IF mode of the RTL2832 is activated, and [`Device::set_center_freq()`] will control the IF-frequency of the DDC,
    /// which can be used to tune from 0 to 28.8 MHz (xtal frequency of the RTL2832).
    /// # Errors
    /// This will return an error if the direct sampling mode cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_direct_sampling(&mut self, mode: DirectSampling) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_direct_sampling(self.dev, mode as i32))?;
        Ok(())
    }

    /// Get state of the offset tuning mode
    /// # Errors
    /// This will return an error if the offset tuning mode cannot be retrieved by librtlsdr (e.g. libusb failure or device is not open)
    pub fn get_offset_tuning(&self) -> Result<bool> {
        rtlsdr_result!(sys::rtlsdr_get_offset_tuning(self.dev)).map(|mode| mode == 1)
    }

    /// Enable or disable offset tuning for zero-IF tuners, which allows to avoid problems caused by the DC offset of the ADCs and 1/f noise.
    /// # Errors
    /// This will return an error if the offset tuning mode cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_offset_tuning(&mut self, enabled: bool) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_offset_tuning(self.dev, i32::from(enabled)))?;
        Ok(())
    }

    /// Reset buffer in RTL2832
    /// # Errors
    /// This will return an error if the buffer cannot be reset by librtlsdr (e.g. libusb failure or device is not open)
    pub fn reset_buffer(&mut self) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_reset_buffer(self.dev))?;
        Ok(())
    }

    /// Read data synchronously
    /// Returns the number of bytes read.
    /// # Errors
    /// This will return an error if the samples cannot be read by librtlsdr (e.g. libusb failure or device is not open)
    /// # Panics
    /// This will panic if the buffer length exceeds the maximum value of i32 (2^31 - 1) bytes
    pub fn read(&self, buf: &mut [u8]) -> Result<i32> {
        let len = i32::try_from(buf.len()).expect("Buffer length exceeds maximum value of u32");

        let mut n_read = 0;

        rtlsdr_result!(sys::rtlsdr_read_sync(
            self.dev,
            buf.as_mut_ptr().cast::<std::ffi::c_void>(),
            len,
            &raw mut n_read
        ))?;

        Ok(n_read)
    }

    /// Read samples from the device asynchronously.
    /// This will block until the asynchronous reading is canceled.
    ///
    /// - `cb`: callback function to return received samples, which should return true to cancel further reading or false to continue reading.
    /// - `buf_num` optional buffer count, `buf_num` * `buf_len` = overall buffer size set to 0 for default buffer count (15)
    /// - `buf_len` optional buffer length, must be multiple of 512, should be a multiple of 16384 (URB size),
    ///   set to 0 for default buffer length (16 * 32 * 512)
    /// # Errors
    /// This will return an error if the asynchronous reading cannot be started by librtlsdr (e.g. libusb failure or device is not open)
    /// # Panics
    /// This will panic if canceling the asynchronous reading from the callback fails
    pub fn start_reading<F>(&self, mut cb: F, buf_num: u32, buf_len: u32) -> Result<()>
    where
        F: FnMut(Vec<u8>) -> bool,
    {
        struct Ctx<'a, F> {
            callback: &'a mut F,
            dev: *mut sys::rtlsdr_dev,
        }

        unsafe extern "C" fn _cb<F>(buf: *mut u8, len: u32, ctx: *mut std::ffi::c_void)
        where
            F: FnMut(Vec<u8>) -> bool,
        {
            let ctx = &mut *ctx.cast::<Ctx<F>>();

            let mut vec = Vec::with_capacity(len as usize);

            let ptr: *mut u8 = vec.as_mut_ptr();
            ptr.copy_from_nonoverlapping(buf, len as usize);

            vec.set_len(len as usize);

            let cancel = (*ctx.callback)(vec);

            if cancel {
                rtlsdr_result!(sys::rtlsdr_cancel_async(ctx.dev))
                    .expect("Failed to cancel async from callback");
            }
        }

        let mut ctx = Ctx {
            callback: &mut cb,
            dev: self.dev,
        };

        rtlsdr_result!(sys::rtlsdr_read_async(
            self.dev,
            Some(_cb::<F>),
            (&raw mut ctx).cast::<std::ffi::c_void>(),
            buf_num,
            buf_len
        ))?;

        Ok(())
    }

    /// Enable or disable (the bias tee on) GPIO PIN 0 - if not reconfigured.
    /// This works for rtl-sdr.com v3 dongles, see <http://www.rtl-sdr.com/rtl-sdr-blog-v-3-dongles-user-guide/>
    /// Note: [`Device::close()`] does not clear GPIO lines, so it leaves the (bias tee) line enabled if a client program
    /// doesn't explictly disable it.
    /// # Errors
    /// This will return an error if the bias tee cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_bias_tee(&mut self, on: bool) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_bias_tee(self.dev, i32::from(on)))?;
        Ok(())
    }

    /// Enable or disable (the bias tee on) the given GPIO pin.
    /// Note: [`Device::close()`] does not clear GPIO lines, so it leaves the (bias tee) lines enabled if a client program
    /// doesn't explictly disable it.
    /// - `gpio_pin` needs to be in 0 .. 7. BUT pin 4 is connected to Tuner RESET.
    ///   and for FC0012 is already connected/reserved pin 6 for switching V/U-HF.
    /// # Errors
    /// This will return an error if the bias tee cannot be set by librtlsdr (e.g. libusb failure or device is not open)
    pub fn set_bias_tee_gpio(&mut self, gpio_pin: i32, on: bool) -> Result<()> {
        rtlsdr_result!(sys::rtlsdr_set_bias_tee_gpio(
            self.dev,
            gpio_pin,
            i32::from(on)
        ))?;
        Ok(())
    }
}

#[doc = "Get all available devices"]
#[must_use]
pub fn get_devices() -> Vec<Device> {
    let n = unsafe { sys::rtlsdr_get_device_count() };

    (0..n)
        .map(|index| Device {
            index,
            dev: std::ptr::null_mut(),
        })
        .collect()
}
