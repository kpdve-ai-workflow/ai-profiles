# ai-profiles

Repozytorium lokalnych profili OpenCode. Umożliwia instalowanie, aktualizowanie,
przełączanie i walidowanie zestawów umiejętności oraz konfiguracji dla projektów.
Wymagania: Rust/Cargo 1.85+, Git i Make na Linux/Unix. Instalator buduje
natywne narzędzia Rust; Python nie jest wymagany. Zainstalowane runtime nie
potrzebują Cargo ani dostępu do repozytorium.

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

Bez Make zbuduj binaria przez `make rust-build`, a następnie używaj
`core/rust/target/release/ai-profile help`.
Po instalacji, aktualizacji lub zmianie profilu zrestartuj OpenCode.

## Dokumentacja

- [Zarządzanie profilami, wersjami i instalacjami](docs/profile-management.md)
- [MCP: rejestr, konfiguracja lokalna, adapter i launchery](mcp/README.md)
- [Instrukcja profilu WordPress](profiles/wordpress/README.md)

Każdy profil może dostarczać własny `.opencode/ai-profile/README.md` z instrukcjami
projektowymi. README profilu WordPress jest samodzielną instrukcją instalowaną
wraz z profilem.
