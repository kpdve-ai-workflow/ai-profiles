# Python → Rust — plan małych, kolejnych etapów

## Stan i sposób pracy

**Gotowość planu: READY.** **Implementacja: T-01…T-24 oraz korekta T-24a IMPLEMENTED. T-25: PASSED po zleconych poprawkach (2026-10-21). Przygotowany zestaw: 22 testy PASSED, 0 FAILED, 0 IGNORED.**

Celem jest funkcjonalny port całego kodu produkcyjnego Python do Rust. Źródła Python pozostają wyłącznie w repozytorium jako referencja. `core/rust/` zawiera implementację oraz przygotowane, nieuruchomione testy.

**Obowiązuje ostatnia decyzja użytkownika: najpierw cała implementacja 1:1, dopiero na końcu jedno zbiorcze uruchomienie walidacji. Nie stosujemy TDD, RED/GREEN, testów pośrednich ani baseline testowego.** Jest to jawny wyjątek od test-first w skillu `make-plan` i od wcześniejszych zaleceń tej rozmowy.

Agent realizuje **jedno zadanie `T-xx` na jedno zlecenie**, zapisuje stan i zatrzymuje się. Nie przechodzi samodzielnie do następnego zadania. `IMPLEMENTED` oznacza powstanie kodu, **nie** potwierdzoną zgodność. Do T-25 weryfikacja zachowania pozostaje `NOT RUN — PROHIBITED AT THIS STAGE`.

**Następne działanie: brak kolejnego zadania w tym planie. Wynik końcowego zestawu zapisano w T-25.**

## Wymagania i decyzje użytkownika

**Najnowsza decyzja użytkownika (T-24a):** używamy `make install`, bez `make install2`. Make, manager, walidacja, testy oraz cały instalowany runtime i konfiguracja mają używać Rust. Python pozostaje wyłącznie jako źródła w repozytorium; nie instalować `.py` do projektów. Ta decyzja zastępuje wcześniejsze wymagania zgodności payloadu Python w K-03/K-05/K-06 i T-07/T-10/T-23. Zachować reguły ownership, hashy i ochrony danych. Zewnętrzne serwery MCP i PHP helpers zachowują własne runtime; upstreamowy serwer MariaDB w Dockerze nie podlega portowaniu.

| ID | Wymaganie | Pochodzenie |
| --- | --- | --- |
| BR-01 | Przenieść całą funkcjonalność produkcyjną Python do natywnego Rust, zachowując zachowanie 1:1. | USER-APPROVED — żądanie portu i ostatnie doprecyzowanie |
| BR-02 | Zachować źródła Python wyłącznie w repozytorium jako referencję; nie instalować ich do projektów. | USER-APPROVED — doprecyzowanie T-24a |
| BR-03 | Grupować implementacje według języka: Python w `core/python/`, Rust w `core/rust/`; bez `implementations/<język>/`. | USER-APPROVED — wybrana struktura repozytorium |
| BR-04 | Agent używa skilla `rust` podczas implementacji i przeglądu Rust. | USER-APPROVED |
| BR-05 | Wykonywać pracę małymi etapami, a nie jednym ogromnym refaktorem. | USER-APPROVED — zlecenie tego planu |
| BR-06 | Nie pisać testów przed implementacją i nie uruchamiać testów pośrednich; na końcu jedno pełne uruchomienie walidacji. | USER-APPROVED — ostatnia wiadomość; zastępuje wcześniejsze test-first |
| BR-07 | Nie naruszać danych projektów, lokalnej konfiguracji, własności plików ani zabezpieczeń. | APPROVED SPEC — `AGENTS.md`, dokumentacja profili i MCP |
| BR-08 | Make, manager, walidacja, test runner, instalowany payload i jego konfiguracja używają Rust. Instalacja przez `make install`, bez `install2`. Zachować źródła Python w repozytorium. | USER-APPROVED — najnowsze doprecyzowanie T-24a |

Wcześniejsze zalecenie pełnych testów przy każdym zakończeniu pracy jest **SUPERSEDED dla tej migracji**. Wcześniej wskazany flow PR `feature → develop`, `develop → main` pozostaje kontekstem CI; nie dodajemy wdrożeń ani wcześniejszych przebiegów testów. T-25 jest ostatnim, wyraźnie dozwolonym etapem pełnej walidacji.

## Ustalenia sprawdzone w repozytorium

Poniższa tabela to historyczny odczyt sprzed T-01, nie aktualny stan checkoutu.
Wpisy o braku Rust, domyślnym Python i lokalnych zmianach nie opisują obecnego stanu.

| Ustalenie | Dowód | Znaczenie |
| --- | --- | --- |
| Rust nie został jeszcze dodany. | Lista `core/` i brak plików `core/rust/**`. | Wszystkie ścieżki Rust poniżej są nowymi, zaplanowanymi artefaktami. |
| Główny launcher znajduje się w `core/python/profile_management/ai-profile.py`. | `Makefile:2`, launcher. | Domyślne Make nadal wskazuje Python. |
| CLI udostępnia `create`, `help`, `install`, `update`, `uninstall`, `switch`, `status`, `validate`, `migrate-legacy`, `bump`. | `core/python/profile_management/cli.py:68–133`. | Port nie może kończyć się na install/update. |
| Instalacja synchronizuje pliki i MCP, kontroluje hashe przed zapisem oraz zapisuje stan formatu 1. | `actions/installation.py:13–123`, `infrastructure/project.py`. | Wymagana zgodność istniejących instalacji. |
| Payload obejmuje skille, adapter, wybrane launchery, skrypty i README profilu. | `infrastructure/payload.py:12–72`. | Port menedżera nie upoważnia do zmiany dostarczanych artefaktów. |
| Adapter to także biblioteka walidacji/merge/env używana przez menedżer. | `mcp_runtime.py:26–264`, `infrastructure/mcp.py`. | Portujemy konfigurację i samodzielny runtime, nie tylko uruchomienie procesu. |
| Pozostałe produkcyjne programy Python to launcher MariaDB i restore WordPress. | `mcp/launchers/mariadb/launch.py`, `profiles/wordpress/scripts/restore-updraft.py`. | Oba należą do zakresu; PHP helpers i upstreamowy serwer MCP nie są kodem do portowania. |
| Restore domyślnie tylko planuje; apply wymaga potwierdzenia, blokady i zweryfikowanego backupu. | `restore-updraft.py:173–352`. | Zakaz uproszczenia zabezpieczeń podczas portu. |
| Testy i ich scenariusze istnieją pod `core/python/tests/`. | `test_*.py`, `runner.py`, `fixture_support.py`. | Źródło przypadków dla walidacji końcowej, nie polecenie ich bieżącego uruchamiania. |
| Nie wszystkie ścieżki po wcześniejszym przeniesieniu są poprawione. | `tests/support.py` używa `parents[4]`, `tests/test_wordpress_restore.py` i `test_mariadb_launcher.py` używają starych `parents[1]`; `cli.py:79` ma `parents[5]`, a launcher przekazuje poprawny kontekst jawnie. | Nie traktować pomyłek lokalizacji jako kontraktu Rust. Naprawy fixture'ów dopiero w T-24; nie refaktorować teraz Pythona. |
| Istnieje niezwiązana modyfikacja użytkownika. | `git status --short`: `.opencode/plugins/documentation-guard.ts`. | Nie edytować ani nie przywracać tego pliku. |

Źródła kontraktów: [profile](docs/profile-management.md), [MCP](mcp/README.md), [WordPress](profiles/wordpress/README.md). Symbole źródłowe w kolejnych zadaniach odnoszą się do `core/python/profile_management/`, chyba że podano inną ścieżkę.

## Podejście techniczne i granice

### Wiążące wybory planisty

| Wybór | Reguła | Pochodzenie |
| --- | --- | --- |
| Jeden pakiet Cargo | `core/rust/Cargo.toml`, biblioteka `ai_profiles_core`, cztery binaria główne i `test-inventory` dodane w T-24a dla wspólnego skilla. Bez workspace i osobnego crate na każdą operację. | PLANNER-SELECTED + korekta T-24a |
| Współistnienie | Make używa Rust. Python pozostaje w repozytorium jako kod referencyjny. | USER-APPROVED — BR-08/T-24a |
| Dystrybucja | `make install` buduje wszystkie binaria release. Manager kopiuje adapter MCP, wybrany launcher MariaDB, restore WordPress i inwentaryzację testów do projektu, nadając 0755. Wycofane zarządzane pliki Python usuwa po kontroli hashy. | USER-APPROVED — BR-08/T-24a |
| Małe etapy | T-01…T-23 to wyłącznie implementacja i statyczny przegląd; T-24 przygotowuje walidator po implementacji; T-25 uruchamia go zbiorczo. | PLANNER-SELECTED w wykonaniu BR-05/06 |
| Kompilacja ≠ test | Po zadaniu wolno wykonać `cargo fmt --manifest-path core/rust/Cargo.toml` i `cargo check --manifest-path core/rust/Cargo.toml --locked --lib --bins`. Nie wolno uruchamiać testów, smoke tests, programów `--help` ani operacji przykładowych. | PLANNER-SELECTED — kontrola poprawności składni bez wykonywania funkcjonalności |
| Brak zmian funkcjonalnych przy okazji | Nie poprawiać modelu profili, protokołu MCP, domyślnych uprawnień ani restore. Nie odtwarzać oczywistych błędów fixture'ów jako funkcji produktu. | PLANNER-SELECTED |

