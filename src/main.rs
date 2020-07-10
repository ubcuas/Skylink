use clap::{load_yaml, App};
use crossbeam::crossbeam_channel;
use mavlink;
use ruuas::telem;
use std::io::Cursor;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

const BUF_SIZE: usize = 256;

struct BufferFrame {
    data: [u8; BUF_SIZE],
    length: usize,
    new_stream: bool,
}

struct TelemetryServerArgs {
    telem_port: u32,
    buffer_reciever: crossbeam_channel::Receiver<BufferFrame>,
}

fn telemetry_server(args: TelemetryServerArgs) -> std::io::Result<()> {
    let mut threads = vec![];
    let connections = Arc::new(Mutex::new(vec![]));

    threads.push(thread::spawn({
        let conn_pool = connections.clone();
        let telem_port = args.telem_port;

        move || loop {
            let connection_string = format!("127.0.0.1:{}", telem_port);
            let listener = TcpListener::bind(connection_string).unwrap();

            for stream in listener.incoming() {
                println!("Telem client connected");

                let mut conn_pool = conn_pool.lock().unwrap();
                conn_pool.push(stream.unwrap());
            }
        }
    }));

    threads.push(thread::spawn({
        let conn_pool = connections.clone();
        let buffer_reciever = args.buffer_reciever;

        let mut byte_buffer: Vec<u8> = Vec::new();
        let mut disconnected_clients: Vec<usize> = Vec::new();

        move || loop {
            if let Ok(bufframe) = buffer_reciever.recv() {
                if bufframe.new_stream == true {
                    byte_buffer.clear();
                }
                byte_buffer.extend_from_slice(&bufframe.data[..bufframe.length]);
                loop {
                    let read_byte_buffer = byte_buffer.clone();
                    let mut buff = Cursor::new(read_byte_buffer);

                    if let Ok((_parsed_header, parsed_msg)) = mavlink::read_v2_msg(&mut buff) {
                        match parsed_msg {
                            mavlink::common::MavMessage::GLOBAL_POSITION_INT(gpi_data) => {
                                let mut conn_pool = conn_pool.lock().unwrap();

                                for (i, mut conn) in conn_pool.iter().enumerate() {
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
                                    match conn.write_all(&data) {
                                        // If it fails for a disconnection, remove it from the pool (after loop finishes).
                                        Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
                                            println!("Telem client disconnect");
                                            disconnected_clients.push(i);
                                        }
                                        // No matter what else, we move on
                                        _ => {}
                                    };
                                }

                                for conn in disconnected_clients.drain(..) {
                                    conn_pool.remove(conn);
                                }
                            }
                            _ => {}
                        }

                        byte_buffer.clear();
                        let start = buff.position() as usize;
                        byte_buffer.extend_from_slice(&buff.into_inner()[start..]);
                    } else {
                        break;
                    }
                }
            }
        }
    }));

    for thread in threads {
        thread.join().unwrap();
    }

    return Ok(());
}

struct MavlinkPassthroughArgs {
    mavsrc_string: String,
    mavdest_string: String,
    buffer_sender: crossbeam_channel::Sender<BufferFrame>,
}

fn mavlink_passthrough_server(args: MavlinkPassthroughArgs) -> std::io::Result<()> {
    println!("Starting remote connection...");
    let mav_src = TcpStream::connect(&args.mavsrc_string)?;

    println!("Awaiting local connection...");
    let mav_dest_listener = TcpListener::bind(&args.mavdest_string)?;
    let mav_dest;
    if let Ok((socket, _addr)) = mav_dest_listener.accept() {
        mav_dest = socket;
    } else {
        return Err(Error::new(ErrorKind::Other, "oh no!"));
    }
    println!("Loop fully connected");

    mav_src.set_nonblocking(true)?;
    mav_dest.set_nonblocking(true)?;
    let mut vehicle = mav_src;
    let mut gcs = mav_dest;

    // Send an empty new stream packet to clear out the telemservers buffers
    let payload = BufferFrame {
        data: [0; BUF_SIZE],
        length: 0,
        new_stream: true,
    };
    &args.buffer_sender.send(payload);

    loop {
        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        match vehicle.read(&mut buf) {
            Ok(bytes_read) => {
                gcs.write_all(&buf[..bytes_read])?;
                let payload = BufferFrame {
                    data: buf,
                    length: bytes_read,
                    new_stream: false,
                };
                &args.buffer_sender.send(payload);
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => panic!("encountered IO error: {}", e),
        };

        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        match gcs.read(&mut buf) {
            Ok(bytes_read) => {
                vehicle.write_all(&buf[..bytes_read])?;
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => panic!("encountered IO error: {}", e),
        };
    }
}

fn main() -> std::io::Result<()> {
    // Load commandline interface from yaml file
    let arg_yml = load_yaml!("cli.yml");
    let matches = App::from(arg_yml).get_matches();

    // Parse commandline arguments, throw error if any fail to parse
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

    let mut threads = vec![];
    let (buffer_sender, buffer_reciever) = crossbeam_channel::unbounded();

    threads.push(thread::spawn(move || loop {
        let args = MavlinkPassthroughArgs {
            mavsrc_string: mavsrc_string.clone(),
            mavdest_string: mavdest_string.clone(),
            buffer_sender: buffer_sender.clone(),
        };

        match mavlink_passthrough_server(args) {
            Ok(_) => {
                println!("Passthrough server exited, exiting...");
                break;
            }
            Err(e) => {
                println!("Passthrough server error: {}", e);
                println!("Passthrough server restarting");
                continue;
            }
        }
    }));

    threads.push(thread::spawn(move || loop {
        let args = TelemetryServerArgs {
            telem_port: telemdest_port.clone(),
            buffer_reciever: buffer_reciever.clone(),
        };

        match telemetry_server(args) {
            Ok(_) => {
                println!("Telemetry server exited, exiting...");
                break;
            }
            Err(e) => {
                println!("Telemetry server error: {}", e);
                println!("Telemetry server restarting");
                continue;
            }
        }
    }));

    for thread in threads {
        thread.join().unwrap();
    }

    return Ok(());
}
