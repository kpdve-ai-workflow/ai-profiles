# Propozycja docelowego logowania diagnostycznego

## Cel i zasady

Wspólna warstwa oparta na `tracing` ma wspierać development zarówno użytkownika, jak i agenta. Priorytetem jest odtworzenie przepływu wykonania, danych rzeczywiście użytych lub przetworzonych w operacji oraz konkretnej przyczyny awarii. Diagnostyka obejmuje poprawne i niepoprawne przebiegi; domyślne ustawienia developerskie mają zapewniać pełną użyteczną diagnostykę, bez konieczności ponownego uruchamiania z `DEBUG`.

Logowanie ogranicza się do danych związanych z wykonywaną operacją: nie skanuje niepowiązanych plików ani nie zbiera danych spoza jej zakresu. Nie stosuje domyślnego maskowania, pomijania payloadów ani cichego obcinania treści. Logi pozostają lokalnie w repozytorium, w katalogu przeznaczonym do ignorowania przez Git. Obecny zakres nie obejmuje wysyłki sieciowej. Grafana Alloy/Loki mogą być osobnym, przyszłym etapem, po odrębnej decyzji o eksporcie.

## Obecny kontekst i architektura

| Obszar | Obecna sytuacja |
| --- | --- |
| `core/rust/src/error.rs` | Typowane błędy `thiserror` — warto zachować strukturę i łańcuch przyczyn. |
| `core/rust/src/profile_management/cli.rs:427` | Błędy są zamieniane na tekst; traci się strukturę i dostęp do przyczyn. |
| `core/rust/src/process.rs` | Wspólny runner procesów — naturalny punkt korelacji, przechwytywania strumieni, czasu, timeoutów i kodów wyjścia. |
| `core/rust/src/profile_management/scoped.rs` | Operacje mogą zakończyć się częściowym sukcesem; trzeba zapisywać wynik każdego klienta i wynik całości. |
| `core/rust/src/mcp/novamira.rs:110` | Obecnie wycisza szczegółowe logi mostu; docelowo potrzebna jest ich diagnostyczna integracja opisana niżej. |

Logger nie przechwytuje automatycznie każdego `Result::Err`. Potrzebne są jawne punkty zapisu błędów oraz zdarzeń poprzedzających awarię. Błąd należy rejestrować przy ostatecznej obsłudze, a etapy wzbogacać kontekstem bez wielokrotnego powielania tego samego błędu. Przy `CLIENT=all` wynik poszczególnych klientów i podsumowanie operacji są odrębnymi zdarzeniami.

```text
CLI / MCP / restore / launcher / runner procesów
                    │
          spans i zdarzenia tracing
                    │
       wspólny schemat i konfiguracja
                    │
     lokalne JSONL i pliki payloadów
```

Proponowane zależności: `tracing` (zdarzenia i spans), `tracing-subscriber` (format JSON i konfiguracja), `tracing-appender` lub równoważny writer (zapis/rotacja). Wersje muszą zachować Rust 1.85. Wspólny moduł, np. `core/rust/src/logging.rs`, odpowiada za inicjalizację raz na poziomie binarium, schemat, korelację i obsługę writerów; nie należy inicjalizować go w funkcjach bibliotecznych ani tworzyć osobnego frameworka. Nie dodawać Tokio wyłącznie do logowania.

## Kontrakt diagnostyczny

Każdy rekord powinien pozwalać ustalić czas UTC, poziom, nazwę i wersję binarium, `run_id`, operację, klienta/profil/projekt (gdy rozpoznane), etap, wynik, czas trwania oraz stabilny `event` i `error_code`. Dla błędu zachować typ, pełny łańcuch przyczyn i dostępne szczegóły diagnostyczne, nie tylko bezpieczne podsumowanie. Pola strukturalne JSON są preferowane dla indeksowalnego kontekstu; pełne treści mogą być powiązanymi fragmentami lub plikami payloadów.

Zapisywać przebieg poprawnych operacji, w tym:

- wejścia i wyjścia etapów, rozpoczęcia/zakończenia, kolejność i czasy;
- dane przekazane etapom i uzyskane z nich, gdy rzeczywiście użyte lub przetworzone;
- decyzje, warunki, pominięcia, ponowienia, fallbacki i ich przyczyny;
- zmiany konfiguracji/stanu i pozostawiony stan po częściowym sukcesie;
- uruchomienie procesów, argumenty i środowisko przekazane, wejścia/wyjścia strumieni, sygnały, timeouty, anulowanie i status zakończenia;
- konflikty, zapisy, wyniki poszczególnych klientów i końcowy rezultat.