### Układ Rust

```text
core/rust/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── src/
│   ├── lib.rs
│   ├── model.rs
│   ├── error.rs
│   ├── codec.rs
│   ├── filesystem.rs
│   ├── repository.rs
│   ├── project.rs
│   ├── process.rs
│   ├── profile_management.rs
│   ├── profile_management/
│   │   ├── profiles.rs
│   │   ├── payload.rs
│   │   ├── installation.rs
│   │   ├── uninstall.rs
│   │   ├── status.rs
│   │   ├── validation.rs
│   │   ├── migration.rs
│   │   └── cli.rs
│   ├── mcp.rs
│   ├── mcp/
│   │   ├── config.rs
│   │   ├── environment.rs
│   │   └── runtime.rs
│   ├── mariadb.rs
│   ├── mariadb/
│   │   ├── archive.rs
│   │   └── runtime.rs
│   ├── wordpress.rs
│   ├── wordpress/
│   │   ├── dump.rs
│   │   ├── mapping.rs
│   │   ├── ddev.rs
│   │   ├── preflight.rs
│   │   ├── localize.rs
│   │   └── restore.rs
│   └── bin/
│       ├── ai-profile.rs
│       ├── ai-profile-mcp.rs
│       ├── mariadb-launcher.rs
│       ├── restore-updraft.rs
│       └── test-inventory.rs         # T-24a: Python osadzony w skillu
└── tests/                         # dopiero T-24
```

Pliki powstają dopiero w swoim zadaniu. Nie generować wszystkich modułów z `todo!()`, pustym sukcesem czy pozornie obsługiwanymi komendami. W T-01 powstaje kompilowalny pakiet biblioteczny; binaria dopiero po implementacji ich operacji.

Wymagany `@rust`: typy domenowe zamiast przypadkowych stringów, enumy dla zamkniętych stanów, błędy strukturalne z bezpiecznym formatowaniem na brzegu, pożyczanie danych i domyślnie prywatne moduły. Nie tworzyć publicznego API dla wszystkich plików. `serde_json::Value` jest dozwolone na granicy JSON i dla zachowywanych obcych pól konfiguracji, nie jako zamiennik całego modelu.

Pakiet: edition `2021`, minimalny Rust `1.85`; zależności `serde 1`, `serde_json 1` z `preserve_order` i `arbitrary_precision`, `sha2 0.10`, `thiserror 2`, `tempfile 3`, `regex 1`, `flate2 1`, `tar 0.4`, `ureq 3`, `fs2 0.4`, `url 2`, `ctrlc 3`. Dokładne wersje utrwala `Cargo.lock`. Jeśli resolver lub toolchain nie spełnia tych wymagań, przerwać T-01 zamiast samodzielnie zmieniać MSRV. Python pozostaje bez nowych zależności.

### Przepływ danych i procesów

`ai-profile → CLI → operacja → jawny RepositoryContext/projekt → odczyt i preflight → synchronizacja → stan`.

`ai-profile-mcp → stan i lokalny override → walidacja → env → exec serwera`.

`mariadb-launcher → walidacja env → obraz/cache/lock → zweryfikowane źródła i build → exec docker run`.

`restore-updraft → odczyt dumpa i preflight → plan → jawne apply/potwierdzenie → lock → ponowny preflight → backup → ochrona → import → lokalizacja → weryfikacja/recovery`.

Platformą tego portu jest Linux/Unix, zgodnie z obecnym użyciem `fcntl` i `execvpe`. Nie dodajemy obsługi Windows. W `process.rs` należy skupić wykonywanie komend: argv bez shella, jawne cwd/env/stdin/stdout/stderr i timeout. Dla wykonywania z wynikiem użyć małego interfejsu `ProcessRunner::run(CommandSpec) -> Result<CommandOutput, ProcessError>`; produkcyjna implementacja używa `std::process::Command`, a implementacja fikcyjna powstaje dopiero w T-24. Wymianę procesu realizuje `CommandExt::exec`, bez pośrednika Python.

### Co oznacza zgodność 1:1

Ta sama obsługiwana operacja, dane, kolejność skutków, kody wyjścia, komunikaty domenowe i zasady błędów. Nie jest wymagany identyczny podział funkcji ani kopiowanie składni Pythona. Nie portujemy Pythona należącego do upstreamowego obrazu MariaDB; polecenie uruchamiające jego serwer nadal zawiera `python src/server.py`.

Nie dopuszczać cichej zmiany kolejności kluczy, obcych pól JSON, wartości hashy, uprawnień plików, trybu dostępu PostgreSQL ani zachowania przy błędzie. Nie obiecywać atomowości odpornej na awarię dysku — obecna implementacja jej nie gwarantuje.

## Kontrakty implementacyjne i walidacja odroczona

Wszystkie poniższe scenariusze uruchamia dopiero T-25. Ich przygotowanie następuje w T-24, po wykonaniu całego kodu. **Brak obowiązku uzyskania RED.** Oczekiwane wyniki wynikają z dokumentacji, jawnych fixture'ów i asercji istniejących testów; porównanie Python/Rust jest dodatkowym dowodem, nie jedynym oracle.

### K-01 — fundament, serializacja i bezpieczeństwo plików

**Powiązania:** BR-01/07; T-02…T-04.

Nowe `model.rs`, `error.rs`, `codec.rs`, `filesystem.rs`, `repository.rs`, `project.rs`, `process.rs` odwzorowują obecne `versioning.py`, `constants.py` i `infrastructure/{filesystem,repository,project}.py`.

- Nazwy profili: lowercase kebab-case; wersje: trzy nieujemne segmenty dziesiętne bez zer wiodących. Nie dodawać ograniczenia `u64`, którego Python nie ma: segment przechowuje zwalidowane cyfry, porównuje długość i zapis leksykograficzny, a bump wykonuje dodawanie dziesiętne.
- `RepositoryContext` ma jawny katalog źródeł. Domyślnie manager znajduje najbliższego przodka własnego pliku wykonywalnego zawierającego `core/rust/src/`, `profiles/`, `common/skills/`, `mcp/servers/`. Nie używa cwd ani `env!("CARGO_MANIFEST_DIR")` jako źródeł użytkownika. Brak właściwego checkoutu jest błędem, nie fallbackiem do przypadkowego katalogu.
- Domyślny projekt to cwd wywołującego; `~` jest rozwijane. Projekt musi istnieć i leżeć poza checkoutem źródeł.
- Ścieżki zarządzane: brak absolutnych, `..` i symlinków w komponentach. SHA-256 dotyczy rzeczywistych bajtów. Plik zmodyfikowany/usunięty albo kolizja z niezarządzanym plikiem blokuje zapis.
- JSON: UTF-8 bez wymuszania ASCII, indent 2, końcowy LF, zachowanie kolejności i nieznanych pól; formatter musi odpowiadać `json_bytes`, także dla pustych kolekcji, Unicode i liczb. Nie zmieniać serializacji override'ów używanej w `mcp_local_hash`.
- Stan zachowuje `format`, `profile`, `version`, `source_commit`, `files`, `mcp`, `mcp_base`, `mcp_local_hash`, `had_config`, `gitignore_added`, `instructions_added`; wspierane są dotychczasowe braki pól legacy. Brak Gita zwraca brak commita, nie blokuje instalacji.

**Końcowa walidacja:** `core/rust/tests/foundation.rs` — fixture'y wersji `1.9.0 < 1.10.0`, duże segmenty, niepoprawne nazwy, JSON Unicode i liczby, brak stanu, uszkodzony stan, `opencode.jsonc`, ścieżka przez symlink. Oracle: `test_profiles.py`, `test_filesystem.py`, `test_validation.py` i jawnie zapisane bajty JSON. Warunek PASS: zgodne wartości/odmowy, zero zapisu przy odmowie.

### K-02 — metadane, create i bump

**Powiązania:** BR-01/02; T-05.

`profile_management/profiles.rs` implementuje `create_profile` i `bump_profile` z `actions/profiles.py`; odczyty i wykrywanie pozostają w `repository.rs`. Create tworzy `profile.json` 1.0.0, `mcp.json` z pustą listą i `skills/.gitkeep`, bez instalacji, nadpisania i zmiany `mcp/example.json`. Bump zachowuje obce pola i prawa metadanych, używa pliku tymczasowego i rename, nie aktualizuje projektów ani innych profili.

