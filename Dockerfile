# syntax=docker/dockerfile:1

# --- Build ---------------------------------------------------------------------------------
FROM rust:1.98-bookworm AS build
WORKDIR /src

# Dependencies are built in a separate layer (cached as long as Cargo.toml/Cargo.lock,
# build.rs and the embedded Typst packages do not change).
COPY Cargo.toml Cargo.lock build.rs ./
COPY packages ./packages
RUN mkdir -p src benches \
    && echo 'fn main() {}' > src/main.rs \
    && touch src/lib.rs \
    && echo 'fn main() {}' > benches/render.rs \
    && cargo build --profile dist --locked \
    && rm -rf src benches

COPY src ./src
COPY benches ./benches
RUN touch src/main.rs src/lib.rs && cargo build --profile dist --locked

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
