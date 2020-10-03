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
docker run -p <TELEM PORT>:<TELEM PORT> -p <MAVDEST PORT>:<MAVDEST PORT> -it --init ubcuas/skylink:latest <MAVLINK_SRC> <MAVLINK_DEST_PORT> <TELEMETRY_DEST_PORT>
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
docker run --rm -p 5050:5050 -it --init --network=gcom-x_uasnet --name=skylink ubcuas/skylink:latest uasitl:5760 5050 5555
```

If you wanted to bridge from the RFD900 to port 5050, with telemetry served on port 5555:
```
docker run --rm -p 5050:5050 -it --init --network=gcom-x_uasnet --name=skylink --device=/dev/ttyUSB0:/dev/ttyUSB0 ubcuas/skylink:latest file:/dev/ttyUSB0 5050 5555
```


## Troubleshooting
----
`docker: Error response from daemon: network gcom-x_uasnet not found.`
> You need to create the network that the containers connect to. Starting up `gcom-x` will create the network.
> It can also manually be created using the command `docker network create gcom-x_uasnet`.

----
`Cannot connect to the Docker daemon at unix:///var/run/docker.sock. Is the docker daemon running?` or similar.
> You need to run the `docker` commands as root. Use sudo: `sudo docker <command>`. Or add yourself to the docker group.

----
`Anything Else`
> Contact `Eric Mikulin`
