FROM ustclug/rocky:9-minimal

LABEL MANTAINER "-veldrane"

COPY target/x86_64-unknown-linux-musl/release/simple-api-rs /simple-api-rs

RUN microdnf install -y net-tools iproute iputils bash \
    && chmod +x /simple-api-rs 

ENTRYPOINT ["/simple-api-rs"]
