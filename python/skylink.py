import time
import socket
import json

from pymavlink import mavutil
from argparse import ArgumentParser

parser = ArgumentParser(description=__doc__)
parser.add_argument("srcport", type=int)
parser.add_argument("dstport", type=int)

args = parser.parse_args()

msrc = mavutil.mavlink_connection('tcp:172.17.0.2:{}'.format(args.srcport), planner_format=False,
                                  notimestamps=True,
                                  robust_parsing=True)

mdst = mavutil.mavlink_connection('tcpin:0.0.0.0:{}'.format(args.dstport), planner_format=False,
                                  notimestamps=True,
                                  robust_parsing=True)

uas_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
uas_socket.bind(('0.0.0.0', 5555))
uas_socket.listen()
conn, addr = uas_socket.accept()

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
        print(msg.lat, msg.lon)
        conn.sendall(json.dumps({'lat': msg.lat, 'lon': msg.lon}).encode('UTF-8'))
        conn.sendall("\n".encode('UTF-8'))