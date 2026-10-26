# Roadmapa loggera

**Cel:** pełna lokalna diagnostyka narzędzi Rust, pozwalająca odtworzyć przepływ operacji i danych bez zmiany zachowania aplikacji. Ten plik jest jedynym źródłem bieżących statusów i kolejności prac.

**Aktualnie:** etap 1 ukończony i zweryfikowany; etap 2 jest w toku. PROCESS-LIFECYCLE i PROCESS-DATA ukończone i zweryfikowane; cztery zadania etapu 2 pozostały.

| Etap | Zakres i odnośnik | Stan wynikający z zadań |
| --- | --- | --- |
| 1 | [Fundament loggera](#etap-1-fundament-loggera) | Ukończony i zweryfikowany: 109/109. |
| 2 | [Instrumentacja operacji](#etap-2-instrumentacja-operacji) | W toku: PROCESS-LIFECYCLE i PROCESS-DATA ukończone; cztery zadania pozostały. |
| 3 | [Diagnostyka Novamiry](#etap-3-diagnostyka-novamiry) | Do wykonania; integracja mostu wymaga ustalenia kanału diagnostycznego. |
| 4 | [Obserwacja po exec](#etap-4-obserwacja-po-exec) | Zablokowany do ustalenia kontraktu nadzorowania lub collectora. |
| 5 | [Opcjonalna integracja Grafana i Loki](#etap-5-opcjonalna-integracja-grafana-i-loki) | Poza bieżącym wdrożeniem; wymaga osobnej zgody. |

**Następny krok:** wskazuje go wyłącznie sekcja [Następne zadanie](#następne-zadanie).

### Zasady aktualizacji i przechodzenia dalej

- `[ ]` — nierozpoczęte; `[~]` — w trakcie; `[x]` — ukończone z wymaganą weryfikacją; `[!]` — zablokowane; `[?]` — brak potwierdzonego stanu.
- Każde zadanie ma dokładnie jeden status w swoim wpisie. Tabela etapów jest podsumowaniem tych wpisów, nie osobnym rejestrem postępu.
- Etap jest ukończony dopiero po wykonaniu jego wymaganych zadań i spełnieniu kryteriów zakończenia. Sam kod bez wymaganej weryfikacji nie zamyka etapu.
- Nie przechodzimy do kolejnego etapu, gdy obecny pozostaje nieukończony. Odstępstwo wymaga jawnej zgody właściciela i zapisu uzasadnienia tutaj; niezależność techniczna nie jest sama w sobie zgodą na zmianę kolejności.
- Zadania w obrębie etapu mogą mieć różne zależności. Blokada jednego nie oznacza automatycznie blokady wszystkich pozostałych; nie wolno jednak omijać jej przy ogłaszaniu ukończenia etapu.
- Jedno zlecenie implementacyjne obejmuje jedno wskazane zadanie, chyba że właściciel wyraźnie ustali inaczej. Zachowujemy stabilne identyfikatory niezależnie od numeracji wyświetlanej w nagłówkach.
- Wyniki, polecenia, wersje kompilatorów i artefakty dopisujemy przy zadaniu. Nie aktualizujemy statusów w archiwalnym `logger.md` ani w specyfikacjach.

## Etap 1: Fundament loggera

**Rezultat:** wspólna infrastruktura logowania gotowa do wykorzystania przez operacje systemu.

- [x] **FOUNDATION-BASE — Wspólny fundament lokalnego loggera.**
  - Wykonano: konfigurację i lokalizacje względem executable, integrację wszystkich dostarczanych punktów wejścia, kontekst wywołania menedżera, sesje, payloady binarne, rotację, ochronę aktywnych sesji podczas retencji, ignorowanie przez Git oraz sygnalizację niekompletności.
  - Kontrakt: [fundament i storage](concepts/foundation.md).
  - Dowody: [pełna bramka](../../.test-vm/logger-stage1-verification.log) — `GATE_PASS`, 109/109; [Rust 1.85.0](../../.test-vm/logger-stage1-msrv.log) — 109/109, `MSRV_PASS`, `SOURCE_AND_ISOLATION_PASS`.
  - Potwierdzona platforma: Linux x86_64 w wymaganej VM. Te wyniki nie potwierdzają jeszcze instrumentacji z kolejnych etapów.

**Kryterium zakończenia:** przyjęte kontrakty infrastruktury i wymagana weryfikacja spełnione. Etap jest zamknięty; nie wdrażamy go ponownie.

## Etap 2: Instrumentacja operacji

**Rezultat:** logi wyjaśniają, co dzieje się między startem a zakończeniem operacji: przepływ, decyzje, użyte dane, błędy i pozostawiony stan.

**Warunek wejścia:** ukończony etap 1. Nie realizujemy całego etapu 2 jako jednego zlecenia.

### Zadanie 2.1: Lifecycle wspólnego runnera procesów

- [x] **PROCESS-LIFECYCLE — Próby uruchomienia i rzeczywiste wyniki procesów.**
  - Zależność: FOUNDATION-BASE.
  - Zakres: skorelowane zdarzenia próby, spawn, zakończenia i błędu; rzeczywisty status, czas, timeout/anulowanie i dostępne przyczyny. Bez przechwytywania strumieni w tym zadaniu.
  - Kontrakt i akceptacja test-first: [lifecycle runnera](concepts/processes.md#runner-lifecycle).
  - Wykonano: każde wywołanie emituje `process.started` i dokładnie jeden terminal; udany spawn raportuje PID dziecka; zdarzenia mają run/process ID, czas, status lub klasyfikację/stage/przyczynę. Zachowano semantykę niezerowego statusu, timeoutu, anulowania i cleanupu. Dodano testy prawdziwych procesów i rejestrację modułu.
  - RED (bazowy runner bez instrumentacji): `cargo test --manifest-path core/Cargo.toml --locked --lib process_logging -- --nocapture` — 8 niepowodzeń lifecycle z powodu braku zdarzeń, test wyniku bez diagnostyki przeszedł; uzasadniony RED. VM Debian 13, Cargo 1.99.1, rustc 1.99.1. [Log RED](../../.test-vm/artifacts/20261026T083916Z-c93259e3-cb4e-4815-b581-26a33ae78f3f/process-lifecycle-red.log).
  - GREEN: to samo polecenie — 9/9 testów przeszło i moduł został wykonany; [log GREEN](../../.test-vm/artifacts/20261026T083940Z-d0252f42-c79a-4a09-92c3-577578ea5ea9/process-lifecycle-green.log). Testy obejmują poprawne i niezerowe zakończenie, sygnał, brak executable, anulowanie przed spawn i w trakcie procesu, timeout, równoległą korelację oraz brak diagnostyki.
  - Bramka końcowa: `make validate-rust-migration` — PASS: format, release-build, clippy, `make validate`, `make test` (118/118, w tym 9 `process_logging_tests`). VM Debian 13, Cargo 1.99.1, rustc 1.99.1; [log bramki](../../.test-vm/artifacts/20261026T084031Z-d00c1450-4d0a-4314-b446-727d87a7ce15/host.log). Dwa wcześniejsze przebiegi przerwały się odpowiednio na `format` ([log](../../.test-vm/artifacts/20261026T083951Z-f44ce505-f081-4be7-9a2f-bac633404c6c/host.log)) i `clippy` ([log](../../.test-vm/artifacts/20261026T084013Z-4c0a67da-f064-4655-98ff-7055dcd7e6ee/host.log)); wskazane problemy poprawiono przed końcowym PASS. Nie zmieniono zależności ani nie użyto API wymagających osobnego dowodu MSRV dla tego zadania.

### Zadanie 2.2: Argumenty, środowisko i strumienie procesów

- [x] **PROCESS-DATA — Obserwacja danych bez zmiany zachowania procesów.**
  - Zależności: PROCESS-LIFECYCLE i rozstrzygnięte [STREAM-BOUNDARY](decisions-and-gaps.md#stream-observation-boundary).
  - Zakres: dokładne argumenty, efektywne środowisko i obserwowane stdin/stdout/stderr połączone z istniejącym API payloadów oraz jawną kompletnością przechwycenia.
  - Kontrakt: [dane procesów](concepts/processes.md#process-data).
  - Właściciel rozstrzygnął: „Zachowanie procesów bez zmian”. Przechwytujemy wszystkie bajty obserwowane przez istniejące operacje runnera, a dane poza tą granicą jawnie oznaczamy jako nieprzechwycone. Wyjątek dotyczy granicy obserwacji strumieni; nie ogranicza kompletności argumentów ani dostępnych danych konfiguracyjnych.
  - Wykonano: dodano obserwację argumentów, efektywnego środowiska, cwd oraz bajtów zaakceptowanych przez istniejące operacje runnera. Payloady są pożyczane we fragmentach do 64 KiB i korelowane z process/stream/scope/index/offset. Stany EOF, limit, Null, Inherit, anulowanie, błąd spawn i awaria storage pozostają jawne; zachowano procesowe I/O, limity, terminy, cleanup i reguły join.
  - RED: `make test` w Debian 13 VM — 118 passed, nowy test `process_context_preserves_argument_boundaries_and_observed_output` failed z powodu braku kontekstu PROCESS-DATA; uzasadniony RED. [Log](../../.test-vm/artifacts/20261026T090733Z-d73a14e5-b3a7-4500-a04b-2b9f13d6a723/export/operation.log).
  - GREEN ukierunkowany: `cargo test --manifest-path core/Cargo.toml --locked --lib process_logging -- --nocapture` jako `tester` w VM, pod blokadą `.test-vm/lock` i świeżym `TMPDIR` — 19/19; [log](../../.test-vm/artifacts/20261026T0929-process-data-focused/targeted.log).
  - Bramka końcowa: `make validate-rust-migration` — PASS: format, release build, Clippy, `make validate`, `make test` — 128 passed; [log](../../.test-vm/artifacts/20261026T092912Z-4a071960-d03e-41af-8a74-75d9cadafdc4/host.log). Debian 13.7, Cargo 1.99.1, rustc 1.99.1. Nie dodano zależności; użyte API są dostępne w Rust 1.85, więc dodatkowy przebieg MSRV nie był wymagany.
  - Próby pośrednie: jedna bramka zatrzymała się na Clippy; kolejna wykryła stan `not_started` w teście anulowania oraz przejściowy `Text file busy` w istniejącym teście Novamiry. Poprawiono obserwację stanu, a końcowa bramka przeszła bez pomijania testów.

### Zadanie 2.3: Instalacja, aktualizacja, zmiana i usuwanie profilu

- [~] **PROFILE-LIFECYCLE — Instrumentacja install/update/switch/uninstall.**
  - Zależność techniczna: FOUNDATION-BASE. Zalecana kolejność w etapie: po PROCESS-LIFECYCLE; nie wymaga to ukończonego przechwytywania strumieni.
  - Zakres: przygotowanie, decyzje, konflikty, zapis/usuwanie, aktywacja, zapis stanu, wynik każdego klienta i zachowane wyniki częściowe.
  - Kontrakt: [lifecycle klientów](concepts/profiles.md#client-lifecycle).
  - Podział: A — wybór i przygotowanie; B — zapis i aktywacja; C — uninstall. A i B ukończone i zweryfikowane; C nierozpoczęta.
  - A — implementacja: dodano korelację `operation_id` z `run_id`, spany operacji/klienta/etapu, wynik manifestu, decyzje wyboru/pominięcia, przygotowanie, planowane ścieżki zmian, preflight uninstallu i błędy przed spłaszczeniem. Status pozostał poza instrumentacją; testy porównują logi z plikami i manifestem, w tym zachowanie zmienionego pliku przy konflikcie uninstallu.
  - A — RED: `make test` w VM Debian 13 — 128 passed, 2 failed; oba nowe testy wykryły brak zdarzeń operacji/przygotowania. [Artefakty](../../.test-vm/artifacts/20261026T100813Z-ebf88692-16f5-435c-afb5-727dd6fe1aba/host.log).
  - A — GREEN: `make test` — 131/131 passed, w tym 26 lifecycle tests. [Artefakty](../../.test-vm/artifacts/20261026T102132Z-57717027-62b3-43d9-bd66-07f7a8f3bd33/host.log).
  - A — bramka: `make validate-rust-migration` — PASS: format, release-build, clippy, make-validate i make-test (132/132; 27 lifecycle tests). VM Debian 13; `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`. [Artefakty](../../.test-vm/artifacts/20261026T102612Z-c7b3d3ff-81ab-4985-9865-7c023c875349/host.log).
  - Próby pośrednie: jedna bramka zatrzymała się na formatowaniu przed innymi kontrolami; późniejsze przebiegi wykryły i poprawiły błąd kompilacji oraz asercje końcowego wyniku. Wystąpiły też przejściowe awarie istniejących testów równoległych/timeoutu; końcowa bramka przeszła bez pomijania testów. Brak zmian zależności.
  - B — implementacja: commit raportuje faktycznie zapisane/usunięte pliki i konfiguracje, wynik aktywacji, powodzenie zapisu manifestu, rezultat klienta oraz błąd z zachowanym stanem. Wynik operacji częściowej zawiera ukończonych, nieudanych i nadal zapisanych klientów.
  - B — RED: `make test` w Debian 13 VM — 132 passed, 2 nowe testy failed dokładnie z powodu braku zdarzeń commit/aktywacji. [Artefakty](../../.test-vm/artifacts/20261026T103620Z-154f57ac-4406-4dae-a47f-f711305839ef/host.log).
  - B — GREEN: `make test` — 134/134 passed. Wcześniejsze przebiegi po RED wykryły problem formatowania, błąd Clippy oraz przejściowe `Text file busy`/timeout w istniejących testach; poprawiono kod, a testów nie pomijano. [Artefakty](../../.test-vm/artifacts/20261026T104642Z-b5f381a3-44b2-4895-918d-9973d15417b8/host.log).
  - B — bramka końcowa: `make validate-rust-migration` — PASS: format, release-build, Clippy, `make validate` i `make test` (134/134), Debian 13. [Artefakty](../../.test-vm/artifacts/20261026T105027Z-dca06e4c-bebc-40e3-ac0b-ba94eb6dc3c9/host.log); [worklog](../../changelogs/10-26-2026-11:51-agent-worklog.md).
  - Następne zadanie: etap 2, zadanie 2.3, część C — instrumentacja usuwania profilu i wyników po błędzie usuwania.

### Zadanie 2.4: Pozostałe komendy menedżera

- [ ] **MANAGER-AUX — Status, walidacja, migracja i zarządzanie profilami.**
  - Zależność: FOUNDATION-BASE.
  - Zakres: status, walidacja źródeł, migracja, tworzenie/zmiana nazwy/wersji profilu; dostępne przyczyny błędów przed spłaszczeniem ich do tekstu.
  - Kontrakt: [pozostałe komendy](concepts/profiles.md#other-manager-commands).
  - Weryfikacja: niewykonana.

### Zadanie 2.5: Przepływ WordPress restore

- [ ] **RESTORE-FLOW — Planowanie, wykonanie i stan po przerwaniu restore.**
  - Zależność: FOUNDATION-BASE. Dane procesów są zależnością interfejsową dla pełnego pokrycia, nie blokadą niezależnych zdarzeń operacji.
  - Zakres i akceptacja: [diagnostyka restore](concepts/restore.md).
  - Weryfikacja: niewykonana. Testy nie mogą wykonywać restore na rzeczywistej bazie użytkownika.

### Zadanie 2.6: Przygotowanie MCP i launcherów

- [ ] **MCP-PREPARE — Konfiguracja i przygotowanie uruchomienia lokalnego MCP.**
  - Zależność: FOUNDATION-BASE.
  - Zakres: faktycznie odczytana konfiguracja/dotenv, rozwiązywanie środowiska, wybór komendy, przygotowanie uruchomienia i błędy do istniejącej granicy przekazania sterowania.
  - Kontrakt: [przygotowanie MCP](concepts/mcp.md).
  - Weryfikacja: niewykonana. Zadanie nie obejmuje wnętrza mostu HTTP ani obserwacji po exec.

**Kryterium zakończenia etapu:** wszystkie sześć zadań zaakceptowane, granica przechwytywania strumieni rozstrzygnięta, przepływ i dane odtwarzalne zgodnie z kontraktami, zachowanie aplikacji niezmienione oraz wymagane kontrole zaliczone. Nie ogłaszamy ukończenia przy zablokowanym PROCESS-DATA.

## Etap 3: Diagnostyka Novamiry

**Rezultat:** szczegóły obserwowane przez wrapper oraz rzeczywista diagnostyka mostu/kontenera i dostępnego ruchu HTTP, bez zakłócania MCP.

**Warunek wejścia:** ukończony etap 2, chyba że właściciel jawnie zatwierdzi zmianę kolejności.

### Zadanie 3.1: Diagnostyka wrappera Rust

- [ ] **NOVAMIRA-WRAPPER — Dane i zdarzenia widoczne na granicy strażnika.**
  - Zależność techniczna: FOUNDATION-BASE; kontekst MCP-PREPARE powinien być wykorzystany do korelacji.
  - Kontrakt: [diagnostyka wrappera](concepts/novamira.md#wrapper-diagnostics).
  - Weryfikacja: niewykonana. Nie włączamy przechwytywania stderr upstreamu w oderwaniu od projektu kanału diagnostycznego.

### Zadanie 3.2: Diagnostyka mostu i HTTP

- [!] **NOVAMIRA-UPSTREAM — Integracja mostu/kontenera i danych HTTP.**
  - Zależności: NOVAMIRA-WRAPPER i rozstrzygnięty [NOVAMIRA-CHANNEL](decisions-and-gaps.md#novamira-diagnostic-channel).
  - Kontrakt: [diagnostyka mostu](concepts/novamira.md#bridge-and-http-diagnostics).
  - Weryfikacja: niewykonana. Potrzebne dowody z rzeczywistego przypiętego mostu/kontenera, nie tylko mock wrappera.

**Kryterium zakończenia etapu:** oba zadania zaakceptowane, zakres rzeczywistej widoczności potwierdzony, kontrolowane żądania/odpowiedzi i awarie odtwarzalne, a protokół oraz cleanup zachowują dotychczasowe działanie.

## Etap 4: Obserwacja po exec

**Rezultat:** widoczność wykonania i rzeczywistego zakończenia procesu po granicy obecnego exec.

**Warunek wejścia:** ukończony etap 3 i rozstrzygnięty kontrakt obserwacji; wcześniejsze rozpoczęcie wymaga jawnej zgody właściciela.

### Zadanie 4.1: Ciągłość diagnostyki po przekazaniu sterowania

- [!] **HANDOFF-OBSERVE — Obserwacja procesu po exec.**
  - Zależności techniczne: FOUNDATION-BASE i rozstrzygnięcie [HANDOFF-CONTRACT](decisions-and-gaps.md#observation-after-exec).
  - Kontrakt: [obserwacja przekazania sterowania](concepts/handoff.md).
  - Weryfikacja: niewykonana. Nie zastępujemy exec supervisorem bez ustalonej semantyki protokołu, sygnałów, statusów i cleanupu.

**Kryterium zakończenia etapu:** wybrany i zaakceptowany mechanizm, rzeczywista obserwacja zakończenia oraz testy procesów/protokołów/sygnałów i cleanupu zgodne z przyjętym kontraktem. Sam wpis handoff nie wystarcza.

## Etap 5: Opcjonalna integracja Grafana i Loki

**Rezultat opcjonalny:** udostępnienie zatwierdzonego zakresu logów przez osobny collector/backend. To nie jest warunek ukończenia lokalnego loggera.

**Warunek wejścia:** osobna zgoda właściciela określająca dopuszczalny eksport. Ukończenie etapów lokalnego logowania nie jest taką zgodą.

### Zadanie 5.1: Zatwierdzona integracja eksportu

- [!] **EXPORT-OPTIONAL — Opcjonalna integracja Alloy/Loki/Grafana.**
  - Blokada: [EXPORT-APPROVAL](decisions-and-gaps.md#export-authorization); zadanie pozostaje poza bieżącym lokalnym wdrożeniem.
  - Kontrakt: [przyszły eksport](concepts/export.md).
  - Weryfikacja: niewykonana. Obecność etapu nie zezwala na wysyłanie danych ani dodanie zależności sieciowej do działania narzędzi.

**Kryterium zakończenia, jeżeli etap zostanie zlecony:** zatwierdzony kontrakt danych i miejsca docelowego, działające zbieranie/odczyt oraz potwierdzona niezależność działania lokalnych narzędzi od eksportu.

## Kryteria ukończenia lokalnego loggera

- Zakończone wymagane etapy 1–4 i zaakceptowane zadania; etap 5 pozostaje opcjonalny.
- Przepływ operacji, dostępne dane i przyczyny błędów są odtwarzalne z właściwą korelacją.
- Brak widoczności lub niekompletność nie są raportowane jako pełne przechwycenie.
- Bajty, statusy, protokoły, timeouty i cleanup zachowują ustalone kontrakty także przy awarii loggera.
- [Macierz pokrycia](coverage-matrix.md) ma powiązania z faktycznymi dowodami; spełnione są [zasady jakości i akceptacji](quality-and-acceptance.md).

## Następne zadanie

**Etap 2, zadanie 2.3 — PROFILE-LIFECYCLE, część C.** Części A i B ukończone i zweryfikowane; kolejnym zakresem jest uninstall.

## Historia aktualizacji

- 2026-10-25: przeniesiono oryginalny dokument bez zmian do `logger.md`, utworzono pakiet dokumentacji i zaimportowano dowody ukończenia fundamentu. Bez nowych zmian aplikacji i uruchamiania testów.
- 2026-10-25: na wyraźne polecenie właściciela przywrócono etapy 1–5 jako główne nagłówki, sześć zadań etapu 2 jako podsekcje, kryteria zakończenia i jawne warunki przechodzenia dalej. Zachowano wszystkie 11 identyfikatorów zadań, ich statusy, zależności techniczne oraz dowody. Poprzedni układ wyłącznie obszarowy jest zastąpiony tym układem.
- 2026-10-26: ukończono PROCESS-LIFECYCLE test-first; zapisano RED/GREEN, wyniki `make validate-rust-migration`, wersje kompilatorów i ścieżki artefaktów. PROCESS-DATA nadal blokuje nierozstrzygnięty STREAM-BOUNDARY.
- 2026-10-26: właściciel rozstrzygnął STREAM-BOUNDARY: zachowanie procesów bez zmian; obserwacja wyłącznie istniejących operacji I/O. PROCESS-DATA rozpoczęte.
- 2026-10-26: ukończono PROCESS-DATA test-first; targeted 19/19 i `make validate-rust-migration` 128/128 w Debian 13.7, Cargo/rustc 1.99.1. Zapisano logi RED/GREEN i bramki. Nie twierdzimy, że wszystkie strumienie dziecka są przechwytywane.
- 2026-10-26: ukończono PROFILE-LIFECYCLE A test-first. RED: 128 passed/2 nowe testy failed; GREEN: 131/131; końcowa `make validate-rust-migration` po dodaniu testu preflight uninstallu PASS, 132/132, Debian 13, rustc/Cargo 1.99.1. Szczegóły i artefakty w sekcji zadania 2.3. B — zapis, aktywacja i wyniki częściowe — jest jedynym następnym krokiem.
- 2026-10-26: rozpoczęto PROFILE-LIFECYCLE B, dodając testy efektów commit/manifestu i zachowanego stanu po błędzie aktywacji. `make test` nie dotarło do Cargo: brak uprawnień hosta do socketu Incus. Nie potwierdzono RED i nie rozpoczęto implementacji; B oczekuje na dostęp do wymaganej VM.
- 2026-10-26: właściciel przywrócił dostęp do VM. PROFILE-LIFECYCLE B ukończono test-first: RED 132/2, GREEN 134/134, końcowa `make validate-rust-migration` PASS (134/134) w Debian 13. Dodano zdarzenia efektów commit, aktywacji, manifestu i częściowych wyników; następne jest C — uninstall. Szczegóły: [worklog](../../changelogs/10-26-2026-11:51-agent-worklog.md).
