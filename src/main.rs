use plotters::prelude::*;
fn main() {
    println!("Hello, world!");
    println!("{:?}", rocketcan::create_saw_signal(1, 10));
    rocketcan::create_demo_plot().unwrap();
    rocketcan::create_saw_plot();
    let signal = rocketcan::create_saw_signal(0, 10);
    let mut x_vals = Vec::new();
    for i in 0..signal.len() as i32 {
        x_vals.push(i);
    }
    rocketcan::create_i32_plot(x_vals, signal, "saw_plot_from_main.png");
}
