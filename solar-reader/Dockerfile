# Dockerfile
FROM rust:1.87
WORKDIR /usr/src/myapp

COPY . .
RUN cargo install --path .

CMD ["solar-reader"]
