# ai-profiles

Repozytorium profili dla narzędzi programistycznych i agentów AI. Umożliwia
instalowanie, aktualizowanie, przełączanie i walidowanie zestawów umiejętności
oraz konfiguracji dla projektów.
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
Po instalacji, aktualizacji lub zmianie profilu uruchom ponownie narzędzie,
w którym korzystasz z profilu.

Wybrane skille są dystrybuowane raz do
`.opencode/ai-profile/plugin/skills/`. OpenCode otrzymuje jawne źródło skills;
repozytorium nie instaluje kopii pod `.agents/skills/` ani `.claude/skills/`.
Wtyczki Claude Code i Codex wymagają natywnej rejestracji i zaufania projektu.
Profile przygotowują wybrane MCP automatycznie dla OpenCode, Codex i Claude Code
na poziomie projektu. Klienci mogą wymagać restartu, zaufania projektu,
zatwierdzenia MCP lub osobnego logowania OAuth. Status potwierdza przygotowanie
konfiguracji, nie połączenie z serwerem.

## Dokumentacja

- [Zarządzanie profilami, wersjami i instalacjami](docs/profile-management.md)
- [Testy Rust i Playwright w trwałej VM Debian 13 (Incus)](docs/testing.md)
- [MCP: rejestr, konfiguracja lokalna, adapter i launchery](mcp/README.md)
- [Instrukcja profilu WordPress](profiles/wordpress/README.md)

Każdy profil może dostarczać własny plik README z instrukcjami projektowymi.
README profilu WordPress jest samodzielną instrukcją instalowaną wraz z profilem.

Testy (`make test`, `make test-playwright`, `make validate-rust-migration`,
`make test-all`) wymagają osobno przygotowanego hosta Linux amd64 z Incus,
QEMU/KVM, siecią/pulą storage i rsync; nie wymagają hostowego Cargo, Node ani
Dockera. `make test-vm-up` przygotowuje VM (4 CPU, 8 GiB RAM, dysk 64 GiB),
`make test-vm-stop` zatrzymuje ją z zachowaniem cache. Testy zawsze wykonują się
ponownie; logi i raporty trafiają do `.test-vm/artifacts/`. Szczegóły wymagań,
zasobów i jawnego re-provisioningu są w instrukcji testowania powyżej.