Nie wykluczać domyślnie środowiska procesów ani odczytanych plików `.env`, pełnych argumentów, stdin/stdout/stderr podprocesów, SQL, przetwarzanych dumpów, komunikatów MCP, żądań/odpowiedzi HTTP ani struktur diagnostycznych wypisywanych przez `Debug`. Pełne dane logować bez cichego filtrowania lub obcięcia; binaria i dane niebędące tekstem muszą mieć reprezentację odtwarzalną. Diagnostyka nie może zmieniać danych przekazywanych właściwej operacji ani jej wyniku.

Przykładowy rekord zdarzenia (pola przykładowe, a nie pełny schemat):

```json
{"timestamp":"2026-10-25T12:00:00Z","level":"ERROR","event":"activation.failed","run_id":"…","binary":"ai-profile","operation":"update","client":"codex","stage":"plugin_add","error_code":"process.timeout","duration_ms":30000,"prepared_state_retained":true,"cause":"…"}
```

Użytkowe komunikaty CLI pozostają niezależne; nie zamieniać mechanicznie `println!` na logi. Logowanie diagnostyczne nie może zanieczyszczać stdout protokołu MCP, wyników JSON ani interaktywnego wyjścia CLI.

## Novamira i granice obserwowalności

Zastąpić wyciszanie szczegółów mostu przechwytywaniem dostępnej diagnostyki: surowych komunikatów, błędów i danych żądań/odpowiedzi, które rzeczywiście są dostępne. Sam sklasyfikowany komunikat typu „uwierzytelnienie odrzucone” nie wystarcza.

Rozróżnić dwa poziomy:

1. **Wrapper Rust:** może rejestrować dane, które sam tworzy, odbiera lub przekazuje — argumenty, konfigurację wejściową, komunikaty/odpowiedzi widoczne na granicy wrappera, błędy mostu oraz kody zakończenia.
2. **Most/kontener:** wewnętrzna komunikacja HTTP, surowe żądania/odpowiedzi lub błędy niewystawiane wrapperowi wymagają punktu integracji w moście (np. hooka/interceptora transportu lub jawnego kanału diagnostycznego) albo zbierania logów kontenera. Dodanie `tracing` w Rust samo nie zapewni dostępu do całego ruchu HTTP. Należy udokumentować, które dane są faktycznie dostępne, a które wymagają zmian w moście lub środowisku uruchomieniowym.

Szczegółowe logi mostu i kontenera kierować do lokalnego pliku/kanału zbierającego, nie do stdout protokołu MCP. Kanały diagnostyczne nie mogą przejmować ani zmieniać standardowych strumieni protokołu.

## Lokalizacja, retencja i kompletność

Logi przechowywać wewnątrz repozytorium, w dedykowanym katalogu. Dla samego menedżera `ai-profile` obowiązuje już `<checkout źródłowy ai-profiles>/.ai-profiles-logs/` i główna reguła `/.ai-profiles-logs/` w `.gitignore`. Lokalizacja zainstalowanych runtime pozostaje nierozstrzygnięta. `.gitignore` zapobiega przypadkowemu śledzeniu przez Git, ale nie jest ochroną przed odczytem ani dostępem HTTP. Zachować uprawnienia katalogu `0700` i plików `0600`, bez podążania za symlinkami.

Sposób wyznaczenia katalogu zależy od sposobu uruchomienia:

- menedżer używa istniejącego `RepositoryContext::discover`: przodkowie położenia binarium, rozpoznanie struktury źródeł, bez wymogu `.git`. Nie używa PROJECT ani CWD do lokalizacji logów;
- dla menedżera brak rozpoznanych źródeł lub błąd przygotowania loggera oznacza jedno ostrzeżenie `WARNING: diagnostics unavailable: …` na stderr i kontynuację CLI z dotychczasowym wynikiem. Nie ma fallbacku do HOME, XDG, `/tmp` ani CWD;
- dla zainstalowanych runtime wybór lokalizacji oraz zachowania przed rozpoznaniem projektu nadal wymaga osobnej decyzji. Nie przenosić automatycznie kontraktu menedżera na runtime.

