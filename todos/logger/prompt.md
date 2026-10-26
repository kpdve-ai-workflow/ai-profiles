1. Kontekst operacji, wybór klientów i przygotowanie.
2. Zapis plików, aktywacja, zapis stanu i częściowe niepowodzenia.
3. Odinstalowanie i zamknięcie całego zadania.

Poniższy prompt jest wspólny dla wszystkich trzech uruchomień. Agent wykonuje jedną część, zapisuje postęp w roadmapie i zatrzymuje się. Przy kolejnym uruchomieniu wklejasz ten sam prompt.

Wykonaj jedną część zadania 2.3 — PROFILE-LIFECYCLE
z todos/logger/roadmap.md.

Repozytorium:
 /var/home/kacperzielinski/it/projects/personal/ai-profiles

Cel całego zadania:
Dodaj diagnostykę install/update/switch/uninstall, dzięki której można
odtworzyć przygotowanie, decyzje, zmiany, aktywację, zapis stanu,
błędy i zachowane wyniki częściowe dla każdego klienta.

Zachowanie aplikacji musi pozostać zgodne z dotychczasowym kontraktem.
To zadanie implementacyjne: wykonaj zmiany i wymaganą weryfikację,
a nie tylko przygotuj kolejny plan.

W jednym uruchomieniu wykonaj wyłącznie jedną z części A, B lub C.
Nie rozpoczynaj następnej części po ukończeniu wybranej.


1. ODCZYTAJ KONTEKST I USTAL WYBRANĄ CZĘŚĆ

Przeczytaj:
- AGENTS.md
- README.md
- docs/testing.md
- todos/logger/AGENTS.md
- todos/logger/README.md
- todos/logger/roadmap.md
- todos/logger/knowledge-base.md
- todos/logger/dependencies.md
- todos/logger/decisions-and-gaps.md
- todos/logger/quality-and-acceptance.md
- todos/logger/concepts/profiles.md

Stosuj dostępne skille:
- test-gap-discovery przy projektowaniu i ocenie testów;
- rust przy pracy nad kodem Rust;
- pozostałe wymagane instrukcjami projektu, jeśli dotyczą zmian.

Jeśli test-gap-discovery jest niedostępny, zgłoś to i nadal stosuj
metodykę z docs/testing.md.

Nie czytaj archiwalnego todos/logger/logger.md jako bieżącego planu.
Bieżące statusy i kolejność należą wyłącznie do roadmap.md.

Sprawdź git status i istniejące zmiany. Zachowaj cudze zmiany.
Przy wznowieniu porównaj zapisany postęp z rzeczywistym kodem,
testami i dowodami. Nie zaczynaj automatycznie od początku.

Wybierz część:
- A, jeśli nie ma dowodów ukończenia A;
- B, jeśli A została wykonana i wymagana weryfikacja przeszła;
- C, jeśli A i B zostały wykonane i zweryfikowane.

Jeśli wybrana część jest rozpoczęta albo zablokowana, wznów ją.
Nie pomijaj jej na rzecz następnej.

Przed zmianami napisz krótko:
- którą część wykonujesz;
- jakie zachowanie będzie obserwowalne po jej ukończeniu;
- jak potwierdzisz poprawność.

Nie twórz drugiej roadmapy ani osobnego rejestru postępu.


2. ZWERYFIKOWANE PUNKTY WEJŚCIA

Główny przepływ produkcyjny:
core/src/profile_management/scoped.rs

Istotne symbole:
- run
- run_with_runner
- Manifest::load
- Manifest::save
- prepare
- Prepared::commit
- preflight_remove
- inspect_configuration
- remove

Przygotowanie i operacje natywne:
core/src/profile_management/native.rs

Istotne symbole:
- project_files_for
- configure_claude
- configure_claude_mcp
- configure_codex
- configure_codex_mcp
- write_config
- activate_claude_with_runner
- activate_selected_codex
- run_cli
- remove_claude
- remove_codex
- cleanup_claude
- cleanup_codex

Payload instalacyjny:
core/src/profile_management/payload.rs
- desired_files_for
- PayloadFile
- PayloadKind

