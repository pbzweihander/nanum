FROM alpine

COPY target/x86_64-unknown-linux-musl/release/nanum /usr/local/bin/nanum

CMD ["nanum"]