**Decyzje do ustalenia dla runtime:** nazwa i reguła lokalnego katalogu; algorytm wykrywania checkoutu (w tym podkatalogi, worktree, projekty bez Git); zachowanie przed wykryciem projektu; czy w takim stanie operacja jest kontynuowana z oznaczoną niekompletnością, czy zatrzymywana; granice współdzielenia katalogu między równoległymi uruchomieniami. Dla menedżera każde wywołanie ma osobny plik JSONL i wspólny dla jego rekordów `run_id`; atomowe create-new zapobiega nadpisaniu, a kolizja nazwy powoduje próbę następnego identyfikatora w tym samym katalogu.

Pełne payloady zmieniają profil zasobów. Writer ma przechwytywać strumieniowo, z ograniczonym zużyciem RAM i backpressure albo kontrolowanym zapisem bezpośrednim; nie gromadzić dowolnie dużych wejść/wyjść w pamięci. JSONL zawiera uporządkowane rekordy/chunki z identyfikatorem payloadu, kolejnością, kodowaniem i informacją o kompletności. Duże dane mogą być w powiązanych plikach w tym samym ignorowanym katalogu, z długością i skrótem kontrolnym pozwalającym zweryfikować odtworzenie. Dane binarne i niepoprawne UTF-8 przechowywać bezstratnie (np. jako bajty/base64 lub plik binarny), nie zastępować ich stratną konwersją tekstową.

Zaprojektować rotację i retencję z jawnym limitem zajętości/wieku oraz sprzątaniem wyłącznie rozpoznanych, nieaktywnych plików loggera. Rotacja nie może po cichu obcinać ani porzucać aktywnego payloadu: kontynuować w nowym segmencie lub powiązanym pliku i zapisać relację między fragmentami. Gdy limit, miejsce na dysku lub błąd I/O uniemożliwi kompletne logowanie, jawnie zarejestrować/zgłosić niekompletność, o ile kanał raportowania jest dostępny. Nie obiecywać gwarantowanego zapisu przy `SIGKILL`, awarii systemu lub pełnym dysku.

Wynik operacji i kompletność logów to osobne wyniki. Awaria loggera domyślnie nie zmienia semantyki instalacji/importu ani pierwotnego błędu operacji, ale musi dawać widoczny sygnał niekompletności, bez podszywania się pod sukces diagnostyczny. Sposób sygnalizacji, gdy stderr jest stdoutem protokołu lub niedostępny, wymaga jawnego projektu; diagnostyka MCP ma trafiać do pliku, nie do stdout protokołu.

## Procesy i przekazanie sterowania

Obecne `std::process::exit` pomija zwykły cleanup, a `exec()` zastępuje proces Rust programem docelowym. Nie można więc zakładać, że bieżący logger zobaczy dalsze wykonanie. Preferować wspólny mechanizm nadzorowania procesów tam, gdzie kontrola procesu jest potrzebna: zachować semantykę stdio (bez przejmowania protokołu lub interaktywnego wejścia), sygnałów, kodów wyjścia, timeoutów, anulowania i cleanupu kontenerów; korelować zdarzenie uruchomienia z wyjściem i zakończeniem. Jeśli interaktywne lub protokołowe stdio musi pozostać bezpośrednie, zastosować osobny kanał diagnostyczny/collector, który nie modyfikuje bajtów przepływu.

Tam, gdzie `exec()` jest wymagany, rozważyć osobny nadzorujący launcher lub niezależny collector, który zapisuje diagnostykę procesu potomnego i jego zakończenia, zachowując przekazywanie sygnałów, kodu wyjścia i cleanup. Każde przekazanie musi mieć korelacyjny identyfikator dostępny po obu stronach. Nie pozostawiać awarii po `exec()` poza zakresem docelowym. Zapis przed przekazaniem jest użyteczny, ale nie gwarantuje przechwycenia późniejszej awarii. `panic` hook może uzupełniać diagnostykę, nie zastępuje nadzorowania i nie chroni przed `SIGKILL`, awarią systemu ani brakiem miejsca.

