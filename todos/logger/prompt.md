Roadmapa oznacza już 2.3 jako ukończone. Następne jest 2.4 — MANAGER-AUX.

Proponuję trzy części:

1. A: status i validate.
2. B: create, rename, bump.
3. C: migrate-legacy i zamknięcie 2.4.

Poniżej prompt do części A:

Wykonaj wyłącznie zadanie 2.4 — MANAGER-AUX, część A:
diagnostykę status i validate.

Repozytorium:
 /var/home/kacperzielinski/it/projects/personal/ai-profiles

To zlecenie implementacyjne. Dodaj diagnostykę, testy i wykonaj
wymaganą weryfikację. Zatrzymaj się po części A.

Nie rozpoczynaj create/rename/bump, migracji ani zadania 2.5.
Nie wykonuj ponownie ukończonego PROFILE-LIFECYCLE.


1. KONTEKST

Przeczytaj:
- AGENTS.md i README.md;
- docs/testing.md;
- todos/logger/AGENTS.md i README.md;
- todos/logger/roadmap.md;
- todos/logger/knowledge-base.md;
- todos/logger/dependencies.md;
- todos/logger/decisions-and-gaps.md;
- todos/logger/quality-and-acceptance.md;
- todos/logger/concepts/profiles.md, szczególnie
  „Other manager commands” i „Shared acceptance”.

Stosuj test-gap-discovery i rust.
Przy niedostępnym skillu testowym zachowaj metodykę docs/testing.md.

Sprawdź git status i zachowaj istniejące zmiany.
Roadmapa jest jedynym rejestrem postępu.

Dopisz przy MANAGER-AUX podział:
A — status/validate,
B — create/rename/bump,
C — migrate-legacy.

Przed rozpoczęciem sprawdź blokady przy MANAGER-AUX. Jeśli nadal
obowiązują, zachowaj [!] i wskaż ich rozwiązanie jako następny krok.
Rozpoczęcie wykonalnej części A oznacz statusem [~] całego MANAGER-AUX.
Nie twórz osobnej roadmapy.


2. CEL I PUNKTY ZMIAN

Z logów ma wynikać:
- jakie dane i zakres wybrano;
- jakie kontrole rzeczywiście wykonano;
- jakie różnice lub błędy wykryto;
- dlaczego pominięto kontrolę;
- jaki jest rzeczywisty wynik komendy.

Główne miejsca:
- core/src/profile_management/status.rs: status;
- core/src/profile_management/validation.rs: validate_sources;
- core/src/profile_management/scoped.rs:
  ścieżka Status, odczyt manifestu i agregacja błędów klientów;
- core/src/profile_management/cli.rs:
  dispatch i odpowiednie granice przed operation_error;
- core/src/profile_management/diagnostics.rs:
  istniejące helpery korelacji i przyczyn błędów.

Obecnie scoped::run_with_runner pomija diagnostics::operation
dla Status. Uwzględnij to przy zapewnianiu korelacji.

Wykorzystaj obecne tracing, sesje i API payloadów.
Dostosuj prywatne helpery tylko w niezbędnym zakresie.
Zachowaj obecne typy błędów i publiczne zachowanie.


3. STATUS

Zapewnij:
- jeden operation_id dla całego wywołania status;
- kontekst klienta przy kontroli każdego wybranego klienta;
- dokładnie jeden terminal operacji;
- wynik każdego klienta przed agregacją błędów do tekstu.

Rejestruj dostępne informacje:
- obecność lub brak instalacji;
- profil, layout, wersję zainstalowaną i dostępną;
- wykryte zmiany zarządzanych plików;
- zmianę zawartości źródeł bez podbicia wersji;
- wynik kontroli konfiguracji i lokalnych nadpisań;
- zapisany stan aktywacji i dostępne przyczyny błędu.

Brak profilu pozostaje poprawnym wynikiem status.
Nie zamieniaj lokalnej modyfikacji pliku w błąd, jeśli obecny
kontrakt jedynie ją raportuje.

Status potwierdza stan konfiguracji.
Nie przedstawiaj go jako dowodu działającego połączenia MCP,
uwierzytelnienia ani załadowania umiejętności przez klienta.

Nie dodawaj uruchamiania CLI, aktywacji, połączeń sieciowych
ani naprawiania konfiguracji.


4. VALIDATE

Zapewnij jedną operację diagnostyczną dla validate_sources,
z informacją o rzeczywistym zakresie:
- wszystkie źródła;
- wskazany profil.

validate działa na repozytorium źródeł. Nie przedstawiaj ścieżki
repozytorium jako projektu docelowego ani nie wymyślaj klienta.

Rejestruj:
- wybrany profil lub pełny zakres;
- rzeczywiście wykonane kontrole metadanych, skilli,
  definicji MCP i mcp/example.json;