**Końcowa walidacja:** `core/rust/tests/profiles.rs`; oracle: `test_profiles.py`. Niezależne fixture'y JSON, nieistniejący profil, kolizja nazwy, pusty opis, wszystkie części bumpu i błąd podmiany. PASS: tylko oczekiwane pliki zmienione, niezależność profili i sprzątnięty plik tymczasowy.

### K-03 — registry i konfiguracja MCP

**Powiązania:** BR-01/07; T-06/T-07.

`mcp/config.rs` przejmuje `infrastructure/mcp.py` i `mcp_runtime.{load_local,merge,validate_servers,resolve,render,changed_override_paths}`. Manifest profilu ma dokładnie `{"servers": [...]}`; serwery są jawnie wybierane z `mcp/servers/`. Walidacja musi obejmować wszystkie warunki obecnego `validate_servers`, nie tylko command/url. Timeout jest dodatnią liczbą całkowitą, nie boolem. Zachować `protocol`, `oauth`, `enabled/disabled`, `codemode`, environment/headers i reguły env_file.

Override obiektów scala się rekurencyjnie, tablice zastępuje; nie wolno zmienić transportu ani nadpisać serwera niezarządzanego. Dla lokalnego postgres dedykowane `access_mode` jest jedynym źródłem prawdy; domyślnie `unrestricted`, jeden końcowy argument `--access-mode=...`, duplikaty/stare niepoprawne argumenty są błędem. `render` usuwa z lokalnego wpisu `env_file/environment/cwd/access_mode`, zachowuje natywny remote i kieruje lokalne MCP do binarium `.opencode/ai-profile-mcp`. Zarządzana konfiguracja używa absolutnej ścieżki adaptera. Sekrety nie trafiają do konfiguracji ani komunikatów zmian upstream.

**Końcowa walidacja:** `core/rust/tests/mcp_config.rs`; oracle: `mcp/README.md`, `test_mcp_configuration.py`, odpowiednie `test_legacy.py`. PASS: dokładny wybór serwerów, błędy przed zapisem, zgodne merge/render i single-flag access mode. Osobno sprawdzić, że walidacja wybranego profilu nie odrzuca niewybranego uszkodzonego registry, a pełna ją odrzuca.

### K-04 — środowisko i samodzielny runtime MCP

**Powiązania:** BR-01/04/07; T-08/T-09.

`mcp/environment.rs` przenosi literalny parser dotenv oraz `environment_for`: BOM, komentarze, `export`, pojedyncze i podwójne cudzysłowy zgodnie z bieżącą obsługą, bez wykonywania shella i interpolacji `$VAR`. Kolejność: inherited → env_file → environment. Wszystkie mapowania `{env:NAME}` czytają ten sam stan sprzed mapowania; brak/pusta wartość przerywa, bez wypisywania wartości sekretu.

`mcp/runtime.rs` i `bin/ai-profile-mcp.rs`: jeden argument SERVER; projekt z katalogu nadrzędnego `.opencode` zawierającego uruchomiony binarny adapter. Do użycia natywnego runtime w walidacji kopiuje się plik wykonywalny do tymczasowego `project/.opencode/ai-profile-mcp`; nie tworzy symlinka do checkoutu. Czyta istniejący stan i override na każdy start, wymaga `mcp_base`, sprawdza lokalny/włączony serwer, ustala cwd i używa `exec` z zachowaniem stdio. Kody 0/1/2 i bezpieczne komunikaty odpowiadają Pythonowi; brak automatycznego restartu i retry.

**Końcowa walidacja:** `core/rust/tests/mcp_runtime.rs`, wyłącznie Rust. Oracle stanowią kontrakty i odczytane asercje `test_mcp_runtime.py`, `test_contract_runtime.py`; nie uruchamiać tych źródeł. Żadnego prawdziwego MCP. PASS: dokładne env i stdio, odczyt zmienionego env_file bez update, błąd bez sekretów, niezależność binarnego runtime od pakietu Python.

### K-05 — payload, lifecycle i legacy

**Powiązania:** BR-01/02/07; T-10…T-14.

`profile_management/{payload,installation,uninstall,status,validation,migration}.rs` odwzorowują wskazane moduły Python. Payload przechowuje ścieżkę, bajty i rodzaj (dane/binarium): common/profile skills i assets, adapter Rust, wybrane launchery, scripts i README profilu. MariaDB dostaje `.opencode/mcp/mariadb/mariadb-launcher`, WordPress `.opencode/ai-profile/scripts/restore-updraft`. Wszystkie binaria pochodzą z `core/rust/target/release/` i są instalowane z 0755; brak binarium blokuje przygotowanie przed zapisem. Nie dostarczać Python. Zachować filtry cache i wyłączenie workflow repozytorium. Nie wykonywać kopiowanych narzędzi.

Apply: odczyt → sprawdzenie downgrade → przygotowanie plików/MCP → kontrola hashy, konfiguracji i wszystkich ścieżek docelowych → usunięcie wycofanych plików i zapis nowych → konfiguracja i cleanup → stan. Update i switch używają tej samej funkcji; CLI odpowiada za rozróżnienie dozwolonych stanów początkowych. Uninstall usuwa tylko własne, niezmodyfikowane zasoby i zachowuje lokalną konfigurację.

Wycofany `workflow-rules.md`, dawny skill i approval gate podlegają istniejącym hashom/ownership. Własny wpis instrukcji usuwa się tylko przy `instructions_added`; własny wpis `.gitignore` tylko przy `gitignore_added`. Legacy MVP wymaga `--accept-existing` i zapisuje istniejące pliki jako baseline, bez odtwarzania nieznanych hashy. Zachować etykiety statusu i scoped/full validate.

**Końcowa walidacja:** `core/rust/tests/lifecycle.rs`; oracle: dokumentacja profili i odczytane asercje `test_operations.py`, `test_legacy.py`, `test_status.py`, `test_payload.py`, `test_validation.py`, `test_wordpress_profile.py`, bez uruchamiania Python. Tymczasowe checkouty/projekty i syntetyczne payloady. PASS: natywne binaria z 0755, hashe, wycofanie zarządzanych `.py`, idempotencja, brak częściowych zmian przy konflikcie, zachowanie obcych plików/MCP/sekretów, zgodne upgrade/switch/uninstall/migrate i etykiety statusu.

### K-06 — CLI i współistnienie implementacji

**Powiązania:** BR-01…BR-04; T-15/T-23.

`profile_management/cli.rs` i `bin/ai-profile.rs` obsługują wszystkie akcje/argumenty z obecnego `cli.main`, łącznie z ograniczeniami `--project`, `--part`, `--description`, `--accept-existing`. Brak akcji = help; parser error = 2; błąd operacji = 1; sukces = 0. Zachować pomoc i komunikaty domenowe; nie wymagać zgodności tekstu surowych błędów bibliotek, które obecny CLI podaje w `ERROR`.

Nie dodawać nowych flag do istniejącego interfejsu. `make rust-build` buduje wszystkie binaria release w stałym `core/rust/target/`. Cele managera Make budują je przed operacją i używają Rust; instalacja to `make install`. Nie instalować payloadu Python ani skryptów osadzających hostowy Python. Stare pliki odczytywać wyłącznie w celu sprawdzenia ownership/hashy przed ich wycofaniem.

**Końcowa walidacja:** `core/rust/tests/cli.rs`; oracle: lista argumentów i warunków w `cli.py`, `test_cli.py` i `test_contract_cli.py`. PASS: wszystkie akcje połączone, zgodne exit codes i cwd, checkout ze spacjami, brak efektów przy błędzie parsera, brak zależności od katalogu użytego przy kompilacji.

### K-07 — launcher MariaDB

**Powiązania:** BR-01/07; T-16/T-17.

`mariadb/archive.rs`, `mariadb/runtime.rs`, `bin/mariadb-launcher.rs` portują `mcp/launchers/mariadb/launch.py`. Zachować literalne COMMIT, ARCHIVE_URL, SHA-256, IMAGE, SOURCE_LABEL i PLATFORM z tego pliku. Limit pobrania 16 MiB, timeout 60 s, hash przed rozpakowaniem; tylko pliki/katalogi pod oczekiwanym korzeniem, bez linków, urządzeń, ścieżek absolutnych i `..`, wymagany Dockerfile.

Build dostaje tylko allowlist env z `docker_environment`; credentials nie przechodzą do inspect/info/build. Inspect/info mają timeout 30 s, build 3600 s. `ensure_image` sprawdza label, działający Linux daemon, cache i blokadę wyłączną, po lock ponownie sprawdza obraz. stdout buildu kierowany na stderr. Runtime wymaga DB settings, port ASCII 1…65535, poprawne flagi bool; domyślnie read-only/block-sensitive = false, network=host. `docker run` otrzymuje nazwy `--env`, nie wartości; kończy się `exec`, nie shellem.

