FROM alpine:edge

RUN apk update && apk add tor

COPY ./docker/tor/torrc /etc/tor/torrc

EXPOSE 9080
USER tor

CMD ["/usr/bin/tor"]