Ważne:
installation::apply_profile jest oznaczone #[cfg(test)] i służy
historycznym fixture’om formatu 1. Nie traktuj go jako głównego
produkcyjnego instalatora. Instrumentuj faktyczny przepływ scoped.rs.
W installation.rs zmieniaj tylko potrzebne helpery produkcyjne.

Istniejąca diagnostyka:
- core/src/logging.rs
- core/src/logging/entry.rs
- core/src/logging/session.rs
- core/src/process.rs
- core/src/process/diagnostics.rs

Wykorzystaj:
- tracing i istniejące sesje;
- logging::current_session;
- Session::payload;
- PayloadSummary;
- istniejącą propagację kontekstu diagnostycznego procesów.

Testy i fixture’y:
- core/tests/lifecycle.rs
- core/tests/process_logging.rs
- core/tests/manager_logging.rs
- core/tests/manager_fixture.rs
- core/src/lib.rs

W lifecycle.rs istnieją przykłady:
- przygotowania syntetycznego repozytorium;
- kontrolowanych runnerów aktywacji;
- konfliktów przed zapisem;
- zachowania wcześniejszych sukcesów przy późniejszej awarii;
- ochrony obcych plików i rekordów klientów.

Przeczytaj odpowiednie fragmenty przed ich rozszerzaniem.


3. WSPÓLNY KONTRAKT IMPLEMENTACYJNY

Poniższe wybory techniczne organizują realizację istniejącego kontraktu.
Nie zmieniają wymagań produktowych.

Korelacja:
- Jeden operation_id identyfikuje jedno wywołanie operacji profilu.
- operation_id jest jednoznaczny w obrębie run_id.
- Zdarzenia zawierają kontekst action, project, client i stage,
  tam gdzie te informacje są dostępne.
- Użyj spanów tracing dla operacji, klienta i etapu.
- Aktywacja ma działać we właściwym kontekście, aby istniejące
  zdarzenia procesów można było powiązać z operacją i klientem.
- Nie zastępuj istniejących identyfikatorów procesów.

Organizacja:
- Wspólne helpery diagnostyczne umieść w nowym prywatnym module
  core/src/profile_management/diagnostics.rs.
- Zarejestruj moduł w core/src/profile_management.rs.
- Nie przenoś do niego logiki biznesowej instalatora.
- Nowe testy diagnostyczne dodawaj do już zarejestrowanego
  core/tests/lifecycle.rs, wykorzystując jego fixture’y.
- Jeśli niezbędny jest osobny moduł testowy, zarejestruj go jawnie
  w core/src/lib.rs. autotests = false.

Zdarzenia:
Używaj stabilnej przestrzeni nazw profile.*.
Rozdziel:
- rozpoczęcie i rzeczywisty wynik operacji;
- wybór lub pominięcie klienta;
- przygotowanie;
- planowaną i wykonaną zmianę;
- aktywację;
- zapis manifestu;
- obsłużony błąd.

Nazwy zdarzeń i ich pola ustal w testach części A, a następnie
rozszerzaj kompatybilnie. Wynik całej operacji nie może sugerować
pełnego pokrycia diagnostycznego etapów jeszcze niewdrożonych.

Prawdziwość:
- Przygotowanie nie jest zapisem.
- Plan usunięcia nie jest wykonanym usunięciem.
- Zapis plików nie dowodzi zapisu manifestu.
- Zmiana mapy klientów w pamięci nie dowodzi utrwalenia stanu.
- Awaria aktywacji nie oznacza cofnięcia przygotowanych plików.
- Nie emituj rollbacku, jeśli aplikacja go nie wykonuje.
- Rozróżniaj wynik operacji, stan aktywacji, trwałość stanu
  i kompletność diagnostyki.

Aktywacja:
Zachowaj rzeczywiste wartości, w tym:
prepared, pending, synchronized, failed i disabled.

Nie traktuj pending jako failed, jeśli aktualny kontrakt pozwala
zwrócić sukces z oczekującą aktywacją.

