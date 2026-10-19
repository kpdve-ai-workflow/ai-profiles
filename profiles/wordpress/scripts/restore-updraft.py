#!/usr/bin/env python3
"""Import a trusted, complete Updraft SQL backup into an existing local DDEV WP."""
from __future__ import annotations

import argparse
import fcntl
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit

SCRIPTS = ".opencode/ai-profile/scripts"
CONTAINER_ROOT = "/var/www/html"


class RestoreError(Exception):
    pass


def inside(project: Path, relative: str) -> Path:
    if Path(relative).is_absolute() or ".." in Path(relative).parts:
        raise RestoreError("Ścieżka musi być wewnątrz projektu")
    path = project
    for part in Path(relative).parts:
        path = path / part
        if path.is_symlink():
            raise RestoreError("Odmowa zapisu/odczytu narzędzi przez symlink w projekcie")
    return path


def origin(value: str) -> str:
    parsed = urlsplit(value)
    if (parsed.scheme not in ("http", "https") or not parsed.hostname
            or parsed.username or parsed.password or parsed.query or parsed.fragment
            or parsed.path not in ("", "/")
            or not re.fullmatch(r"[A-Za-z0-9.-]+(?::[0-9]+)?", parsed.netloc)):
        raise RestoreError("Mapowanie wymaga origin URL, np. https://example.pl (bez ścieżki)")
    return f"{parsed.scheme}://{parsed.netloc.lower()}"


def mappings(values: list[str]) -> dict[str, str]:
    result = {}
    hosts = {}
    for value in values:
        old, separator, new = value.partition("=")
        if not separator:
            raise RestoreError("Użyj --map https://zrodlo.pl=https://projekt.ddev.site")
        old, new = origin(old), origin(new)
        if old in result or old == new:
            raise RestoreError("Powielone lub identyczne mapowanie domen")
        source_host, target_host = urlsplit(old).netloc, urlsplit(new).netloc
        if source_host in hosts and hosts[source_host] != target_host:
            raise RestoreError("Jeden host źródłowy nie może wskazywać różnych hostów lokalnych")
        result[old] = new
        hosts[source_host] = target_host
    if not result or any(old in new for old in result for new in result.values()):
        raise RestoreError("Wymagane są niełańcuchowe, jawne mapowania domen")
    if len(set(hosts.values())) != len(hosts):
        raise RestoreError("Nie łącz różnych witryn pod jednym hostem w tym importerze")
    return dict(sorted(result.items(), key=lambda item: -len(item[0])))


def mapped_url(value: str, mapping: dict[str, str]) -> str:
    parsed = urlsplit(value)
    source = f"{parsed.scheme}://{parsed.netloc}"
    if source not in mapping:
        raise RestoreError("Backup zawiera URL witryny bez jawnego mapowania; baza wymaga odtworzenia lub dokończenia")
    return mapping[source] + value[len(source):]


def inspect_dump(path: Path) -> dict:
    if not path.is_file() or path.stat().st_size == 0:
        raise RestoreError("Brak lub pusty plik backupu")
    with path.open("rb") as source:
        compressed = source.read(2) == b"\x1f\x8b"
    if path.suffix.lower() not in (".sql", ".gz"):
        raise RestoreError("Obsługiwane są tylko nieszyfrowane .sql lub .gz, nie ZIP/.crypt")
    tables = set()
    reader = gzip.open if compressed else open
    with reader(path, "rb") as source:
        for line in source:
            if re.match(rb"\s*(?:USE\s|(?:CREATE|DROP)\s+DATABASE\b|GRANT\s|CREATE\s+USER\b|SOURCE\s|\\!)", line, re.I):
                raise RestoreError("Dump zawiera niedozwolone dyrektywy baz/użytkowników lub klienta SQL")
            match = re.match(rb"\s*CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?`([A-Za-z0-9_]+)`", line, re.I)
            if match:
                tables.add(match[1].decode("ascii"))
    prefixes = [table[:-7] for table in tables if table.endswith("options")
                and all(table[:-7] + suffix in tables for suffix in ("posts", "users", "usermeta"))]
    if len(prefixes) != 1:
        raise RestoreError("Nie rozpoznano jednej kompletnej bazy WP; sprawdź pełny dump i prefiks")
    prefix = prefixes[0]
    multisite = prefix + "blogs" in tables
    if multisite and not {prefix + "site", prefix + "sitemeta"} <= tables:
        raise RestoreError("Niekompletny backup multisite")
    return {"prefix": prefix, "multisite": multisite, "gzip": compressed, "sha256": file_hash(path)}


