use plotters::prelude::*;
pub fn create_saw_signal(start: i32, end: i32) -> Vec<i32> {
    let mut ret: Vec<i32> = Vec::new();
    let end = end.saturating_add(1);
    for i in start..end {
        ret.push(i);
    }
    for i in (start..end - 1).rev() {
        ret.push(i)
    }
    return ret;
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

#[cfg(test)]
mod tests {
    use super::*;
}
