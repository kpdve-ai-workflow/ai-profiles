# Playwright gotowy po instalacji profilu — handoff

## Stan

Plan readiness: READY

Postęp: T-01–T-05 ukończone i zweryfikowane. READY oznacza gotowe kontrakty; dowody ukończenia i zakres weryfikacji znajdują się poniżej.

Next action: Brak — wszystkie zadania planu ukończone i zweryfikowane.

## Cel i granice

Profil wybierający Playwright ma dostarczyć narzędzie przygotowane do uruchomienia w lokalnym Docker Engine na Linuxie. Użytkownik wykonuje ręczny build dostarczonego obrazu, ewentualnie tworzy jeden prywatny plik Basic Auth i restartuje klienta AI. Nie edytuje Compose ani `local.json` dla zwykłego użycia.

VPN, DDEV, Docker Engine, Compose oraz konfiguracja sieci komputera są zastanym środowiskiem. Nie instalujemy ich, nie uruchamiamy VPN, nie wykrywamy jego dostawcy i nie zmieniamy tras, DNS, firewalla ani magazynów certyfikatów hosta. Dopuszczona jest konfiguracja samego kontenera, aby korzystał z istniejącego środowiska hosta.

Nie przechodzimy na hostowe `npx`, nie tworzymy własnego serwera MCP/proxy protokołu, usługi HTTP, demona ani globalnych wpisów MCP. Nie zmieniamy funkcji innych serwerów i nie wykonujemy prac w WFOSiGW.

## Wymagania

- R-01 — Instalacja profilu dostarcza komplet zarządzanych plików Playwright. Po ręcznym buildzie i restarcie klienta publiczna strona działa bez dodatkowej konfiguracji. Docker i Compose są już dostępne. Źródło: doprecyzowania użytkownika w rozmowie.
- R-02 — Ogólnym katalogiem artefaktów jest `reports/`, nie `reports/wcag/`. Katalog powstaje automatycznie; użytkownik nie wykonuje `mkdir`. Budowa obrazu nie tworzy katalogu projektu hosta, dlatego przygotowanie następuje przy uruchomieniu dostarczonego narzędzia. Źródło: wymaganie użytkownika i granica filesystemu Docker build.
- R-03 — Basic Auth jest opcjonalny. Użytkownik może utworzyć jeden plik zawierający `username`, `password`, `origin`, bez dalszego konfigurowania MCP. Użytkownik jawnie zaakceptował te trzy pola w odpowiedzi na pytanie o bezpieczne ograniczenie domeny.
- R-04 — Kontener korzysta z istniejącej łączności hosta, w tym już skonfigurowanego i uruchomionego VPN. Brak automatyzacji VPN lub zmian konfiguracji hosta. Źródło: ostatnie sprostowanie użytkownika; ma pierwszeństwo przed wcześniejszymi sugestiami asystenta.
- R-05 — Zminimalizować konfigurację TLS narzędzia dla istniejącego lokalnego DDEV/mkcert. Weryfikacja TLS pozostaje włączona; importujemy wyłącznie dostępny publiczny CA, nigdy klucz CA. Nie obiecujemy obsługi nieznanego firmowego CA bez źródła zaufania.
- R-06 — Zachować niezależność klientów, stan formatu 2, ochronę lokalnych zmian, brak sekretów w obrazie/stanie/logach/raportach oraz istniejące jawne rozszerzenia konfiguracji. Projekt pozostaje read-only poza `reports/`.

## Zweryfikowane ustalenia

