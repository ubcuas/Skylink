# Skylink

## Usage

```
docker build . -t skylink
docker run -p [TELEM PORT]:[TELEM PORT] -p [DEST PORT]:[DEST PORT] -it skylink python skylink.py [SOURCE STRING] [DEST PORT] [TELEM PORT]
```
## Examples

If you wanted to bridge from the RFD to port 1234, with telem on 5555
`docker run -p 5555:5555 -p 1234:1234 -it skylink python skylink.py /dev/ttyUSB0 1234 5555`

If you wanted to bridge from the docker SITL with docker ip 172.0.0.6 and port 5670 to port 1234, with telem on 5555
`docker run -p 5555:5555 -p 1234:1234 -it skylink python skylink.py tcp:172.0.0.6:5760 1234 5555`
