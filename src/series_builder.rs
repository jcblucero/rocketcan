/*!
 * Builds time series of values
 */

use std::time::Duration;

/// Interface to store timeseries and build common waveforms
struct TimeSeries {
    current_time: f64,
    time_step: f64,
    values: Vec<f64>,
    time: Vec<f64>,
}

impl TimeSeries {
    /// Create a new time series starting with start_time and 
    pub fn new(start_time_s: f64, time_step: Duration) -> TimeSeries {
        TimeSeries { 
            current_time: start_time_s,
            time_step: time_step.as_secs_f64(),
            values: Default::default(), 
            time: Default::default()
        }
    }

    /// Append a point to the series and increment time
    pub fn add_point(&mut self, x: f64, y: f64) {
        self.time.push(x);
        self.values.push(y);
        self.current_time += self.time_step;
    }

    /// Add to series a ramp from start_val to end_val over time_s seconds
    pub fn ramp(&mut self, start_val: f64, end_val: f64, duration: Duration) {

        let mut current_y = start_val;
        let num_points = {
            let ramp_time = duration.as_secs_f64();
            ramp_time / self.time_step
        };
        let y_step = (end_val - start_val) / num_points;

        while current_y <= end_val  {
            self.add_point(self.current_time, current_y);
            current_y += y_step;
        }
    }
}