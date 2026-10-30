use cpal::traits::{DeviceTrait, HostTrait};

#[test]
fn list_cpal_devices() {
    let host = cpal::default_host();
    println!("Host: {:?}", host.id());
    println!("--- Input Devices ---");
    if let Ok(devs) = host.input_devices() {
        for (i, d) in devs.enumerate() {
            println!("  [{}] Input: {}", i, d.name().unwrap_or_default());
        }
    }
    println!("--- Output Devices ---");
    if let Ok(devs) = host.output_devices() {
        for (i, d) in devs.enumerate() {
            println!("  [{}] Output: {}", i, d.name().unwrap_or_default());
        }
    }
}
