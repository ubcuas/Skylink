use clap::{load_yaml, App};
use crossbeam::crossbeam_channel;
use mavlink;
use ruuas::telem;
use std::io::{Read, Write, Error, ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

fn telemetry_server(
    telem_port: u32,
    telemframe_reciever: crossbeam_channel::Receiver<mavlink::ardupilotmega::MavMessage>,
) {
    let mut threads = vec![];

    let mut connections = Arc::new(Mutex::new(vec![]));

    threads.push(thread::spawn({
        let conn_pool = connections.clone();

        move || loop {
            let connection_string = format!("127.0.0.1:{}", telem_port);
            let listener = TcpListener::bind(connection_string).unwrap();

            for stream in listener.incoming() {
                let mut conn_pool = conn_pool.lock().unwrap();

                conn_pool.push(stream.unwrap());
            }
        }
    }));

    threads.push(thread::spawn({
        let conn_pool = connections.clone();

        move || loop {
            if let Ok(telemframe) = telemframe_reciever.recv() {
                if let mavlink::ardupilotmega::MavMessage::common(common_msg) = telemframe {
                    match common_msg {
                        mavlink::common::MavMessage::GLOBAL_POSITION_INT(gpi_data) => {
                            let mut conn_pool = conn_pool.lock().unwrap();

                            for mut conn in conn_pool.iter() {
                                let telem_args = telem::TelemetryArgs {
                                    latitude: gpi_data.lat,
                                    longitude: gpi_data.lon,
                                    altitude_agl_meters: gpi_data.relative_alt as i32,
                                    altitude_msl_meters: gpi_data.alt as i32,
                                    heading_degrees: gpi_data.hdg as u32,
                                    velocity_x_cm_s: gpi_data.vx as i32,
                                    velocity_y_cm_s: gpi_data.vy as i32,
                                    velocity_z_cm_s: gpi_data.vz as i32,
                                    timestamp_telem_ms: gpi_data.time_boot_ms as u64,
                                    /* EMPTY RN */
                                    roll_rad: 0.0,
                                    pitch_rad: 0.0,
                                    yaw_rad: 0.0,
                                    rollspeed_rad_s: 0.0,
                                    pitchspeed_rad_s: 0.0,
                                    yawspeed_rad_s: 0.0,
                                    timestamp_msg_ms: 0,
                                };
                                let msg = telem::new_telem_msg(telem_args);
                                let data = telem::serialize_telem_msg(msg);
                                conn.write_all(&data);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }));

    for thread in threads {
        thread.join().unwrap();
    }
}

fn main() -> std::io::Result<()> {
    let arg_yml = load_yaml!("cli.yml");
    let matches = App::from(arg_yml).get_matches();

    let mavsrc_string = matches.value_of("mavsrc").unwrap().to_string();
    let mavdest_port = matches.value_of("mavdest").unwrap().parse::<u32>().unwrap();
    let mavdest_string = format!("0.0.0.0:{}", mavdest_port);
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
    let mav_src = TcpStream::connect(&mavsrc_string)?;

    println!("Awaiting local connection...");
    let mav_dest_listener = TcpListener::bind(&mavdest_string)?;
    let mav_dest;
    if let Ok((socket, _addr)) = mav_dest_listener.accept() {
        mav_dest = socket;
    } else {
        return Err(Error::new(ErrorKind::Other, "oh no!"));
    }
    println!("Loop fully connected");

    let mut threads = vec![];
    // let (telemframe_sender, telemframe_reciever) = crossbeam_channel::unbounded();

    let vehicle = Arc::new(Mutex::new(mav_src));
    let gcs = Arc::new(Mutex::new(mav_dest));

    threads.push(thread::spawn({
        let vehicle = vehicle.clone();
        let gcs = gcs.clone();
        move || loop {
            let mut buf: [u8; 1024] = [0; 1024];
            let bytes_read: usize = vehicle.lock().unwrap().read(&mut buf).unwrap();
            gcs.lock().unwrap().write_all(&buf[..bytes_read]).unwrap();
        }
    }));

    threads.push(thread::spawn({
        let vehicle = vehicle.clone();
        let gcs = gcs.clone();
        move || loop {
            let mut buf: [u8; 1024] = [0; 1024];
            let bytes_read: usize = gcs.lock().unwrap().read(&mut buf).unwrap();
            vehicle.lock().unwrap().write_all(&buf[..bytes_read]).unwrap();
        }
    }));

    // threads.push(thread::spawn(move || {
    //     telemetry_server(telemdest_port, telemframe_reciever)
    // }));

    for thread in threads {
        thread.join().unwrap();
    }

    return Ok(());
}
