# Skylink

## Usage

```
docker build . -t skylink
docker run -p [TELEM PORT]:[TELEM PORT] -p [DEST PORT]:[DEST PORT] -it skylink [SOURCE STRING] [DEST PORT] [TELEM PORT]
```
## Examples

If you wanted to bridge from the RFD to port 5050, with telem on 5555
`docker run -p 5555:5555 -p 5050:5050 --device=/dev/ttyUSB0:/dev/ttyUSB0 -it skylink file:/dev/ttyUSB0 5050 5555`

If you wanted to bridge from the docker SITL with docker ip 172.0.0.6 and port 5670 to port 5050, with telem on 5555
`docker run -p 5555:5555 -p 5050:5050 -it skylink tcpout:172.0.0.6:5760 5050 5555`