**Końcowa walidacja:** `core/rust/tests/mariadb.rs`; oracle: `test_mariadb_launcher.py` oraz jawne argv/env z oryginalnego źródła. HTTP jest wstrzykniętym downloaderem zwracającym bajty fixture'a; Docker fikcyjnym ProcessRunner. PASS: verified image pomija pobranie/build, recheck pod lock działa, niepoprawne archiwum lub daemon blokuje build, błędy nie ujawniają sekretów. Nie pobierać obrazu/archiwum upstream nawet w finale.

### K-08 — mapowania i inspekcja dumpa WordPress

**Powiązania:** BR-01/07; T-18.

`wordpress/{mapping,dump}.rs` odpowiada `origin`, `mappings`, `mapped_url`, `inspect_dump`, `file_hash`. Origin to http/https + host/opcjonalny port, bez auth/query/fragment/path poza `/`; zachować obecne reguły hostów. Mapowania są jawne, niepuste, bez identyczności, duplikatów, łańcuchów i scalania witryn, stosowane od najdłuższego origin.

Dump: niepusty `.sql`/`.gz`, gzip rozpoznany po magic bytes i odczytany do końca z weryfikacją CRC; brak ZIP/crypt. Zachować reguły odrzucania dyrektyw baz/użytkowników/klienta SQL i wykrywania jednego pełnego prefiksu WP oraz multisite. Wynik obejmuje prefix, multisite, gzip i SHA-256 oryginalnych bajtów. Przetwarzać strumieniowo; nie ładować całej bazy do JSON/stringa.

**Końcowa walidacja:** `core/rust/tests/wordpress_dump.rs`; oracle: `test_wordpress_restore.py::{test_plain_and_gzip_dump_validation_and_prefix_detection,test_rejects_encrypted_incomplete_corrupted_or_database_switching_dumps,test_mapping_requires_explicit_unambiguous_origins}` i małe syntetyczne dumpy. PASS: zgodne wyniki i odmowy, brak efektów w projekcie.

### K-09 — DDEV, preflight i lokalizacja WordPress

**Powiązania:** BR-01/07; T-19…T-21.

`wordpress/{ddev,preflight,localize}.rs` odwzorowują `Ddev`, `literal`, `topology`, `preflight`, `localize`. DDEV jest zewnętrzną granicą przez ProcessRunner; `describe` obsługuje JSON i JSON-lines/raw. WP zawsze w `ddev exec`, z container docroot, `local-guard.php`, `--skip-plugins`, `--skip-themes`; żadne lokalne `wp` ani shell.

Preflight sprawdza pliki helperów, projekt DDEV typu wordpress/running i zgodny approot, bezpieczny docroot, wp-config/wp-content, docelowe hosty z describe, prefiks, tożsamość lokalnej bazy `db`, zgodność single/multisite i brak obcego pliku safety. Nie zmienia konfiguracji DDEV ani danych.

Lokalizacja najpierw sprawdza komplet mapowań wszystkich blogów/sieci. Ustawia bootstrapowe adresy/home/siteurl i domeny przez SQL z UTF-8 hex literals, ale dane serializowane zmienia wyłącznie WP-CLI `search-replace --precise --all-tables-with-prefix --skip-columns=guid`. Wymaga liczbowego wyniku i dry-run=0; ostrzeżenie WP-CLI to błąd. Zachowuje IDs/paths/liczbę witryn/sieci, ustawia blog_public=0 i potwierdza instalację WP.

**Końcowa walidacja:** `core/rust/tests/wordpress_preflight.rs`, `wordpress_localize.rs`; oracle: odpowiednie testy `test_wordpress_restore.py`. Fikcyjny DDEV zwraca zaplanowaną kolejkę wyników; żadnej prawdziwej bazy. PASS: błędny host/prefiks/database/JSON blokuje, brak mapowania nie daje żadnego zapisu, komendy i kolejność lokalizacji zachowane, brak sekretów w diagnostyce.

### K-10 — restore, backup i recovery WordPress

**Powiązania:** BR-01/07; T-22.

`wordpress/restore.rs`, `bin/restore-updraft.rs`: te same `dump`, wymagane `--project`, wielokrotne `--map`, opcjonalne `--apply`. Plan wykonuje tylko odczyt i diagnostykę: bez export/import, plików safety i backupów. Apply wymaga wpisania nazwy projektu; anulowanie niczego nie zapisuje.

Po potwierdzeniu: backup_root poza projektem pod XDG_STATE_HOME, prywatne katalogi, nieblokujący lock, powtórny preflight i porównanie źródła/celu, prywatna kopia incoming i hash, export-db, pełna weryfikacja backupu, dopiero safety i import-db. Incoming/backup 0600, nowe katalogi backupu 0700; safety PHP 0644 i jego nowy parent 0755. Nie osłabiać tego do chmod dopiero po zapisaniu sekretów. Nie nadpisywać obcego safety.

Po rozpoczęciu importu błąd/przerwanie zachowuje backup, usuwa incoming i podaje bezpiecznie cytowane polecenie ręcznego recovery; brak automatycznego rollbacku. SIGINT obsłużyć przez sygnał anulowania (`ctrlc`) i kontrolowaną ścieżkę sprzątania, nie `panic!`/natychmiastowy exit omijający recovery. RunDir backupu nie jest automatycznie usuwany po sukcesie ani błędzie.

**Końcowa walidacja:** `core/rust/tests/wordpress_restore.rs`; oracle: README WordPress i wszystkie testy restore. PASS: brak zapisu w plan/cancel, backup i CRC przed importem, zmiana źródła po potwierdzeniu blokuje, błędy przed backupem nie instalują safety, błąd po imporcie zachowuje recovery i backup. Wszystko tylko na fixture'ach i ProcessRunner.

## Zadania implementacyjne — jedyna kolejka wykonania

**Stan początkowy wszystkich zadań:** wykonanie `TODO`, testy `NOT RUN`. T-01…T-23 mają testy `NOT RUN — PROHIBITED AT THIS STAGE`, także po oznaczeniu kodu `IMPLEMENTED`. T-24 przygotowuje testy, ale ich nie uruchamia. T-25 jako jedyny może zmienić wynik walidacji na `PASSED` albo `FAILED`.

Procedura wspólna: sprawdź poprzedniki → przeczytaj wyłącznie wskazane źródło i kontrakt → załaduj `@rust` → wykonaj kroki zadania → statycznie porównaj pokrycie symboli i skutków → ewentualnie sformatuj/skompiluj bez wykonywania programu → zapisz w tym planie zmienione pliki, stan kompilacji i odroczenie walidacji → **STOP, czekaj na zlecenie następnego zadania**. Nie składać kilku etapów w jeden duży patch.

### T-01 — kompilowalny fundament i reguły pracy

**Stan: IMPLEMENTED.** Walidacja zachowania/testy: `NOT RUN — PROHIBITED AT THIS STAGE`. `cargo fmt` i `cargo check --locked --lib` zakończone powodzeniem; bez uruchamiania programów.

**Zależności:** brak. **Kontrakt:** granice techniczne, BR-03…BR-06.

1. Dodać `core/rust/Cargo.toml`, `Cargo.lock`, `src/lib.rs` i `README.md` zgodnie z wybranym pojedynczym pakietem; bez placeholderów czterech programów.
2. Uzupełnić `.gitignore` o `core/rust/target/`. Cargo może pobrać zależności do budowy; to nie jest pobieranie archiwów/obrazów przez migrowane narzędzia.
3. W `AGENTS.md` zapisać rzeczywistą regułę tej migracji: zachowanie Python, obowiązkowy `@rust`, jedno zadanie na zlecenie, brak testów do T-25 i jedna końcowa komenda. Rozdzielić regułę stdlib Pythona od zależności Cargo. Nie wpisywać raportu zadania.

**Weryfikacja:** tylko manifest, `cargo generate-lockfile --manifest-path core/rust/Cargo.toml`, format i kompilacja biblioteki. **Gotowe, gdy:** brak kodu operacji, poprawny fundament i zachowane niepowiązane zmiany. Testy nieuruchomione.

### T-02 — typy, wersje, błędy i JSON

**Stan: IMPLEMENTED.** Dodano zwalidowane nazwy profili i nieograniczone segmenty wersji, typowane akcje/bump/context, błędy strukturalne oraz codec JSON/SHA-256. `cargo fmt` i `cargo check --locked --lib` zakończone powodzeniem. Testy i zachowanie: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-01. **Kontrakt:** K-01.

