FROM registry.fedoraproject.org/fedora:32

RUN dnf update -y && \
    dnf install rust cargo openssl-devel -y && \
    dnf clean all

WORKDIR /code
COPY . .
RUN cargo build --release

ENTRYPOINT ["/code/target/release/graph-breaker"]
