FROM debian:12-slim

RUN apt-get update && apt-get install -y ca-certificates build-essential libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*
RUN dpkg --compare-versions "$(dpkg-query -W -f='${Version}' ca-certificates)" ge 20250419~deb12u2

ARG TARGETARCH
ARG BIN
COPY bin/${BIN}-Linux-${TARGETARCH} /bin/entrypoint
RUN chmod +x /bin/entrypoint

ENTRYPOINT [ "/bin/entrypoint" ]
