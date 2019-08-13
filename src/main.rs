use std::io::{stdin, stdout, Write};
use std::sync::{RwLock, Arc};
use std::error::Error;
use std::f64::consts::PI;
use portaudio as pa;

use midir::{MidiInput, Ignore};

use minifb::{Key, KeyRepeat, WindowOptions, Window};

pub mod vcf;
pub use vcf::VCF;

// Currently supports i8, i32, f32.
pub type AudioSample = f32;
pub type Input = AudioSample;
pub type Output = AudioSample;


const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 64;

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

pub struct Voice {
  note: u8,
  frequency: f32
}

pub struct AppState {
  amp: f32,
  to_amp: f32,
  signal: f32,
  controller_1: f32,
  controller_2: f32,
  controller_3: f32,
  controller_4: f32,
  previous_signal: f32,
  voices: Vec<Voice>
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
      voices: Vec::new(),
      amp: 0.0,
      to_amp: 0.0,
      signal: 0.0,
      controller_1: 0.0,
      controller_2: 0.0,
      controller_3: 0.0,
      controller_4: 0.0,
      previous_signal: 0.0
    }));

    // let app_state = Arc::new(RwLock::new(AppState {
    //   frequency: 440.0,
    //   amp: 0.0,
    //   to_amp: 0.0
    // }));

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    // Get an input port (read from console if multiple are available)
    let has_midi;
    let in_port = match midi_in.port_count() {
        0 => {
          has_midi = false;
          0
        },
        1 => {
          has_midi = true;
          println!("Choosing the only available input port: {}", midi_in.port_name(0).unwrap());
          0
        },
        _ => {
          has_midi = true;
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

    if has_midi {
      println!("\nOpening connection");
      let _in_port_name = midi_in.port_name(in_port)?;

      // let mut _app_state_reference = app_state.clone();

      let app_state_clone = app_state_arc.clone();
      let _conn_in = midi_in.connect(in_port, "midir-read-input", move |_stamp, message, _| {
        println!("{}", message[0]);

        let mut app_state = app_state_clone.write().unwrap();
        if message[0] == 176 {
          if message[1] == 1 {
            app_state.controller_1 = message[2] as f32 / 127.0;
          }
          if message[1] == 2 {
            app_state.controller_2 = message[2] as f32 / 127.0;
          }
          if message[1] == 3 {
            app_state.controller_3 = message[2] as f32 / 127.0;
          }
          if message[1] == 4 {
            app_state.controller_4 = message[2] as f32 / 127.0;
          }
        }

        if message.len() > 0 {


          if message[0] >> 4 == 0b1001 {
            println!("NOTE ON");
            println!("{} (v={})", message[1], message[2]);
            let voice = Voice {
              note: message[1],
              frequency: 440.0 * (2.0 as f32).powf((message[1] as f32 - 69.0) / 12.0)
            };

            // let mut _app_state = _app_state_reference.write().unwrap();
            // *_app_state.amp = 1.0;
            // *_app_state.to_amp = 1.0;
            // *_app_state.frequency = 440.0 * (2.0 as f32).powf((m - 69.0) / 12.0);
            //app_state.frequency = 440.0 * (2.0 as f32).powf((m - 69.0) / 12.0);
            println!("{} {}", voice.note, app_state.amp);
            println!("{}", app_state.amp);

            app_state.voices.push(voice);
            app_state.to_amp = 1.0;
          }
          if message[0] >> 4 == 0b1000 {
            // let mut _app_state = _app_state_reference.write().unwrap();
            app_state.voices.retain(|voice| voice.note != message[1]);
            app_state.to_amp = 1.0;
            println!("NOTE OFF");
            println!("{} (v={})", message[1], message[2]);
          }
        }
      }, ())?;
    }

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
    let mut filter = VCF {
      cutoff_frequency: 50.0,
      resonance: 1.0,
      modulation_volume: 1.0,

      q: 1.0,

      a0: 1.0,
      a1: 1.0,
      a2: 1.0,
      b0_b2: 1.0,
      b1: 1.0,

      x1: 1.0,
      x2: 1.0,
      y0: 1.0,
      y1: 1.0,
      y2: 1.0
    };

    let app_state_clone = app_state_arc.clone();
    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        let mut app_state = app_state_clone.write().unwrap();

        let mut idx = 0;
        for _ in 0..frames {
            let a = app_state.amp;
            let previous_signal = app_state.previous_signal;
            let mut signal = 0.0;

            for voice in &app_state.voices {
              let sm = voice.frequency;
              let mut s = 0.0;
              if (time as f64 * sm as f64 * PI * 2.0).sin() as f32 * a > 0.0 {
                s = 1.0;
              } else {
                s = -1.0;
              }

//              let s = (time as f64 * sm as f64 * PI * 2.0).sin() as f32 * a;

              signal += s;
            }

            // let max_freq = (500.0 * app_state.controller_1) as f64;
            // let lfo = (50.0 * app_state.controller_2) as f64;

            // let freq = (time as f64 * lfo * PI * 2.0).sin() * max_freq / 2.0 + max_freq / 2.0;
            // let rc = 1.0/(freq as f32 * 2.0 * 3.14);
            // let dt = 1.0/SAMPLE_RATE as f32;
            // let alpha = dt/(rc+dt);
            // signal = previous_signal + (alpha * (signal - previous_signal));


// double RC = 1.0/(CUTOFF*2*3.14);
//     double dt = 1.0/SAMPLE_RATE;
//     double alpha = dt/(RC+dt);
//     output[0] = input[0]
//     for(int i = 1; i < points; ++i)
//     {
//         output[i] = output[i-1] + (alpha*(input[i] - output[i-1]));
//     }

            let cutoff_value = (100.0 * app_state.controller_1) as f64;
            let lfo = (50.0 * app_state.controller_2) as f64;
            let freq = (time as f64 * lfo * PI * 2.0).sin() * cutoff_value / 2.0 + cutoff_value / 2.0;

            filter.set_cutoff_frequency(freq as f32);
            filter.set_resonance_frequencey(1.0);
            filter.next_sample(signal);
            signal = filter.get_output();

            app_state.signal = signal;

            buffer[idx] = app_state.signal;
            buffer[idx + 1] = app_state.signal;

            app_state.previous_signal = app_state.signal;
            app_state.amp = app_state.amp + (app_state.to_amp - app_state.amp) / 64.0;

            idx += 2;
            time += 1.0 / SAMPLE_RATE;
        }
        pa::Continue
    };

    let mut stream = pa.open_non_blocking_stream(settings, callback)?;

    stream.start()?;
    println!("Playing sound");

    // input.clear();
    // stdin().read_line(&mut input)?; // wait for next enter key press

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new("Test - ESC to exit",
                                  WIDTH,
                                  HEIGHT,
                                  WindowOptions::default()).unwrap_or_else(|e| {
      panic!("{}", e);
    });

    while window.is_open() && !window.is_key_down(Key::Escape) {
      if window.is_key_released(Key::A) {
        let mut app_state = app_state_arc.write().unwrap();

        app_state.voices.retain(|voice| voice.note != 50);
        app_state.to_amp = 1.0;
      }
      if window.is_key_pressed(Key::A, KeyRepeat::No) {
        let mut app_state = app_state_arc.write().unwrap();
        let voice = Voice {
          note: 50,
          frequency: 440.0 * (2.0 as f32).powf((50.0 - 69.0) / 12.0)
        };

        app_state.voices.push(voice);
        app_state.to_amp = 1.0;

      }
      if window.is_key_released(Key::S) {
        let mut app_state = app_state_arc.write().unwrap();

        app_state.voices.retain(|voice| voice.note != 52);
        app_state.to_amp = 1.0;
      }
      if window.is_key_pressed(Key::S, KeyRepeat::No) {
        let mut app_state = app_state_arc.write().unwrap();
        let voice = Voice {
          note: 52,
          frequency: 440.0 * (2.0 as f32).powf((52.0 - 69.0) / 12.0)
        };

        app_state.voices.push(voice);
        app_state.to_amp = 1.0;

      }

      for i in buffer.iter_mut() {
        *i = 0; // write something more funny here!
      }

      // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
      window.update_with_buffer(&buffer).unwrap();
    }

    println!("Closing connection");

    stream.stop()?;
    stream.close()?;

    println!("Test finished.");

    Ok(())
}