| Ustalenie | Sprawdzone źródło | Znaczenie / ograniczenie |
| --- | --- | --- |
| React i WordPress wybierają Playwright. | `profiles/react/mcp.json`, `profiles/wordpress/mcp.json` | Pozostałych profili nie rozszerzamy. |
| Dziś rejestr uruchamia bezpośrednio `docker compose run`, bez buildu/pull. | `mcp/servers/playwright.json`, `mcp/example.json` | Brakuje przygotowania runtime przed Compose. |
| Adapter odnajduje projekt i klienta ze swojego położenia, ustawia zastrzeżone `AI_PROFILE_PROJECT_DIR`. | `core/rust/src/mcp/runtime.rs`: `identity_from_adapter`, `apply_environment` | Nie ustalamy projektu z PWD. |
| Dostarczanie obrazu jest specjalizacją `build_files`; binaria są kopiowane przez `insert_native`. Ścieżki legacy są renderowane do payloadu klienta. | `core/rust/src/profile_management/payload.rs`, `core/rust/src/profile_management/scoped.rs`: `render_mcp`, `files_for` | Launcher jest dystrybuowany i hashowany jak inne pliki runtime. |
| Domyślne Compose używa bridge, projektu RO i zapisywalnego `reports/wcag`. | `mcp/images/playwright/compose.yaml` | Zmiana katalogu i sieci to zmiana kontraktu runtime. |
| Entrypoint uruchamia Chromium headless/isolated, ładuje axe-core, importuje pojedynczy CA do systemu i NSS, obsługuje storage state i Basic Auth. | `mcp/images/playwright/entrypoint.sh`, `compose.ca.yaml`, `compose.auth.yaml`, `compose.basic-auth.yaml` | Zachowujemy ten mechanizm, nie reimplementujemy MCP. |
| Helper wymaga dokładnie `username`, `password`, `origin`, normalizuje domyślne porty i nie wypisuje sekretów. | `mcp/images/playwright/basic-auth.cjs`, `tests/basic-auth.test.cjs` w tym katalogu | Schemat pliku jest gotowy do ponownego użycia. |
| Jawny względny `filename` jest rozwiązywany przez upstream względem MCP root, a nie outputDir. | `mcp/images/playwright/README.md`, `skills/wcag-audit/SKILL.md`; README upstream `https://raw.githubusercontent.com/microsoft/playwright-mcp/v0.0.83/README.md` | Samo ustawienie outputDir nie przenosi `screenshot.png` do raportów. |
| Istnieją prawdziwe testy stdio/Chromium/axe/TLS/storage state i instalowanego adaptera; Basic Auth sprawdzany jest osobno przez helper. | `mcp/images/playwright/tests/run.sh`, `smoke.cjs`, `basic-auth.test.cjs` | Potrzebny rzeczywisty test HTTP 401/Authorization, nie tylko test JSON. |
| Eksporter VM obecnie filtruje `*/reports/wcag/*` oraz rozszerzenia JSON/PNG/HTML. | `tests/vm/guest.sh`: `collect` | Zmiana raportów musi zachować eksport po błędzie oraz granice prywatności. |
| Host networking współdzieli namespace sieciowy na Linux Docker Engine. Pliki hosts/DNS są odrębną kwestią. | `https://docs.docker.com/engine/network/drivers/host/`, `https://docs.docker.com/engine/network/` | Nie jest to konfiguracja VPN ani gwarancja działania dowolnej polityki firmowej. |
| `mkcert -CAROOT` wypisuje lokalizację i kończy działanie przed tworzeniem CA; `rootCA.pem` jest publiczny, `rootCA-key.pem` nie wolno udostępniać. | `https://raw.githubusercontent.com/FiloSottile/mkcert/master/main.go` — obsługa `carootFlag`; README mkcert | Można użyć odczytowego wykrywania istniejącego CA. |
| Aktualne wersje profili to React 2.2.2 i WordPress 1.5.2. | `profiles/react/profile.json`, `profiles/wordpress/profile.json` | Publikacja zmiany raportów wymaga świadomego bumpu. |

Ustalenia pochodzą z odczytu plików i dokumentacji. Historyczne logi nie są dowodem weryfikacji nowej implementacji. Nie sprawdzono rzeczywistego firmowego VPN ani gotowości VM w tej sesji.

## Decyzje Brain

| ID | Wiążąca decyzja i powód | Pochodzenie |
| --- | --- | --- |
| D-01 | Zachować Docker i oficjalne MCP 0.0.83 oraz axe-core 4.14.0. Nie łączyć zadania z aktualizacją zależności. | BRAIN-SELECTED, zgodne z R-01 |
| D-02 | Dodać cienki natywny `playwright-launcher` w Rust. Przygotowuje wyłącznie uruchomienie narzędzia: katalog raportów i opcjonalne wejścia Compose. Nie buduje ani nie pobiera obrazu. | BRAIN-SELECTED |
| D-03 | Domyślny plik Basic Auth: `.private/playwright-basic-auth.json`, współdzielony celowo w obrębie projektu. Jawne ustawienie per klient nadal ma pierwszeństwo. | BRAIN-SELECTED; pola zaakceptowane przez użytkownika |
| D-04 | Zachować upstreamowe reguły `filename`. Automatyczne pliki trafiają do `reports/`; agent dostaje instrukcję jawnych nazw `reports/...`. Nie tworzyć proxy zmieniającego argumenty narzędzi. | BRAIN-SELECTED; minimalny zakres R-02 |
| D-05 | Dla lokalnego Linux Docker Engine domyślnie host networking (T-02), a następnie read-only dostęp do istniejących `/etc/resolv.conf` oraz `/etc/hosts` przez automatyczny `compose.host.yaml` (T-03). Jawny tryb inny niż host nie otrzymuje tego overlay. Nie analizować VPN ani nie modyfikować plików hosta. | BRAIN-SELECTED, R-04 |
| D-06 | Automatycznie odnaleźć istniejący publiczny CA mkcert; import wyłącznie wewnątrz kontenera. Jawny CA użytkownika ma pierwszeństwo. Brak CA nie wyłącza TLS i nie blokuje publicznych stron. | BRAIN-SELECTED, R-05 |
| D-07 | Nowy obraz: `ai-profiles-playwright:0.2.0`. Docelowe wersje profili: React 3.0.0, WordPress 2.0.0, ze względu na zmianę domyślnego katalogu i sieci. | BRAIN-SELECTED |

## Kontrakty

### C-01 — Launcher i dystrybucja

