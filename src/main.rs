use std::io::{stdin, stdout, Write};
use std::sync::{RwLock, Arc};
use std::error::Error;
use portaudio as pa;

use midir::{MidiInput, Ignore};

// Currently supports i8, i32, f32.
pub type AudioSample = f32;
pub type Input = AudioSample;
pub type Output = AudioSample;

const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 64;

fn main() {
    match run() {
        Ok(_) => {}
        e => {
            eprintln!("Example failed with the following: {:?}", e);
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();
    let saw_mod_state = Arc::new(RwLock::new(0.02));

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    // Get an input port (read from console if multiple are available)
    let in_port = match midi_in.port_count() {
        0 => return Err("no input port found".into()),
        1 => {
            println!("Choosing the only available input port: {}", midi_in.port_name(0).unwrap());
            0
        },
        _ => {
            println!("\nAvailable input ports:");
            for i in 0..midi_in.port_count() {
                println!("{}: {}", i, midi_in.port_name(i).unwrap());
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            input.trim().parse()?
        }
    };

    println!("\nOpening connection");
    let _in_port_name = midi_in.port_name(in_port)?;

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let inner_saw_mod_state = saw_mod_state.clone();
    let _conn_in = midi_in.connect(in_port, "midir-read-input", move |_stamp, message, _| {
      if message.len() > 0 {
        if message[0] >> 4 == 0b1001 {
          println!("NOTE ON");
          println!("{} (v={})", message[1], message[2]);
          let m = 0.00 + message[1] as f32;
          let sm = (m - 60.0)/100.0 + 0.01;

          let mut n = inner_saw_mod_state.write().unwrap();
          *n = sm;

          // let mut sm = saw_mod_state.lock().unwrap();
          // *sm = (m - 60.0)/100.0 + 0.01;
          println!("{}", *n);
        }
        if message[0] >> 4 == 0b1000 {
          println!("NOTE OFF");
          println!("{} (v={})", message[1], message[2]);
        }
      }
    }, ())?;

    println!(
        "PortAudio Test: output sawtooth wave. SR = {}, BufSize = {}",
        SAMPLE_RATE, FRAMES_PER_BUFFER
    );

    let mut left_saw = 0.0;
    let mut right_saw = 0.0;

    let pa = pa::PortAudio::new()?;

    let mut settings =
        pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER)?;
    // we won't output out of range samples so don't bother clipping them.
    settings.flags = pa::stream_flags::CLIP_OFF;

    // This routine will be called by the PortAudio engine when audio is needed. It may called at
    // interrupt level on some machines so don't do anything that could mess up the system like
    // dynamic resource allocation or IO.
    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        let mut idx = 0;
        for _ in 0..frames {
            buffer[idx] = left_saw;
            buffer[idx + 1] = right_saw;

            let sm = saw_mod_state.read().unwrap();

            left_saw += *sm;
            if left_saw >= 1.0 {
                left_saw -= 2.0;
            }
            right_saw += *sm;
            if right_saw >= 1.0 {
                right_saw -= 2.0;
            }
            idx += 2;
        }
        pa::Continue
    };

    let mut stream = pa.open_non_blocking_stream(settings, callback)?;

    stream.start()?;

    // println!("Play for {} seconds.", NUM_SECONDS);
    // pa.sleep(NUM_SECONDS * 1_000);

    println!("Playing sound");

    input.clear();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connection");



    stream.stop()?;
    stream.close()?;

    println!("Test finished.");

    Ok(())
}
