pub mod can_decoder;
pub mod canlog_reader;

pub use canlog_reader::CanFrame;

use std::i8;
pub fn create_saw_signal(start: i32, end: i32) -> Vec<i32> {
    let mut ret = Vec::new();
    let i = ret.iter();
    let end = end.saturating_add(1);
    for i in start..end {
        ret.push(i as i32);
    }
    for i in (start..end - 1).rev() {
        ret.push(i as i32);
    }
    return ret;
}
/* What we need for a plot
A Chart
* backend
* chart
    * Config: margins, label area
* title
A series
* Name (for text display)
* x-range (min..max)
* y-range (min..max)
* x-values
* y-values
*/
/*
pub fn create_f64_plot(x_data: Vec<f64>, y_data: Vec<f64>, plot_name: &str) {
    let x_max = x_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let y_max = y_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let x_min = x_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let y_min = y_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let x_range = x_min..x_max;
    let y_range = y_min..y_max;
    let root = BitMapBackend::new(plot_name, (500, 500)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    //Set up chart, rane, and label areas
    let mut chart = ChartBuilder::on(&root)
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(x_range, y_range)
        .unwrap();

    let data_x_y: Vec<_> = x_data.iter().zip(y_data.iter()).collect();
    let data_x_y = data_x_y.into_iter().map(|(x, y)| (*x, *y));
    let line_series = LineSeries::new(data_x_y, &RED);
    chart.draw_series(line_series).unwrap();

    chart
        .configure_mesh()
        //.x_labels(3)
        //.y_labels(3)
        .draw()
        .unwrap();

    root.present().unwrap();
}*/

pub struct SignalSeries<T> {
    time: Vec<T>,
    values: Vec<T>,
}

/// Create a time series that steps up by 1 each time step.
/// Rolls over at uint8 max, 255, to 0.
fn create_step_time_series(num_series: i32) -> SignalSeries<i32> {
    let mut series = SignalSeries {
        time: Vec::new(),
        values: Vec::new(),
    };
    let max = u8::MAX;
    let x_max = num_series * (max as i32);
    let mut signal_val = 0;
    for i in 0..x_max {
        println!("i {i}");
        series.time.push(i);
        series.values.push(signal_val);
        signal_val = (signal_val + 1) % 256;
    }
    return series;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_plot() {
        // Create fake signal data
        //TODO: Update with real signal data
        let series = create_step_time_series(4);
        //Create and save plot
        //create_i32_plot(series.time, series.values, "test_signal_plot.png");
    }
}
