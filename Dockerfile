FROM rust:1.50.0 as builder

WORKDIR /usr/src/drovah
COPY . .

RUN cargo install --path .

FROM debian:latest

RUN apt-get update && apt-get upgrade -y
RUN apt-get install mariadb-client -y

COPY --from=builder /usr/local/cargo/bin/drovah /

COPY static /static
COPY migrations /migrations

CMD ["./drovah"]