- Nowe źródła: `core/rust/src/playwright.rs`, `core/rust/src/playwright/runtime.rs`, `core/rust/src/bin/playwright-launcher.rs`. Rejestracja modułu w `core/rust/src/lib.rs`; Cargo wykrywa nowy bin przez istniejący układ `src/bin/`.
- Instalacja binarium: `.ai-profiles/<klient>/mcp/playwright/playwright-launcher`, wyłącznie gdy profil wybiera ten serwer. Użyć `insert_native`, nie skryptu z hostowym Node/Python/jq.
- Rejestr MCP wskazuje `.opencode/mcp/playwright/playwright-launcher`; istniejący renderer zmienia tę referencję na payload klienta. Zachować bazowy `environment.COMPOSE_FILE` i jego renderowanie.
- Launcher nie przyjmuje argumentów. Odnajduje swój katalog przez `current_exe().canonicalize()`, akceptuje wyłącznie zainstalowany układ `.ai-profiles/<znany-klient>/mcp/playwright/`. Wyprowadza z niego projekt; wymaga zgodności z `AI_PROFILE_PROJECT_DIR` przekazanym przez adapter. Nie ufa PWD ani odziedziczonej wartości zastępującej położenie programu.
- Przed Compose tworzy brakujące `reports/` jako bieżący użytkownik, idempotentnie. Istniejącego katalogu i zawartości nie zmienia; plik lub symlink na tej ścieżce daje czytelny błąd przed uruchomieniem Dockera. Stosować ochronę ścieżek jak w `filesystem::file_path`.
- Następnie wykonuje bez powłoki `docker compose --project-directory PROJECT run --rm --no-deps --pull never -T playwright`, z cwd projektu, dziedziczonym stdio i efektywnym `COMPOSE_FILE`. `exec`, jak w istniejącym adapterze, zachowuje protokół/obsługę procesu. Własna diagnostyka tylko na stderr.
- Brak obrazu nie uruchamia instalacji: zachować błąd Dockera na stderr i podać procedurę ręcznego buildu 0.2.0 w dostarczonej instrukcji. Nie dodawać preflightu wymagającego jednego sztywnego tagu mimo jawnego nadpisania obrazu. Nie dodawać `PREPARE_MCP`, automatycznego buildu, pull ani zależności produkcyjnych hosta.
- `COMPOSE_FILE` użytkownika pozostaje uporządkowaną listą. Potrzebne automatyczne rozszerzenia, których nie ma już pod tą samą rozwiązaną ścieżką, wstawić **przed** oryginalną listą, w kolejności host (wprowadzone w T-03), CA, storage state, Basic Auth. Oryginalnych plików nie usuwać i nie przestawiać. W bazowym compose nie ma konkurencyjnych mountów host/Auth/CA, a późniejsze jawne pliki użytkownika zachowują priorytet (także testowy resolver i obraz). Rozszerzenie obecne już w oryginalnej liście pozostaje na swoim miejscu. Nie dopisywać host overlay na końcu w sposób nadpisujący świadomą konfigurację użytkownika.
- Nie zmieniać semantyki ogólnego adaptera ani formatu stanu. Pomocnicze typy i błędy launchera mają być domenowe, bez `Debug` ujawniającego credentials; bez nowych bibliotek.

### C-02 — Raporty i granice plików

- Compose montuje projekt pod rzeczywistą ścieżką hosta RO, a całe `PROJECT/reports` RW; `PLAYWRIGHT_MCP_OUTPUT_DIR=PROJECT/reports`. UID/GID nadal wynikają z właściciela raportów z możliwością jawnego nadpisania.
- Automatyczny screenshot bez `filename` trafia do `reports/`. Jawne nazwy to `reports/...` lub odpowiadająca ścieżka bezwzględna. `screenshot.png` w korzeniu projektu nadal jest odrzucane; nie jest to czynność konfiguracyjna użytkownika.
- Skill WCAG zapisuje własny przebieg w `reports/<timestamp>/`, bez narzucania kategorii WCAG całemu narzędziu. Nie zmieniać nazwy skilla ani reguł audytu.
- Stare raporty pozostają nietknięte; update/uninstall nie przenosi i nie usuwa danych użytkownika. Istniejące `reports/wcag` pozostaje zwykłym podkatalogiem nowego zapisywalnego `reports`.
- Eksporter VM zbiera zwykłe pliki JSON/PNG/HTML z `*/reports/*`, nadal bez śledzenia symlinków. Nie eksportuje `.private`, kluczy, auth storage ani całego projektu. Nie poszerzać whitelisty rozszerzeń w tym zadaniu.

### C-03 — Opcjonalny Basic Auth i istniejące rozszerzenia

