# Etapa de compilación
FROM rust:latest AS builder

WORKDIR /usr/src/app

# Copiar todo el contenido de la carpeta rust (Cargo.toml y src/)
COPY rust/ .

# Compilar en modo release
RUN cargo build --release

# Imagen final
FROM debian:bookworm-slim

# Instalar iproute2 para tc y netcat-openbsd para healthchecks (opcional)
RUN apt-get update && apt-get install -y iproute2 netcat-openbsd && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copiar el binario compilado
COPY --from=builder /usr/src/app/target/release/algoritmo_distribuido .

# Copiar el script de entrada
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["./algoritmo_distribuido"]