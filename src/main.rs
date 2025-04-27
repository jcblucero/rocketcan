use can_dbc::DBC;
use plotters::prelude::*;
use rocketcan::canlog_reader;
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
    let desired_signal_name = "";
    let designed_message_name = "";
    let can_dbc = match rocketcan::can_decoder::load_dbc("my_dbc") {
        Ok(can_dbc) => can_dbc,
        Err(err) => {
            panic!("Error loading dbc: {err}");
        }
    };
    /*let log_reader = canlog_reader::CanLogReader::from_file("candump.log");
    let mut timestamps = Vec::new();
    let mut data = Vec::new();
    let target_message = can_dbc
        .messages()
        .iter()
        .find(|elm| elm.message_name() == designed_message_name);
    let target_signal = target_message
        .unwrap()
        .signals()
        .iter()
        .find(|signal| signal.name() == desired_signal_name);
    for can_frame in log_reader {
        timestamps.push(can_frame.timestamp);
        //Check can_frame ID
        //sv = can_dbc.get_signal(can_frame,signal)
        signal_value = rocketcan::can_decoder::get_signal(can_frame, target_signal);
        data.push(signal_value)
    }
    //rocketcan::create_i32_plot(timestamps, data, "MySignalPlot");*/

    /*
    Use case 2: Printing data values
    for can_frame in log_reader {
        signal_map = can_dbc.decode_message(can_frame)
        print(signal_map)
    }
     */
    /* converting formats
    for can_frame in log_reader {
        let blf_line: str = can_frame.into_blf_line();
        file.write(blf_line)
    }
    log_reader.iter().map(|frame| frame.into_blf_line()).write
     */
}
