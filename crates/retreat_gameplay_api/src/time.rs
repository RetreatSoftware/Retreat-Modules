//! Config times are seconds; the simulation counts fixed ticks.

use std::time::Duration;

/// Absorbs floating point noise, so that a whole number of ticks is not rounded one up.
const ROUNDING_TOLERANCE: f64 = 1.0e-4;

/// The ticks of length `timestep` that cover `seconds`, rounded up. Zero for zero, negative or
/// not-a-number input.
pub fn ticks_from_seconds(seconds: f32, timestep: Duration) -> u32 {
    // `as` saturates: negative and NaN become 0.
    (f64::from(seconds) / timestep.as_secs_f64() - ROUNDING_TOLERANCE).ceil() as u32
}