Kolejkę/buforowanie dobrać po pomiarze przy pełnych payloadach. Rozważyć ograniczony bufor/asynchroniczny writer z backpressure i jawnie kontrolowanym flush przy normalnym zakończeniu; unikać nieograniczonej pamięci i utraty końcowych rekordów. Granice `exit()`/`exec()` wymagają flush/acknowledgement lub zewnętrznego nadzorcy; nawet wtedy nie gwarantuje się trwałości przy nagłej awarii.

## Etapy realizacji

### Ograniczony fundament JSONL — 2026-10-25

- Dodano `core/rust/src/logging.rs`: `build_subscriber(directory, filename, RunMetadata)` zwraca synchroniczny subscriber, bez globalnej instalacji. Standardowy formatter JSON `tracing-subscriber` zachowuje UTC, poziom, `fields.event`, pola i spans; dołączane są `binary`, `version`, `run_id`. Domyślnie TRACE, bez maskowania/obcinania. Mutex serializuje całe rekordy, bez kolejki, wątku ani Tokio.
- Plik jest wyłącznie nowo tworzony (0600), nowe katalogi mają 0700. Operacje deskryptorowe `rustix` nie podążają za symlinkami w żadnym komponencie; istniejące pliki nie są nadpisywane. Katalog i pojedynczą nazwę pliku podaje wywołujący; błędy przygotowania są zwracane bez fallbacku. `..` jest odrzucane, uprawnienia istniejących katalogów pozostają bez zmian.
- Test-first wyłącznie w VM: po początkowym błędzie Incus ponowne `make test-vm-up` potwierdziło gotowość. `make test` dało RED przez brak `crate::logging` (artefakty `20261025T154825Z-ea34671b-5aa1-46a7-9be7-a607479e2d3c`). Po implementacji wszystkie 7 `logging_tests` przeszło. Rejestracja jest jawna w `src/lib.rs`; syntetyczne testy obejmują JSONL/metadane/spans, TRACE, dokładne wartości z Unicode i nowymi liniami, 640 równoległych rekordów, uprawnienia, symlinki, kolizje i błędy przygotowania. Bez globalnej inicjalizacji i rzeczywistego HOME.
- `make validate-rust-migration` w VM: po poprawieniu kolejności modułów format, release-build, Clippy i make-validate **PASS**; make-test **FAIL** (82 przeszły, 3 błędy Novamiry: dwa brakujące fixture Dockerfile w lifecycle i niezgodna ścieżka launchera w novamira_bundle). Te same błędy odtworzono przez `make test` na niezmienionym HEAD w osobnej kopii VM (tam dodatkowo błąd cleanup kontenera; próbę z wadliwie umieszczonym TMPDIR powtórzono poza kopią). Logi: `.test-vm/artifacts/20261025T155423Z-1d0fe859-4910-4eef-aec4-9eb6f122b2c0/export/operation.log` i `baseline-head.log`. **Bramka niezaliczona**, bez lokalnego fallbacku i zmian Novamiry.
- Nowe zależności deklarują MSRV ≤ 1.85; kod nie wymaga nowszych funkcji języka. Pełnego projektu nie zweryfikowano kompilatorem 1.85: zastany lockfile zawiera ICU wymagające 1.88 i idna_adapter wymagający 1.86. Nie aktualizowano istniejących wersji zależności poza dodaniem loggera.
- Późniejszy wynik z raportu i artefaktu `.test-vm/artifacts/20261025T161429Z-922def68-5a03-4811-ace3-df8b44ed67a1/export/operation.log`: `make validate-rust-migration` w VM **PASS**, wszystkie grupy zaliczone, **85/85** testów (0 failed/ignored). Odczyt artefaktu potwierdził raport; wcześniejsze niepowodzenia powyżej pozostają historią. Ten log nie podaje wersji kompilatora i sam nie potwierdza zgodności Rust 1.85.
- **Etap 1 pozostaje nieukończony.** Poza tym fragmentem pozostają inicjalizacja binariów i instrumentacja, wykrywanie repozytorium, zachowanie przed jego rozpoznaniem, rotacja/retencja, duże/binarnie kodowane payloady, docelowa sygnalizacja niekompletności oraz `.gitignore`. Logger nie został uruchomiony na rzeczywistych danych. Nie rozpoczęto kolejnego zadania.

### Przywrócenie i weryfikacja Rust 1.85 — 2026-10-25

