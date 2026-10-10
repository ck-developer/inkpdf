# syntax=docker/dockerfile:1

# --- Build ---------------------------------------------------------------------------------
FROM rust:1.98-bookworm AS build
WORKDIR /src

# Dependencies are built in a separate layer, cached as long as the manifests, build.rs and
# the embedded Typst packages do not change. `xtask` is a workspace member that is not part of
# the image: only its manifest is copied, with an empty main.
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY crates/inkpdf/Cargo.toml crates/inkpdf/build.rs ./crates/inkpdf/
COPY xtask/Cargo.toml ./xtask/
COPY packages ./packages
RUN mkdir -p crates/inkpdf/src crates/inkpdf/benches xtask/src \
    && echo 'fn main() {}' > crates/inkpdf/src/main.rs \
    && touch crates/inkpdf/src/lib.rs \
    && echo 'fn main() {}' > crates/inkpdf/benches/render.rs \
    && echo 'fn main() {}' > xtask/src/main.rs \
    && cargo build --profile dist --locked -p inkpdf \
    && rm -rf crates/inkpdf/src crates/inkpdf/benches

COPY crates/inkpdf/src ./crates/inkpdf/src
COPY crates/inkpdf/benches ./crates/inkpdf/benches
RUN touch crates/inkpdf/src/main.rs crates/inkpdf/src/lib.rs \
    && cargo build --profile dist --locked -p inkpdf

# --- Runtime -------------------------------------------------------------------------------
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=build /src/target/dist/inkpdf /usr/local/bin/inkpdf

ENV INKPDF_TEMPLATES_DIR=/templates
VOLUME /templates
EXPOSE 3000
USER nonroot

# The image has neither a shell nor curl: the binary queries /health itself.
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/inkpdf", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/inkpdf"]
