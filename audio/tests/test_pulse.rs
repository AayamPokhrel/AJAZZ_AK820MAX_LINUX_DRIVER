use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

#[test]
fn test_pulse_loopback() {
    let spec = Spec {
        format: Format::S16NE,
        channels: 1,
        rate: 44100,
    };
    assert!(spec.is_valid());

    println!("Attempting to connect to PulseAudio/PipeWire @DEFAULT_MONITOR@...");
    match Simple::new(
        None,
        "ajazz-audio-test",
        Direction::Record,
        Some("@DEFAULT_MONITOR@"),
        "Music Visualizer Monitor",
        &spec,
        None,
        None,
    ) {
        Ok(s) => {
            println!("Successfully connected to @DEFAULT_MONITOR@!");
            let mut buf = [0u8; 1024];
            match s.read(&mut buf) {
                Ok(_) => println!("Successfully read 1024 bytes from desktop loopback monitor!"),
                Err(e) => println!("Read error: {}", e),
            }
        }
        Err(e) => {
            println!("Could not connect to @DEFAULT_MONITOR@: {}. Trying default source...", e);
            match Simple::new(
                None,
                "ajazz-audio-test",
                Direction::Record,
                None,
                "Music Visualizer Default",
                &spec,
                None,
                None,
            ) {
                Ok(s) => {
                    println!("Connected to default pulse source!");
                    let mut buf = [0u8; 1024];
                    let _ = s.read(&mut buf);
                }
                Err(e2) => println!("Default source error: {}", e2),
            }
        }
    }
}
