use std::f64::consts::PI;

const SAMPLE_RATE: f64 = 44_100.0;

pub struct VCF {
  pub cutoff_frequency: f32,
  pub resonance: f32,
  pub modulation_volume: f32,

  pub q: f32,

  pub a0: f32,
  pub a1: f32,
  pub a2: f32,
  pub b0_b2: f32,
  pub b1: f32,

  pub x1: f32,
  pub x2: f32,
  pub y0: f32,
  pub y1: f32,
  pub y2: f32
}

impl VCF {
  pub fn set_cutoff_frequency(&mut self, percent: f32) {
    self.cutoff_frequency = percent;
  }

  pub fn set_resonance_frequencey(&mut self, percent: f32) {
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

  pub fn next_sample(&mut self, input: f32) {
    self.calculate_coefficients(self.cutoff_frequency);
    let x0 = input;

    self.y0 = (self.b0_b2 * x0 + self.b1 * self.x1 + self.b0_b2 * self.x2 - self.a1*self.y1 - self.a2 * self.y2) / self.a0;
    //m_Y0 =  (m_B0_B2*X0      + m_B1*m_X1         + m_B0_B2*m_X2         - m_A1*m_Y1       - m_A2*m_Y2        ) / m_A0;

    self.x2 = self.x1;
    self.y2 = self.y1;
    self.x1 = x0;
    self.y1 = self.y0;
  }

  pub fn get_output(&mut self) -> f32 {
    return self.y0;
  }
}
