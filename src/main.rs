use clap::{App, Arg};
use mavlink;
use std::sync::Arc;
use std::thread;
use std::time;

fn main() {
    let matches = App::new("skylink")
        .version("0.1.0")
        .author("Eric M. <ericm99@gmail.com>")
        .about("Everyone gets telemetry!")
        .arg(
            Arg::with_name("mavsrc")
                .value_name("MAVLINK_SRC")
                .help("Mavlink souce input string.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("mavdest")
                .value_name("MAVLINK_DEST_PORT")
                .help("Mavlink destination server port.")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("telemdest")
                .value_name("TELEMETRY_DEST_PORT")
                .help("Telemtry server port.")
                .required(true)
                .index(3),
        )
        .get_matches();

    let mavsrc_string = matches.value_of("mavsrc").unwrap().to_string();
    let mavdest_port = matches.value_of("mavdest").unwrap().parse::<u32>().unwrap();
    let mavdest_string = format!("tcpin:0.0.0.0:{}", mavdest_port);
    let telemdest_port = matches
        .value_of("telemdest")
        .unwrap()
        .parse::<u32>()
        .unwrap();

    println!(
        "Starting passthrough from {} --> {}",
        mavsrc_string, mavdest_string
    );
    println!("Starting remote connection...");
    let mav_src =
        mavlink::connect::<mavlink::ardupilotmega::MavMessage>(&mavsrc_string).unwrap();
    // mav_src.set_protocol_version(mavlink::MavlinkVersion::V2);

    println!("Awaiting local connection...");
    let mav_dest =
        mavlink::connect::<mavlink::ardupilotmega::MavMessage>(&mavdest_string).unwrap();
    // mav_dest.set_protocol_version(mavlink::MavlinkVersion::V2);
    println!("Loop fully connected");

    let vehicle = Arc::new(mav_src);
    let gcs = Arc::new(mav_dest);

    let vehicle_thread = thread::spawn({
        let vehicle = vehicle.clone();
        let gcs = gcs.clone();
        move || loop {
            if let Ok((msg_header, msg_data)) = vehicle.recv() {
                println!("m1: {:?} {:?}", msg_header, msg_data);
                gcs.send(&msg_header, &msg_data).unwrap();
            }
        }
    });

    let gcs_thread = thread::spawn({
        let vehicle = vehicle.clone();
        let gcs = gcs.clone();
        move || loop {
            if let Ok((msg_header, msg_data)) = gcs.recv() {
                println!("m2: {:?} {:?}", msg_header, msg_data);
                vehicle.send(&msg_header, &msg_data).unwrap();
            }
        }
    });

    vehicle_thread.join();
    gcs_thread.join();
}
