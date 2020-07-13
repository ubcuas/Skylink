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

fn send_telem_to_connections(
    conn_pool: &Arc<Mutex<Vec<TcpStream>>>,
    disconnected_clients: &mut Vec<usize>,
    mavtelem_args: ruuas::telem::MavlinkTelemetryArgs,
) {
    let msg = telem::new_telem_msg(telem::TelemetryArgs::from_mavlinkargs(mavtelem_args));
    let data = telem::serialize_telem_msg(msg);

    let mut conn_pool = conn_pool.lock().unwrap();
    for (i, mut conn) in conn_pool.iter().enumerate() {
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

        let mut mavtelem_args = telem::MavlinkTelemetryArgs::default();

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
                                mavtelem_args.latitude = gpi_data.lat;
                                mavtelem_args.longitude = gpi_data.lon;
                                mavtelem_args.altitude_agl_mm = gpi_data.relative_alt;
                                mavtelem_args.altitude_msl_mm = gpi_data.alt;
                                mavtelem_args.heading_cdeg = gpi_data.hdg;
                                mavtelem_args.velocityx_cm_s = gpi_data.vx;
                                mavtelem_args.velocityy_cm_s = gpi_data.vy;
                                mavtelem_args.velocityz_cm_s = gpi_data.vz;

                                send_telem_to_connections(
                                    &conn_pool,
                                    &mut disconnected_clients,
                                    mavtelem_args.clone(),
                                );
                            }
                            mavlink::common::MavMessage::ATTITUDE(attitude_data) => {
                                mavtelem_args.roll_rad = attitude_data.roll;
                                mavtelem_args.pitch_rad = attitude_data.pitch;
                                mavtelem_args.yaw_rad = attitude_data.yaw;
                                mavtelem_args.rollspeed_rad_s = attitude_data.rollspeed;
                                mavtelem_args.pitchspeed_rad_s = attitude_data.pitchspeed;
                                mavtelem_args.yawspeed_rad_s = attitude_data.yawspeed;
                                mavtelem_args.timestamp_pixhawk_ms = attitude_data.time_boot_ms;

                                send_telem_to_connections(
                                    &conn_pool,
                                    &mut disconnected_clients,
                                    mavtelem_args.clone(),
                                );
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
