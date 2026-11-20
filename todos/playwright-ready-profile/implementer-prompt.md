# Implementer — dokładnie jedno zadanie

Pracujesz w repozytorium `ai-profiles`. Handoff: `todos/playwright-ready-profile/`. W tej sesji wykonaj dokładnie jedno zadanie, potem zgłoś wynik i zatrzymaj się. Polecenie „wykonaj plan” lub „kontynuuj” nie upoważnia do realizacji całej kolejki.

1. Przeczytaj `todos/playwright-ready-profile/plan.md`: stan, kontrakty, blokady, kolejkę i jedyne `Next action`. Jest to jedyny rejestr statusów, zależności i dowodów. Nie opieraj się na historii czatu. Jeżeli wszystko jest zaznaczone, zgłoś zakończenie bez ponawiania pracy.
2. Postępuj według wskaźnika. Przy blokadzie lub oczekiwanej akceptacji zajmij się wyłącznie jej rozstrzygnięciem/zgłoszeniem. Nie wybieraj innego zadania. W przeciwnym razie otwórz tylko wybrany plik zadania i potrzebne źródła/kontekst; nie czytaj domyślnie wszystkich przyszłych zadań.
3. Potwierdź ukończenie zależności i wykonalność zakresu/weryfikacji. Przeczytaj `AGENTS.md`, `docs/testing.md` i ewentualne lokalne instrukcje. Przed projektowaniem, dodawaniem, poprawianiem lub oceną testów wczytaj skill `test-gap-discovery`, jeśli dostępny. Brak skilla zgłoś i nadal stosuj tę metodykę. Przy Rust korzystaj ze skilla `rust`, jeśli dostępny.
4. Sprawdź rzeczywisty working tree. Zachowaj zastane zmiany użytkownika, także w instrukcjach i `docs/testing.md`. Wznawiaj przerwane zadanie na podstawie zapisu oraz aktualnych plików, nie przez ślepy restart. Nie powtarzaj zaakceptowanych zadań automatycznie.
5. Przed implementacją ustaw w planie wybrane zadanie na `IN PROGRESS`. Wykonaj tylko jego kontrakt, test-first i wymagane bramki. Nie projektuj architektury od nowa, nie implementuj kolejnego zadania i nie automatyzuj VPN ani konfiguracji komputera.
6. Uzupełnij wyłącznie w `plan.md`: zmienione pliki, faktyczne polecenia i wyniki, nazwane nowe testy, postęp implementacji, pozostałe ograniczenia i dokładne blokady. Zachowaj historię. Pliki zadań pozostają kontraktami, nie drugim dziennikiem statusów.
7. Checkbox zaznacz dopiero po spełnieniu kryteriów i przejściu wymaganej weryfikacji. Zaktualizuj podsumowanie i dokładnie jeden `Next action`: następne niezaznaczone zadanie, bez rozpoczynania go. Przy przerwaniu/blokadzie/oczekującej akceptacji wskaźnik pozostaje przy tym zadaniu. Gdy cała kolejka jest zaznaczona, wskaż zgłoszenie zakończenia bez dodawania pracy.
8. Krótko zgłoś wynik tej sesji, zmiany, rzeczywiste sprawdzenia i ewentualną blokadę. Zatrzymaj się.

## Statusy i akceptacja

- Execution: `TODO`, `IN PROGRESS`, `IMPLEMENTED`, `BLOCKED`.
- Verification: `NOT RUN`, `PASSED`, `FAILED`, `MANUAL PENDING`, `NOT APPLICABLE`.
- `IMPLEMENTED` / `MANUAL PENDING` pozostaje niezaznaczone. Nie kasuj wykonanego postępu.
- Niedostępna wymagana weryfikacja oznacza `BLOCKED` oraz `NOT RUN` lub `FAILED`, jeżeli rzeczywiście podjęto próbę i zakończyła się błędem. Zachowaj zmiany i dokładny błąd; nie przesuwaj kolejki.
- `NOT APPLICABLE` wymaga zadaniowego uzasadnienia, że sprawdzenie rzeczywiście nie dotyczy zakresu. Nie zastępuje brakującej VM, uprawnień, czasu lub nieudanego testu.

## Obowiązkowa metodyka testowania

Najpierw ustal obserwowalne wymaganie i niezależny oracle: wymaganie użytkownika/kontrakt/standard/sprawdzony przykład/niezmiennik. Obecny wynik implementacji nie jest dowodem poprawności. Wyprowadź istotne scenariusze granic, błędnych danych, sekwencji, ponowień, częściowych awarii i izolacji. Wskaż wiarygodny błędny wariant i asercję, która go wykryje; dobierz poziom, fixture i granice mockowania.

Dla nowego/naprawianego zachowania: oczekiwanie → test → uzasadnione RED dla brakującego zachowania → minimalna implementacja → GREEN → wymagana regresja. Błąd infrastruktury nie jest RED produktu. Poprawny test istniejącego zachowania może od razu być GREEN; zbadaj jego czułość i nie psuj kodu, aby wymusić RED. Dla dokumentacji stosuj kontrolę treści/linków/spójności, bez sztucznego RED.

Mock specyfikacji procesu nie dowodzi prawdziwej integracji. Nie mockuj HTTP/TLS/Chromium, jeśli ich działanie test ma potwierdzić. Testy Rust i Playwright wykonuj wyłącznie w wymaganej VM przez istniejące Make zgodnie z `docs/testing.md`. Testy Rust w nowym pliku muszą być jawnie zarejestrowane w lib.rs i widoczne w wynikach. Kontrola orkiestracji nie zastępuje E2E. Nie uruchamiaj testów aplikacji lokalnie ani nie korzystaj z prawdziwych credentials/firmowego VPN jako fixture.

Diagnozuj niepowodzenie: defekt produktu, błędne oczekiwanie, wadliwe dane, niedeterministyczność lub infrastruktura. Nie ma obowiązku znalezienia błędu ani minimalnej liczby przypadków. Mutacje wymagają osobnej zgody i jednorazowej izolacji; nie są wymagane. Nie twórz dodatkowego raportu dla każdego testu.

## Zatrzymanie

Zatrzymaj się na brakującym uprawnieniu, niedostępnej wymaganej weryfikacji, nieoczekiwanym błędzie, niejasności lub sprzeczności wpływającej na wybrane zadanie. Zapisz dokładną blokadę i faktyczne wyniki w planie. Nie zgaduj, nie zmieniaj jednostronnie kontraktu testowego, nie osłabiaj asercji, nie dodawaj skipów dla GREEN i nie obchodź izolacji. Nie resetuj/revertuj cudzych zmian, nie sprzątaj spekulacyjnie i nie przechodź do innego zadania.

To instrukcje workflow, a nie techniczna granica uprawnień.
