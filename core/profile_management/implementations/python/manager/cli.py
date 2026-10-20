from __future__ import annotations
import argparse
import sys
from pathlib import Path
from core.profile_management.implementations.python.runtime import mcp_runtime
from core.profile_management.implementations.python.manager.context import RepositoryContext
from core.profile_management.implementations.python.manager.errors import ProfileError
from core.profile_management.implementations.python.manager.infrastructure.repository import profile_names
from core.profile_management.implementations.python.manager.actions.profiles import create_profile, bump_profile
from core.profile_management.implementations.python.manager.actions.validation import validate_sources
from core.profile_management.implementations.python.manager.infrastructure.project import selected_project, state_for
from core.profile_management.implementations.python.manager.actions.installation import apply_profile, uninstall
from core.profile_management.implementations.python.manager.actions.migration import migrate_legacy
from core.profile_management.implementations.python.manager.actions.status import status


COMMAND_HELP = """ai-profiles — zarządzanie profilami

TWORZENIE PROFILU
  make create PROFILE=NAZWA
    Tworzy profil 1.0.0 z pustym MCP i katalogiem skilli. Nie nadpisuje plików.
    Opcjonalny opis: DESCRIPTION="Opis profilu"

PRACA Z PROJEKTEM
  make install PROFILE=NAZWA PROJECT=SCIEZKA_DO_PROJEKTU
    Instaluje profil w projekcie, który jeszcze go nie ma.
  make switch PROFILE=NAZWA PROJECT=SCIEZKA_DO_PROJEKTU
    Zmienia zainstalowany profil na inny.
  make status PROJECT=SCIEZKA_DO_PROJEKTU
    Pokazuje wersje i lokalne zmiany.
  make update PROJECT=SCIEZKA_DO_PROJEKTU
    Aktualizuje profil z lokalnych źródeł, bez pobierania zmian z sieci.
  make pull-update PROJECT=SCIEZKA_DO_PROJEKTU
    Pobiera źródła przez git pull --ff-only, potem aktualizuje profil.
  make uninstall PROJECT=SCIEZKA_DO_PROJEKTU
    Usuwa profil, zachowując własne pliki i ustawienia użytkownika.

UTRZYMANIE REPOZYTORIUM
  make bump PROFILE=NAZWA
    Zwiększa wersję profilu. Opcjonalnie: PART=patch|minor|major (domyślnie patch).
  make validate
    Sprawdza wszystkie profile, skille i konfigurację MCP.
  make validate PROFILE=NAZWA
    Sprawdza tylko wybrany profil, wspólne skille i jego wpisy w mcp/example.json.
    Cały rejestr MCP i nadmiarowe wpisy sprawdza pełne make validate.
  make test
    Uruchamia testy w tymczasowych kopiach repozytorium.
  make help
    Pokazuje pomoc. Samo make również pokazuje pomoc.

ARGUMENTY
  PROFILE      Wymagany tam, gdzie podano. Nie ma domyślnego profilu.
               create: nowa nazwa, małe litery/cyfry i pojedyncze myślniki.
               install/switch/bump/validate: nazwa istniejącego profilu.
               validate: PROFILE jest opcjonalny; bez niego sprawdza wszystko.
  PROJECT      Wymagany tam, gdzie podano: istniejący katalog poza ai-profiles.
  DESCRIPTION  Opcjonalny opis dla create; domyślnie „Profil NAZWA”.
  PART         Opcjonalny rodzaj zmiany wersji dla bump.

"""


class ProfileArgumentParser(argparse.ArgumentParser):
    def format_help(self) -> str:
        return COMMAND_HELP


def main(argv: list[str] | None = None, *, context: RepositoryContext | None = None) -> int:
    parser = ProfileArgumentParser(
        description="Zarządzanie profilami OpenCode z lokalnego ai-profiles",
        usage="%(prog)s KOMENDA [NAZWA] [opcje] (pomoc: make help)")
    parser.add_argument("action", nargs="?", choices=["create", "help", "install", "update", "uninstall", "switch", "status", "validate", "migrate-legacy", "bump"], help="Komenda (domyślnie help)")
    parser.add_argument("profile", nargs="?", help="Nazwa profilu; wymagana przez create/install/switch/bump")
    parser.add_argument("--project", type=Path, help="Katalog projektu (domyślnie bieżący katalog)")
    parser.add_argument("--accept-existing", action="store_true", help="Świadomie przejmij pliki starego instalatora")
    parser.add_argument("--part", choices=["major", "minor", "patch"], help="Składnik wersji zwiększany przez bump (domyślnie patch)")
    parser.add_argument("--description", help="Opis nowego profilu (tylko create)")
    opts = parser.parse_args(argv)
    context = context or RepositoryContext(Path(__file__).resolve().parents[5])
    if opts.part is not None and opts.action != "bump":
        parser.error("--part jest dostępne tylko dla bump")
    if opts.description is not None and opts.action != "create":
        parser.error("--description jest dostępne tylko dla create")
    if opts.profile is not None and opts.action not in ("create", "install", "switch", "bump", "validate"):
        parser.error("Ta komenda nie przyjmuje nazwy profilu")
    if opts.project is not None and opts.action in (None, "help", "create", "bump", "validate"):
        parser.error("Ta komenda nie używa --project")
    if opts.accept_existing and opts.action != "migrate-legacy":
        parser.error("--accept-existing jest dostępne tylko dla migrate-legacy")
    try:
        if opts.action in (None, "help"):
            parser.print_help()
            return 0
        if opts.action in ("create", "install", "switch", "bump") and not opts.profile:
            parser.error(f"{opts.action} wymaga nazwy profilu")
        if opts.action in ("install", "switch", "bump") and opts.profile not in profile_names(context, ):
            parser.error(f"Nieznany profil: {opts.profile}")
        if opts.action == "create":
            create_profile(context, opts.profile, opts.description)
            return 0
        if opts.action == "bump":
            if not opts.profile or opts.project is not None or opts.accept_existing:
                parser.error("bump wymaga nazwy profilu źródłowego; nie używa --project ani --accept-existing")
            bump_profile(context, opts.profile, opts.part or "patch")
            return 0
        if opts.action == "validate":
            validate_sources(context, opts.profile)
            return 0
        project = selected_project(context, opts.project)
        if opts.action in ("install", "switch"):
            if not opts.profile:
                parser.error("install/switch wymagają nazwy profilu")
            old = state_for(context, project)
            if opts.action == "install" and old:
                raise ProfileError("Profil już zainstalowany; użyj update albo switch")
            if opts.action == "switch" and not old:
                raise ProfileError("Brak profilu do zmiany; użyj install")
            apply_profile(context, project, opts.profile)
        elif opts.action == "update":
            old = state_for(context, project)
            if not old:
                raise ProfileError("Brak zainstalowanego profilu. Użyj install")
            apply_profile(context, project, old["profile"])
        elif opts.action == "uninstall":
            uninstall(context, project)
        elif opts.action == "migrate-legacy":
            migrate_legacy(context, project, opts.accept_existing)
        else:
            status(context, project)
        return 0
    except (ProfileError, mcp_runtime.ConfigError, OSError, ValueError, TypeError, KeyError) as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1
