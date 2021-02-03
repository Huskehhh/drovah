FROM rust:1.49.0 as builder

WORKDIR /usr/src/drovah
COPY . .

RUN cargo install --path .

FROM alpine:latest

RUN apk add --no-cache \
        ca-certificates \
        gcc \ 
        mariadb-dev

COPY --from=builder /usr/local/cargo/bin/drovah /usr/local/bin/drovah

COPY .env /.env
COPY static /static
COPY migrations /migrations

CMD ["drovah"]