use plotters::prelude::*;
fn main() {
    println!("Hello, world!");
    println!("{:?}", rocketcan::create_saw_signal(1, 10));
    rocketcan::create_demo_plot().unwrap();
}