Błędy:
- Rejestruj dostępną przyczynę przy rzeczywistej granicy obsługi.
- Zrób to przed redukcją błędu do nazwy klienta lub tekstu.
- Zachowuj dostępny łańcuch source i dane błędu I/O.
- Nie wymyślaj struktury, której błąd źródłowy nie udostępnia.
- Nie przebudowuj wszystkich typów błędów na potrzeby loggera.
- Podsumowanie klienta i operacji może odwoływać się do błędu;
  nie przedstawiaj jego propagacji jako kolejnych niezależnych awarii.

Dane:
- Utrwalaj potrzebną treść przez istniejące API payloadów.
- Wykorzystuj dane już odczytywane lub przygotowane przez operację.
- Gdy potrzebne są surowe bajty konfiguracji, przechwyć je przy
  istniejącym odczycie, przed parsowaniem.
- Nie skanuj niezwiązanych plików ani całego repozytorium.
- Nie wprowadzaj maskowania, cichego skracania ani stratnej
  konwersji bajtów.
- Referencja payloadu musi wskazywać rolę danych, ścieżkę,
  korelację, długość, checksum i rzeczywistą kompletność.

Niezmienione zachowanie:
- Zachowaj przygotowanie bez zapisu do projektu docelowego.
- Zachowaj kolejność klientów: OpenCode, Claude, Codex.
- Zachowaj istniejące granice commitów i obsługi błędów.
- Zachowaj stdout, komunikaty, kody, pliki, uprawnienia,
  aktywację i cleanup.
- Awaria loggera nie może zastąpić wyniku operacji.
- Korzystaj z obecnej sygnalizacji degradacji diagnostyki.
- Nie dodawaj nowych retry, rollbacków ani transakcji.
- Nie dodawaj zależności, Tokio, eksportu ani nowego loggera.

Zakres:
Nie realizuj MANAGER-AUX, restore, MCP runtime, wnętrza Novamiry,
obserwacji po exec ani eksportu.
Wspólne funkcje konfiguracji instrumentuj tylko w zakresie
wywołania należącego do PROFILE-LIFECYCLE.


4. CZĘŚĆ A — WYBÓR KLIENTÓW I PRZYGOTOWANIE

Rezultat:
Z logów można odtworzyć rozpoczęcie operacji, wybranych klientów,
użyty profil, poprzedni stan, pominięcia, przygotowanie i konflikt
zatrzymujący operację przed commitami.

Zakres:
- Wspólny kontekst operacji dla czterech akcji.
- Rozpoczęcie i rzeczywisty końcowy wynik wywołania.
- Odczyt manifestu i dostępne błędy jego odczytu.
- Wybór klientów i efektywnego profilu.
- Rozpoczęcie oraz wynik przygotowania.
- Planowane zmiany wynikające z przygotowania.
- Preflight odinstalowania.
- Pominięcia i konflikty.

Nie instrumentuj status jako części tego zadania.

Wymagane rozróżnienia:
- install all pomija klienta z tym samym profilem;
- install jednego już zainstalowanego klienta zachowuje błąd;
- install all odrzuca konflikt profilu przed commitami;
- update/switch/uninstall all pomijają niezainstalowanych klientów;
- brak zainstalowanych klientów przy tych operacjach zachowuje błąd;
- klient przygotowany przed późniejszym konfliktem nie jest
  raportowany jako zapisany.

Testy wyprowadź z kontraktu klientów i rzeczywistego stanu fixture.
Najważniejsze scenariusze:
- przygotowanie poprawnej instalacji;
- pominięcie zgodnego profilu i pełny no-op;
- konflikt późniejszego klienta przed jakimkolwiek commitem;
- brak instalacji;
- nieprawidłowy manifest;
- dwa wywołania w jednej sesji z różnymi operation_id.

Asercje muszą porównywać logi z plikami i manifestem.
Test konfliktu ma wykrywać wiarygodny błąd: raportowanie wcześniejszego
klienta jako zmienionego mimo braku commitów.

Zakończ po RED, implementacji, GREEN, wymaganej bramce i aktualizacji
postępu A. Nie rozpoczynaj B.


5. CZĘŚĆ B — ZAPIS, AKTYWACJA I WYNIKI CZĘŚCIOWE