1. Dodać `src/{model,error,codec}.rs` i rejestrację w `lib.rs`; przenieść reguły z `versioning.py`, `constants.py`, `filesystem.{sha,json_bytes,read_json}`.
2. Wprowadzić zwalidowane ProfileName/Version, jawne warianty bump/action, kontekst źródeł i błędy per jednostka awarii; formatter błędów nie wypisuje sekretów.
3. Zaimplementować python-compatible JSON dla stanu/config/hash i SHA-256. Nie zastępować tego wywołaniem interpretera.

**Weryfikacja:** przegląd K-01 i kompilacja. **Gotowe, gdy:** nie ma limitu segmentu u64, serializacja ma określone Unicode/indent/LF i nie gubi obcych pól. Zgodność runtime odroczona.

### T-03 — bezpieczny system plików i granica procesów

**Stan: IMPLEMENTED.** Dodano narzędzia ścieżek/ownership/zapisu/cleanup oraz `ProcessRunner` z argv, cwd/env, sterowaniem stdin/stdout/stderr, timeoutem, kill+wait i tokenem anulowania. `cargo fmt` i `cargo check --locked --lib` zakończone powodzeniem. Testy i procesy zewnętrzne: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-02. **Kontrakt:** K-01 i przepływ procesów.

1. Dodać `src/{filesystem,process}.rs`; przenieść `file_path`, `check_files`, `write_bytes`, `cleanup_empty_directories`.
2. Zdefiniować CommandSpec/CommandOutput i ProcessRunner z cwd, env, trybem strumieni, timeoutem; żadnych interpolowanych komend shella.
3. Zachować przerwanie przed zapisem przy konflikcie; timeout zabija i zbiera proces potomny, anulowanie umożliwia kontrolowany powrót do restore.

**Weryfikacja:** przegląd zakazanych ścieżek, sekwencji efektów i kompilacja. **Gotowe, gdy:** istnieją wszystkie narzędzia plikowe oraz jawna granica zewnętrznych procesów, bez ich uruchamiania.

### T-04 — źródła repozytorium i stan projektu

**Stan: IMPLEMENTED.** Dodano odkrywanie checkoutu względem lokalizacji binarium, odczyt profili/metadanych i opcjonalnego commita, wybór projektu, stan/config oraz czyszczenie wyłącznie własnej linii `.gitignore`. `cargo fmt` i `cargo check --locked --lib` zakończone powodzeniem. Testy i zachowanie: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-03. **Kontrakt:** K-01.

1. Dodać `src/{repository,project}.rs`; przenieść wszystkie symbole z `infrastructure/{repository,project}.py`.
2. Implementować wybór katalogów według K-01, odczyt metadanych/stanu/config, walidację wersji legacy i odrzucanie `opencode.jsonc`.
3. Zachować cleanup wyłącznie własnej linii `.gitignore` i opcjonalność commita Git.

**Weryfikacja:** statyczne porównanie pól i warunków oraz kompilacja. **Gotowe, gdy:** nie ma zależności od cwd dla źródeł ani baked-in ścieżki builda; projekt domyślny nadal pochodzi z cwd.

### T-05 — tworzenie i bump profilu

**Stan: IMPLEMENTED.** Dodano `create_profile` i `bump_profile` ze stagingiem, `.gitkeep`, zachowaniem obcych pól i uprawnień metadata oraz automatycznym sprzątaniem pliku tymczasowego przy błędzie. `cargo fmt` i `cargo check --locked --lib` zakończone powodzeniem. Testy i operacje na profilach: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-04. **Kontrakt:** K-02.

1. Dodać `src/profile_management.rs` i `profile_management/profiles.rs`.
2. Przenieść tylko `create_profile` i `bump_profile`, używając odczytów T-04 i typów T-02.
3. Zachować staging, .gitkeep, uprawnienia i sprzątanie tymczasowego metadata przy błędzie.

**Weryfikacja:** przegląd efektów i kompilacja, bez tworzenia prawdziwych profili. **Gotowe, gdy:** oba działania są kompletne i nie wymagają decyzji instalatora.

### T-06 — manifesty i typy MCP

**Stan: IMPLEMENTED.** Dodano typowany model MCP oraz odczyt definicji/manifestów, wykrywanie duplikatów i walidację serwerów obejmującą transport, command/url, environment/headers, flagi, cwd, timeouty bez akceptacji bool jako int, OAuth, protocol, access_mode i env_file z ochroną ścieżek/symlinków. `cargo fmt` i `cargo check --locked --lib` zakończone powodzeniem. Testy: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-04. **Kontrakt:** K-03.

1. Dodać `src/mcp.rs` i `mcp/config.rs`; przenieść `mcp_definition`, `desired_mcp`, `validate_servers`, `project_path`.
2. Zakodować wszystkie walidowane pola i odróżnić bool od timeout int; zachować nazwy/unikalność serwerów i brak symlinków.

**Weryfikacja:** lista warunków z `mcp_runtime.py:104–146` i kompilacja. **Gotowe, gdy:** registry i manifest nie mają pominiętych reguł walidacji.

### T-07 — override, postgres i render MCP

**Stan: IMPLEMENTED.** Dodano local override loading/ownership, rekurencyjny merge, walidację i pojedynczy argument access mode PostgreSQL, resolve/render, ścieżkę lokalnego launchera oraz kontrolę konfliktów i zmienionych ścieżek override. Synchronizacja nie odczytuje `env_file`. `cargo fmt` i `cargo check --locked --lib` zakończone powodzeniem. Testy i zachowanie: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-06. **Kontrakt:** K-03.

1. Uzupełnić `mcp/config.rs` o `load_local`, `merge`, `split_access_mode`, `apply_access_mode`, `resolve`, `render`, `changed_override_paths` oraz odpowiedniki `configured_mcp/check_mcp`.
2. Zachować bezpieczne ostrzeżenia i nie odczytywać sekretów env_file przy synchronizacji. Historyczna ścieżka adaptera Python została zastąpiona binarium Rust w T-24a.

**Weryfikacja:** przegląd całej ścieżki base→overrides→effective→rendered i kompilacja. **Gotowe, gdy:** brak domyślnych zmian access_mode, zdalne MCP pozostają natywne, lokalne prywatne pola nie są renderowane.

### T-08 — dotenv i środowisko

**Stan: IMPLEMENTED.** Dodano literalny parser dotenv (BOM, podział linii, komentarze, `export`, quoting bez shella) i resolver środowiska z kolejnością inherited → env_file → environment oraz podstawianiem wszystkich mapowań ze wspólnego stanu wejściowego. Błędy nie ujawniają wartości sekretów. `cargo fmt` i `cargo check --locked --lib` zakończone powodzeniem. Testy i parser na fixture'ach: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-07. **Kontrakt:** K-04.

1. Dodać `src/mcp/environment.rs`; odwzorować `dotenv` i `environment_for` bez shella.
2. Zachować quoting, komentarze/BOM, precedence i równoczesną interpolację mapowań.

**Weryfikacja:** przegląd źródła `mcp_runtime.py:183–227`, bezpieczeństwa błędów i kompilacja. **Gotowe, gdy:** parser i resolution są kompletne; nie wykonywano parsera na fixture'ach.

### T-09 — natywny adapter MCP

**Stan: IMPLEMENTED.** Dodano samodzielne binarium adaptera, wybór projektu z położenia własnego pliku wykonywalnego w `.opencode`, walidację stanu/serwera, override/env/cwd oraz Unix `exec` z odziedziczonymi stdio. Błędy wykonania nie ujawniają ścieżek ani wartości sekretów. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Testy i adapter: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-08. **Kontrakt:** K-04.

1. Dodać `src/mcp/runtime.rs` i pierwsze binarium `src/bin/ai-profile-mcp.rs`.
2. Przenieść `launch/main`: wybór projektu z pliku wykonywalnego, odczyt stanu, odmowy, env/cwd i Unix exec.

**Weryfikacja:** przegląd streamów/kodów i kompilacja, bez uruchamiania adaptera. **Gotowe, gdy:** nie wywołuje własnego kodu Python i nie wymaga całego repozytorium po skopiowaniu do `.opencode`.

### T-10 — zbieranie payloadu

**Stan: IMPLEMENTED.** Dodano kolekcję uporządkowanych ścieżek i bajtów: common/profile skills z kontrolą frontmatter i symlinków, oryginalny adapter Python, wybrane launchery MCP, skrypty oraz README profilu. Zachowano filtry `.gitkeep`, `__pycache__` i `.pyc`; nie dodano ani nie wykonano binariów Rust. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Testy i kolekcja payloadu na danych: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-07. **Kontrakt:** K-05.

1. Dodać `src/profile_management/payload.rs`; odwzorować wszystkie gałęzie `desired_files`.
2. Zachować bajty i ścieżki oryginalnych `.py`, selekcję launcherów, assets i odrzucanie symlinków/kolizji.

