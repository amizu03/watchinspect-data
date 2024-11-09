use rodio::{source::SineWave, OutputStream, Sink};
use std::{thread, time::Duration};

pub fn beep() {
    thread::spawn(move || {
        // Initialize audio output stream
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();

        // Create a new sink for the sine wave sound
        let sink = Sink::try_new(&stream_handle).unwrap();
        let source = SineWave::new(783.0); // G5 @ 783hz
        sink.append(source);

        thread::sleep(Duration::from_millis(250));

        sink.stop(); // Stop sound
    });
}