- pominięcie kontroli globalnych przy walidacji pojedynczego
  profilu, zgodnie z istniejącym przepływem;
- wynik porównania przykładu z odpowiednimi definicjami;
- dostępne przyczyny błędów i końcowy wynik.

Nie zmieniaj zakresu walidacji, kolejności kontroli,
warunków błędów ani komunikatów.


5. WSPÓLNE ZASADY

- Zachowaj stdout/stderr, komunikaty i kody zakończenia.
- Status nie może zmieniać projektu docelowego.
- Validate nie może modyfikować źródeł aplikacji.
- Istniejący zapis diagnostyki i reguła ignorowania logów
  pozostają objęte kontraktem fundamentu loggera.
- Rejestruj dane faktycznie wykorzystane, bez dodatkowego
  skanowania niezwiązanych plików.
- Potrzebne surowe dane utrwalaj istniejącym Session::payload.
  Nie ograniczaj wymaganych danych do samego hasha.
- Nie powielaj zbierania argv i środowiska z cli.context.*.
- Zachowaj dostępne przyczyny przed .to_string() i agregacją.
- Nie przedstawiaj propagacji jednego błędu jako wielu awarii.
- Awaria diagnostyki nie może zmieniać wyniku komendy.
- Nie dodawaj frameworka, zależności, retry ani mechanizmu naprawy.


6. TEST-FIRST

Przed implementacją ustal niezależne oczekiwania na podstawie
kontraktu, kontrolowanych plików i manifestu fixture.

Wykorzystaj:
- core/tests/lifecycle.rs dla statusu i konfiguracji klientów;
- core/tests/profiles.rs dla źródeł i walidacji;
- core/tests/manager_logging.rs oraz manager_fixture.rs
  dla prawdziwego procesu CLI.

Dobierz przypadki obejmujące:
- status bez instalacji;
- status poprawnej instalacji;
- lokalnie zmieniony lub brakujący plik;
- wadliwą konfigurację albo manifest;
- status all z błędem jednego klienta i wynikami pozostałych;
- poprawne validate całych źródeł i wybranego profilu;
- nieznany profil i niespójny przykład MCP;
- zachowanie wyniku przy niedostępnej diagnostyce.

Potwierdź:
- logi odpowiadają rzeczywistym kontrolom i danym;
- projekt oraz źródła aplikacji nie zostały zmienione;
- zdarzenia mają poprawną korelację;
- każda operacja ma jeden terminal;
- prawdziwy proces zachowuje oczekiwany wynik i komunikaty.

Nowe zdarzenia domenowe mogą wymagać dostosowania helperów
manager_logging. Zachowaj ścisłe asercje lifecycle CLI;
nie usuwaj ich ani nie ignoruj dowolnych dodatkowych zdarzeń.

W core/Cargo.toml już obowiązuje autotests = false; to ustawienie
wyłącza automatyczne wykrywanie testów, a nie rejestruje modułów.
Jeśli dodasz osobny plik testowy, dołącz go jawnie jako moduł
w core/src/lib.rs, zgodnie z istniejącą konwencją #[cfg(test)]
i #[path]. Potwierdź w wynikach wykonanie nowych testów.

Cykl:
oczekiwanie → test → uzasadnione RED →
minimalna implementacja → GREEN → bramka.

Testy Rust wyłącznie w wymaganej VM Debian 13.
RED: make test.
Końcowa bramka: make validate-rust-migration.
Dodatkowo: git diff --check.

Zachowaj Rust 1.85 i wymagania dotyczące dowodów MSRV.
Nie powtarzaj testów zawartych w udanej bramce dla formalności.

Brak wymaganej weryfikacji oznacza blokadę.
Zachowaj zmiany, zapisz przyczynę i rzeczywiste wyniki, ustaw
MANAGER-AUX na [!] w roadmap.md i zatrzymaj pracę. Jako następny
krok wskaż usunięcie blokady i dokończenie weryfikacji części A.
Nie osłabiaj testów ani izolacji.


7. ZAKOŃCZENIE

W roadmap.md zapisz:
- zakres wykonanej części A;
- rzeczywiste wyniki RED/GREEN i bramki;
- wersje kompilatora i Cargo;
- ścieżki artefaktów oraz ograniczenia.

Jeśli część A została ukończona i wymagana weryfikacja przeszła:
- MANAGER-AUX pozostaje [~], ponieważ części B i C nadal są otwarte;
- następny krok: część B — create/rename/bump.

Jeśli część A jest zablokowana:
- MANAGER-AUX ma status [!] z konkretną przyczyną;
- osobno zapisz wykonane zmiany i stan weryfikacji;
- następny krok rozwiązuje blokadę i wznawia część A;
- nie wskazuj części B jako następnej do wykonania.

Wykonaj wymagany worklog.
Podaj krótki raport zmian i weryfikacji.
Zatrzymaj się bez rozpoczynania części B.
