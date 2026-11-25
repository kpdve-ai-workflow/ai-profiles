# Instrukcje dla agentów — ai-profiles

## Cel i zakres

`ai-profiles` zarządza zestawami umiejętności i konfiguracją MCP dla projektów korzystających z OpenCode, Claude Code i Codex. Pozwala tworzyć i wersjonować profile oraz instalować, aktualizować, przełączać, sprawdzać i usuwać ich instalacje. Repozytorium jest źródłem instalatora i dostarczanych zasobów; projekt docelowy jest osobnym katalogiem i zawiera zainstalowaną konfigurację. Nie uruchamiaj operacji zmieniających projekt docelowy bez potrzeby wynikającej z zadania.

Przed zmianą sprawdź stan Git, przeczytaj instrukcje dotyczące modyfikowanego katalogu i odpowiednią dokumentację. Zachowuj niezwiązane zmiany. W razie rozbieżności sprawdź rzeczywiste zachowanie kodu i testy; nie traktuj planów z `todos/` jako dowodu wdrożenia. Instrukcje w zagnieżdżonych `AGENTS.md` dotyczą również ich podkatalogów.

## Mapa repozytorium

- `core/` — produkcyjna implementacja Rust; `core/src/bin/` zawiera punkty wejścia, `core/src/profile_management/` operacje menedżera, `core/src/mcp/` konfigurację i runtime adaptera, a `core/src/wordpress/` operacje odtwarzania WordPressa. Moduły współdzielone obejmują m.in. model, system plików, repozytorium, procesy i logowanie. `core/tests/` zawiera testy; patrz rejestracja poniżej.
- `profiles/<nazwa>/` — metadane `profile.json`, wybór umiejętności w `skills.json`, wybór serwerów w `mcp.json` i opcjonalne zasoby profilu. Dostępne profile to `dotnet`, `golang`, `react`, `wordpress`; nazwy i zawartość sprawdzaj w plikach.
- `skills/` — źródła umiejętności dostarczanych profilom; szczegóły w `skills/README.md`. Repozytoryjne umiejętności OpenCode są w `.opencode/skills/`, a `.agents/skills/` udostępnia je Codexowi przez symlinki.
- `mcp/servers/` — centralne definicje serwerów; `mcp/images/` zawiera zasoby konkretnych integracji; opcjonalne launchery mogą znajdować się w `mcp/launchers/`. Kontrakty i konfiguracja: `mcp/README.md`.
- `make/` — implementacje i pomoc celów Make; główny `Makefile` je dołącza. `tests/vm/` obsługuje izolowaną maszynę testową. `docs/` opisuje zarządzanie profilami i testy, `todos/` zawiera plany, a `changelogs/` wcześniejsze zapisy prac.
- `core/README.md` opisuje binaria i mapowanie historycznej implementacji Python na Rust. Python jest materiałem referencyjnym migracji, a nie silnikiem produkcyjnych operacji menedżera.

## Przepływ i niezmienniki

`make install CLIENT=opencode|claude|codex|all PROFILE=<profil> PROJECT=<projekt>` buduje binaria i instaluje wybrany profil. Wymagania produkcyjnego builda: Linux/Unix, Rust/Cargo 1.85+, Git i Make. `make rust-build` buduje binaria w `core/target/release/`: `ai-profile`, `ai-profile-mcp`, `restore-updraft` i `test-inventory`. Menedżer odnajduje checkout względem położenia własnego binarium; zainstalowany adapter MCP działa samodzielnie, bez checkoutu i Cargo.

Instalator przygotowuje osobny payload w `.ai-profiles/<klient>/`, natywną konfigurację klienta i stan formatu 2 w `.ai-profiles/state.json`. Każdy klient ma odrębny profil i zarządzane zasoby. `local.json` danego klienta oraz sekrety należą do użytkownika: nie zastępuj ich ani nie usuwaj. Zarządzane pliki i wpisy podlegają kontroli własności/hashów i konfliktów; nie obchodź jej. Status konfiguracji MCP nie dowodzi połączenia z serwerem.