**Weryfikacja:** przegląd każdej gałęzi `infrastructure/payload.py` i kompilacja. **Aktualizacja T-24a:** payload dostarcza binaria Rust, bez ich wykonywania; pierwotny zakaz ich instalowania jest SUPERSEDED.

### T-11 — wspólna synchronizacja install/update/switch

**Stan: IMPLEMENTED.** Dodano wspólne `apply_profile`: downgrade check, payload/MCP, ownership i konflikt preflight wszystkich celów, ostrzeżenia override, synchronizację konfiguracji, usuwanie wycofanych plików, zapis nowych plików/config/stanu oraz cleanup własnych markerów. `check_files` przyjmuje pożyczone ścieżki, zachowując kolejność payloadu bez kopiowania bajtów. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Testy i operacje na projekcie: `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-10. **Kontrakt:** K-05.

1. Dodać `src/profile_management/installation.rs`; przenieść `apply_profile` bez CLI i bez uninstall.
2. Zachować downgrade, preflight wszystkich celów, ostrzeżenia override, kolejność mutacji i dokładne pola updated_state.
3. Zachować cleanup ownership wycofanych instrukcji i markerów.

**Weryfikacja:** przejście źródła linia po linii pod kątem efektów i kompilacja. **Gotowe, gdy:** nie ma żadnego zapisu przed zakończeniem kontroli konfliktów ani opcji force.

### T-12 — uninstall

**Stan: IMPLEMENTED.** Dodano `uninstall` z kontrolą stanu, hashy i ownership MCP przed zmianami; usuwa wyłącznie zarządzane pliki/konfigurację, zachowuje obce wpisy, respektuje `instructions_added` i `had_config`, usuwa własne markery oraz sprząta puste katalogi. `opencode.json` znika tylko przy warunku schema-only. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Testów ani operacji uninstall nie uruchamiałem.

**Zależności:** T-11. **Kontrakt:** K-05.

1. Dodać `src/profile_management/uninstall.rs`; przenieść `uninstall` z zachowaniem preflight, had_config i własności instrukcji/MCP.
2. Usuwać konfigurację główną tylko przy dotychczasowym warunku schema-only; zachować lokalny config i obce pliki.

**Weryfikacja:** przegląd listy usuwanych ścieżek i kompilacja. **Gotowe, gdy:** usuwanie zależy od zweryfikowanego stanu, nie od szerokiego kasowania katalogów.

### T-13 — status i walidacja źródeł

**Stan: IMPLEMENTED.** Dodano `status` z osobną oceną wersji, zmian treści, lokalnych modyfikacji plików/MCP i hashy override’ów, zachowując wszystkie etykiety. Dodano scoped/full `validate_sources`, w tym pełny registry scan i odpowiednie porównanie z `mcp/example.json`. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Testów, `status` ani walidacji źródeł nie uruchamiałem.

**Zależności:** T-10. **Kontrakt:** K-05.

1. Dodać `src/profile_management/{status,validation}.rs`; przenieść `status` i `validate_sources`.
2. Zachować odrębność version/content/local modifications, wszystkie etykiety i scoped/full registry/example union.

**Weryfikacja:** przegląd tabeli warunków statusu i kompilacja; nie uruchamiać jeszcze validate. **Gotowe, gdy:** nie zmienia danych ani nie pobiera upstream.

### T-14 — legacy migration

**Stan: IMPLEMENTED.** Dodano jawnie potwierdzaną adopcję starego stanu: dokładne mapowania dwóch profili/skillów, wymagany wcześniejszy MCP i lista pięciu historycznych plików. Zapisywane są ich bieżące hashe jako baseline, bez zmian treści; gate jest usuwany dopiero po zapisaniu stanu. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Testów ani migracji na projekcie nie uruchamiałem.

**Zależności:** T-12. **Kontrakt:** K-05.

1. Dodać `src/profile_management/migration.rs`; przenieść `migrate_legacy`, historyczny mapping profili i listę plików do adopcji.
2. Wymagać `accept_existing`, zachować format przejściowy i instrukcję późniejszego update.

**Weryfikacja:** przegląd źródłowego `actions/migration.py` i kompilacja. **Gotowe, gdy:** nie zgaduje historycznych hashy i nie przejmuje dowolnej nieznanej instalacji.

### T-15 — połączenie całego CLI menedżera

**Stan: IMPLEMENTED.** Dodano ręczny parser `args_os`, binarium `ai-profile`, pełną pomoc, ograniczenia opcji i połączenie wszystkich dziesięciu akcji z implementacjami Rust. Zachowano kody parsera 2, błędu operacji 1 i sukcesu 0; `--project` pozostaje ścieżką OS, także ze spacjami. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Nie uruchamiałem CLI ani `--help`; testy pozostają `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-05, T-11, T-12, T-13, T-14. **Kontrakt:** K-06.

1. Dodać `src/profile_management/cli.rs` i `src/bin/ai-profile.rs`; ręcznie sparsować istniejący niewielki zestaw akcji/opcji przez `std::env::args_os`, bez nowej konwencji Clap help.
2. Przenieść pomoc i wszystkie reguły argumentów z `cli.main`; połączyć każde działanie z gotowym modułem.
3. Zachować warunki install vs switch vs update i kody 0/1/2.

**Weryfikacja:** tabelaryczny przegląd akcja→funkcja→dopuszczalne opcje i kompilacja. **Gotowe, gdy:** każda akcja ma rzeczywistą implementację; nadal bez uruchamiania `--help`.

### T-16 — wejścia i bezpieczne źródła MariaDB

**Stan: IMPLEMENTED.** Dodano stałe zgodne ze źródłem, allowlistę środowiska Docker, walidację wymaganych ustawień/portu/bool oraz wstrzykiwalny interfejs downloadera z implementacją `ureq` (limit 16 MiB, timeout 60 s). Ekstrakcja weryfikuje SHA-256 przed ręcznym rozpakowaniem i dopuszcza wyłącznie regularne pliki/katalogi pod przypiętym korzeniem, odrzucając ścieżki traversal i niedozwolone typy wpisów. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Nie pobierałem archiwum ani nie uruchamiałem Docker/testów.

**Zależności:** T-03. **Kontrakt:** K-07.

1. Dodać `src/mariadb.rs`, `mariadb/archive.rs` i przygotować część `mariadb/runtime.rs` walidującą DB/env.
2. Przenieść stałe, allowlist env, limity/HTTP/hash i ręczną bezpieczną ekstrakcję. Realny downloader `ureq` ukryć za małym interfejsem dostarczenia bajtów, aby finał nie wymagał sieci.

**Weryfikacja:** przegląd limitów i warunków odmowy z `launch.py:16–71,105–114`, kompilacja. **Gotowe, gdy:** ekstrakcja nie używa niesprawdzonego unpack całego archiwum.

### T-17 — obraz, blokada i uruchomienie MariaDB

**Stan: IMPLEMENTED.** Dodano inspect/info, allowlistę env dla komend Docker, cache i blokadę z recheckiem po locku, zweryfikowane pobranie/build obrazu oraz `docker run` z nazwami `--env` i Unix `exec`. Build kieruje stdout do stderr; credentials pozostają wyłącznie w środowisku runtime. Dodano binarium `mariadb-launcher`. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Nie uruchamiałem binarium, Docker ani HTTP; testy pozostają `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-16. **Kontrakt:** K-07.

1. Uzupełnić `mariadb/runtime.rs` o inspect/info/cache/lock/recheck/build i docker run.
2. Dodać `src/bin/mariadb-launcher.rs`; zachować timeouts, stderr buildu i exec.

**Weryfikacja:** przegląd argv/env i kompilacja, bez Docker/HTTP. **Gotowe, gdy:** wszystkie ścieżki błędów są bezpieczne, gotowy obraz nie uruchamia buildu, credentials nie trafiają do builda.

### T-18 — mapowania i dump WordPress

**Stan: IMPLEMENTED.** Dodano typowane originy/mapowania z walidacją hostów, duplikatów, łańcuchów i kolizji oraz `mapped_url`. Dump `.sql`/`.gz` jest przetwarzany strumieniowo; wykrywane są zakazane dyrektywy, kompletny prefiks single/multisite, a gzip jest czytany do EOF z weryfikacją CRC. SHA-256 dotyczy oryginalnych bajtów. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Nie inspekowałem dumpów ani nie uruchamiałem testów.

**Zależności:** T-03. **Kontrakt:** K-08.

1. Dodać `src/wordpress.rs`, `wordpress/{mapping,dump}.rs`.
2. Przenieść `origin/mappings/mapped_url/inspect_dump/file_hash`, ze strumieniowym gzip i zachowaniem reguł wykrywania tabel.

**Weryfikacja:** przegląd wszystkich regexów, ograniczeń mapowań i kompilacja. **Gotowe, gdy:** nie ma mechanizmu zmiany bazy ani pomijania CRC.

### T-19 — DDEV i topologia

**Stan: IMPLEMENTED.** Dodano `Ddev` przez `ProcessRunner` z argv/cwd, odrzucaniem szczegółów mogących ujawnić sekrety, kontrolą WP-CLI warnings, obsługą JSON i JSON-lines `describe`, komendami WP przez `ddev exec`, zapytaniami oraz hex-literal i topologią site/network. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Nie uruchamiałem DDEV, WP-CLI, bazy ani testów.

**Zależności:** T-18. **Kontrakt:** K-09.

1. Dodać `src/wordpress/ddev.rs`; przenieść Ddev.run/describe/wp/query, literal i topology przez ProcessRunner.
2. Zachować guard, container docroot, format zapytań i sanitizację błędów/WP warnings.

**Weryfikacja:** statyczny przegląd komend i kompilacja. **Gotowe, gdy:** nie ma bezpośredniego dostępu do DB ani wykonywania WP poza DDEV.

### T-20 — wyłącznie preflight WordPress

**Stan: IMPLEMENTED.** Dodano preflight DDEV/WordPress bez zapisów: walidację plików projektu i helperów, approot/status/type/docroot, hostów URL, lokalnej bazy/prefiksu, zgodności dumpa i topologii oraz konfliktu lokalnego pliku safety. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Testów, DDEV, WP-CLI, bazy ani dumpów nie uruchamiałem; walidacja zachowania `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-19. **Kontrakt:** K-09.

