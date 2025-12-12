use rocketcan::{can_decoder, canlog_reader};
fn main() {
    println!("Hello, world!");
    println!("{:?}", rocketcan::create_saw_signal(1, 10));
    let signal = rocketcan::create_saw_signal(0, 10);
    let mut x_vals = Vec::new();
    for i in 0..signal.len() as i32 {
        x_vals.push(i);
    }
    rocketcan::create_i32_plot(x_vals, signal, "saw_plot_from_main.png");

    //Use Case 1: Parse and plot a signal from canfile
    let desired_signal_name = "s7big";
    let desired_message_name = "Message378910";
    let can_dbc = match rocketcan::can_decoder::load_dbc("signed.dbc") {
        Ok(can_dbc) => can_dbc,
        Err(err) => {
            panic!("Error loading dbc: {err}");
        }
    };
    let log_reader = canlog_reader::CanLogReader::from_file("s7big.log");
    let target_message = can_decoder::get_message_spec(&can_dbc, desired_message_name).unwrap();
    let target_signal =
        can_decoder::get_signal_spec(&target_message, &desired_signal_name).unwrap();
    let mut timestamps = Vec::new();
    let mut data = Vec::new();
    for can_frame in log_reader {
        timestamps.push(can_frame.timestamp);
        //Check can_frame ID
        //sv = can_dbc.get_signal(can_frame,signal)
        let signal_value =
            rocketcan::can_decoder::decode_signal_by_bytes(&can_frame, &target_signal);
        data.push(signal_value)
    }
    println!("{:?}", data);
    //rocketcan::create_f64_plot(timestamps, data, "s7big-plot.png");
    //rocketcan::create_i32_plot(timestamps, data, "MySignalPlot");
    //Use case 2: Printing data values
    let log_reader = canlog_reader::CanLogReader::from_file("s7big.log");
    for can_frame in log_reader {
        let can_msg = can_decoder::decode_message(&can_frame, &target_message);
        print!("{:#}", can_msg);
    }

    /* converting formats
    for can_frame in log_reader {
        let blf_line: str = can_frame.into_blf_line();
        file.write(blf_line)
    }
    log_reader.iter().map(|frame| frame.into_blf_line()).write
     */
}