Zależność: część A ukończona i zweryfikowana.

Rezultat:
Install/update/switch pokazują faktyczne zmiany plików, wynik aktywacji,
zapis manifestu i stan pozostawiony po awarii.

Zakres:
- Prepared::commit;
- usuwanie wycofanych zarządzanych plików;
- zapisy danych i plików wykonywalnych;
- zmiany konfiguracji;
- potrzebne payloady danych i stanu;
- aktywacja Claude/Codex oraz prepared dla OpenCode;
- Manifest::save;
- błędy klienta przed spłaszczeniem do errors.push(client);
- końcowe podsumowanie wyników klientów.

Przy zmianach plików rozróżniaj próbę, sukces i błąd.
Sukces zapisu manifestu emituj po rzeczywistym sukcesie save_manifest.

Oddziel:
- przygotowany payload;
- wykonane zmiany;
- wynik aktywacji;
- rekord w pamięci;
- potwierdzony zapis manifestu.

Nie deklaruj, że stan został utrwalony, jeśli zapis manifestu zawiódł.
Nie zmieniaj istniejącego przepływu po takim błędzie.

Testy:
- poprawny install/update/switch;
- update usuwający wycofany plik;
- aktywacja pending przy niedostępnym CLI;
- aktywacja synchronized;
- aktywacja failed z zachowanym przygotowanym stanem;
- późniejsza awaria klienta przy zachowaniu wcześniejszych sukcesów;
- błąd zapisu pliku lub manifestu;
- utrata sprawności loggera przy zachowanym wyniku operacji;
- odtworzenie kontrolowanych bajtów z payloadu.

Dla CLIENT=all potwierdź osobno stan wcześniejszych klientów,
stan klienta z błędem oraz wynik całej operacji.

Wykorzystaj kontrolowane ProcessRunner do wyników aktywacji.
Nie przedstawiaj tych testów jako dowodu działania prawdziwych
zewnętrznych CLI.

Awarię zapisu wywołuj deterministycznie w jednorazowym fixture,
np. przez kontrolowaną kolizję ścieżki we właściwej granicy operacji.
Nie opieraj testu na wyścigu, losowym sleep ani zachowaniu chmod
zależnym od uruchomienia jako root.

Zakończ po RED, implementacji, GREEN, wymaganej bramce i aktualizacji
postępu B. Nie rozpoczynaj C.


6. CZĘŚĆ C — ODINSTALOWANIE I ZAMKNIĘCIE 2.3

Zależność: części A i B ukończone i zweryfikowane.

Rezultat:
Uninstall pokazuje preflight, cleanup, faktyczne usunięcia i zmiany
konfiguracji, zapis manifestu oraz pozostawiony stan przy awarii.

Zakres:
- preflight_remove i remove;
- potrzebne granice remove_claude/remove_codex;
- cleanup_claude/cleanup_codex i ich ostrzeżenia;
- usuwanie zarządzanych plików;
- przywracanie lub zachowanie konfiguracji;
- usunięcie rekordu klienta i rzeczywisty zapis manifestu;
- błędy i wyniki częściowe.

Ostrzeżenie cleanupu nie może zostać automatycznie przekształcone
w błąd uninstall, jeśli aktualny przepływ kontynuuje pracę.

Nie usuwaj obcych plików, lokalnych nadpisań, danych użytkownika
ani zamrożonych zasobów legacy poza istniejącym kontraktem.

Testy:
- poprawne usunięcie wybranego klienta;
- zachowanie rekordów i plików innych klientów;
- zachowanie plików użytkownika;
- konflikt preflight bez usuwania;
- awaria podczas usuwania;
- awaria zapisu manifestu po wykonanych usunięciach;
- uninstall all z wynikami częściowymi;
- zachowanie wyniku uninstall przy niedostępnej diagnostyce.

Dodaj proporcjonalną weryfikację prawdziwego procesu ai-profile
dla objętej funkcjonalności, korzystając z jednorazowego checkoutu
i wzorców manager_fixture.rs.

