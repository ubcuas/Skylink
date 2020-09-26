# Skylink
`Skylink` is our combination mavlink proxy/forwarder and UAS telemetry server.

## Connections
```
[GCS]---<tcp/mavlink>---[SkyLink]---<tcp/mavlink>---[Pixhawk][SITL]
                            |
                          <TCP>
                            |
        [Smurfette][SkyPasta][Antenna Tracker]
```

## Dependencies
**Docker:**
- Docker

**Local:**
- Rust + Cargo
- Libuuas

## Installation
The image can be directly pulled from DockerHub:
```
docker pull ubcuas/skylink:latest
```
The image can also be built locally:
```
docker build --tag ubcuas/skylink:latest .
```
`Skylink` can also be built locally directly:
```
cargo build --release
```

## Usage
General usage is as follows:
```
docker run -p <TELEM PORT>:<TELEM PORT> -p <MAVDEST PORT>:<MAVDEST PORT> -it ubcuas/skylink:latest <MAVLINK_SRC> <MAVLINK_DEST_PORT> <TELEMETRY_DEST_PORT>
```

Full command line options are as follows:
```
skylink 0.1.0
Everyone gets telemetry!

USAGE:
    skylink <MAVLINK_SRC> <MAVLINK_DEST_PORT> <TELEMETRY_DEST_PORT>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <MAVLINK_SRC>            Mavlink source input string.
    <MAVLINK_DEST_PORT>      Mavlink destination server port.
    <TELEMETRY_DEST_PORT>    Telemtry server port.
```


If you wanted to bridge from the docker UASITL running locally and on port 5670 to port 5050, with telemetry served on port 5555:
```
docker run -p 5555:5555 -p 5050:5050 -it --init ubcuas/skylink:latest 127.0.0.1:5760 5050 5555
```

If you wanted to bridge from the RFD900 to port 5050, with telemetry served on port 5555:
```
docker run -p 5555:5555 -p 5050:5050 --device=/dev/ttyUSB0:/dev/ttyUSB0 -it --init ubcuas/skylink:latest file:/dev/ttyUSB0 5050 5555
```

## Troubleshooting
Contact `Eric Mikulin`
