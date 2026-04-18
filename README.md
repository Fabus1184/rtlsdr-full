# rtlsdr-full

[![Rust](https://github.com/Fabus1184/rtlsdr-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/Fabus1184/rtlsdr-rs/actions/workflows/rust.yml)

Complete high-level rust bindings for librtlsdr

## Usage

```rust
let devices = rtlsdr::get_devices();
println!("Found {} device(s)", devices.len());

let mut device = devices[0];
device.open()?;

// Get and print the device information strings
let (manufacturer, product, serial) = device.get_usb_device_strings()?;
println!("Device: {manufacturer} {product} {serial}");

// Configure the device with some common settings
device.set_center_freq(101_700_000)?; // 101.7 MHz
device.set_direct_sampling(rtlsdr::DirectSampling::Disabled)?;
device.set_tuner_bandwidth(0)?;
device.set_agc_mode(true)?;
device.set_tuner_gain_mode(false)?;
device.reset_buffer()?;

// read some samples synchronously
let mut buf = [0u8; 1024];
let n_read = device.read(&mut buf)?;
println!("Read {} bytes of data", n_read);

// start asynchronous reading
println!("Starting asynchronous reading for 3 seconds...");
let start = std::time::Instant::now();
device.start_reading(
    |data| {
        let avg = data.iter().map(|&b| b as u32).sum::<u32>() as f32 / data.len() as f32;

        println!(
            "Received {} bytes of data, average value: {:.2}",
            data.len(),
            avg
        );

        // return false to stop reading after 3 seconds
        start.elapsed().as_secs_f32() >= 3.0
    },
    0,
    0,
)?;

println!("Finished asynchronous reading.");
```
See the [examples](examples).