- Kontrakt/oracle: deklarowane `rust-version = "1.85"` wymaga rzeczywistej kompilacji binariów i wykonania testów na 1.85.0, nie tylko zgodnych deklaracji MSRV. Scenariusze: zastany lockfile + 1.85.0 → odtworzenie niezgodności; poprawiony lockfile + 1.85.0 → release binaries i pełne testy `--locked`; regresja → obowiązkowa bramka. Taki sprawdzian wykrywa realistyczny błąd: lockfile działający na stable, ale nie na minimalnym kompilatorze. Nie zmieniano ani nie osłabiano testów.
- Środowisko: przygotowana VM Incus Debian 13, `tester` UID/GID 1000:1000, sprawdzony marker `/var/lib/ai-profiles-vm/ready`. Najpierw `INCUS_SOCKET=/run/incus/unix.socket make test-vm-sync` (**PASS**). Dodatkowe polecenia wykonywano pod wspólnym `flock -n .test-vm/lock`, przez `INCUS_SOCKET=/run/incus/unix.socket incus --force-local --project default exec "$(jq -r .name .test-vm/instance.json)" -- runuser -u tester -- env -i HOME=/home/tester USER=tester PATH=/home/tester/.cargo/bin:/usr/local/bin:/usr/bin:/bin bash -c '…'`. Cargo uruchamiano wyłącznie w VM, z katalogu `/home/tester/ai-profiles`; bez zmiany domyślnego toolchaina i bez hostowego Cargo.
- **RED przed zmianą**: w VM wykonano `rustup show; rustc -Vv; cargo -V; rustup toolchain install 1.85.0 --profile minimal; rustc +1.85.0 -Vv; cargo +1.85.0 -V; cargo +1.85.0 build --manifest-path core/rust/Cargo.toml --locked --release --bins`. Faktycznie użyto **rustc 1.85.0 (4d91de4e4 2025-02-17)** i **cargo 1.85.0 (d73d2caf9 2024-12-31)**, host target `x86_64-unknown-linux-gnu`. Build zakończył się kodem **101**: `rustc 1.85.0 is not supported` przez `icu_collections`, `icu_locale_core`, `icu_normalizer`, `icu_normalizer_data`, `icu_properties`, `icu_properties_data` 2.3.0 oraz `icu_provider` 2.3.1 (wymagają Rust 1.88), a także `idna_adapter` 1.2.2 (wymaga 1.86). To odrzucenie zależności przez Cargo przed kompilacją projektu, nie błąd infrastruktury. Pełny log: `.test-vm/msrv-before.log`.
- Łańcuch potwierdzony w VM poleceniami `cargo +1.85.0 tree --manifest-path core/rust/Cargo.toml --locked -i idna_adapter` oraz analogicznie `-i icu_provider`: `ai-profiles-core → url 2.5.8 → idna 1.1.0 → idna_adapter 1.2.2 → icu_normalizer / icu_properties → pozostałe ICU`. Wcześniejsza diagnoza została potwierdzona rzeczywistym Rust 1.85.0.
- Minimalna korekta: tylko osiem niezgodnych pakietów w `core/rust/Cargo.lock`. W VM wykonano `cargo +1.85.0 --config 'resolver.incompatible-rust-versions="fallback"' update --manifest-path core/rust/Cargo.toml -p idna_adapter -p icu_collections -p icu_locale_core -p icu_normalizer -p icu_normalizer_data -p icu_properties -p icu_properties_data -p icu_provider`. Resolver wybrał najnowsze zgodne z 1.85 wersje: `idna_adapter` **1.2.2 → 1.2.1**, `icu_collections`, `icu_locale_core`, `icu_normalizer`, `icu_normalizer_data` **2.3.0 → 2.1.1**, `icu_provider` **2.3.1 → 2.1.1**, `icu_properties`, `icu_properties_data` **2.3.0 → 2.1.2**. Inne wersje bez zmian; `url` i `idna` bez zmian. Log drzewa i rozwiązywania: `.test-vm/msrv-resolution.log`. Wygenerowany lockfile przeniesiono do checkoutu i porównano bajtowo (`cmp`, PASS). `Cargo.toml`, w tym `rust-version = "1.85"`, pozostaje bez zmian; fallback zastosowano tylko do tej celowanej operacji, bez trwałej konfiguracji i masowej aktualizacji.
- **GREEN na Rust 1.85.0**: w VM `export RUSTUP_TOOLCHAIN=1.85.0; rustc -Vv; cargo -V`, następnie `cargo build --manifest-path core/rust/Cargo.toml --locked --release --bins` (**PASS**, 17.63 s) i `cargo test --manifest-path core/rust/Cargo.toml --locked --all-targets` (**PASS**, **85/85**, 0 failed/ignored, 41.52 s wykonania testów). Wspólny wybór toolchaina obejmował także buildy wywoływane przez testy. Świeży TMPDIR: `/home/tester/test-runs/msrv-0609a95a-275d-4225-9ddf-3a485bbe7510/tmp`. Log poleceń, wersji i wyników: `.test-vm/msrv-after.log`. SHA-256 lockfile przed i po: `c73d7fe4f567ba2b6c315a9ef0114022ed2985582255a0faba748b307c900e8e`.
- **Bramka po zmianie zależności**: `INCUS_SOCKET=/run/incus/unix.socket make validate-rust-migration` (**PASS**, kod 0): format, release-build, Clippy `-D warnings`, make-validate oraz make-test (**85/85**). Artefakt: `.test-vm/artifacts/20261025T162504Z-29bd1927-e3ce-43df-a588-9a41e52ee9de/export/operation.log`; log hosta także `.test-vm/msrv-gate-host.log`. Bramka działała na domyślnym **rustc 1.99.1 (58cf8f9c3 2026-10-13)** / **cargo 1.99.1 (5f94df478 2026-08-27)** — potwierdzono toolchain przed próbami oraz po bramce (`rustup show active-toolchain; rustc -Vv; cargo -V` i `core/rust/target/.rustc_info.json`, log `.test-vm/msrv-gate-toolchain.log`). SHA lockfile identyczny. Ten PASS jest regresją na nowszym kompilatorze; dowodem zgodności 1.85 jest osobny GREEN powyżej, nie sama bramka.
- Zadanie zgodności **zweryfikowane dla Linux x86_64 w wymaganej VM**. Bez `--ignore-rust-version`, zmian loggera, Novamiry ani orkiestracji VM. **Cały etap 1 loggera nadal nieukończony; nie rozpoczęto następnego zadania.**