- Domyślny plik: `.private/playwright-basic-auth.json`, dokładnie pola `username`, `password`, `origin`, zgodnie z istniejącym helperem. Plik ma prawa 0600 lub bardziej restrykcyjne i jest zwykłym plikiem, nie symlinkiem; `.private` nie może być symlinkiem. Nie odczytywać prawdziwych sekretów użytkownika podczas implementacji/diagnozy.
- Nie ma pliku: nie włączać Basic Auth, nie wymagać żadnego env/local.json i nie tworzyć pustego pliku z błędnymi danymi.
- Jest plik: launcher automatycznie ustawia istniejące `AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE` i włącza `compose.basic-auth.yaml` zgodnie z kolejnością C-01. Sprawdza wyłącznie bezpieczną ścieżkę i metadane; parsowanie pozostaje w istniejącym helperze wewnątrz obrazu.
- Jawne niepuste `AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE` ma pierwszeństwo nad konwencją. Brak jawnie wskazanego pliku jest błędem, nie fallbackiem. Względne ścieżki są względem projektu, bez `..` i symlinków; jawne bezwzględne ścieżki do prywatnych plików pozostają obsługiwane. Nie akceptować credentials/storage state wewnątrz `reports/`.
- `origin` ogranicza credentials do schematu/hosta/portu. Bez zgadywania domeny z DDEV i bez globalnego wysyłania Authorization. Błędne dane/prawa/format mają zatrzymać start bez ujawniania wartości.
- Istniejące `AI_PROFILE_PLAYWRIGHT_STORAGE_STATE` również automatycznie włącza `compose.auth.yaml`; nadal obowiązują prywatność i prawa. Nie wprowadzać nowej domyślnej lokalizacji storage state.
- Instalacja/update/switch klienta wybierającego Playwright dodaje do projektowego `.gitignore` wyłącznie brakującą regułę `/.private/playwright-basic-auth.json`. Zachować inne bajty i styl zakończeń linii; nie zarządzać hashem całego pliku. Powtórzenie nie dubluje reguły. Symlink lub plik innego typu blokuje operację przed zapisem zarządzanych plików. Reguła i credentials pozostają po uninstall — nie odsłaniać zachowanego sekretu. Profile bez Playwright nie dodają reguły. To wąska, jawna zmiana konfiguracji narzędzia, nie czyszczenie Git ani automatyczne odśledzanie plików.
- Sekrety nie trafiają do argumentów CLI, Dockerfile/build context, środowiska kontenera jako username/password, manifestu instalacji ani raportów. Dotychczasowy `.dockerignore` jest allowlistą; zachować tę granicę.

### C-04 — Korzystanie z gotowego środowiska hosta

- `network_mode` domyślnie `host`; istniejące jawne `AI_PROFILE_PLAYWRIGHT_NETWORK` pozostaje escape hatchem kompatybilności, bez obietnicy odziedziczenia hostowego localhost w bridge.
- W domyślnym zakresie Linux/local Engine launcher automatycznie dodaje nowe `compose.host.yaml`, gdy `AI_PROFILE_PLAYWRIGHT_NETWORK` jest nieustawione, puste lub równe `host`. Overlay montuje read-only pliki hosta `/etc/resolv.conf` i `/etc/hosts` pod odpowiadającymi ścieżkami, `create_host_path: false`. Dla jawnego innego trybu nie dodaje tego overlay, żeby nie kierować bridge do hostowego loopback DNS. Nie montować `/run`, HOME, socketu Docker ani całego `/etc`. Rozszerzyć listę payloadu i znanych ścieżek `render_mcp` o `compose.host.yaml`.
- Nie wpisywać publicznych DNS, nie wykonywać `nmcli`, `resolvectl`, poleceń VPN ani edycji tras. Wymagamy wykorzystania gotowej konfiguracji hosta, nie jej naprawiania. Po zmianie środowiska hosta dopuszczony jest restart MCP/klienta; nie budować monitora VPN.
- Testy udowadniają localhost, odwzorowanie plików resolvera/hosts i syntetyczną prywatną domenę. Nie korzystają z firmowej sieci i nie są dowodem działania konkretnego VPN. Brak dostępu do prywatnej strony nie uruchamia automatycznych napraw hosta. Użytkownik świadomie zmieniający `network_mode` wyłącznie we własnym pliku Compose odpowiada za zgodność swoich nadpisań; launcher nie uruchamia `compose config`, aby zgadywać ich znaczenie i wypisywać rozwinięte wartości.

### C-05 — CA dostępny dla narzędzia

- Priorytet: jawne `AI_PROFILE_PLAYWRIGHT_CA_CERT`; następnie istniejący `CAROOT/rootCA.pem`; następnie wynik odczytowego `mkcert -CAROOT` plus `rootCA.pem`, jeżeli `mkcert` jest już na PATH. Nigdy nie uruchamiać `mkcert -install`, nie generować CA i nie instalować mkcert.
- Dla ustawionego CAROOT z brakującym rootCA brak autoimportu (strony z publicznym CA nadal mogą działać). Jawny CA z błędną ścieżką jest błędem. Istniejący wybrany CA z błędną zawartością jest błędem, nie powodem wyłączenia TLS. Odczyt `mkcert -CAROOT` ma limit 5 s; niepowodzenie daje krótką diagnostykę na stderr i brak autoimportu, nie blokuje publicznego użycia.
- Wykrywanie CA nie czyta katalogu rekurencyjnie. Sprawdza wyłącznie `rootCA.pem`; nigdy nie otwiera ani nie montuje `rootCA-key.pem`. Obsłużyć spacje w ścieżkach bez powłoki.
- Wybrany certyfikat jest montowany osobno RO przez istniejące `compose.ca.yaml`; import do systemu/NSS oraz `NODE_EXTRA_CA_CERTS` pozostają wewnątrz kontenera. Nie kopiować certyfikatów konkretnej maszyny do obrazu ani repozytorium.
- Nie implementować automatycznego importu całego systemowego trust store lub skanowania firmowych CA. Niestandardowy CA pozostaje obsługiwany przez istniejące jawne ustawienie; jest to udokumentowany wyjątek, nie obietnica pełnego zero-config.

