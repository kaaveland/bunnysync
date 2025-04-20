FROM alpine

ARG TARGETARCH
COPY ./thumper-${TARGETARCH} /usr/local/bin/thumper
RUN chmod +x /usr/local/bin/thumper

RUN adduser -D thumper
USER thumper
ENTRYPOINT ["/usr/local/bin/thumper"]