### Podłączenie JSONL wyłącznie do menedżera — 2026-10-25

- **Wykonany zakres:** punkt wejścia `src/bin/ai-profile.rs` przygotowuje i instaluje scoped subscriber raz, przed parsowaniem. Zapisuje `cli.started` i `cli.finished` z rzeczywistym `exit_code`; kończy zakres przed zwróceniem `ExitCode` (dotychczasowe 0/1/2). Ostateczna obsługa CLI zapisuje `cli.failed` z dostępnym `message` oraz `error_kind = arguments|operation`. Wspólne metadane loggera zapewniają ten sam `run_id`. Pozostawiono dotychczasowe stdout i komunikaty użytkowe. Przygotowanie korzysta z istniejącego wykrywania źródeł i istniejącego bezpiecznego writera; bez nowej zależności i bez zmian `logging.rs` ani algorytmu `discover`.
- **Oracle i scenariusze przed implementacją:** kontrakt tego kroku i istniejący kontrakt użytkowego CLI. Prawdziwe binarium w checkoutach bez `.git`: create → sukces, ponowne create → błąd operacji; nieznany argument, niedozwolona kombinacja i brak nazwy → błąd argumentów; brak argumentów/`--help`/`help` → pełna pomoc. Asercje obejmują kolejność i liczbę rekordów, kody, komunikaty, korelację i dokładne bajty stdout. Obcy CWD i odrębny PROJECT nie zmieniają lokalizacji; 24 równoległe wywołania dają 24 pliki i identyfikatory. Plik blokujący katalog loggera nie zmienia sukcesu, pomocy ani kodów błędów; jeden warning na wywołanie, bez fallbacku. Binarium poza źródłami, nawet uruchomione z poprawnego checkoutu jako CWD, ostrzega zamiast odkrywać źródła z CWD. To wykrywa m.in. inicjalizację po parserze, zlewanie plików, gubienie zakończenia i maskowanie pierwotnego wyniku awarią loggera.
- **Izolacja:** `tests/manager_logging.rs` i `tests/manager_fixture.rs`, jawna rejestracja w `src/lib.rs`; żadnego globalnego subscribera w procesie testów. Syntetyczne HOME/XDG/TMPDIR/CWD/PROJECT nowych wywołań, jednorazowe checkouty i prawdziwe procesy. Istniejące trzy miejsca uruchamiania menedżera w testach Novamiry przeniesiono do kopii źródeł z payloadami binarnymi, bez zmiany ich asercji. Cała bramka również w jednorazowej kopii, bo `make validate` uruchamia menedżer. Bez testowych logów w roboczym checkoutcie lub rzeczywistym HOME.
- **RED:** po `INCUS_SOCKET=/run/incus/unix.socket make test-vm-sync` uruchomiono w przygotowanej VM `cargo test --manifest-path core/rust/Cargo.toml --locked --lib manager_logging -- --nocapture` (świeży TMPDIR). **0/5 PASS, kod 101**, brak katalogu logów lub wymaganego ostrzeżenia, przy poprawnie zbudowanym i uruchomionym binarium. Log `.test-vm/manager-red.log`. Następnie po podłączeniu **5/5 PASS** (`.test-vm/manager-green.log`). Później dodano osobną kontrolę PROJECT i wzmocniono asercje komunikatów/błędów przy niedostępnej diagnostyce.
- **Pierwsza pełna bramka:** `.test-vm/manager-gate.log`: format/release/Clippy/validate PASS, testy **87/91**, kod 2. Diagnoza: brak `test-inventory` w nowej kopii payloadów oraz wyścig fixture `ETXTBSY` przy kopiowaniu wykonywalnych plików w wielowątkowym procesie testów. Uzupełniono payload i przeniesiono kopiowanie binariów do procesu `cp`, który zamyka deskryptory przed powrotem. Bez pomijania testów ani osłabiania asercji.
- **GREEN / bramka:** w VM Debian 13 amd64, jako `tester` 1000:1000, marker `/var/lib/ai-profiles-vm/ready`, wspólny hostowy `flock -n .test-vm/lock`, lokalny Incus z `INCUS_SOCKET=/run/incus/unix.socket`. Po synchronizacji wykonano `cargo fmt --manifest-path core/rust/Cargo.toml -- --check`, następnie `make validate-rust-migration` z głównego katalogu kopii `/home/tester/test-runs/manager-gate-noik9WPg/checkout`, z osobnym TMPDIR. **PASS, kod 0**: format, release-build, Clippy `-D warnings`, make-validate i **91/91 testów** (6 nowych, 0 failed/ignored). Toolchain **rustc 1.99.1 / cargo 1.99.1**. Log `.test-vm/manager-gate-after.log`.
- **Rust 1.85.0:** w tej samej jednorazowej kopii VM, osobny świeży TMPDIR i `RUSTUP_TOOLCHAIN=1.85.0` dla całego drzewa procesów: `rustc -Vv`, `cargo -V`, `cargo build --manifest-path core/rust/Cargo.toml --locked --release --bins`, `cargo test --manifest-path core/rust/Cargo.toml --locked --all-targets`. **PASS, kod 0, 91/91**, w tym wszystkie 6 nowych testów. Faktyczne wersje **rustc 1.85.0 (4d91de4e4)** / **cargo 1.85.0 (d73d2caf9)**; log `.test-vm/manager-msrv.log`. Końcowe kontrole braku `/home/tester/ai-profiles/.ai-profiles-logs` i `/home/tester/.ai-profiles-logs` również PASS. Cargo/buildy/testy wyłącznie w VM; nie zmieniano zastanego lockfile.
- **Ten ograniczony krok zweryfikowany; cały etap 1 pozostaje nieukończony.** To nie jest kompletna diagnostyka systemu. Bez instrumentacji innych binariów/runtime, etapów, procesów, wejść, środowiska, strumieni i payloadów; bez panic hooka, retencji/rotacji, zmian Novamiry, `exec()` ani eksportu sieciowego. Nie rozstrzygnięto lokalizacji runtime i nie rozpoczęto następnego zadania.