### C-06 — Testowanie, kompatybilność i publikacja

- Wiążące instrukcje: [AGENTS.md](../../AGENTS.md) i [docs/testing.md](../../docs/testing.md). Brain zastosował `test-gap-discovery`; Implementer ma wczytać ten skill przed projektowaniem/dodawaniem/oceną testów. Gdy niedostępny: zgłosić, zachować tę samą metodykę.
- Niezależny oracle: R-01–R-06, reguły filesystemu/HTTP Auth/TLS i udokumentowane zachowanie upstream. Najpierw oczekiwanie i test, potem uzasadnione RED dla brakującego zachowania, minimalna implementacja, GREEN i bramka. Nie wyprowadzać oczekiwań z obecnego wyniku.
- Rust i Playwright uruchamiać wyłącznie w wymaganej VM Debian 13 przez istniejące Make. Dla zmian wspólnych `make test-all`; dla zmiany eksportera dodatkowo `bash tests/vm/check.sh` na hoście. `make test` i `make test-playwright` nadają się do fazy RED; bramka końcowa może zawierać je ponownie.
- Testy Rust: nowe `core/rust/tests/playwright.rs`, jawnie dołączone jako `playwright_tests` w `core/rust/src/lib.rs` (`autotests=false`). Potwierdzać konkretne nowe testy w wynikach. Nie jest potrzebny nowy target Make ani runner.
- Zachować testy rzeczywistego adaptera z cwd `/`, MCP roots, ścieżkę projektu ze spacjami i przecinkiem, uprawnienia nie-root, granice RO i inicjalizację axe po nawigacji. Mock może sprawdzać specyfikację procesu/odkrywanie CA, ale nie zastępuje Chromium, HTTP/TLS lub uruchomienia dystrybuowanego launchera.
- Brak środowiska/uprawnień lub nieoczekiwany błąd blokuje zadanie. Utrzymać izolację i ochronę testów; nie uznawać błędu infrastruktury za RED produktu. Nie obniżać asercji, nie dodawać skipów i nie zmieniać oczekiwań jednostronnie.
- Dane testowe wyłącznie syntetyczne w scratch VM. Nie używać prawdziwego HOME klientów ani realnych credentials, stron firmowych lub VPN. Rejestracja/aktywacja pluginów pozostaje mockowana tak jak obecnie; test konfiguracji klienta nie jest dowodem załadowania MCP w realnej aplikacji klienta.
- Nowy launcher obsługuje nowy payload isolated; nie modyfikuje zamrożonych zasobów legacy. Update wybranego klienta nadal przenosi jego konfigurację zgodnie z istniejącym lifecycle, pozostali zachowują własny rekord/payload. Ręczne modyfikacje zarządzanych plików nadal blokują aktualizację.
- Zmiana domyślnej ścieżki raportów i sieci jest publikowana jawnie: obraz 0.2.0, React 3.0.0, WordPress 2.0.0. Manualny build i restart pozostają wymagane. Uninstall nie usuwa obrazów, raportów, credentials ani reguły chroniącej credentials.

## Kolejka wykonania

- [x] [T-01 — Ogólny katalog raportów i bezpieczny eksport](tasks/T-01.md)
  - Execution: IMPLEMENTED
  - Verification: PASSED
  - Depends on: brak
  - Evidence: uzasadnione RED w `make test-playwright`: rzeczywisty MCP/Chromium zwrócił `EROFS` dla `reports/relative.png` przy starym mountcie `reports/wcag`; po implementacji `make test-all` PASSED (48 testów Rust oraz bezpośredni i instalowany Playwright adapter w VM), `bash tests/vm/check.sh` PASSED, `git diff --check` PASSED. Kontrola eksportera sprawdza top-level/nested JSON/PNG/HTML oraz wyklucza `.private`, rozszerzenie poza whitelistą i symlink.
- [x] [T-02 — Dostarczany launcher i opcjonalny Basic Auth bez local.json](tasks/T-02.md)
  - Execution: IMPLEMENTED
  - Verification: PASSED
  - Depends on: T-01
  - Evidence: `make test-all` PASSED (59 testów Rust, walidacja/Clippy/build oraz testy Playwright), nowy launcher uruchomił MCP z `cwd=/`; czysty projekt bez `reports` i `local.json` przeszedł publiczną nawigację, Basic Auth 401/200, zły port/hasło i izolację nagłówka Authorization od drugiego origin/redirectu. `git check-ignore --no-index` sprawdził domyślny plik. T-03 pozostaje osobnym zadaniem.
