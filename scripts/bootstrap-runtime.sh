#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Rakitray — Zero-Bloat Local Runtime Seeder (v0.1.0)
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUNTIME_DIR="${ROOT_DIR}/runtime"

echo "[Rakitray] Seeding local runtime structure at: ${RUNTIME_DIR}"

# 1. Ensure core directories exist
mkdir -p "${RUNTIME_DIR}/bin/webserver/nginx/1.26.2"
mkdir -p "${RUNTIME_DIR}/bin/runtime/php/8.3.14"
mkdir -p "${RUNTIME_DIR}/bin/runtime/php/8.2.20"
mkdir -p "${RUNTIME_DIR}/bin/database/mariadb/11.4.4/bin"
mkdir -p "${RUNTIME_DIR}/config"
mkdir -p "${RUNTIME_DIR}/data/mariadb"
mkdir -p "${RUNTIME_DIR}/logs"
mkdir -p "${RUNTIME_DIR}/tmp"
mkdir -p "${RUNTIME_DIR}/www"

# 2. Helper: generate smart mock daemon if host binary is missing
create_mock_daemon() {
    local target_path="$1"
    local name="$2"
    local default_port="$3"

    cat << 'EOF' > "${target_path}"
#!/usr/bin/env python3
import sys, os, signal, time, socket, argparse

parser = argparse.ArgumentParser(description="Rakitray Mock Daemon")
parser.add_argument("-b", "--bind", default="", help="bind address")
parser.add_argument("-p", "--port", type=int, default=0, help="port number")
parser.add_argument("-c", "--config", default="", help="config file")
parser.add_argument("-g", "--global-directive", default="", help="global directive")
args, unknown = parser.parse_known_args()

name = os.environ.get("MOCK_NAME", "daemon")
port = int(os.environ.get("MOCK_PORT", "0"))

if args.port > 0:
    port = args.port
elif args.bind and ":" in args.bind:
    try:
        port = int(args.bind.split(":")[-1])
    except:
        pass

for u in unknown:
    if u.startswith("--port="):
        try:
            port = int(u.split("=")[-1])
        except:
            pass

running = True
def handle_sig(sig, frame):
    global running
    sys.stdout.write(f"[{name}] Caught SIGTERM, shutting down cleanly...\n")
    sys.stdout.flush()
    running = False

signal.signal(signal.SIGTERM, handle_sig)
signal.signal(signal.SIGINT, handle_sig)

sys.stdout.write(f"[{name}] Starting daemon (PID {os.getpid()})...\n")
sys.stdout.flush()

sock = None
if port > 0:
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(('127.0.0.1', port))
        sock.listen(10)
        sock.setblocking(False)
        sys.stdout.write(f"[{name}] Listening successfully on 127.0.0.1:{port}\n")
    except Exception as e:
        sys.stderr.write(f"[{name}] Port {port} bind notice: {e}\n")
sys.stdout.flush()

while running:
    if sock:
        try:
            conn, addr = sock.accept()
            # If HTTP port, send mock response
            try:
                conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nHello from Rakitray mock service!\n")
            except:
                pass
            conn.close()
        except BlockingIOError:
            pass
        except:
            pass
    time.sleep(0.1)

if sock:
    sock.close()
sys.stdout.write(f"[{name}] Service stopped.\n")
sys.stdout.flush()
EOF

    chmod +x "${target_path}"
}

# 3. Seed Nginx (1.26.2)
NGINX_BIN="${RUNTIME_DIR}/bin/webserver/nginx/1.26.2/nginx"
if command -v nginx >/dev/null 2>&1; then
    echo "[Rakitray] Host nginx found: $(command -v nginx)"
    cat << EOF > "${NGINX_BIN}"
#!/usr/bin/env bash
exec "$(command -v nginx)" "\$@"
EOF
    chmod +x "${NGINX_BIN}"
else
    echo "[Rakitray] Host nginx not found. Generating smart mock daemon for Nginx 1.26.2..."
    create_mock_daemon "${NGINX_BIN}" "nginx@1.26.2" "8088"
fi

# 4. Seed PHP 8.3.14
PHP83_BIN="${RUNTIME_DIR}/bin/runtime/php/8.3.14/php-cgi"
if [ -x "/opt/lampp/bin/php" ]; then
    echo "[Rakitray] Using host PHP from /opt/lampp/bin/php"
    cat << EOF > "${PHP83_BIN}"
#!/usr/bin/env bash
exec /opt/lampp/bin/php "\$@"
EOF
    chmod +x "${PHP83_BIN}"
    # Also create 'php' link
    ln -sf php-cgi "${RUNTIME_DIR}/bin/runtime/php/8.3.14/php"
else
    echo "[Rakitray] Host PHP not found. Generating smart mock daemon for PHP 8.3.14..."
    create_mock_daemon "${PHP83_BIN}" "php-cgi@8.3.14" "9000"
    ln -sf php-cgi "${RUNTIME_DIR}/bin/runtime/php/8.3.14/php"
fi

# 5. Seed PHP 8.2.20 (Second version for multi-PHP testing)
PHP82_BIN="${RUNTIME_DIR}/bin/runtime/php/8.2.20/php-cgi"
echo "[Rakitray] Generating smart mock daemon for PHP 8.2.20 (port 9002)..."
create_mock_daemon "${PHP82_BIN}" "php-cgi@8.2.20" "9002"
ln -sf php-cgi "${RUNTIME_DIR}/bin/runtime/php/8.2.20/php"

# 6. Seed MariaDB 11.4.4
MARIADB_BIN="${RUNTIME_DIR}/bin/database/mariadb/11.4.4/bin/mariadbd"
if command -v mariadbd >/dev/null 2>&1; then
    echo "[Rakitray] Host mariadbd found: $(command -v mariadbd)"
    cat << EOF > "${MARIADB_BIN}"
#!/usr/bin/env bash
exec "$(command -v mariadbd)" "\$@"
EOF
    chmod +x "${MARIADB_BIN}"
elif command -v mysqld >/dev/null 2>&1; then
    echo "[Rakitray] Host mysqld found: $(command -v mysqld)"
    cat << EOF > "${MARIADB_BIN}"
#!/usr/bin/env bash
exec "$(command -v mysqld)" "\$@"
EOF
    chmod +x "${MARIADB_BIN}"
else
    echo "[Rakitray] Host mariadb not found. Generating smart mock daemon for MariaDB 11.4.4..."
    create_mock_daemon "${MARIADB_BIN}" "mariadbd@11.4.4" "3306"
fi

echo "[Rakitray] Runtime seeder completed successfully!"

