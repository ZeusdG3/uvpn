#!/bin/bash
set -e

echo "=== Entrypoint: Directorio actual = $(pwd) ==="
echo "Contenido de /usr/src/app:"
ls -la /usr/src/app

# Verificar que el binario existe
if [ ! -f /usr/src/app/algoritmo_distribuido ]; then
    echo "ERROR: Binario /usr/src/app/algoritmo_distribuido no encontrado"
    exit 1
fi

# Aplicar reglas tc solo si WORKER_ID está definido y no vacío
if [ -n "$WORKER_ID" ]; then
    echo "Aplicando reglas tc para worker $WORKER_ID"
    sleep 2
    case "$WORKER_ID" in
        1) tc qdisc add dev eth0 root netem delay 120ms 30ms loss 2% rate 20Mbit ;;
        2) tc qdisc add dev eth0 root netem delay 60ms 10ms rate 50Mbit ;;
        3) tc qdisc add dev eth0 root netem delay 30ms 5ms loss 1% rate 30Mbit ;;
        4) tc qdisc add dev eth0 root netem delay 80ms 40ms rate 10Mbit ;;
        *) tc qdisc add dev eth0 root netem delay 50ms 10ms rate 100Mbit ;;
    esac
    echo "Reglas tc aplicadas:"
    tc qdisc show dev eth0
else
    echo "No se aplican reglas tc (WORKER_ID no definido)"
fi

echo "Ejecutando: $@"
exec "$@"