# Roadmapa loggera

**Cel:** pełna lokalna diagnostyka narzędzi Rust, pozwalająca odtworzyć przepływ operacji i danych bez zmiany zachowania aplikacji. Ten plik jest jedynym źródłem bieżących statusów i kolejności prac.

**Aktualnie:** etap 1 ukończony i zweryfikowany; realizacja etapu 2 jeszcze się nie rozpoczęła. Ta aktualizacja porządkuje dokumentację, nie wdraża nowych funkcji.

| Etap | Zakres i odnośnik | Stan wynikający z zadań |
| --- | --- | --- |
| 1 | [Fundament loggera](#etap-1-fundament-loggera) | Ukończony i zweryfikowany: 109/109. |
| 2 | [Instrumentacja operacji](#etap-2-instrumentacja-operacji) | Do wykonania; sześć mniejszych zadań. Zadanie 2.2 wymaga rozstrzygnięcia granicy przechwytywania strumieni. |
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

- [ ] **PROCESS-LIFECYCLE — Próby uruchomienia i rzeczywiste wyniki procesów.**
  - Zależność: FOUNDATION-BASE.
  - Zakres: skorelowane zdarzenia próby, spawn, zakończenia i błędu; rzeczywisty status, czas, timeout/anulowanie i dostępne przyczyny. Bez przechwytywania strumieni w tym zadaniu.
  - Kontrakt i akceptacja test-first: [lifecycle runnera](concepts/processes.md#runner-lifecycle).
  - Weryfikacja: niewykonana dla tego zadania.

### Zadanie 2.2: Argumenty, środowisko i strumienie procesów

- [!] **PROCESS-DATA — Przechwytywanie danych bez zmiany semantyki procesu.**
  - Zależności: PROCESS-LIFECYCLE i rozstrzygnięcie [STREAM-BOUNDARY](decisions-and-gaps.md#stream-observation-boundary).
  - Zakres: dokładne argumenty, efektywne środowisko i obserwowane stdin/stdout/stderr połączone z istniejącym API payloadów oraz jawną kompletnością przechwycenia.
  - Kontrakt: [dane procesów](concepts/processes.md#process-data).
  - Weryfikacja: niewykonana. Nie zgadujemy zachowania dla strumieni ograniczonych, odrzucanych i dziedziczonych.

### Zadanie 2.3: Instalacja, aktualizacja, zmiana i usuwanie profilu

- [ ] **PROFILE-LIFECYCLE — Instrumentacja install/update/switch/uninstall.**
  - Zależność techniczna: FOUNDATION-BASE. Zalecana kolejność w etapie: po PROCESS-LIFECYCLE; nie wymaga to ukończonego przechwytywania strumieni.
  - Zakres: przygotowanie, decyzje, konflikty, zapis/usuwanie, aktywacja, zapis stanu, wynik każdego klienta i zachowane wyniki częściowe.
  - Kontrakt: [lifecycle klientów](concepts/profiles.md#client-lifecycle).
  - Weryfikacja: niewykonana.

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

**Etap 2, zadanie 2.1 — PROCESS-LIFECYCLE.** Następne zlecenie obejmuje wyłącznie to zadanie. Przed testami i kodem przeczytaj [kontrakt lifecycle runnera](concepts/processes.md#runner-lifecycle). Nie rozpoczynaj zablokowanych zadań ani kolejnych etapów bez wymaganej zgody.

## Historia aktualizacji

- 2026-10-25: przeniesiono oryginalny dokument bez zmian do `logger.md`, utworzono pakiet dokumentacji i zaimportowano dowody ukończenia fundamentu. Bez nowych zmian aplikacji i uruchamiania testów.
- 2026-10-25: na wyraźne polecenie właściciela przywrócono etapy 1–5 jako główne nagłówki, sześć zadań etapu 2 jako podsekcje, kryteria zakończenia i jawne warunki przechodzenia dalej. Zachowano wszystkie 11 identyfikatorów zadań, ich statusy, zależności techniczne oraz dowody. Poprzedni układ wyłącznie obszarowy jest zastąpiony tym układem.
