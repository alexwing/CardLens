#!/usr/bin/env python3
"""Sube la landing provisional (landing/index.html) a la raiz del subdominio por FTPS.

Reutiliza las credenciales de .env.upload (gitignored). Si existe el placeholder
`default.php` de Hostinger en la raiz, lo aparta a `_default.php.hostinger.bak`
para que se sirva nuestra index.html.

Uso (desde la raiz del repo):
    python scripts/upload_landing.py
"""
from __future__ import annotations

import os
import ssl
import sys
from ftplib import FTP, FTP_TLS, error_perm
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
PUBLIC_HOST = "cardlens.mappuzzle.xyz"


def load_env_file() -> None:
    env_file = REPO_ROOT / ".env.upload"
    if not env_file.exists():
        return
    for line in env_file.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            key, value = line.split("=", 1)
            os.environ.setdefault(key.strip(), value.strip())


def connect() -> FTP:
    host = os.environ.get("CARDLENS_FTP_HOST")
    user = os.environ.get("CARDLENS_FTP_USER")
    password = os.environ.get("CARDLENS_FTP_PASS")
    port = int(os.environ.get("CARDLENS_FTP_PORT", "21"))
    if not host or not user or not password:
        sys.exit("Faltan credenciales (define .env.upload o variables de entorno).")
    if os.environ.get("CARDLENS_FTP_PLAIN") == "1":
        print("AVISO: conexion FTP SIN CIFRAR (texto plano).")
        ftp: FTP = FTP()
        ftp.connect(host, port, timeout=30)
        ftp.login(user, password)
        return ftp
    context = ssl._create_unverified_context()
    tls = FTP_TLS(context=context)
    tls.connect(host, port, timeout=30)
    tls.login(user, password)
    tls.prot_p()
    return tls


def main() -> int:
    load_env_file()
    local = REPO_ROOT / "landing" / "index.html"
    if not local.exists():
        sys.exit(f"No existe {local}")

    ftp = connect()
    print("Conectado:", ftp.getwelcome())

    try:
        names = set(ftp.nlst())
    except error_perm:
        names = set()

    if "default.php" in names and "_default.php.hostinger.bak" not in names:
        try:
            ftp.rename("default.php", "_default.php.hostinger.bak")
            print("Placeholder default.php apartado a _default.php.hostinger.bak")
        except Exception as exc:  # noqa: BLE001
            print("Aviso: no se pudo renombrar default.php:", exc)

    with local.open("rb") as handle:
        ftp.storbinary("STOR index.html", handle)
    print(f"Subida la landing -> https://{PUBLIC_HOST}/")
    ftp.quit()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
