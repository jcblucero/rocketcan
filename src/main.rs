use can_dbc::DBC;
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

    //Parse and plot a signal from canfile
    let can_dbc = match rocketcan::can_decoder::load_dbc("my_dbc") {
        Ok(can_dbc) => can_dbc,
        Err(err) => {
            panic!("Error loading dbc: {err}");
        }
    };
    let log_reader = canlog_reader::CanLogReader::from("my_candump_file");
    /*let mut timestamps = Vec::new();
    let mut data = Vec::new();
    let target_message = can_dbc
        .messages()
        .iter()
        .find(|elm| elm.message_name() == "target");
    let target_signal = target_message
        .unwrap()
        .signals()
        .iter()
        .find(|signal| signal.name() == "target_signal");
    for can_frame in log_reader {
        timestamps.push(can_frame);
        signal_value = rocketcan::can_decoder::get_signal(can_frame, target_signal);
        data.push(can_frame)
    }
    rocketcan::create_i32_plot(timestamps, data, "MySignalPlot");*/
}
