FROM alpine

COPY target/x86_64-unknown-linux-musl/release/nanum /usr/local/bin/nanum
COPY target/x86_64-unknown-linux-musl/release/nanum-admin /usr/local/bin/nanum-admin

CMD ["nanum"]