1. Dodać `src/wordpress/preflight.rs`; przenieść preflight i inside.
2. Odwzorować sprawdzenie approot/status/type/docroot/hosts/db/prefix/multisite/helperów/safety bez zapisów.

**Weryfikacja:** lista wszystkich źródłowych warunków odmowy i kompilacja. **Gotowe, gdy:** żaden krok nie eksportuje/importuje ani nie instaluje safety.

### T-21 — lokalizacja WordPress

**Stan: IMPLEMENTED.** Dodano `wordpress/localize.rs`: pełna walidacja mapowań wszystkich witryn i sieci przed pierwszą zmianą, bootstrap URL/domen SQL przez UTF-8 hex literals, WP-CLI search-replace z dry-run, ustawienie `blog_public=0`, weryfikację topologii i `core is-installed`. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Testów, DDEV, WP-CLI ani bazy nie uruchamiałem; walidacja zachowania `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-20. **Kontrakt:** K-09.

1. Dodać `src/wordpress/localize.rs`; przenieść localize, zachowując kontrolę wszystkich mapowań przed pierwszą zmianą.
2. Zachować hex SQL, sekwencję bootstrap domen→WP search-replace→dry-run→blog_public→topologia→core is-installed.

**Weryfikacja:** porównanie kolejności efektów i kompilacja. **Gotowe, gdy:** nie ma raw string replace serializowanych danych ani ignorowania ostrzeżeń.

### T-22 — apply, backup, recovery i CLI restore

**Stan: IMPLEMENTED.** Dodano `wordpress/restore.rs` i binarium `restore-updraft`: jawne tryby plan/apply, wymagane potwierdzenie, prywatny katalog stanu i nieblokująca blokada, ponowny preflight, prywatna kopia incoming i hash, eksport/pełna weryfikacja backupu przed safety/importem, kontrolowane anulowanie, recovery bez rollbacku oraz cleanup incoming. `cargo fmt` i `cargo check --locked --lib --bins` zakończone powodzeniem. Nie uruchamiałem testów, binarium, DDEV ani usług zewnętrznych; walidacja zachowania `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-21. **Kontrakt:** K-10.

1. Dodać `src/wordpress/restore.rs` i `src/bin/restore-updraft.rs`; przenieść restore/main.
2. Oddzielić plan od apply enumem i wyraźnym przejściem po potwierdzeniu; przenieść lock, recheck, backup, safety, import i lokalizację.
3. Zaimplementować kontrolowane anulowanie, zachowanie backupu i cleanup incoming bez automatycznego rollbacku.

**Weryfikacja:** ręczny przegląd stanów przed/po import_started, uprawnień i kompilacja. **Gotowe, gdy:** wszystkie cztery natywne binaria są funkcjonalnie zaimplementowane, ale jeszcze nieuruchomione.

### T-23 — spięcie budowania i dokumentacji, bez przełączenia domyślnego silnika

**Stan: IMPLEMENTED, później zmieniony na mocy BR-08.** `make rust-build` oraz mapa produkcyjnego kodu Python→Rust zostały dodane. Pierwotne „Python pozostaje domyślny w Make” jest **SUPERSEDED przez BR-08**: od T-24 Make uruchamia wyłącznie Rust. Historyczny release build tego zadania zakończył się powodzeniem; binaria nie były uruchamiane.

**Zależności:** T-09, T-15, T-17, T-22. **Kontrakt:** K-06, BR-02/03/05.

1. W Makefile dodać wyłącznie `rust-build` z `cargo build --manifest-path core/rust/Cargo.toml --locked --release --bins`; nie uruchamiać wybudowanych programów.
2. Uzupełnić `core/rust/README.md`: cztery binaria, sposób wskazania checkoutu przez lokalizację binarnego managera, jawne komendy i zachowanie Pythona/payloadu.
3. Zapisać tabelę wszystkich produkcyjnych symboli Python i ich lokalizacji Rust. Samo istnienie pliku lub stub nie oznacza pokrycia. Potwierdzić brak odwołań Rust do interpretera dla własnej implementacji.

**Weryfikacja:** przegląd kompletności tabeli i kompilacja release bez uruchamiania. **Gotowe, gdy:** T-01…T-23 są `IMPLEMENTED`, nadal wszystkie zachowania `NOT RUN`. Nie oznaczać migracji zakończoną.

### T-24 — przygotowanie jednej walidacji końcowej Rust-only, bez uruchamiania

**Stan: IMPLEMENTED.** Dodano Rust-only test suites dla fundamentu, profili, MCP, lifecycle/CLI, MariaDB i restore WordPress, zmieniono Make manager/test na Rust i przygotowano `validate-rust-migration` z kontrolą statusu oraz obecności każdej wymaganej grupy. Zachowano źródła Python; nie używa się ich w Make ani walidacji migracji. Nie uruchamiałem testów, clippy, walidatora ani binariów; testy/walidacja zachowania pozostają `NOT RUN — PROHIBITED AT THIS STAGE`.

**Zależności:** T-23. **Kontrakt:** wszystkie K-01…K-10 i BR-06.

1. Dodać Rust tests w `core/rust/tests/` dla kontraktów K-01…K-10: fikcyjne `ProcessRunner`/downloader, syntetyczne archiwa i dumpy oraz scenariusze negatywne. Nie wykonywać RED ani testów pośrednich. Nie dodałem pełnego historycznego Python parity; oracle stanowią jawne kontrakty i niezależne asercje Rust.
2. Wszystkie Make targets managera i `make test` mają uruchamiać Rust; żadna reguła nie wywołuje `python`, `python3` ani unittest. `make validate` uruchamia walidację źródeł Rust.
3. `validate-rust-migration` wykonuje tylko Rust: `cargo fmt --check`, release build, Clippy `-D warnings`, `make validate` i `make test`. Loguje status każdej grupy, odróżnia `FAILED` od `NOT RUN`, wymaga niezerowej liczby testów i kończy się błędem przy dowolnej awarii. Nie używa skip, cache ani wymuszonego sukcesu.
4. Harness/testy nie używają Pythona ani prawdziwych Docker/DDEV/DB/HTTP. Testy procesu i pobierania używają granic fikcyjnych; nie wykonują produkcyjnych binariów ani zewnętrznych usług.

**Weryfikacja:** wyłącznie przegląd kodu runnera, scenariuszy i poleceń. **Gotowe, gdy:** walidator i testy Rust są przygotowane po implementacji, nie zostały jeszcze uruchomione i nie wymagają Python ani prawdziwego Docker/DDEV/DB/HTTP. Nie tworzyć golden files przez bezkrytyczne zapisanie aktualnego wyniku.

### T-24a — korekta spięcia i dystrybucji Rust-only

**Stan: IMPLEMENTED.** Osobne zlecenie użytkownika po T-24. `make install` buduje i uruchamia Rust; usunięto alias `install2`. Make buduje wszystkie runtime w stałym `core/rust/target/release/`. Payload kopiuje adapter MCP, wybrany launcher MariaDB oraz restore WordPress jako binaria z 0755; historyczne `.py` pozostają w repozytorium. Rejestr i przykład MariaDB wskazują natywny launcher. Update wycofuje zarządzane `.py` po kontroli hashy i zastępuje zarządzaną konfigurację MCP. Dokumentacja i AGENTS.md wskazują Rust. Przygotowano asercje dystrybucji i migracji do T-25. Testy, binaria i walidator: **NOT RUN — PROHIBITED AT THIS STAGE**.

