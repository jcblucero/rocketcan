pub mod can_decoder;
pub mod canlog_reader;

use std::i8;

use plotters::coord::ranged1d::{self, AsRangedCoord};
use plotters::prelude::*;
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

pub fn create_i32_plot(x_data: Vec<i32>, y_data: Vec<i32>, plot_name: &str) {
    let x_range = x_data.iter().min().unwrap().clone()..x_data.iter().max().unwrap().clone();
    let y_range = y_data.iter().min().unwrap().clone()..y_data.iter().max().unwrap().clone();
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
}

pub fn create_saw_plot() {
    let range_start = 0i32;
    let range_end = 20i32;
    let root = BitMapBackend::new("saw_plot.png", (200, 200)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let x_range = range_start..range_end;
    let mut chart = ChartBuilder::on(&root)
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(x_range, range_start..range_end)
        .unwrap();
    let saw_data = create_saw_signal(range_start as i32, 10 as i32);
    //let line = LineSeries::new(saw_data, &RED);
    /*let line2 = LineSeries::new(
        (-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
        &RED,
    );*/
    //chart.draw_series(LineSeries::new((0..10).map(|x| (x as f32, x as f32)), &RED));
    let data_x_y: Vec<_> = (0..saw_data.len())
        .map(|x| x as i32)
        .zip(saw_data.iter().map(|x| *x as i32))
        .collect();
    for &item in data_x_y.iter() {
        println!("{:?}", item);
    }
    let line_series = LineSeries::new(data_x_y.clone(), &RED);
    println!("{:?}", data_x_y);
    chart.draw_series(line_series).unwrap();
    /*chart
    .draw_series(LineSeries::new(
        (-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
        &RED,
    ))
    .unwrap();*/
    chart
        .configure_mesh()
        .x_labels(3)
        .y_labels(3)
        .draw()
        .unwrap();
    root.present().unwrap();
}

pub fn create_demo_plot() -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new("1.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption("y=x^2", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(-1f32..1f32, -0.1f32..1f32)?;
    /*let mut chart = ChartBuilder::on(&root)
    .build_cartesian_2d(-1f32..1f32, -0.1f32..1f32)
    .unwrap();*/

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            (-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
            &RED,
        ))?
        .label("y = x^2")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}

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
        create_i32_plot(series.time, series.values, "test_signal_plot.png");
    }
}
