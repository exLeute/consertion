//! Logarithmic and other non-linear scales (dBm, pH, wire gauge, MIDI notes, stops). These are the only units
//! computed in floating point, so their results are marked "≈".

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    /// Power level, base watt.
    Dbm,
    Dbw,
    /// Power ratio, base decimal fraction.
    DbPower,
    /// Amplitude (field) ratio.
    DbAmplitude,
    /// H⁺ concentration, base mol/L.
    Ph,
    /// American wire gauge, base meter of diameter.
    Awg,
    /// Wind force, base m/s (v = 0.836 · B^1.5).
    Beaufort,
    /// Sound pressure level, base pascal (re 20 µPa).
    DbSpl,
    /// MIDI note number, base hertz (69 = A4 = 440 Hz).
    Midi,
    /// Frequency ratio, base cents (2:1 = 1200).
    FreqRatio,
    /// Exposure stops, base light factor (3 stops = ×8).
    Stops,
    /// Optical density of a filter, base light factor (0.9 = ×8).
    OpticalDensity,
}

impl Scale {
    pub fn to_base(self, v: f64) -> f64 {
        match self {
            Scale::Dbm => 10f64.powf(v / 10.0) / 1000.0,
            Scale::Dbw | Scale::DbPower => 10f64.powf(v / 10.0),
            Scale::DbAmplitude => 10f64.powf(v / 20.0),
            Scale::Ph => 10f64.powf(-v),
            Scale::Awg => 0.000127 * 92f64.powf((36.0 - v) / 39.0),
            Scale::Beaufort => 0.836 * v.max(0.0).powf(1.5),
            Scale::DbSpl => 20e-6 * 10f64.powf(v / 20.0),
            Scale::Midi => super::music::hz_of(v),
            Scale::FreqRatio => 1200.0 * v.log2(),
            Scale::Stops => 2f64.powf(v),
            Scale::OpticalDensity => 10f64.powf(v),
        }
    }

    pub fn base_to_unit(self, b: f64) -> Result<f64, &'static str> {
        if self == Scale::Beaufort {
            return if b < 0.0 { Err("Wind speed can't be negative") } else { Ok((b / 0.836).powf(2.0 / 3.0)) };
        }
        if self == Scale::FreqRatio {
            return Ok(2f64.powf(b / 1200.0));
        }
        if b <= 0.0 {
            return Err("A logarithmic scale needs a value above zero");
        }
        Ok(match self {
            Scale::Dbm => 10.0 * (b * 1000.0).log10(),
            Scale::Dbw | Scale::DbPower => 10.0 * b.log10(),
            Scale::DbAmplitude => 20.0 * b.log10(),
            Scale::Ph => -b.log10(),
            Scale::Awg => 36.0 - 39.0 * (b / 0.000127).ln() / 92f64.ln(),
            Scale::DbSpl => 20.0 * (b / 20e-6).log10(),
            Scale::Midi => super::music::midi_of(b),
            Scale::Stops => b.log2(),
            Scale::OpticalDensity => b.log10(),
            Scale::Beaufort | Scale::FreqRatio => unreachable!(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Scale;

    #[test]
    fn scales() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9 * b.abs().max(1.0);
        assert!(close(Scale::Dbm.to_base(30.0), 1.0));
        assert!(close(Scale::Ph.base_to_unit(1e-7).unwrap(), 7.0));
        assert!(close(Scale::Awg.to_base(36.0), 0.000127));
        // AWG 10 is 2.588 mm.
        assert!((Scale::Awg.to_base(10.0) - 0.002588).abs() < 1e-6);
        assert!((Scale::DbSpl.to_base(94.0) - 1.0024).abs() < 1e-4);
        assert!((Scale::Beaufort.base_to_unit(10.8).unwrap() - 5.5).abs() < 0.05);
        assert!(close(Scale::Midi.to_base(69.0), 440.0));
        assert!(close(Scale::FreqRatio.base_to_unit(1200.0).unwrap(), 2.0));
        assert!(close(Scale::Stops.to_base(3.0), Scale::OpticalDensity.to_base(0.9030899869919435)));
        for s in [Scale::Dbm, Scale::Dbw, Scale::DbPower, Scale::DbAmplitude, Scale::Ph, Scale::Awg, Scale::Beaufort, Scale::DbSpl, Scale::Midi, Scale::FreqRatio, Scale::Stops, Scale::OpticalDensity] {
            assert!(close(s.base_to_unit(s.to_base(3.5)).unwrap(), 3.5));
        }
    }
}