`CLIENT=all` działa kolejno dla klientów, bez transakcji między nimi: późniejszy błąd nie cofa wcześniejszych sukcesów. Starszy format stanu i współdzielony payload pozostają zasobami zgodności; szczegóły migracji i pierwszeństwa stanu opisuje `docs/profile-management.md`. Zmieniając lifecycle lub format, sprawdź izolację klientów, zachowanie plików użytkownika, konflikty i częściowe awarie.

Menedżer łączy definicje MCP profilu z lokalnymi nadpisaniami i generuje konfigurację klienta. Lokalny adapter uruchamia proces serwera i rozwiązuje środowisko przy starcie; zdalne serwery uruchamia natywnie klient. Nie umieszczaj sekretów w repozytorium, logach ani wygenerowanych przykładach. Integracje Playwright i Novamira mają dodatkowe wymagania i granice opisane w `mcp/README.md` oraz `profiles/wordpress/README.md`; nie zakładaj, że instalacja profilu buduje obrazy Dockera lub konfiguruje OAuth.

## Przydatne polecenia

Uruchamiaj Make z katalogu głównego. `make help` pokazuje aktualne cele. Przykłady:

```sh
make install CLIENT=codex PROFILE=react PROJECT=/ścieżka/do/projektu
make status PROJECT=/ścieżka/do/projektu
make update CLIENT=codex PROJECT=/ścieżka/do/projektu
make switch CLIENT=codex PROFILE=golang PROJECT=/ścieżka/do/projektu
make uninstall CLIENT=codex PROJECT=/ścieżka/do/projektu
make validate
make validate PROFILE=react
```

`install`, `update`, `switch` i `uninstall` wymagają `CLIENT` i `PROJECT`; install i switch wymagają też `PROFILE`. Status bez `CLIENT` pokazuje wszystkich klientów. `make validate` sprawdza źródła, a wariant z `PROFILE` ma węższy zakres. Te operacje produkcyjne działają lokalnie; nie są zastępstwem wymaganych testów w VM. Dodatkowe operacje, wersjonowanie i migrację sprawdzaj w `make help` i `docs/profile-management.md`.

## Obowiązkowa polityka testowania

- Przed projektowaniem, dodawaniem, modyfikowaniem lub oceną testów przeczytaj [docs/testing.md](docs/testing.md) i stosuj metodykę `test-gap-discovery`; wczytaj skill, jeśli jest dostępny. Gdy jest niedostępny, zgłoś to i nadal stosuj metodykę opisaną w tym dokumencie, zamiast zastępować ją dowolnym podejściem.
- Zmiana zachowania aplikacji wymaga adekwatnej ochrony testowej i wykonania wymaganej weryfikacji zgodnie z `docs/testing.md`. Obowiązuje to również bez Brain i handoffu.
- Brak środowiska, uprawnień lub możliwości wykonania wymaganych testów blokuje weryfikację; nie oznacza sukcesu. Zatrzymaj pracę i zgłoś blokadę, zachowując informację o wykonanych zmianach.
- Nie osłabiaj testów, nie pomijaj wymaganych kontroli ani nie obchodź izolacji, aby zakończyć zadanie. Wymagana VM dotyczy testów Rust i integracji Playwright, nie lokalnych operacji produkcyjnych, takich jak instalacja profilu.

Dokument testowy określa test-first dla nowego lub naprawianego zachowania, źródła niezależnych oczekiwań i wymaganą bramkę: dla `core` `make validate-rust-migration`, dla integracji/obrazu Playwright `make test-playwright`, dla obu `make test-all`, a dla orkiestracji VM dodatkowo `bash tests/vm/check.sh` i `make test-all`. Zmiany wyłącznie dokumentacji wymagają kontroli treści, odnośników i spójności. Nie powtarzaj kontroli zawartych już w szerszej bramce bez powodu. Testy Rust i integracji Playwright wykonuj w opisanej w dokumencie trwałej VM Debian 13 Incus, bez lokalnego fallbacku.

`core/Cargo.toml` ustawia `autotests = false`: nowy plik w `core/tests/` musi być odpowiednio zarejestrowany w `core/src/lib.rs`, a jego wykonanie potwierdzone w wynikach. W podsumowaniu podaj rzeczywiście wykonane kontrole, wyniki i ograniczenia; oddziel zmianę kodu od jej weryfikacji.
