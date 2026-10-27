# Roadmapa loggera

**Cel:** pełna lokalna diagnostyka narzędzi Rust, pozwalająca odtworzyć przepływ operacji i danych bez zmiany zachowania aplikacji. Ten plik jest jedynym źródłem bieżących statusów i kolejności prac.

**Aktualnie:** etap 1 ukończony i zweryfikowany; etap 2 jest w toku. PROCESS-LIFECYCLE, PROCESS-DATA, PROFILE-LIFECYCLE, MANAGER-AUX i RESTORE-FLOW ukończone i zweryfikowane; MCP-PREPARE pozostało.

| Etap | Zakres i odnośnik | Stan wynikający z zadań |
| --- | --- | --- |
| 1 | [Fundament loggera](#etap-1-fundament-loggera) | Ukończony i zweryfikowany: 109/109. |
| 2 | [Instrumentacja operacji](#etap-2-instrumentacja-operacji) | W toku: pięć zadań ukończonych; MCP-PREPARE pozostało. |
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

- [x] **PROFILE-LIFECYCLE — Instrumentacja install/update/switch/uninstall.**
  - Zależność techniczna: FOUNDATION-BASE. Zalecana kolejność w etapie: po PROCESS-LIFECYCLE; nie wymaga to ukończonego przechwytywania strumieni.
  - Zakres: przygotowanie, decyzje, konflikty, zapis/usuwanie, aktywacja, zapis stanu, wynik każdego klienta i zachowane wyniki częściowe.
  - Kontrakt: [lifecycle klientów](concepts/profiles.md#client-lifecycle).
  - Podział: A — wybór i przygotowanie; B — zapis i aktywacja; C — uninstall. Wszystkie części ukończone i zweryfikowane.
  - A — implementacja: dodano korelację `operation_id` z `run_id`, spany operacji/klienta/etapu, wynik manifestu, decyzje wyboru/pominięcia, przygotowanie, planowane ścieżki zmian, preflight uninstallu i błędy przed spłaszczeniem. Status pozostał poza instrumentacją; testy porównują logi z plikami i manifestem, w tym zachowanie zmienionego pliku przy konflikcie uninstallu.
  - A — RED: `make test` w VM Debian 13 — 128 passed, 2 failed; oba nowe testy wykryły brak zdarzeń operacji/przygotowania. [Artefakty](../../.test-vm/artifacts/20261026T100813Z-ebf88692-16f5-435c-afb5-727dd6fe1aba/host.log).
  - A — GREEN: `make test` — 131/131 passed, w tym 26 lifecycle tests. [Artefakty](../../.test-vm/artifacts/20261026T102132Z-57717027-62b3-43d9-bd66-07f7a8f3bd33/host.log).
  - A — bramka: `make validate-rust-migration` — PASS: format, release-build, clippy, make-validate i make-test (132/132; 27 lifecycle tests). VM Debian 13; `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`. [Artefakty](../../.test-vm/artifacts/20261026T102612Z-c7b3d3ff-81ab-4985-9865-7c023c875349/host.log).
  - Próby pośrednie: jedna bramka zatrzymała się na formatowaniu przed innymi kontrolami; późniejsze przebiegi wykryły i poprawiły błąd kompilacji oraz asercje końcowego wyniku. Wystąpiły też przejściowe awarie istniejących testów równoległych/timeoutu; końcowa bramka przeszła bez pomijania testów. Brak zmian zależności.
  - B — implementacja: commit raportuje faktycznie zapisane/usunięte pliki i konfiguracje, wynik aktywacji, powodzenie zapisu manifestu, rezultat klienta oraz błąd z zachowanym stanem. Wynik operacji częściowej zawiera ukończonych, nieudanych i nadal zapisanych klientów.
  - B — RED: `make test` w Debian 13 VM — 132 passed, 2 nowe testy failed dokładnie z powodu braku zdarzeń commit/aktywacji. [Artefakty](../../.test-vm/artifacts/20261026T103620Z-154f57ac-4406-4dae-a47f-f711305839ef/host.log).
  - B — GREEN: `make test` — 134/134 passed. Wcześniejsze przebiegi po RED wykryły problem formatowania, błąd Clippy oraz przejściowe `Text file busy`/timeout w istniejących testach; poprawiono kod, a testów nie pomijano. [Artefakty](../../.test-vm/artifacts/20261026T104642Z-b5f381a3-44b2-4895-918d-9973d15417b8/host.log).
  - B — bramka końcowa: `make validate-rust-migration` — PASS: format, release-build, Clippy, `make validate` i `make test` (134/134), Debian 13. [Artefakty](../../.test-vm/artifacts/20261026T105027Z-dca06e4c-bebc-40e3-ac0b-ba94eb6dc3c9/host.log); [worklog](../../changelogs/10-26-2026-11:51-agent-worklog.md).
  - C — implementacja: preflight i usuwanie raportują stan faktyczny zmian plików/konfiguracji, cleanup i ostrzeżenia, zapis manifestu, wynik klienta oraz stan zachowany po błędzie usuwania lub zapisu. Błąd zapisu manifestu jest logowany, a następnie propagowany bez zmiany dotychczasowego przerwania operacji; wcześniejsze wyniki pozostają zapisane.
  - C — RED: `make test` przed instrumentacją — 134 passed, 4 failed; testy wykryły brak zdarzeń zmian, manifestu i wyniku uninstallu. W tym samym przebiegu wykryto błędną kolejność argumentu profilu w fixture prawdziwego procesu; poprawiono fixture, nie oczekiwania produktu. [Artefakty](../../.test-vm/artifacts/20261026T115013Z-e6e55c5c-4ca3-401f-85ed-d28a0312cc00/host.log).
  - C — GREEN: `make test` — 139/139 passed. Pokrycie obejmuje konflikt preflight i zachowanie edytowanego pliku, poprawny uninstall wybranego klienta z zachowaniem drugiego klienta i `context.txt`, awarię usuwania z `uninstall all` i wynikiem częściowym, awarię zapisu manifestu po usunięciu, brak diagnostyki oraz rzeczywisty proces CLI. [Artefakty](../../.test-vm/artifacts/20261026T115326Z-b23553ee-ab16-40b0-81c8-a6d135644da4/host.log).
  - C — bramka: `make validate-rust-migration` — PASS: format, release-build, Clippy, `make validate` i `make test` (139/139), Debian 13. [Artefakty](../../.test-vm/artifacts/20261026T120026Z-3406a102-917c-479e-936f-3fa7f3b3abde/host.log); [worklog](../../changelogs/10-26-2026-12:56-agent-worklog.md).
  - Próby pośrednie: początkowy przebieg po implementacji ujawnił zbyt wąską asercję stanu po błędzie (usunięcie `.gitignore` zostało prawidłowo zachowane) oraz przejściowe `Text file busy` w istniejącym teście Novamiry. Asercję poprawiono zgodnie z faktycznym stanem częściowym; późniejszy pełny przebieg przeszedł bez pomijania testów. Kolejne bramki zatrzymały się na formatowaniu i ostrzeżeniu Clippy; oba problemy poprawiono. Końcowa implementacja zachowuje dotychczasowy tekst błędu CLI i zakres operacji uninstallu.
  - Następne zadanie: etap 2, zadanie 2.4 — MANAGER-AUX.

### Zadanie 2.4: Pozostałe komendy menedżera

- [x] **MANAGER-AUX — Status, walidacja, migracja i zarządzanie profilami.**
  - Zależność: FOUNDATION-BASE.
  - Zakres: status, walidacja źródeł, migracja, tworzenie/zmiana nazwy/wersji profilu; dostępne przyczyny błędów przed spłaszczeniem ich do tekstu.
  - Podział: A — status/validate; B — create/rename/bump; C — migrate-legacy.
  - Kontrakt: [pozostałe komendy](concepts/profiles.md#other-manager-commands).
  - A — wykonano: `status` tworzy jedną skorelowaną operację, zapisuje wynik każdego klienta przed agregacją, obserwowane różnice plików/źródeł, konfigurację, nadpisania i aktywację; surowe zarządzane pliki trafiają do payloadów. `validate_sources` zapisuje zakres repozytorium, wykonane i pominięte kontrole, wyniki porównania MCP, przyczyny błędów oraz użyte dane w payloadach. Zachowano teksty i kody CLI; testy potwierdzają brak mutacji projektu/źródeł oraz zachowanie wyniku przy niedostępnych logach.
  - A — RED: `make test` w VM wykazało brak zdarzeń dla statusu i walidacji; przed implementacją poprawiono dwa błędy konstrukcji fixture (brak syntetycznego executable i executable bit). W ustabilizowanym RED: 140 passed/6 failed, z czego cztery testy ujawniły brak diagnostyki, a dwa błędne fixture walidacji; [artefakty](../../.test-vm/artifacts/20261026T121944Z-fae4df9b-3ad7-41c2-98a8-bc361cb0e6fe/host.log).
  - A — GREEN: `make test` — 146/146 passed po instrumentacji; [artefakty](../../.test-vm/artifacts/20261026T122753Z-57f70b00-d633-42cc-8d65-afc5ff9aa0b5/host.log). Dodano później test awarii diagnostyki dla `validate`.
  - A — bramka końcowa: `make validate-rust-migration` — PASS: format, release-build, Clippy, `make validate` i `make test` (147/147); Debian 13, rustc `1.99.1 (58cf8f9c3 2026-10-13)`, Cargo `1.99.1 (5f94df478 2026-08-27)`. [Artefakty](../../.test-vm/artifacts/20261026T123442Z-973a1441-89cf-4a57-8f6f-636e33fd3e7e/host.log); [worklog](../../changelogs/10-26-2026-13:35-agent-worklog.md).
  - Dodatkowe kontrole: `git diff --check` — PASS. Brak nowych zależności; nie uruchomiono osobnego Rust 1.85 przebiegu, ponieważ nie dodano zależności ani API wymagającego dodatkowego dowodu MSRV. Status nie weryfikuje połączenia/uwierzytelnienia MCP ani ładowania umiejętności.
  - Próba pośrednia końcowej bramki: format, build, Clippy i `make validate` przeszły, lecz istniejący test Novamiry zakończył się `Text file busy` (146/147); pełna bramka po ponowieniu przeszła bez pomijania testów. [Artefakty próby](../../.test-vm/artifacts/20261026T123355Z-afae9402-1697-426e-bf8b-5406fe80e7a3/host.log).
  - B — wykonano: `create`, `rename` i `bump` otwierają pojedynczą operację skorelowaną z repozytorium źródeł. Logowane są wejścia/transformacje, opublikowane pliki, wynik oraz pełne bajty zmienianych metadanych w payloadach; zachowano typy i łańcuchy błędów, stdout/stderr i kody CLI. Test prawdziwego procesu potwierdza zawartość źródeł po każdej komendzie, payloady, konflikt create i brak fałszywego celu projektowego.
  - B — RED: `make test` w Debian 13 VM — 147 passed/1 failed; nowy test wykrył brak zdarzeń operacji dla create/rename/bump. Dwie wcześniejsze próby przed uzyskaniem RED ujawniły błędy fixture testu (nie-ASCII byte literal i konflikt create po zwolnieniu nazwy przez rename); poprawiono wyłącznie test. [Artefakt RED](../../.test-vm/artifacts/20261026T130334Z-7873863c-f144-4567-9b0e-1d4363299887/host.log).
  - B — GREEN: `make test` — 148/148 passed. Jedna próba pośrednia zakończyła się wyłącznie zbyt szeroką asercją helpera lifecycle CLI (dodatkowe zdarzenia domenowe); helper rozdziela teraz zdarzenia `profile.*`/`payload.*`, pozostawiając ścisłe asercje cyklu CLI. [Artefakt GREEN](../../.test-vm/artifacts/20261026T130609Z-437690a5-21ab-406f-9114-72355568598b/host.log).
  - B — bramka końcowa: `make validate-rust-migration` — PASS: format, release-build, Clippy, `make validate` i `make test` (148/148), VM Debian 13; rustc `1.99.1 (58cf8f9c3 2026-10-13)`, Cargo `1.99.1 (5f94df478 2026-08-27)`. [Artefakty](../../.test-vm/artifacts/20261026T130747Z-199d7cde-378e-419b-8a80-88f27a851020/host.log). Pierwsza próba końcowej bramki zatrzymała się na formatowaniu, poprawiono formatowanie bez zmiany zachowania.
  - B — dodatkowo: `git diff --check` — PASS. Brak nowych zależności. Nie uruchomiono osobnego Rust 1.85 przebiegu; nie dodano zależności ani API wymagających dodatkowego dowodu MSRV.
  - C — wykonano: `migrate-legacy` jest jedną operacją źródłową z terminalem wyniku i pełnym łańcuchem błędu. Zdarzenia opisują wymaganie zgody, rozpoznanie formatu i pary profile/skill, wybór MCP, obecność użytych plików, zapis stanu i usunięcie gate. Surowe bajty legacy state, konfiguracji MCP, sprawdzanych plików i nowego stanu są dostępne przez payloady.
  - C — RED: `make test` w VM — 148 passed/1 failed; nowy real-process test wykrył brak operacji i zdarzeń migracji. Pierwsza próba testu zakończyła się na wadliwym fixture (źródło binarium MCP); poprawiono fixture, następny RED był merytoryczny. [Artefakt RED](../../.test-vm/artifacts/20261026T131744Z-f1d3ccbe-2689-4f3e-9679-a2de0d2dcca3/host.log).
  - C — GREEN: `make test` — 149/149 passed, w tym test procesu migracji potwierdzający zgodę, korelację, jeden terminal, wynik plików oraz dokładne bajty payloadów stanu i pliku zarządzanego. [Artefakt GREEN](../../.test-vm/artifacts/20261026T132202Z-25cad0c1-da88-4bdd-9c52-ef59786558f4/export/operation.log).
  - C — bramka końcowa: `make validate-rust-migration` — PASS: format, release build, Clippy, `make validate`, testy Rust 149/149. Debian 13; rustc `1.99.1 (58cf8f9c3 2026-10-13)`, Cargo `1.99.1 (5f94df478 2026-08-27)`. [Artefakty](../../.test-vm/artifacts/20261026T132202Z-25cad0c1-da88-4bdd-9c52-ef59786558f4/host.log).
  - Próba bramki przed formatowaniem zatrzymała się na format; formatowanie wykonano w VM, po czym pełna bramka przeszła. `git diff --check` — PASS. Brak nowych zależności; nie uruchomiono osobnego Rust 1.85 przebiegu, bo nie dodano zależności ani API wymagających dodatkowego dowodu MSRV. Brak testu awarii storage specyficznego dla migracji.

### Zadanie 2.5: Przepływ WordPress restore

- [x] **RESTORE-FLOW — Planowanie, wykonanie i stan po przerwaniu restore.**
  - Zależność: FOUNDATION-BASE. Dane procesów są zależnością interfejsową dla pełnego pokrycia, nie blokadą niezależnych zdarzeń operacji.
  - Zakres i akceptacja: [diagnostyka restore](concepts/restore.md).
  - Wykonano: jedna skorelowana operacja raportuje tryb, plan, preflight, decyzję potwierdzenia/anulowania, blokadę, kopię i weryfikację backupu, rozpoczęcie mutacji, lokalizację, weryfikację końcową oraz odzyskiwanie po błędzie. Surowe bajty dumpa, wejścia importu, backupu i użytego pliku ochronnego są utrwalane przez payloady podczas istniejących odczytów; awaria zapisu diagnostyki nie zastępuje wyniku komendy. Zachowano dotychczasowe stdout/stderr, kody wyjścia, reguły potwierdzenia, cleanup i brak automatycznego rollbacku.
  - Test-first: RED `make test` w Debian 13 VM — 149 passed/1 failed; nowy test planu wykrył brak zdarzeń startu operacji. [Artefakt RED](../../.test-vm/artifacts/20261026T133301Z-3271e49d-e5d6-463b-b72d-fc3ac42f9ce4/host.log). GREEN `make test` — 157/157 passed, w tym prawdziwy proces CLI z kontrolowanym DDEV dla planu, odmowy potwierdzenia, udanego apply, błędu przed importem i błędu po rozpoczęciu mutacji. [Artefakt GREEN](../../.test-vm/artifacts/20261026T134914Z-be6233ef-acc2-4d36-9b71-def7b3f3db0e/host.log).
  - Bramka końcowa: `make validate-rust-migration` — PASS: format, release-build, Clippy, `make validate` i `make test` (157/157). Debian 13.7, rustc `1.99.1 (58cf8f9c3 2026-10-13)`, Cargo `1.99.1 (5f94df478 2026-08-27)`. [Artefakty](../../.test-vm/artifacts/20261026T135050Z-2097f5d9-6e0a-4e76-992a-f6923d4df99a/host.log). `git diff --check` — PASS. Brak nowych zależności; nie wykonano osobnego Rust 1.85 przebiegu, bo nie dodano zależności ani API wymagających osobnego dowodu MSRV.
  - Próby pośrednie: odrębny GREEN poprzedził końcowy rozszerzony zestaw; przy dwóch przebiegach wystąpiło przejściowe `Text file busy` w istniejącym teście Novamiry, bez zmian w tym obszarze. Końcowa bramka przeszła bez pomijania testów.
  - Ograniczenie: scenariusze apply używają prawdziwego procesu CLI i izolowanego projektu z kontrolowanym executable `ddev`; nie wykonują importu do rzeczywistej bazy WordPress.

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

**Etap 2, zadanie 2.6 — MCP-PREPARE / konfiguracja i przygotowanie uruchomienia lokalnego MCP.** RESTORE-FLOW ukończone i zweryfikowane.

## Historia aktualizacji

- 2026-10-25: przeniesiono oryginalny dokument bez zmian do `logger.md`, utworzono pakiet dokumentacji i zaimportowano dowody ukończenia fundamentu. Bez nowych zmian aplikacji i uruchamiania testów.
- 2026-10-25: na wyraźne polecenie właściciela przywrócono etapy 1–5 jako główne nagłówki, sześć zadań etapu 2 jako podsekcje, kryteria zakończenia i jawne warunki przechodzenia dalej. Zachowano wszystkie 11 identyfikatorów zadań, ich statusy, zależności techniczne oraz dowody. Poprzedni układ wyłącznie obszarowy jest zastąpiony tym układem.
- 2026-10-26: ukończono PROCESS-LIFECYCLE test-first; zapisano RED/GREEN, wyniki `make validate-rust-migration`, wersje kompilatorów i ścieżki artefaktów. PROCESS-DATA nadal blokuje nierozstrzygnięty STREAM-BOUNDARY.
- 2026-10-26: właściciel rozstrzygnął STREAM-BOUNDARY: zachowanie procesów bez zmian; obserwacja wyłącznie istniejących operacji I/O. PROCESS-DATA rozpoczęte.
- 2026-10-26: ukończono PROCESS-DATA test-first; targeted 19/19 i `make validate-rust-migration` 128/128 w Debian 13.7, Cargo/rustc 1.99.1. Zapisano logi RED/GREEN i bramki. Nie twierdzimy, że wszystkie strumienie dziecka są przechwytywane.
- 2026-10-26: ukończono PROFILE-LIFECYCLE A test-first. RED: 128 passed/2 nowe testy failed; GREEN: 131/131; końcowa `make validate-rust-migration` po dodaniu testu preflight uninstallu PASS, 132/132, Debian 13, rustc/Cargo 1.99.1. Szczegóły i artefakty w sekcji zadania 2.3. B — zapis, aktywacja i wyniki częściowe — jest jedynym następnym krokiem.
- 2026-10-26: rozpoczęto PROFILE-LIFECYCLE B, dodając testy efektów commit/manifestu i zachowanego stanu po błędzie aktywacji. `make test` nie dotarło do Cargo: brak uprawnień hosta do socketu Incus. Nie potwierdzono RED i nie rozpoczęto implementacji; B oczekuje na dostęp do wymaganej VM.
- 2026-10-26: właściciel przywrócił dostęp do VM. PROFILE-LIFECYCLE B ukończono test-first: RED 132/2, GREEN 134/134, końcowa `make validate-rust-migration` PASS (134/134) w Debian 13. Dodano zdarzenia efektów commit, aktywacji, manifestu i częściowych wyników; następne jest C — uninstall. Szczegóły: [worklog](../../changelogs/10-26-2026-11:51-agent-worklog.md).
- 2026-10-26: PROFILE-LIFECYCLE C ukończono test-first w Debian 13: RED 134/4 przed instrumentacją; końcowa bramka PASS (139/139). Dodano faktyczne zmiany uninstallu, ostrzeżenia cleanupu, błędy usuwania/zapisu manifestu i wyniki częściowe przy zachowaniu dotychczasowych błędów CLI. `context.txt` pozostaje nietknięty przez logikę uninstallu. Następne: MANAGER-AUX. Szczegóły: [worklog](../../changelogs/10-26-2026-12:56-agent-worklog.md).
- 2026-10-26: MANAGER-AUX A — status/validate — ukończone test-first. Dodano korelację i terminal statusu, per-klientowe wyniki oraz obserwacje konfiguracji/różnic/payloadów; walidacja raportuje rzeczywisty zakres, kontrole, pominięcia, porównanie MCP i przyczyny. RED ujawnił brak zdarzeń status/validate; końcowa bramka w Debian 13 PASS (147/147). Szczegóły, wersje i artefakty w sekcji zadania 2.4. Następne: część B — create/rename/bump.
- 2026-10-26: MANAGER-AUX B — create/rename/bump — ukończone test-first. Pojedyncza operacja wskazuje repozytorium źródeł, rejestruje transformacje i opublikowane pliki, a bajty metadanych są dostępne w payloadach; zachowano CLI i łańcuch błędu. RED 147/1, GREEN 148/148, końcowa bramka `make validate-rust-migration` PASS (148/148), Debian 13, rustc/Cargo 1.99.1. `git diff --check` PASS. Szczegóły i artefakty w sekcji 2.4. Następne: część C — migrate-legacy.
- 2026-10-26: rozpoczęto MANAGER-AUX C testem prawdziwego procesu migracji legacy; `make test` nie dotarło do Cargo z powodu braku uprawnień do `/run/incus/unix.socket` (`operation not permitted`). Nie potwierdzono RED ani nie rozpoczęto implementacji produkcyjnej. Zadanie oznaczono `[!]`; pierwszym krokiem po odblokowaniu jest ponowienie `make test` w VM. [Worklog](../../changelogs/10-26-2026-14:16-agent-worklog.md).
- 2026-10-26: po przywróceniu uprawnień VM ukończono MANAGER-AUX C test-first. RED 148/1; GREEN i `make validate-rust-migration` PASS (149/149), Debian 13, rustc/Cargo 1.99.1. Zapisano wyniki partial state migration, kontrolę zgodności legacy, payloady i usunięcie approval gate. `git diff --check` PASS. MANAGER-AUX ukończone; następne RESTORE-FLOW. [Worklog](../../changelogs/10-26-2026-14:23-agent-worklog.md).
- 2026-10-26: ukończono RESTORE-FLOW test-first. RED 149/1 ujawnił brak korelacji planu; końcowa `make validate-rust-migration` PASS (157/157) w Debian 13.7, rustc/Cargo 1.99.1. Potwierdzono bezmutacyjny plan, dokładne potwierdzenie, pełny kontrolowany apply i zachowanie backupu przy awarii przed i po rozpoczęciu mutacji; surowe payloady i awaria diagnostyki zachowują wynik. Dwa wcześniejsze pełne przebiegi trafiły na istniejący przejściowy `Text file busy` testu Novamiry; bramka końcowa przeszła bez pomijania testów. `git diff --check` PASS. Następne: MCP-PREPARE. [Worklog](../../changelogs/10-26-2026-14:52-agent-worklog.md).
