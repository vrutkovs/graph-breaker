FROM registry.fedoraproject.org/fedora:32 as builder

RUN dnf update -y && \
    dnf install rust cargo openssl-devel -y && \
    dnf clean all

WORKDIR /code
COPY . .
RUN cargo install --path .

FROM registry.access.redhat.com/ubi8/ubi

COPY --from=builder /root/.cargo/bin/graph-breaker /usr/local/bin/graph-breaker

ENTRYPOINT ["/usr/local/bin/graph-breaker"]