1. Domknąć kontrakt lokalizacji runtime, zachowania przed wykryciem projektu, retencji i obsługi niekompletności oraz pozostałą wspólną konfigurację/schemat i obsługę payloadów. Fundament JSONL i podłączenie samego menedżera opisano powyżej; nie oznacza to ukończenia etapu.
2. Pokryć operacje, etapy, zmiany stanu, błędy typowane, procesy, restore i przypadki częściowego sukcesu; osobno uwzględnić binaria kończące przez `exit()`.
3. Dodać wymagane punkty diagnostyczne Novamiry po stronie Rust oraz mostu/kontenera; zweryfikować zakres widoczności i brak zakłócenia MCP.
4. Domknąć obserwowalność po granicach `exec()` przez nadzorowanie lub osobne zbieranie, po wyborze zachowania zachowującego semantykę uruchamianych programów.
5. Grafana Alloy/Loki pozostawić jako opcjonalny etap przyszły. Wysyłka poza środowisko lokalne wymaga osobnej decyzji i nie jest częścią domyślnej konfiguracji.

Jeśli później zatwierdzony zostanie eksport do Loki, używać etykiet o małej liczbie wartości (np. binarium, poziom); `run_id`, ścieżki i identyfikatory projektów pozostawić w JSON, nie jako etykiety o wysokiej kardynalności.