def file_hash(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


class Ddev:
    def __init__(self, project: Path):
        self.project = project
        self.docroot = ""

    def run(self, args: list[str]) -> str:
        result = subprocess.run(["ddev", *args], cwd=self.project, stdin=subprocess.DEVNULL,
                                capture_output=True, text=True)
        if result.returncode:
            # SQL dumps, plugin errors and CLI diagnostics can contain secrets.
            raise RestoreError(f"Błąd kroku DDEV: {args[0]}; przerwano (szczegóły nie są wypisywane z uwagi na sekrety)")
        if "search-replace" in args and re.search(r"\b(?:warning|error)\b", result.stderr, re.I):
            raise RestoreError("WP-CLI zgłosił ostrzeżenie/błąd search-replace; nie uznaję częściowej zamiany za sukces")
        return result.stdout.strip()

    def describe(self) -> dict:
        text = self.run(["describe", "--json-output"])
        try:
            messages = [json.loads(text)]
        except ValueError:
            messages = [json.loads(line) for line in text.splitlines() if line.startswith("{")]
        for message in reversed(messages):
            value = message.get("raw", message)
            if isinstance(value, dict) and "approot" in value:
                return value
        raise RestoreError("Nie rozpoznano JSON ddev describe")

    def wp(self, *args: str) -> str:
        directory = CONTAINER_ROOT + ("/" + self.docroot if self.docroot else "")
        return self.run(["exec", "--dir=" + directory, "--raw", "--", "wp",
                         "--require=" + CONTAINER_ROOT + "/" + SCRIPTS + "/local-guard.php",
                         "--skip-plugins", "--skip-themes", *args])

    def query(self, sql: str) -> list[list[str]]:
        text = self.wp("db", "query", sql, "--batch", "--skip-column-names")
        return [line.split("\t") for line in text.splitlines() if line]


def literal(value: str) -> str:
    # Hex literals avoid SQL-mode-dependent escaping entirely.
    return "CONVERT(0x" + value.encode("utf-8").hex() + " USING utf8mb4)"


def topology(ddev: Ddev, prefix: str) -> dict:
    tables = {row[0] for row in ddev.query("SHOW TABLES")}
    blogs = ddev.query(f"SELECT blog_id,domain,path FROM `{prefix}blogs` ORDER BY blog_id") if prefix + "blogs" in tables else [["1", "", "/"]]
    sites = []
    for blog_id, domain, path in blogs:
        if not blog_id.isdigit():
            raise RestoreError("Nieprawidłowy identyfikator bloga")
        options = prefix + (blog_id + "_" if blog_id != "1" else "") + "options"
        rows = dict(ddev.query(f"SELECT option_name,option_value FROM `{options}` WHERE option_name IN ('home','siteurl')"))
        if set(rows) != {"home", "siteurl"}:
            raise RestoreError("Brak home/siteurl dla jednej z witryn")
        sites.append({"id": blog_id, "domain": domain, "path": path, "options": options, **rows})
    networks = ddev.query(f"SELECT id,domain,path FROM `{prefix}site` ORDER BY id") if prefix + "site" in tables else []
    return {"sites": sites, "networks": networks, "multisite": prefix + "blogs" in tables}


def preflight(project: Path, dump: Path, mapping: dict[str, str], ddev: Ddev) -> tuple[dict, dict, dict]:
    if not inside(project, ".ddev/config.yaml").is_file():
        raise RestoreError("Wymagany jest istniejący projekt DDEV")
    for name in ("local-guard.php", "local-safety.php"):
        if not inside(project, SCRIPTS + "/" + name).is_file():
            raise RestoreError("Najpierw zainstaluj/zaktualizuj profil WordPress w tym projekcie")
    info = ddev.describe()
    if (Path(info["approot"]).resolve() != project or info.get("status") != "running"
            or info.get("type") != "wordpress"):
        raise RestoreError("Cel musi być wskazanym, uruchomionym projektem DDEV typu wordpress")
    ddev.docroot = info.get("docroot", "")
    inside(project, ddev.docroot)
    if not inside(project, str(Path(ddev.docroot) / "wp-config.php")).is_file():
        raise RestoreError("Wersja 1 obsługuje standardowy WordPress z wp-config.php w docroot")
    if not inside(project, str(Path(ddev.docroot) / "wp-content")).is_dir():
        raise RestoreError("Brak standardowego katalogu wp-content")
    urls = info.get("urls", [])
    if not isinstance(urls, list):
        raise RestoreError("Nieprawidłowe URL w konfiguracji DDEV")
    urls = list(urls)
    urls += [info.get(key, "") for key in ("primary_url", "httpurl", "httpsurl")]
    hosts = {urlsplit(value).netloc for value in urls if isinstance(value, str) and value}
    if any(urlsplit(target).netloc not in hosts for target in mapping.values()):
        raise RestoreError("Każdy docelowy host musi występować w ddev describe; nie zmieniamy konfiguracji DDEV")
    prefix = ddev.wp("ai-profile-local-prefix")
    if not re.fullmatch(r"[A-Za-z0-9_]+", prefix):
        raise RestoreError("Nieprawidłowy prefiks tabel lokalnej bazy")
    # This query also executes the PHP DB_HOST/DB_NAME/environment guard.
    connection = ddev.query("SELECT @@hostname, DATABASE()")
    native = ddev.run(["mysql", "--database=db", "--batch", "--skip-column-names",
                       "--execute=SELECT @@hostname, DATABASE()"])
    if not connection or connection[0][-1] != "db" or "\t".join(connection[0]) != native:
        raise RestoreError("WordPress i DDEV nie wskazują tej samej lokalnej bazy db")
    current = topology(ddev, prefix)
    source = inspect_dump(dump)
    if source["prefix"] != prefix or source["multisite"] != current["multisite"]:
        raise RestoreError("Dump i lokalny WordPress muszą mieć ten sam prefiks i tryb single/multisite")
    guard = inside(project, str(Path(ddev.docroot) / "wp-content/mu-plugins/ai-profile-local-safety.php"))
    safety = inside(project, SCRIPTS + "/local-safety.php").read_bytes()
    if guard.exists() and guard.read_bytes() != safety:
        raise RestoreError("Lokalny plik ochrony jest inny; nie nadpisuję go")
    return info, source, current


def localize(ddev: Ddev, prefix: str, mapping: dict[str, str], imported: dict) -> None:
    host_map = {urlsplit(old).netloc: urlsplit(new).netloc for old, new in mapping.items()}
    # Check every imported site/network before any replacement or WP bootstrap.
    for site in imported["sites"]:
        for key in ("home", "siteurl"):
            mapped_url(site[key], mapping)
        if imported["multisite"] and site["domain"] not in host_map:
            raise RestoreError("Brak mapowania hosta bloga multisite")
    if any(domain not in host_map or not network_id.isdigit() for network_id, domain, path in imported["networks"]):
        raise RestoreError("Brak mapowania lub nieprawidłowy identyfikator sieci multisite")
    for site in imported["sites"]:
        for key in ("home", "siteurl"):
            ddev.query(f"UPDATE `{site['options']}` SET option_value={literal(mapped_url(site[key], mapping))} WHERE option_name='{key}'")
        if imported["multisite"]:
            ddev.query(f"UPDATE `{prefix}blogs` SET domain={literal(host_map[site['domain']])} WHERE blog_id={site['id']}")
    for network_id, domain, path in imported["networks"]:
        ddev.query(f"UPDATE `{prefix}site` SET domain={literal(host_map[domain])} WHERE id={network_id}")
    # Now WP can bootstrap against local multisite domains. All data replacements
    # (including serialized plugin data) are performed by WP-CLI, never raw SQL.
    target = mapped_url(imported["sites"][0]["home"], mapping)
    for old, new in mapping.items():
        args = ("--url=" + target, "search-replace", old, new, "--all-tables-with-prefix",
                "--precise", "--skip-columns=guid", "--report-changed-only", "--format=count")
        count = ddev.wp(*args)
        if not count.isdigit() or ddev.wp(*args, "--dry-run") != "0":
            raise RestoreError("Search-replace nie zastąpił wszystkich żądanych URL")
    for site in imported["sites"]:
        ddev.query(f"UPDATE `{site['options']}` SET option_value='0' WHERE option_name='blog_public'")
    final = topology(ddev, prefix)
    if len(final["sites"]) != len(imported["sites"]):
        raise RestoreError("Zmieniła się liczba blogów podczas importu")
    for before, after in zip(imported["sites"], final["sites"]):
        if (after["id"] != before["id"] or after["path"] != before["path"]
                or any(after[key] != mapped_url(before[key], mapping) for key in ("home", "siteurl"))
                or (imported["multisite"] and after["domain"] != host_map[before["domain"]])):
            raise RestoreError("Weryfikacja URL po imporcie nie powiodła się")
    if final["networks"] != [[network_id, host_map[domain], path]
                             for network_id, domain, path in imported["networks"]]:
        raise RestoreError("Weryfikacja domen sieci multisite nie powiodła się")
    ddev.wp("--url=" + target, "core", "is-installed")


def restore(project: Path, dump: Path, mapping: dict[str, str], apply: bool) -> Path | None:
    ddev = Ddev(project)
    info, source, current = preflight(project, dump, mapping, ddev)
    name = info["name"]
    if not re.fullmatch(r"[A-Za-z0-9_-]+", name):
        raise RestoreError("Nieprawidłowa nazwa projektu DDEV")
    print(f"Projekt: {name}\nKatalog: {project}\nCel: lokalna usługa DDEV db / baza db")
    print(f"Backup źródłowy: {dump}\nSHA-256: {source['sha256']}\nPrefiks: {source['prefix']}; multisite: {source['multisite']}")
    for old, new in mapping.items():
        print(f"  {old} → {new}")
    print("Plan: backup lokalnej bazy → ochrona mail/HTTP/cron → zastąpienie bazy → mapowanie URL → weryfikacja.")
    if not apply:
        print("PLAN: bez zapisu. Dodaj --apply, aby wykonać import z potwierdzeniem.")
        return None
    if input(f"Lokalna baza zostanie ZASTĄPIONA. Wpisz nazwę projektu {name}: ").strip() != name:
        raise RestoreError("Anulowano; baza nie została zmieniona")
    state_home = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")).expanduser().resolve()
    if state_home == project or project in state_home.parents:
        raise RestoreError("Katalog backupów XDG_STATE_HOME musi być poza projektem i docroot")
    backup_root = state_home / "ai-profiles" / "wordpress-backups" / name
    backup_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    # A lock prevents competing imports through this script, not external writers.
    with (backup_root / "restore.lock").open("a") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise RestoreError("Inny import tego projektu już działa") from error
        info2, source2, current2 = preflight(project, dump, mapping, ddev)
        if source2 != source or info2["name"] != name or current2 != current:
            raise RestoreError("Źródło lub cel zmieniły się po potwierdzeniu; uruchom plan ponownie")
        run_dir = Path(tempfile.mkdtemp(prefix="restore-", dir=backup_root))
        backup = run_dir / "before-import.sql.gz"
        incoming = run_dir / ("incoming.sql.gz" if source["gzip"] else "incoming.sql")
        import_started = False
        try:
            shutil.copyfile(dump, incoming)
            incoming.chmod(0o600)
            if file_hash(incoming) != source["sha256"]:
                raise RestoreError("Plik źródłowy zmienił się podczas kopiowania")
            ddev.run(["export-db", "--database=db", "--gzip", "--file=" + str(backup)])
            backup.chmod(0o600)
            verified_backup = inspect_dump(backup)  # verifies full gzip CRC before any drop
            if (verified_backup["prefix"] != source["prefix"]
                    or verified_backup["multisite"] != current["multisite"]):
                raise RestoreError("Backup lokalnej bazy nie odpowiada potwierdzonemu celowi")
            print(f"Backup lokalny: {backup}")
            safety = inside(project, str(Path(ddev.docroot) / "wp-content/mu-plugins/ai-profile-local-safety.php"))
            if not safety.parent.exists():
                safety.parent.mkdir()
                safety.parent.chmod(0o755)
            payload = inside(project, SCRIPTS + "/local-safety.php").read_bytes()
            if safety.exists() and safety.read_bytes() != payload:
                raise RestoreError("Plik ochrony zmienił się po potwierdzeniu; nie nadpisuję go")
            if not safety.exists():
                with safety.open("xb") as output:
                    output.write(payload)
                safety.chmod(0o644)  # public code must be readable by the local PHP worker
            import_started = True
            ddev.run(["import-db", "--database=db", "--file=" + str(incoming)])
            imported = topology(ddev, source["prefix"])
            if imported["multisite"] != source["multisite"]:
                raise RestoreError("Zaimportowana topologia różni się od dumpa")
            localize(ddev, source["prefix"], mapping, imported)
        except (RestoreError, KeyboardInterrupt, OSError, ValueError, IndexError, KeyError):
            if import_started:
                print("Import przerwany. Brak automatycznego rollbacku. Przywróć backup:", file=sys.stderr)
                print("cd " + shlex.quote(str(project)), file=sys.stderr)
                print("ddev import-db --database=db --file=" + shlex.quote(str(backup)), file=sys.stderr)
            raise
        finally:
            incoming.unlink(missing_ok=True)
        print("OK: baza zaimportowana i URL sprawdzone. Ochrona lokalna mail/HTTP/cron pozostaje aktywna.")
        print("Sprawdź oba hosty multisite, cache, AI Abilities Novamiry i odtwórz lokalne hasło aplikacyjne, jeśli import je zastąpił.")
        return backup


def main() -> int:
    parser = argparse.ArgumentParser(description="Lokalny import pełnej bazy UpdraftPlus do DDEV; domyślnie tylko plan.")
    parser.add_argument("dump", type=Path, help="Zaufany, pełny dump .sql lub .gz, np. z Downloads")
    parser.add_argument("--project", required=True, type=Path, help="Katalog projektu DDEV z zainstalowanym profilem wordpress")
    parser.add_argument("--map", action="append", required=True, metavar="SOURCE=TARGET", help="Jawne origin URL; powtórz dla każdego hosta/schematu")
    parser.add_argument("--apply", action="store_true", help="Wykonaj import po wpisaniu nazwy projektu; wcześniej obowiązkowy backup")
    args = parser.parse_args()
    os.umask(0o077)
    try:
        restore(args.project.expanduser().resolve(), args.dump.expanduser().resolve(), mappings(args.map), args.apply)
        return 0
    except RestoreError as error:
        print(f"BŁĄD: {error}", file=sys.stderr)
    except (OSError, ValueError, EOFError, IndexError, KeyError):
        print("BŁĄD: sprawdź plik dumpa, dostęp do DDEV/WP-CLI i lokalną konfigurację; szczegóły mogą zawierać sekrety", file=sys.stderr)
    except KeyboardInterrupt:
        print("Przerwano przez użytkownika", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
