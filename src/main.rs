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

pub struct VCF {
  cutoff_frequency: f32,
  resonance: f32,
  modulation_volume: f32,

  q: f32,

  a0: f32,
  a1: f32,
  a2: f32,
  b0_b2: f32,
  b1: f32,

  x1: f32,
  x2: f32,
  y0: f32,
  y1: f32,
  y2: f32
}

impl VCF {
  fn set_cutoff_frequency(&mut self, percent: f32) {
    self.cutoff_frequency = percent;
  }

  fn set_resonance_frequencey(&mut self, percent: f32) {
    self.resonance = percent;
    self.q = 1.0;
    //m_Q = expf (LOG_SQRT2 * (m_fResonance - 100.0/5.0) / (100.0/5.0));
  }

  fn calculate_coefficients(&mut self, cutoff_frequency: f32) {
    let max_freq = 20000 as f32;
    let log_2 = 0.69314718;
    let f0 = (log_2 * (cutoff_frequency-100.0) / 10.0).exp() * max_freq;

    let w0 = 2.0 * PI as f32 * f0 / SAMPLE_RATE as f32;
    let alpha = w0.sin() / (2.0*self.q);
    let cos_w0 = w0.cos();

    self.a0 =  1.0 + alpha;
    self.a1 = -2.0 * cos_w0;
    self.a2 =  1.0 - alpha;
    self.b1 =  1.0 - cos_w0;
    self.b0_b2 = self.b1 / 2.0;
  }

  fn next_sample(&mut self, input: f32) {
    self.calculate_coefficients(self.cutoff_frequency);
    let x0 = input;

    self.y0 = (self.b0_b2 * x0 + self.b1 * self.x1 + self.b0_b2 * self.x2 - self.a1*self.y1 - self.a2 * self.y2) / self.a0;
    //m_Y0 =  (m_B0_B2*X0      + m_B1*m_X1         + m_B0_B2*m_X2         - m_A1*m_Y1       - m_A2*m_Y2        ) / m_A0;

    self.x2 = self.x1;
    self.y2 = self.y1;
    self.x1 = x0;
    self.y1 = self.y0;
  }

  fn get_output(&mut self) -> f32 {
    return self.y0;
  }
}

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

    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        let mut app_state = app_state_arc.write().unwrap();

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

    input.clear();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connection");

    stream.stop()?;
    stream.close()?;

    println!("Test finished.");

    Ok(())
}