**Zależności:** T-24. **Kontrakt:** BR-02/08, K-03/05/06. Ta korekta zastępuje historyczny kontrakt dystrybucji Python. Nie obejmuje uruchomienia T-25 ani migracji prawdziwych projektów.

Przegląd wykrył także Python osadzony w `common/skills/test-gap-discovery/scripts/inventory-tests.sh`. Dodano piąte binarium `test-inventory`, instalowane jako `scripts/inventory-tests` tego skilla; historyczny skrypt pozostaje w repozytorium. Aktualizacja usuwa starą zarządzaną kopię `.sh` zgodnie z hashami. Przygotowane testy obejmują dystrybucję, 0755, brak binarium przed zapisem, migrację starego adaptera, idempotencję i odmowę przy lokalnych zmianach. `cargo fmt` oraz `cargo check --locked --offline --lib --bins`: **PASSED**. Testów ani binariów nie uruchamiano.

### T-25 — jedno pełne uruchomienie walidacji

**Stan: PASSED — ponowienie 2026-10-21 po osobnym zleceniu użytkownika „To napraw”.** Usunięto podwójny import, uproszczono nazwy wariantów `CodecError`, zastąpiono krotki wyniku MCP strukturą `McpConfiguration`, zebrano dane budowania stanu w `InstallationState` oraz poprawiono `ok_or`, filtrowanie i stabilne sortowanie. Zachowano komunikaty błędów, strukturę JSON i kolejność mapowań. Nie wyciszano reguł Clippy ani nie usuwano testów.

Wykonano jedno ponowienie `make validate-rust-migration`: format, release build, Clippy, walidacja źródeł i testy **PASSED**. Pełny log: [core/rust/validation-t25-retry1.log](core/rust/validation-t25-retry1.log). Łącznie 22 testy, 0 niepowodzeń, 0 pominiętych. Wszystkie 11 wymaganych grup zawiera testy. Pokrycie przygotowanego zestawu:

| Grupa | Kontrakt | Testy PASSED |
| --- | --- | --- |
| foundation_tests | K-01 | 3 |
| profiles_tests | K-02 | 1 |
| mcp_config_tests | K-03 | 3 |
| mcp_runtime_tests | K-04 | 1 |
| lifecycle_tests | K-05, T-24a | 4 |
| cli_tests | K-06 | 1 |
| mariadb_tests | K-07 | 2 |
| wordpress_dump_tests | K-08 | 2 |
| wordpress_preflight_tests | K-09 | 1 |
| wordpress_localize_tests | K-09 | 2 |
| restore_tests | K-10 | 2 |

Wynik dotyczy przygotowanych scenariuszy Rust; nie jest dowodem pełnej zgodności 1:1 każdego zachowania historycznej implementacji. Nie uruchamiano Python, prawdziwych Docker/DDEV/DB/HTTP ani operacji na rzeczywistych projektach.

**Historia: FAILED — pierwszy przebieg 2026-10-21.** Na jawne zlecenie użytkownika uruchomiono jeden raz `make validate-rust-migration`. Doinstalowano wymagany komponent Clippy. Pełny log: [core/rust/validation-t25.log](core/rust/validation-t25.log).

| Etap | Wynik |
| --- | --- |
| Formatowanie | PASSED |
| Release build wszystkich binariów | PASSED |
| Clippy `--all-targets -- -D warnings` | FAILED, exit 101 |
| `make validate` | NOT RUN — zatrzymanie po Clippy |
| `make test` | NOT RUN — zatrzymanie po Clippy; nie uruchomiono żadnego testu |

Przyczyny: podwójny import `std::ffi::OsString` w `core/rust/tests/cli.rs:1,3` (E0252 i unused import) oraz osiem naruszeń Clippy: wspólny sufiks wariantów `CodecError`, dwa złożone typy zwracane w `mcp/config.rs`, dziewięć argumentów `build_updated_state`, zbędne `ok_or_else` w `profiles.rs`, `filter_map` z `bool::then` w `status.rs`, porównanie z pustym stringiem i `sort_by` zamiast `sort_by_key` w `wordpress/mapping.rs`. Nie zmieniano kodu po wyniku, nie uruchamiano pojedynczych testów ani kolejnego przebiegu. Zgodność zachowania pozostaje niepotwierdzona.

**Zależności:** T-24 i T-24a; wszystkie wcześniejsze zadania muszą być `IMPLEMENTED`. **Kontrakt:** wszystkie K-01…K-10.

Uruchomić z root repozytorium **jedno polecenie**:

```bash
make validate-rust-migration
```

Jest to jeden zbiorczy przebieg, a nie jeden smoke test. Cel `validate-rust-migration` wykonuje sekwencję Rust-only opisaną w T-24: format, release build, Clippy, `make validate` i `make test`. Żaden etap nie uruchamia interpretera Python. Błędy kompilacji/przygotowania blokują dalsze wykonywanie. Raport musi odróżniać `FAILED` od grup `NOT RUN` po wcześniejszym przerwaniu.

**Gotowe, gdy:** wszystkie grupy `PASSED`, każda ma niezerową liczbę testów, kompletna tabela pokrycia, brak nieuzasadnionych skipów, źródła Python zachowane w repozytorium i natywny payload zgodny z T-24a. Przy dowolnym FAIL: oznaczyć T-25 `FAILED`, zachować zmiany i raport, zatrzymać się. Nie deklarować sukcesu ani automatycznie uruchamiać kolejnych prób; poprawki i ponowienie tego samego finału wymagają kolejnego zlecenia.

## Informacje brakujące i blokady

**Dawna blokada BR-08 — rozwiązana implementacyjnie w T-24a na osobne zlecenie użytkownika.** Binaria Rust zastępują adapter i narzędzia Python w payloadzie i konfiguracji. Potwierdzenie zachowania pozostaje odroczone do T-25; nie deklarować zweryfikowanej zgodności przed walidacją.

**Na moment zapisania planu nie stwierdzono materialnego braku decyzji blokującego T-01.** Dalsze uruchomienie kompilatora/resolvera nie zostało sprawdzone; brak narzędzi lub niezgodny lock jest blokadą środowiskową, nie zgodą na zmianę architektury.

Jeżeli pojawi się nieopisana różnica między dokumentacją, Pythonem i oczekiwaniem 1:1, agent wpisuje tu: dokładne pytanie, pliki/dowody, dotknięty kontrakt i zadanie, możliwy wpływ na dane oraz warunek odblokowania. Takie zadanie i jego zależności otrzymują `BLOCKED`. **Nie wolno wybierać interpretacji, zmieniać kontraktu ani usuwać testu, aby ukończyć etap.** Jeżeli wpływu nie da się odizolować, zatrzymać całą realizację.

## Ryzyka i świadomie wyłączony zakres

| Ryzyko | Skutek | Obsługa |
| --- | --- | --- |
| Testy dopiero na końcu — świadoma decyzja użytkownika. | Błędy semantyczne mogą przejść przez kilka etapów. | Małe zadania, statyczna lista pokrycia, kompilacja bez uruchamiania; brak deklaracji zgodności przed T-25. |
| Pythonowe fixture'y mają znane błędne ścieżki. | Historyczne Python suite nie jest częścią migracyjnej walidacji po BR-08. | Nie uruchamiać ani nie modyfikować Pythonowych fixture'ów w T-24/T-25. |
| Różnice JSON, quoting, regex, symlinków i sygnałów. | Rozbieżne hashe, błędny target lub niedziałające recovery. | Jawne K-01/K-04/K-07/K-10 i końcowe scenariusze negatywne; nie zastępować ich porównaniem samych exit codes. |
| Wykonanie restore/buildu na prawdziwym środowisku. | Utrata danych, ujawnienie sekretów, niezamierzone pobrania. | Brak uruchomień programów w etapach implementacji; w finale wyłącznie temp/mock/fake i brak prawdziwego DB/Docker/DDEV/HTTP. |
| Nowy Rust target skopiowany do każdego fixture'a. | Niepotrzebny wzrost czasu i dysku. | Rust testy używają tymczasowych fixture'ów danych, bez kopiowania targetu. |

Poza zakresem: usuwanie zachowanych źródeł Python, zmiana wersji profili, nowe usługi, async/FFI, cache wyników testów, równoległość testów, migracja upstreamowego serwera MariaDB/PHP helpers, tworzenie nowego pipeline wdrożeniowego oraz obietnica przyspieszenia bez pomiarów release. Dystrybucja natywnych runtime jest objęta T-24a.

## Następne działanie

**T-25 PASSED po osobno zleconych poprawkach i jednym ponowieniu.** Wszystkie etapy walidatora i 22 przygotowane testy przeszły. Brak kolejnego zadania w planie. Nie wykonywano instalacji ani operacji na rzeczywistych projektach.