- [x] [T-03 — Istniejące nazwy hostów i DNS dostępne w kontenerze](tasks/T-03.md)
  - Execution: IMPLEMENTED
  - Verification: PASSED
  - Depends on: T-02
  - Evidence: RED `make test` wykazał brak host overlay; RED `make test-playwright` po udanej nawigacji localhost wykazał `missing host mount /etc/hosts`. Końcowe `make test-all` PASSED (60 testów Rust, format/build/Clippy/walidacja i integracja Playwright). Rzeczywisty instalowany launcher z cwd `/` potwierdził mounty RO zgodne z hosts/resolv hosta VM, loopback HTTP oraz syntetyczny resolver + host networking (`app.corp.test` w Chromium, zarejestrowane zapytanie DNS i priorytet testowego overlay). `git diff --check` PASSED.
- [x] [T-04 — Automatyczny import istniejącego publicznego CA mkcert](tasks/T-04.md)
  - Execution: IMPLEMENTED
  - Verification: PASSED
  - Depends on: T-03
  - Evidence: RED `make test` (60 PASSED / 3 FAILED) wykazał brak wyboru CA; RED `make test-playwright` wykazał `ERR_CERT_AUTHORITY_INVALID` na poprawnej fixture podpisanej CA z CAROOT. Końcowe `make test-all` PASSED: 69 testów Rust, format/build/Clippy/walidacja, rzeczywiste Chromium HTTPS z automatycznym CA, odrzucenie obcego CA, CA + Basic Auth + storage state, inspect pojedynczego mountu publicznego pliku RO oraz brak sentinel/credentials w protokole/logach/stanie/raportach. `git diff --check` PASSED.
- [x] [T-05 — Publikacja profili i krótka instrukcja użytkownika](tasks/T-05.md)
  - Execution: IMPLEMENTED
  - Verification: PASSED
  - Depends on: T-04
  - Evidence: React 3.0.0 / WordPress 2.0.0; ujednolicone instrukcje builda 0.2.0, automatycznego przygotowania reports i opcjonalnego Basic Auth. Kontrola treści/lokalnych linków/kotwic PASSED; instalacja dwóch profili dla trzech klientów w scratch VM potwierdziła wersje i rozwinięte README/WCAG. `make test-all` PASSED (69 testów Rust, format/build/Clippy/walidacja i pełny Playwright w VM). `git diff --check` PASSED.

## Dowody wykonania

T-01: Compose/output przełączone na `reports/`; obraz i instrukcje zaktualizowane do 0.2.0; smoke test sprawdza względny i bezwzględny zapis, automatyczny filename i zachowane granice RO; eksporter używa wspólnego `report_files ROOT`, a test selekcji korzysta z tej samej funkcji.

T-02: Dodano natywny launcher Rust i jego binarium do payloadu wyłącznie klientów wybierających Playwright; instalacja tworzy `reports/` przy starcie, automatycznie dołącza bezpieczne pliki Basic Auth/storage state i zachowuje ręczną listę Compose. Dla Playwright instalacja dopisuje do `.gitignore` brakującą regułę bez zarządzania jej hashem; uninstall jej nie usuwa. Domyślny Compose używa host network.

T-03: Dodano dystrybuowany i hashowany `compose.host.yaml` z wąskimi mountami RO `/etc/hosts` i `/etc/resolv.conf`, `create_host_path: false`. Launcher dołącza go przed oryginalną listą Compose tylko dla nieustawionego/pustego/jawnego `host`; jawny inny tryb nie otrzymuje overlay. Zachowano kolejność i brak duplikatów jawnych plików. Test lifecycle potwierdza dystrybucję i renderowanie ścieżek dla wszystkich trzech klientów. Dokumentacja runtime opisuje lokalny Linux Engine, brak automatyzacji VPN i ograniczenia bridge/remote/Desktop.

2026-10-22 — Brain: odczytowy skrypt `python3` sprawdził 7 plików handoffu: komplet plików, lokalne linki i kotwice, pięć niezaznaczonych zadań, dokładnie jeden `Next action`, brak drugiego rejestru statusów w zadaniach, końcowe nowe linie, whitespace i domknięcie bloków kodu. Wynik: exit 0. To kontrola dokumentów, nie test zachowania aplikacji lub agentów.

2026-10-22 — Implementer, T-01: test-first RED potwierdził `EROFS` przy próbie zapisu rzeczywistym Chromium przez MCP do `reports/relative.png` ze starym układem mountów. Wczesne iteracje fixture kończyły się błędami samej asercji mountu, `chown` lub diagnostycznego zapisu; nie zostały uznane za RED produktu i poprawiono je przed uzyskaniem powyższego wyniku. Po zmianach `make test-all` zakończone powodzeniem: format, release build, Clippy, walidacja oraz 48 testów Rust; bezpośredni smoke kontenera i test MCP przez zainstalowany adapter przeszły. `bash tests/vm/check.sh` zakończone kodem 0 z kontrolą składni i fixture filtra; wyświetliło ostrzeżenia `cannot delete non-empty directory` w sprzątaniu istniejącej scratch kopii, ale finalny wynik to `OK`. `git diff --check` zakończone bez błędów. Ostatnia bramka `make test-all` uruchomiona ponownie po korekcie formatowania również PASSED.