## Kryteria akceptacji przyszłej implementacji

Testy projektować test-first, z niezależnym kontraktem, scenariuszami przed dopasowaniem do implementacji i asercjami wykrywającymi wiarygodne błędne zachowanie. Nie pisać ani nie uruchamiać testów w ramach tej propozycji. Przyszłe kryteria powinny potwierdzić co najmniej:

- na kontrolowanych danych pełne argumenty i środowisko procesu oraz stdin/stdout/stderr są odtwarzalne, a logowanie nie zmienia przekazanych bajtów;
- poprawne przebiegi rejestrują wejścia/wyjścia etapów, decyzje, pominięcia, retry, fallbacki, zmiany stanu, kolejność i czasy;
- duże payloady, fragmenty rotowane, dane binarne i niepoprawne UTF-8 dają się odtworzyć dokładnie; sprawdzane są kolejność i kompletność, bez cichego obcięcia;
- przekazywanie danych MCP/CLI/podprocesów i ich stdio, sygnały, kody wyjścia, timeouty i cleanup zachowują dotychczasową semantykę;
- przepływ jest skorelowany między procesami, także przez przekazanie sterowania/nadzorcę;
- kontrolowany błąd Novamiry zawiera surowe dostępne komunikaty, szczegóły błędu i dane żądania/odpowiedzi; oddzielnie weryfikuje się granicę widoczności wrappera oraz integrację mostu/kontenera;
- błąd zapisu, limit miejsca/retencji lub brak lokalizacji jawnie oznacza niekompletność logów i nie jest mylony z wynikiem operacji;
- syntetyczne dane diagnostyczne, w tym wartości umyślnie wrażliwe, są kompletne i odtwarzalne zgodnie z kontraktem, również w konfiguracji domyślnej — nie oczekuje się ich maskowania ani pominięcia;
- logi trafiają wyłącznie do lokalnego ignorowanego katalogu, a żadna domyślna ścieżka nie eksportuje danych przez sieć;
- awaria logowania nie maskuje pierwotnego błędu operacji, a częściowy sukces nie jest raportowany jako pełny sukces ani rollback.

Weryfikacja zachowania Rust i integracji Playwright wymaga test-first oraz właściwej bramki w VM, zgodnie z `docs/testing.md` (m.in. `make validate-rust-migration` dla Rust). Brak wymaganej VM lub innego środowiska blokuje weryfikację; nie jest sukcesem ani powodem do lokalnego obejścia.

## Decyzje wymagające doprecyzowania

- nazwa/konstrukcja katalogu i detekcja checkoutu dla zainstalowanych runtime, w tym worktree oraz projektów bez Git (kontrakt samego menedżera ustalony powyżej);
- polityka runtime dla logowania przed rozpoznaniem projektu i operacji uruchamianych poza checkoutem;
- konkretne limity retencji, rotacji, rozmiaru payloadów i reguły sprzątania;
- wybór formatu fragmentów/binarnych plików oraz mechanizm backpressure i sygnalizacji błędu writerowi;
- punkty integracji Novamiry i zakres danych dostępnych w moście/kontenerze;
- wybór nadzorcy/collectora dla ścieżek `exec()`, wymagania interaktywnego stdio i lifecycle kontenerów;
- przyszła decyzja, czy w ogóle dopuścić eksport poza lokalne repozytorium (np. Alloy/Loki).
