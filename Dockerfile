FROM ubcuas/rustuuas:latest as build

RUN mkdir -p /uas/skylink
WORKDIR /uas/skylink

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release


FROM ubcuas/rustuuas:latest as runner

RUN mkdir -p /uas/skylink
WORKDIR /uas/skylink

COPY --from=build /uas/skylink/target/release/skylink /uas/skylink/

ENTRYPOINT ["/uas/skylink/skylink"]
