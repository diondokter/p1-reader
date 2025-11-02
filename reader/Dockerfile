# Dockerfile
FROM rust:1.90
WORKDIR /usr/src/myapp

# Install libudev
RUN apt-get update && apt-get install -y libudev-dev

COPY ./reader ./reader
COPY ./grid-meter ./grid-meter
RUN cargo install --path ./reader

CMD ["reader"]
