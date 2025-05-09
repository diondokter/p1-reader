# Dockerfile
FROM rust:1.86
WORKDIR /usr/src/myapp

# Install libudev
RUN apt-get update && apt-get install -y libudev-dev

COPY . .
RUN cargo install --path .

CMD ["reader"]
