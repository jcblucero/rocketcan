use ablf::BlfFile;
use rocketcan::{can_decoder, canlog_reader};
use std::{fs::File, io::Write};
fn main() {
    println!("Hello, world!");
    println!("{:?}", rocketcan::create_saw_signal(1, 10));
    let signal = rocketcan::create_saw_signal(0, 10);
    let mut x_vals = Vec::new();
    for i in 0..signal.len() as i32 {
        x_vals.push(i);
    }

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

    /// Re-write timestamps
    let log = canlog_reader::CanLogReader::from_file(
        "/home/jlucero/projects/rocketcan/can_samples/aphryx-canx-nissan-leaf/demo_meet_200k.log",
    );
    //let writer = canlog_reader::CanLogWriter("")
    let output_path =
        "/home/jlucero/projects/rocketcan/can_samples/aphryx-canx-nissan-leaf/demo_meet_200k_revised.log";
    let mut output_file = std::fs::File::create(output_path).unwrap();

    let mut time = 0.;
    let mut first_time = true;
    for mut can_frame in log {
        if first_time {
            time = can_frame.timestamp;
            first_time = false;
        }

        can_frame.timestamp = time;

        writeln!(
            output_file,
            "{}",
            canlog_reader::frame_to_candump_line(can_frame)
        )
        .unwrap();
        time += 0.1;
    }

    //TESTING BLF READING
    /*println!("--Testing BLF Reading---");
    let file = File::open("can_samples/gpl-licensed-blf-technica/test_CanFdMessage.blf").unwrap();
    let reader = std::io::BufReader::new(file);
    let blf = BlfFile::from_reader(reader).unwrap();
    println!("{:?}", blf.file_stats);
    for t in blf {
        dbg!(t);
    }*/
    

    //let filename = "~/rust_projects/aphryx-canx-nissan-leaf/demo_meet_200k.log";
    //or line in
}
