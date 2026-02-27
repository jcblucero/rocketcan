use can_dbc::{DBC, Message};
use ::rocketcan::SignalSeries;
//use ablf::BlfFile;
use rocketcan::{CanFrame, can_decoder, can_encoder, canlog_reader, canlog_writer::{self, CandumpWriter}};
use std::{fs::File, io::Write};
use rand::Rng;
use rocketcan::series_builder;
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
            canlog_writer::frame_to_candump_line(&can_frame)
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
    let out_file = "generated-demo-3.log";
    gen_demo_file(out_file);
}

fn gen_demo_file(output_path: &str) {
    let dbc = can_decoder::load_dbc("can_samples/chrysler_cusw.dbc")
        .expect("failed to load Chrysler DBC");

    let steering_msg = can_decoder::get_message_spec(&dbc, "STEERING").unwrap();
    let levers_msg = can_decoder::get_message_spec(&dbc, "STEERING_LEVERS").unwrap();
    let gearbox_msg = can_decoder::get_message_spec(&dbc, "GEARBOX_1").unwrap();
    let brake1_msg = can_decoder::get_message_spec(&dbc, "BRAKE_1").unwrap();
    let brake2_msg = can_decoder::get_message_spec(&dbc, "BRAKE_2").unwrap();
    let wheels_rear_msg = can_decoder::get_message_spec(&dbc, "WHEEL_SPEEDS_REAR").unwrap();
    let wheels_front_msg = can_decoder::get_message_spec(&dbc, "WHEEL_SPEEDS_FRONT").unwrap();

    let mut writer = CandumpWriter::from_path(output_path)
        .expect("failed to create output file");

    let dt = 0.01_f64;
    let duration = 10.0_f64;
    let steps = (duration / dt) as usize;

    // Steering: sine wave -90..90 deg, 4-second period
    let steer_period = 4.0_f64;
    let steer_omega = 2.0 * std::f64::consts::PI / steer_period;
    let steer_amplitude = 90.0_f64;

    // Speed profile: ramp 0→18 m/s, coast, brake to 0
    let ramp_end = 3.0_f64;
    let coast_end = 7.0_f64;
    let target_speed = 18.0_f64;
    let brake_torque_val = 500.0_f64;

    let mut prev_angle = 0.0_f64;

    // Per-message 4-bit counters (0-15, rollover)
    let mut steering_ctr: u64 = 0;
    let mut gearbox_ctr: u64 = 0;
    let mut brake1_ctr: u64 = 0;
    let mut brake2_ctr: u64 = 0;
    let mut wheels_rear_ctr: u64 = 0;
    let mut wheels_front_ctr: u64 = 0;

    // Wheel speed sensor noise: fraction of current speed (0.05 = ±5%)
    let wheel_noise_factor = 0.03_f64;
    let mut rng = rand::rng();

    for i in 0..steps {
        let t = i as f64 * dt;

        // --- STEERING (494) ---
        let steer_angle = steer_amplitude * (steer_omega * t).sin();
        let steer_rate = (steer_angle - prev_angle) / dt;
        prev_angle = steer_angle;

        let frame = can_encoder::CanFrameBuilder::new(&steering_msg)
            .set("STEER_ANGLE", steer_angle).unwrap()
            .set("STEERING_RATE", steer_rate).unwrap()
            .set("COUNTER", steering_ctr as f64).unwrap()
            .timestamp(t)
            .channel("vcan0".into())
            .build();
        writer.write(&frame).unwrap();
        steering_ctr = (steering_ctr + 1) % 16;

        // --- STEERING_LEVERS (1264): turn signals track steering direction ---
        let turn_signal = if steer_angle < -5.0 {
            1.0 // left blinker
        } else if steer_angle > 5.0 {
            2.0 // right blinker
        } else {
            0.0 // off
        };
        let frame = can_encoder::CanFrameBuilder::new(&levers_msg)
            .set("TURN_SIGNALS", turn_signal).unwrap()
            .timestamp(t)
            .channel("vcan0".into())
            .build();
        writer.write(&frame).unwrap();

        // --- GEARBOX_1 (500): always in Drive (4) ---
        let frame = can_encoder::CanFrameBuilder::new(&gearbox_msg)
            .set("DESIRED_GEAR", 4.0).unwrap()
            .set("ACTUAL_GEAR", 4.0).unwrap()
            .set("COUNTER", gearbox_ctr as f64).unwrap()
            .timestamp(t)
            .channel("vcan0".into())
            .build();
        writer.write(&frame).unwrap();
        gearbox_ctr = (gearbox_ctr + 1) % 16;

        // --- BRAKE_1 (484): vehicle speed + brake PSI ---
        let speed = if t < ramp_end {
            target_speed * t / ramp_end
        } else if t < coast_end {
            target_speed
        } else {
            (target_speed * (duration - t) / (duration - coast_end)).max(0.0)
        };

        // BRAKE_PSI: quick spike to 800 at brake onset, then bleed off
        let psi_ramp_dur = 0.5;
        let psi_bleed_dur = 1.0;
        let brake_psi = if t < coast_end {
            0.0
        } else if t < coast_end + psi_ramp_dur {
            800.0 * (t - coast_end) / psi_ramp_dur
        } else if t < coast_end + psi_ramp_dur + psi_bleed_dur {
            800.0 * (1.0 - (t - coast_end - psi_ramp_dur) / psi_bleed_dur)
        } else {
            0.0
        };

        let frame = can_encoder::CanFrameBuilder::new(&brake1_msg)
            .set("VEHICLE_SPEED", speed).unwrap()
            .set("BRAKE_PSI", brake_psi).unwrap()
            .set("COUNTER", brake1_ctr as f64).unwrap()
            .timestamp(t)
            .channel("vcan0".into())
            .build();
        writer.write(&frame).unwrap();
        brake1_ctr = (brake1_ctr + 1) % 16;

        // --- BRAKE_2 (738): brake torque, lights, human ---
        let torque_ramp_dur = 0.5;
        let is_braking = t >= coast_end && speed > 0.0;
        let brake_torque = if !is_braking {
            0.0
        } else if t < coast_end + torque_ramp_dur {
            brake_torque_val * (t - coast_end) / torque_ramp_dur
        } else {
            brake_torque_val
        };
        let brake_lights = if brake_torque > 0.0 { 1.0 } else { 0.0 };
        let brake_human = if brake_torque > 0.0 { 1.0 } else { 0.0 };
        let frame = can_encoder::CanFrameBuilder::new(&brake2_msg)
            .set("BRAKE_TORQUE", brake_torque).unwrap()
            .set("BRAKE_LIGHTS", brake_lights).unwrap()
            .set("BRAKE_HUMAN", brake_human).unwrap()
            .set("COUNTER", brake2_ctr as f64).unwrap()
            .timestamp(t)
            .channel("vcan0".into())
            .build();
        writer.write(&frame).unwrap();
        brake2_ctr = (brake2_ctr + 1) % 16;

        // --- WHEEL_SPEEDS_REAR (740) ---
        let noise_rl = speed * wheel_noise_factor * rng.gen_range(-1.0..1.0_f64);
        let noise_rr = speed * wheel_noise_factor * rng.gen_range(-1.0..1.0_f64);
        let frame = can_encoder::CanFrameBuilder::new(&wheels_rear_msg)
            .set("WHEEL_SPEED_RL", (speed + noise_rl).max(0.0)).unwrap()
            .set("WHEEL_SPEED_RR", (speed + noise_rr).max(0.0)).unwrap()
            .set("COUNTER", wheels_rear_ctr as f64).unwrap()
            .timestamp(t)
            .channel("vcan0".into())
            .build();
        writer.write(&frame).unwrap();
        wheels_rear_ctr = (wheels_rear_ctr + 1) % 16;

        // --- WHEEL_SPEEDS_FRONT (742) ---
        let noise_fl = speed * wheel_noise_factor * rng.gen_range(-1.0..1.0_f64);
        let noise_fr = speed * wheel_noise_factor * rng.gen_range(-1.0..1.0_f64);
        let frame = can_encoder::CanFrameBuilder::new(&wheels_front_msg)
            .set("WHEEL_SPEED_FL", (speed + noise_fl).max(0.0)).unwrap()
            .set("WHEEL_SPEED_FR", (speed + noise_fr).max(0.0)).unwrap()
            .set("COUNTER", wheels_front_ctr as f64).unwrap()
            .timestamp(t)
            .channel("vcan0".into())
            .build();
        writer.write(&frame).unwrap();
        wheels_front_ctr = (wheels_front_ctr + 1) % 16;
    }

    writer.flush().unwrap();
    println!("Wrote {} frames to {}", steps * 7, output_path);
}
