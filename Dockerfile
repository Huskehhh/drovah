FROM rust:1.49 as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/drovah
COPY . .

RUN cargo install --path .

FROM gcr.io/distroless/cc-debian10

COPY --from=build /usr/local/cargo/bin/drovah /usr/local/bin/drovah

CMD ["drovah"]
