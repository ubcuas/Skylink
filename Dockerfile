FROM ubcuas/rustuuas:latest AS builder

RUN mkdir -p /uas/skylink
WORKDIR /uas/skylink

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release


FROM ubcuas/cppuuas:latest AS runner

RUN mkdir -p /uas/skylink
WORKDIR /uas/skylink

COPY --from=builder /uas/skylink/target/release/skylink /uas/skylink/

ENTRYPOINT ["/uas/skylink/skylink"]
