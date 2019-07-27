use std::io::{stdin, stdout, Write};
use std::sync::{RwLock, Arc};
use std::error::Error;
use std::f64::consts::PI;
use portaudio as pa;

use midir::{MidiInput, Ignore};

// Currently supports i8, i32, f32.
pub type AudioSample = f32;
pub type Input = AudioSample;
pub type Output = AudioSample;

const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 64;

pub struct AppState {
  frequency: f32,
  amp: f32,
  to_amp: f32
}

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
    let app_state_arc = Arc::new(RwLock::new(AppState {
      frequency: 440.0,
      amp: 0.0,
      to_amp: 0.0
    }));

    // let app_state = Arc::new(RwLock::new(AppState {
    //   frequency: 440.0,
    //   amp: 0.0,
    //   to_amp: 0.0
    // }));

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

    // let mut _app_state_reference = app_state.clone();
    let app_state_clone = app_state_arc.clone();
    let _conn_in = midi_in.connect(in_port, "midir-read-input", move |_stamp, message, _| {
      if message.len() > 0 {
        let mut app_state = app_state_clone.write().unwrap();

        if message[0] >> 4 == 0b1001 {
          println!("NOTE ON");
          println!("{} (v={})", message[1], message[2]);
          let m = 0.00 + message[1] as f32;

          // let mut _app_state = _app_state_reference.write().unwrap();
          // *_app_state.amp = 1.0;
          // *_app_state.to_amp = 1.0;
          // *_app_state.frequency = 440.0 * (2.0 as f32).powf((m - 69.0) / 12.0);
          app_state.to_amp = 1.0;
          app_state.frequency = 440.0 * (2.0 as f32).powf((m - 69.0) / 12.0);
          println!("{} {}", m, app_state.amp);
          println!("{}", app_state.amp);
        }
        if message[0] >> 4 == 0b1000 {
          // let mut _app_state = _app_state_reference.write().unwrap();
          app_state.to_amp = 0.0;
          println!("NOTE OFF");
          println!("{} (v={})", message[1], message[2]);
        }
      }
    }, ())?;

    println!(
        "PortAudio Test: output sawtooth wave. SR = {}, BufSize = {}",
        SAMPLE_RATE, FRAMES_PER_BUFFER
    );

    let mut time = 0.0;

    let pa = pa::PortAudio::new()?;

    let mut settings =
        pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER)?;
    // we won't output out of range samples so don't bother clipping them.
    settings.flags = pa::stream_flags::CLIP_OFF;

    // This routine will be called by the PortAudio engine when audio is needed. It may called at
    // interrupt level on some machines so don't do anything that could mess up the system like
    // dynamic resource allocation or IO.

    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        let mut app_state = app_state_arc.write().unwrap();

        let mut idx = 0;
        for _ in 0..frames {

            let a = app_state.amp;
            let sm = app_state.frequency;

            let s = (time as f64 * sm as f64 * PI * 2.0).sin() as f32 * a;

            buffer[idx] = s;
            buffer[idx + 1] = s;

            app_state.amp = app_state.amp + (app_state.to_amp - app_state.amp) / 64.0;

            idx += 2;
            time += 1.0 / SAMPLE_RATE;
        }
        pa::Continue
    };

    let mut stream = pa.open_non_blocking_stream(settings, callback)?;

    stream.start()?;
    println!("Playing sound");

    input.clear();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connection");

    stream.stop()?;
    stream.close()?;

    println!("Test finished.");

    Ok(())
}
