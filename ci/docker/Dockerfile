#
# This Dockerfile is used to provide a binary-only container that other
# Dockerfiles can `COPY` from.
#
# Use like this in your project's Dockerfile:
#
#    `COPY --from=kaspar030/git-cache:<version>-<codename>` /git-cache /usr/bin
#
# `version`: every semver combination should work (0, 0.1, 0.1.17, ...)
# `codename` is either one of "buster" and "bullseye", or one of "focal" and "jammy",
# or "debian-latest", or "ubuntu-latest".
#
ARG DEBIAN_FROM=ubuntu:latest
FROM ${DEBIAN_FROM} AS builder

# docker needs this here *again*
ARG DEBIAN_FROM

LABEL maintainer="Kaspar Schleiser <kaspar@schleiser.de>"

ENV DEBIAN_FRONTEND noninteractive

# install rust if not available.
RUN if test -z "$(command -v cargo)"; then apt-get update && apt-get -y install build-essential curl ; fi
RUN if test -z "$(command -v cargo)"; then curl https://sh.rustup.rs -sSf | sh -s -- -y ; fi

# rust:buster ships 1.79, bump that
RUN if test "${DEBIAN_FROM}" = "rust:buster"; then rustup default 1.82 ; fi

# this works on both debian and ubuntu
ENV PATH="/root/.cargo/bin:${PATH}"

# override this to e.g., "--git https://github.com/kaspar030/git-cache --rev foo"
ARG SOURCE=git-cache

# build git-cache
RUN cargo install ${SOURCE}
RUN cp $(command -v git-cache) /

FROM scratch

COPY --from=builder /git-cache /
