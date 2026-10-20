# ai-profiles

Repozytorium lokalnych profili OpenCode. Umożliwia instalowanie, aktualizowanie,
przełączanie i walidowanie zestawów umiejętności oraz konfiguracji dla projektów.
Wymagania: Python 3.10+, Git i opcjonalnie Make.

## Szybki start

Uruchamiaj polecenia z katalogu sklonowanego repozytorium:

```bash
make help
make install PROFILE=backend-go PROJECT=~/projects/my-api
make status PROJECT=~/projects/my-api
make update PROJECT=~/projects/my-api
make validate
make test
```

Bez Make można użyć `python3 core/profile_management/implementations/python/ai-profile.py help`.
Po instalacji, aktualizacji lub zmianie profilu zrestartuj OpenCode.

## Dokumentacja

- [Zarządzanie profilami, wersjami i instalacjami](docs/profile-management.md)
- [MCP: rejestr, konfiguracja lokalna, adapter i launchery](mcp/README.md)
- [Instrukcja profilu WordPress](profiles/wordpress/README.md)

Każdy profil może dostarczać własny `.opencode/ai-profile/README.md` z instrukcjami
projektowymi. README profilu WordPress jest samodzielną instrukcją instalowaną
wraz z profilem.
