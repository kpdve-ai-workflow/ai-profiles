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
make install CLIENT=opencode PROFILE=backend-go PROJECT=~/projects/my-api
make install-codex PROFILE=react PROJECT=~/projects/my-api
make status PROJECT=~/projects/my-api
make update CLIENT=codex PROJECT=~/projects/my-api
make validate
make test
```

Bez Make zbuduj binaria przez `make rust-build`, a następnie używaj
`core/rust/target/release/ai-profile help`.
Po instalacji, aktualizacji lub zmianie profilu uruchom ponownie narzędzie,
w którym korzystasz z profilu.

Wybrane skille są kopiowane niezależnie do
`.ai-profiles/<klient>/plugin/skills/`. OpenCode otrzymuje jawne źródło skills;
repozytorium nie instaluje kopii pod `.agents/skills/` ani `.claude/skills/`.
Wtyczki Claude Code i Codex wymagają natywnej rejestracji i zaufania projektu.
Profile przygotowują wybrane MCP tylko dla wskazanego klienta
na poziomie projektu. Klienci mogą wymagać restartu, zaufania projektu,
zatwierdzenia MCP lub osobnego logowania OAuth. Status potwierdza przygotowanie
konfiguracji, nie połączenie z serwerem.

`CLIENT=opencode|claude|codex|all` jest wymagany przy install/update/switch/uninstall
i pull-update. Aliasy: `install-opencode`, `install-claude`, `install-codex`.
`status` bez CLIENT pokazuje wszystkie trzy integracje, z CLIENT tylko wybraną.
Każdy klient może mieć inny profil i wersję; operacja dla jednego nie zmienia
plików ani rekordu pozostałych. Stan formatu 2: `.ai-profiles/state.json`;
lokalne nadpisania MCP: `.ai-profiles/<klient>/local.json` (plik użytkownika).

Profil WordPress domyślnie włącza Novamirę i przygotowuje pusty szablon
`.env.novamira` z prawami `0600`, bez nadpisywania istniejącego pliku.
Uzupełnij endpoint, login WordPressa i **hasło aplikacyjne** (nie hasło wp-admin),
potem zrestartuj klienta/MCP. Instalator nie dodaje reguły `.gitignore`:
samodzielnie zabezpiecz plik przed commitem i dostępem przez HTTP.

`install CLIENT=all` dodaje brakujących klientów, pomija zgodne profile i odrzuca
konflikt profilu przed zapisem. Pozostałe operacje `all` dotyczą tylko
zainstalowanych klientów; brak instalacji jest błędem. Kolejność: OpenCode,
Claude, Codex. Nie ma transakcji między klientami: przygotowany stan i sukcesy
pozostają zapisane także przy późniejszym błędzie aktywacji.

Starszy stan formatu 1 jest rozpoznawany bez zapisu podczas statusu. Dopiero
update/switch wybranego klienta przenosi jego konfigurację do osobnego payloadu.
Stary stan `.opencode/ai-profile-state.json` i współdzielony payload pozostają
zamrożonymi zasobami kompatybilności — nie są automatycznie usuwane, również
po odinstalowaniu. Format 2 ma pierwszeństwo, także z pustą mapą klientów.
Instalator nie instaluje aplikacji klientów, nie uruchamia Dockera ani OAuth.
Współistnienie konfiguracji nie oznacza bezpiecznej równoległej edycji plików
aplikacji przez agentów ani obsługi równoległych instalatorów.

### Pozostałe instrukcje

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