2026-10-22 — Implementer, T-02: test-first RED w VM wykazał brak zarządzanego `playwright-launcher` w instalowanym payloadzie oraz `InvalidLauncher` w nowych kontraktach Rust. Pierwsze błędy kompilacji i fixture testowych poprawiono; nie zaliczono ich jako RED produktu. Po implementacji końcowe `make test-all` PASSED: format, release build, Clippy, `make validate`, 59 testów Rust oraz integracja Playwright w VM (bezpośredni kontener, istniejący adapter oraz czysty projekt uruchamiający dystrybuowany launcher). E2E sprawdziło prawdziwe HTTP 401 bez/ze złym hasłem i złym portem, 200 przy poprawnych danych, oraz brak Authorization na drugim origin po redirect. `git check-ignore --no-index` potwierdził regułę dla standardowego pliku Basic Auth. Wczesne iteracje auth-smoke miały błąd synchronizacji fixture i błędne założenie o wyniku `browser_navigate` przy 401; poprawiono je tak, by asercja opierała się na rzeczywistym statusie/nagłówku odebranym przez serwer HTTP.

Zastane modyfikacje użytkownika, których nie wolno nadpisywać: `.opencode/skills/brain-to-implementer/SKILL.md`, `.opencode/skills/brain-to-implementer/references/behavior-checks.md`, `.opencode/skills/brain-to-implementer/references/handoff-layout.md`, `AGENTS.md`, `docs/testing.md`. Przed każdym zadaniem porównać ten zapis z rzeczywistym working tree. W `docs/testing.md` wolno zmienić tylko wymagane opisy ścieżek raportów/nowego scenariusza, zachowując politykę i pozostałe zmiany użytkownika.

2026-10-22 — Implementer, T-03: working tree przed zadaniem był czysty; nie zmieniano `AGENTS.md`, `docs/testing.md` ani skillów użytkownika. Test-first: `make test` zakończył się 57 PASSED / 3 FAILED z powodu brakującego automatycznego host overlay; `make test-playwright` potwierdził działający localhost, następnie zasadny RED `missing host mount /etc/hosts`. Po implementacji dwie wczesne bramki zatrzymały się na formatowaniu nowych testów Rust — poprawiono format, bez zmiany oczekiwań. Dwie kolejne bramki `make test-all` PASSED, ostatnia po zmianach dokumentacji: `.test-vm/artifacts/20261022T150811Z-709e8d0f-04e6-4713-8441-29a1f42359d2/`. W wynikach wykonano 60 testów Rust, w tym `host_overlay_tracks_network_mode_and_preserves_explicit_compose_precedence` i rozszerzony lifecycle wszystkich klientów. Integracja użyła zainstalowanego adaptera/launchera z cwd `/`, projektu ze spacjami i przecinkiem, prawdziwego Chromium i syntetycznego DNS na `127.203.77.53:53`. Potwierdzono bindy hosts/resolv RO oraz identyczne treści z hostem VM, localhost bez `local.json`, odmowę zapisu w korzeniu projektu, prywatną domenę `app.corp.test`, query DNS i screenshot w `reports/`. Nie zmieniano plików `/etc`, sieci hosta/VM, VPN, Incus ani polityki TLS. Testowy resolver jest osobnym jednorazowym kontenerem sprzątanym przez istniejący trap. Dowód dotyczy „syntetyczny resolver + host networking”, nie rzeczywistego firmowego VPN. `git diff --check` PASSED. T-04 i T-05 nie były wykonywane.

