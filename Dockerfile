FROM rust:latest as build

RUN mkdir -p /uas/skylink
WORKDIR /uas/skylink

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release


FROM debian:latest as runner

RUN mkdir -p /uas/skylink
WORKDIR /uas/skylink

COPY --from=build /uas/skylink/target/release/skylink /uas/skylink/

CMD ["/uas/skylink/skylink"]