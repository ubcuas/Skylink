import asyncio
import json
import socket
import threading
import time

from argparse import ArgumentParser
from pymavlink import mavutil

jsonmsg = ""
jsonmsg_lock = threading.Lock()

def start_background_eventloop(loop: asyncio.AbstractEventLoop) -> None:
    asyncio.set_event_loop(loop)
    loop.run_forever()


async def telemserver(reader, writer):
    global jsonmsg
    while True:
        with jsonmsg_lock:
            if type(jsonmsg) != type("str"):
                writer.write(jsonmsg)
        await writer.drain()
        await asyncio.sleep(1)
    writer.close()


async def telemserver_main(host, port):
    server = await asyncio.start_server(telemserver, host, port)
    await server.serve_forever()


def passthrough_main() -> None:
    global jsonmsg
    msrc = mavutil.mavlink_connection('tcp:172.17.0.2:{}'.format(args.srcport), planner_format=False,
                                      notimestamps=True,
                                      robust_parsing=True)

    mdst = mavutil.mavlink_connection('tcpin:0.0.0.0:{}'.format(args.dstport), planner_format=False,
                                      notimestamps=True,
                                      robust_parsing=True)

    # Trigger sending GPS data streams once a second until the GCS connects
    msrc.mav.request_data_stream_send(msrc.target_system, msrc.target_component, mavutil.mavlink.MAV_DATA_STREAM_ALL, 1, 1)

    while True:
        # SRC -> DEST
        src_msg = msrc.recv()
        if type(src_msg) != type("str"):
            mdst.write(src_msg)

        # DEST -> SRC
        dst_msg = mdst.recv()
        if type(dst_msg) != type("str"):
            msrc.write(dst_msg)

        msg = msrc.mav.parse_char(src_msg)

        if msg and msg.get_type() == 'GPS_RAW_INT':
            jsonmsg_str = json.dumps({'lat': msg.lat, 'lon': msg.lon}) + "\n"
            with jsonmsg_lock:
                jsonmsg = jsonmsg_str.encode('UTF-8')
            last_updated = time.time()


if __name__ == "__main__":
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("srcport", type=int)
    parser.add_argument("dstport", type=int)
    parser.add_argument("telemport", type=int)
    args = parser.parse_args()

    loop = asyncio.new_event_loop()
    t = threading.Thread(target=start_background_eventloop, args=(loop,), daemon=True)
    t.start()
    telem_output_task = asyncio.run_coroutine_threadsafe(telemserver_main("0.0.0.0", args.telemport), loop)

    passthrough_main()