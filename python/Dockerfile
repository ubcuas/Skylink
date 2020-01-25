FROM python:3.7

RUN mkdir -p /uas/skylink
WORKDIR /uas/skylink

COPY requirements.txt .
RUN pip install -r requirements.txt

COPY skylink.py .
