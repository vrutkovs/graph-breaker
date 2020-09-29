FROM registry.access.redhat.com/devtools/rust-toolset-rhel7:1.43.1 as builder

WORKDIR /opt/app-root/src/
COPY . .
RUN bash -c "source /opt/app-root/etc/scl_enable && cargo build --release"

FROM centos:7

ENV RUST_LOG=actix_web=error,dkregistry=error

RUN yum update -y && \
    yum install -y openssl && \
    yum clean all

COPY --from=builder /opt/app-root/src/target/release/graph-breaker /usr/local/bin/graph-breaker

ENTRYPOINT ["/usr/local/bin/graph-breaker"]
