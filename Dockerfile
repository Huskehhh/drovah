FROM rust:1.49-alpine as build

RUN apk add alpine-sdk
RUN apk add mariadb-dev

WORKDIR /usr/src/drovah
COPY . .

RUN cargo install --path .

FROM alpine:latest

RUN apk add mariadb-dev

COPY --from=build /usr/local/cargo/bin/drovah /usr/local/bin/drovah

CMD ["drovah"]