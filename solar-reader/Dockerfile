# Dockerfile
FROM rust:1.87
WORKDIR /usr/src/myapp

COPY ./solar-reader ./solar-reader
COPY ./grid-meter ./grid-meter
RUN cargo install --path ./solar-reader

CMD ["solar-reader"]
