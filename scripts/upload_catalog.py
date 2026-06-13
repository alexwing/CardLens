#!/usr/bin/env python3
"""Sube el catalogo de imagenes de cartas al hosting (cardlens.mappuzzle.xyz) por FTPS.

Las credenciales se leen del ENTORNO o de un fichero `.env.upload` (gitignored)
en la raiz del repo. NUNCA se escribe la contrasena en este script ni se sube a git.

Uso (desde la raiz del repo):
    python scripts/upload_catalog.py --test       # sube un fichero de prueba y para
    python scripts/upload_catalog.py --limit 50    # sube solo las primeras 50 (prueba)
    python scripts/upload_catalog.py               # sube las imagenes que falten

Variables (en .env.upload o en el entorno):
    CARDLENS_FTP_HOST    (p. ej. 191.96.63.58)
    CARDLENS_FTP_USER
    CARDLENS_FTP_PASS
    CARDLENS_FTP_PORT    (default: 21)
    CARDLENS_FTP_REMOTE  (default: public_html/catalog)
    CARDLENS_FTP_PLAIN   (=1 para FTP sin cifrar; por defecto intenta FTPS)
    CARDLENS_LOCAL_IMAGES (default: <repo>/data/images)
"""
from __future__ import annotations

import argparse
import io
import os
import ssl
import sys
from ftplib import FTP, FTP_TLS, error_perm
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
PUBLIC_HOST = "cardlens.mappuzzle.xyz"


def load_env_file() -> None:
    """Carga .env.upload (KEY=VALUE) si existe, sin pisar variables ya definidas."""
    env_file = REPO_ROOT / ".env.upload"
    if not env_file.exists():
        return
    for line in env_file.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        os.environ.setdefault(key.strip(), value.strip())


def connect() -> FTP:
    host = os.environ.get("CARDLENS_FTP_HOST")
    user = os.environ.get("CARDLENS_FTP_USER")
    password = os.environ.get("CARDLENS_FTP_PASS")
    port = int(os.environ.get("CARDLENS_FTP_PORT", "21"))
    if not host or not user or not password:
        sys.exit(
            "Faltan credenciales. Crea .env.upload (copia .env.upload.example) "
            "o define CARDLENS_FTP_HOST/USER/PASS en el entorno."
        )

    if os.environ.get("CARDLENS_FTP_PLAIN") == "1":
        print("AVISO: conexion FTP SIN CIFRAR (texto plano).")
        ftp: FTP = FTP()
        ftp.connect(host, port, timeout=30)
        ftp.login(user, password)
        return ftp

    # FTPS explicito (AUTH TLS). Contexto sin verificar el certificado porque el
    # host suele darse como IP: cifra el trafico en transito (evita sniffing de la
    # contrasena) aunque no valide el certificado del servidor.
    context = ssl._create_unverified_context()
    tls = FTP_TLS(context=context)
    tls.connect(host, port, timeout=30)
    tls.login(user, password)
    tls.prot_p()  # cifra tambien el canal de datos
    return tls


def ensure_remote_dir(ftp: FTP, remote_dir: str) -> None:
    """Entra en remote_dir creando los componentes que falten."""
    for part in remote_dir.strip("/").split("/"):
        if not part:
            continue
        try:
            ftp.cwd(part)
        except error_perm:
            ftp.mkd(part)
            ftp.cwd(part)


def public_url(remote_dir: str, name: str) -> str:
    sub = remote_dir.replace("public_html", "").strip("/")
    path = f"{sub}/{name}" if sub else name
    return f"https://{PUBLIC_HOST}/{path}"


def main() -> int:
    load_env_file()
    parser = argparse.ArgumentParser(description="Sube el catalogo de imagenes por FTPS.")
    parser.add_argument("--test", action="store_true", help="sube un fichero de prueba y termina")
    parser.add_argument("--limit", type=int, default=0, help="sube como mucho N imagenes")
    args = parser.parse_args()

    remote_dir = os.environ.get("CARDLENS_FTP_REMOTE", "public_html/catalog")
    images_dir = Path(
        os.environ.get("CARDLENS_LOCAL_IMAGES", REPO_ROOT / "data" / "images")
    )

    ftp = connect()
    print("Conectado:", ftp.getwelcome())
    ensure_remote_dir(ftp, remote_dir)

    if args.test:
        name = "_cardlens_test.txt"
        ftp.storbinary(f"STOR {name}", io.BytesIO(b"cardlens ftps ok\n"))
        print("Fichero de prueba subido. Compruebalo en el navegador:")
        print("  " + public_url(remote_dir, name))
        print("(borralo luego desde el administrador de archivos de Hostinger).")
        ftp.quit()
        return 0

    existing: set[str] = set()
    try:
        existing = set(ftp.nlst())
    except error_perm:
        pass

    pngs = sorted(images_dir.glob("*.png"))
    if args.limit:
        pngs = pngs[: args.limit]
    todo = [p for p in pngs if p.name not in existing]
    print(
        f"{len(pngs)} imagenes locales | {len(existing)} ya en el servidor | "
        f"{len(todo)} por subir."
    )

    done = 0
    for path in todo:
        with path.open("rb") as handle:
            ftp.storbinary(f"STOR {path.name}", handle)
        done += 1
        if done % 100 == 0 or done == len(todo):
            print(f"  subidas {done}/{len(todo)}")
    print(f"Listo: {done} imagenes nuevas subidas a {remote_dir}.")
    ftp.quit()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