Sprawdź wynik procesu, stdout/stderr, pliki, manifest i diagnostykę.
Nie uruchamiaj aktywacji na rzeczywistych kontach użytkownika.
Nie osłabiaj istniejących asercji CLI.

Po wymaganej bramce oceń kompletność A–C.
Oznacz PROFILE-LIFECYCLE jako ukończone dopiero wtedy, gdy wszystkie
części mają potwierdzoną akceptację.


7. TEST-FIRST I ŚRODOWISKO

Każda część musi zawierać pełny cykl:

niezależne oczekiwanie → test → uzasadnione RED →
minimalna implementacja → GREEN → wymagana bramka.

Przed testem określ:
- wymaganie i niezależne źródło oczekiwania;
- obserwowalny wynik;
- wiarygodny błędny wariant wykrywany przez asercję;
- fixture, poziom testu i granicę mockowania.

Nie kopiuj oczekiwań wyłącznie z aktualnego wyniku programu.
Test ma potwierdzać prawdziwość logów i rzeczywiste skutki operacji.

Testy Rust wykonuj wyłącznie w wymaganej VM Debian 13 QEMU/KVM,
przez istniejącą orkiestrację, jako tester, z wymaganą izolacją.

Do RED użyj istniejącego:
make test

Po implementacji wykonaj:
make validate-rust-migration

Bramka obejmuje testy Rust, więc po jej sukcesie nie powtarzaj
make test wyłącznie dla formalności.

Ukierunkowane polecenia Cargo są dopuszczalne tylko wewnątrz VM,
zgodnie z istniejącymi mechanizmami synchronizacji, blokady,
użytkownika i TMPDIR. Nie uruchamiaj ich na hoście.

Zachowaj Rust 1.85.
Zmiana zależności albo użycie API wymagających dodatkowego dowodu
kompatybilności wymaga odpowiedniej weryfikacji MSRV w VM.

RED ma wynikać z brakującej diagnostyki, a nie z braku infrastruktury.
Test istniejącego poprawnego zachowania może od razu przejść.
Nie psuj kodu w celu sztucznego uzyskania RED.

Brak VM, uprawnień lub wymaganej weryfikacji:
- zatrzymaj pracę zależną od tej weryfikacji;
- zachowaj wykonane zmiany;
- zapisz dokładną blokadę;
- nie akceptuj części ani całego zadania.

Nie dodawaj skipów, nie osłabiaj asercji i nie obchodź izolacji.
Nie naprawiaj obcych problemów przez nieuzgodnione zmiany zakresu.


8. ROADMAPA I RAPORT KOŃCOWY

Postęp zapisuj przy istniejącym PROFILE-LIFECYCLE w roadmap.md.
Zachowaj stabilny identyfikator i jeden status nadrzędnego zadania.

Po rozpoczęciu:
- ustaw [~];
- dopisz podział A/B/C jako opis postępu;
- ustaw „Następne zadanie” na wybraną część 2.3.

Dla każdej części zapisuj osobno:
- implementację;
- weryfikację;
- wykonane polecenia;
- wynik RED/GREEN;
- wersje rustc/Cargo;
- ścieżki artefaktów;
- rzeczywiste ograniczenia lub blokady.

Nie twórz dodatkowych głównych zadań roadmapy.
Nie aktualizuj statusów w logger.md ani w specyfikacjach.

Po A lub B:
- PROFILE-LIFECYCLE pozostaje [~];
- wskaż następną część;
- zatrzymaj się.

Po C:
- ustaw [x] tylko przy pełnym potwierdzeniu A–C;
- zaktualizuj podsumowanie etapu 2;
- wskaż 2.4 — MANAGER-AUX jako następne zadanie;
- nie rozpoczynaj 2.4.

Jeśli nastąpi blokada, wskaż jej rozwiązanie jako następny krok,
zamiast przechodzić do kolejnej części.

Raport końcowy ma zawierać:
- wykonaną część;
- zmienione zachowanie diagnostyczne;
- istotne pliki;
- rzeczywiste wyniki weryfikacji;
- ograniczenia lub blokadę;
- aktualny status 2.3 i jeden następny krok.

Zakończ po jednej części.