2026-10-22 — Implementer, T-04: zachowano zastane, niezacommitowane zmiany T-03; nie zmieniano `AGENTS.md`, `docs/testing.md` ani skillów użytkownika. Launcher wybiera jawny CA, następnie istniejący `CAROOT/rootCA.pem`, następnie wynik odczytowego `mkcert -CAROOT` (tylko gdy program jest na PATH). Wybór pliku opiera się na metadanych; parser/import certyfikatu pozostaje w kontenerze. Ręczne CA/Compose zachowują priorytet i nie są dublowane; zachowano jawne symlinki do publicznego CA, montując docelowy plik, z odrzuceniem znanej ścieżki klucza. Dodano ograniczony capture 8192 + 1 bajt wykrycia overflow, środowisko mkcert ograniczone do PATH/HOME/XDG_DATA_HOME, brak stdin i stderr procesu oraz deadline 5 s. Dodatkowy zasadny RED runnera pokazał przekroczenie deadline przez odziedziczony stdout przy nadal działającym potomku i przy już zakończonym procesie głównym; poprawiono obsługę błędów/oczekiwania na ograniczony capture, bez zmiany oczekiwań. Testy obejmują spec procesu, spacje w CAROOT/PATH, priorytet i brak fallbacku dla ustawionego CAROOT, brak programu/pliku, timeout/błąd, błędny UTF-8/wieloliniowy/względny/NUL/nadmierny wynik, typ pliku oraz brak dostępu do klucza. Wczesne bramki wykryły formatowanie i zapis praw 0 zamiast 0o0 w teście (Clippy); poprawiono je. Pierwsza iteracja GREEN integracji ujawniła wyścig fixture po celowej nawigacji na stronę z obcym CA: screenshot przeniesiono przed tę nawigację, bez osłabienia asercji błędu certyfikatu; dalsze scenariusze używają świeżego kontekstu. Końcowe `make test-all` PASSED: `.test-vm/artifacts/20261022T153653Z-0dfad998-0733-46f8-90c8-82fa7613f4a6/` (69 testów Rust i pełny Playwright w wymaganej VM). CAROOT fixture leży poza projektem; syntetyczny klucz ma sentinel i prawa 000. HTTPS z wykrytym CA i Basic Auth/storage state działa, obcy CA daje `ERR_CERT_AUTHORITY_INVALID`, inspect nie zawiera mountu całego CAROOT/klucza, sentinel i credentials nie występują w protokole/logach/stanie/raportach. Brak automatycznego pliku CA nie blokuje inicjalizacji i zwykłego HTTP; wadliwy istniejący certyfikat zatrzymuje start z bezpieczną diagnostyką. Zachowano wcześniejszy test jawnego CA. Nie instalowano mkcert, nie zmieniano zaufania/sieci hosta ani provisioningu VM; nie wykonywano T-05. `git diff --check` PASSED.

2026-10-22 — Implementer, T-05: working tree przed zadaniem był czysty. Zmieniono wyłącznie dokumentację/instrukcje oraz wersje React 3.0.0 i WordPress 2.0.0; bez nowych funkcji, zmian kodu Rust, testów, orkiestracji lub zależności. Zachowano `AGENTS.md`, `docs/testing.md` i skille repozytorium. Niezależną checklistę treści wyprowadzono z R-01–R-06 i kontraktu T-05: instalacja, ręczny build 0.2.0, opcjonalny plik username/password/origin, restart i natywne zgody; brak ręcznego tworzenia reports lub edycji Compose/local.json w standardowym workflow; ochrona sekretów, TLS i granice VPN/hosta; migracja i priorytet starych nadpisań. Poprawiono nieaktualne instrukcje Basic Auth i wzmianki o braku launchera, dodano opis zachowania danych po uninstall. Nie wymuszano RED dla dokumentacji/metadanych. Odczytowe kontrole Python na hoście potwierdziły treść, lokalne linki/kotwice i bloki kodu siedmiu dokumentów oraz bump wyłącznie dwóch wskazanych profili. Dodatkowa kontrola instalacji w VM przez `incus exec` jako UID/GID 1000:1000 (Node jako harness, istniejący release manager, syntetyczny HOME, natywna aktywacja Claude/Codex jawnie wyłączona) sprawdziła React i WordPress dla wszystkich trzech klientów: wersje w stanie, README/WCAG bez nierozwiniętych placeholderów, ścieżki build/local.json i brak reports/credentials tworzonych podczas instalacji. Scratch: `/home/tester/test-runs/20261022T154315Z-41d063cd-1374-4006-b5ea-8f1f7d6d60de/t05-docs-v6kDvJ/`. Dwie bramki `make test-all` PASSED; ostatnia po uzupełnieniu listy Playwright w instrukcji WordPress: `.test-vm/artifacts/20261022T154453Z-1beab462-c6d6-4009-b800-f30029daab5b/` — 69 testów Rust (w tym lifecycle wszystkich klientów), format/release/Clippy/walidacja, rzeczywisty Playwright/Chromium z adapterem i launcherem, Basic Auth, syntetyczny DNS oraz CA/TLS. Po bramce poprawiono wyłącznie wcięcie kontynuacji punktu w skillu WCAG i zapis dowodów w planie. `git diff --check` PASSED. Eksportera ani orkiestracji nie zmieniano; zachowano istniejący dowód `bash tests/vm/check.sh` z T-01, bez formalnego powtarzania. Nie sprawdzano prawdziwego firmowego VPN, Docker Desktop, arm64 ani sesji rzeczywistych aplikacji klientów. Publikacja oznacza przygotowane metadane i instrukcje w checkoutcie, nie commit/tag/push ani instalację do projektu użytkownika.

## Blokady

Brak znanych blokad projektowych. Pytanie o dodatkowe pole `origin` rozstrzygnął użytkownik: zaakceptował jeden plik z loginem, hasłem i origin.

Gotowość środowiska testowego nie została sprawdzona przez Brain. Implementer potwierdził działanie wymaganej VM dla T-01–T-05. Brak znanych blokad końcowej weryfikacji; ograniczenia zakresu dowodów opisano powyżej.
