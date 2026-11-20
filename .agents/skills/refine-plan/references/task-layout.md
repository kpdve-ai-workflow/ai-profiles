# Task Instruction Layout

Use this reference when writing `<plan-directory>/tasks/T-xx.md`. Return to the [skill workflow](../SKILL.md) for the dialogue and approval rules.

The template below uses Polish headings as an example. Localize it to the user's language. Preserve the existing task ID, requirement IDs, scope and observed evidence. Replace placeholders with concrete content; a ready instruction cannot contain implementation-relevant placeholders.

## Ownership and working drafts

The parent plan remains the execution queue and source of execution/verification status. A task instruction is not a new task, decision backlog or audit report. Keep shared contracts in the parent plan and reference their exact sections.

During the workshop, an incomplete file is legitimate if it is marked `DRAFT`, identifies missing answers and has exactly one next question. Unresolved questions belong under blockers, not among binding instructions. Present a complete revision as `AWAITING APPROVAL`; mark it `APPROVED` only after the user approves that whole revision. Keep handoff readiness `BLOCKED` while any execution blocker remains. Preserve a previous approval reference if the new revision becomes `STALE`.

Include the two walkthroughs even for a small task: how the change will be introduced and how the resulting behavior will run. Compress them when simple; do not replace them with a file list. Other sections may be shortened or omitted when genuinely irrelevant. Do not invent interfaces, persistent state or rollout operations to fill the template.

## Template

````md
# T-xx — <nazwa istniejącego zadania>

**Plan i zadanie:** <względny link do nadrzędnego wpisu T-xx>.
**Rewizja instrukcji:** r1.
**Zatwierdzenie instrukcji:** DRAFT | AWAITING APPROVAL | APPROVED | STALE.
**Gotowość przekazania:** READY | BLOCKED — <konkretny powód, jeśli zablokowane>.
**Akceptacja całej rewizji:** <rzeczywiste potwierdzenie użytkownika lub „brak”>.
**Wykonanie i weryfikacja:** źródłem statusu jest nadrzędny wpis T-xx; zatwierdzenie tej instrukcji nie oznacza wykonania ani przejścia testów.
**Następne działanie:** <jedno pytanie, prośba o akceptację albo konkretny krok po zatwierdzeniu>.

> <Dla niezatwierdzonej lub zablokowanej instrukcji: nie wykonywać tej rewizji. Wskazać, czy wcześniejszy zatwierdzony kontrakt nadal obowiązuje, czy ujawniona luka blokuje również dotychczasowy plan.>

## Cel i granice

<Obserwowalny rezultat zadania i powiązane wymagania. Linki do nadrzędnego zakresu i wspólnych kontraktów; tylko ograniczenia istotne dla tego zadania.>

**Zależności:** <rzeczywiste wymagane zadania/kontrakty i warunek ich gotowości>.
**Bez zmian:** <elementy, których zadanie nie może zmienić>.

## Ustalenia potwierdzone w źródłach

| Ustalenie | Dowód i zakres aktualności | Znaczenie dla zadania |
| --- | --- | --- |
| <fakt, nie wybór projektowy> | <odczytany plik i symbol, stan kodu lub sprawdzone źródło> | <konsekwencja> |

<Oddziel aktualnie odczytane fakty od informacji tylko zgłoszonych w planie. Nie twierdzić, że odczyt kodu potwierdza działanie runtime.>

## Wybrane rozwiązanie i zgody

| Problem | Jedno wybrane rozwiązanie | Uzasadnienie | Pochodzenie / zatwierdzenie |
| --- | --- | --- | --- |
| <konkretny problem> | <wiążąca reguła albo jawnie oznaczona propozycja> | <istotny powód> | <USER-APPROVED / APPROVED SPEC / PLANNER-SELECTED w granicach jawnej delegacji / PROPOSED; rzeczywiste źródło> |

<Odsyłacze do już obowiązujących decyzji zamiast ich ponownego uzgadniania. Brak wyboru pomiędzy równorzędnymi alternatywami w instrukcji gotowej do wykonania.>

## Jak będzie wyglądało wdrożenie — opis dla człowieka

<Krótki, spójny opis stanu początkowego i całej drogi do rezultatu. Wyjaśnić, co zobaczy użytkownik, jeśli dotyczy.>

1. <Od czego zaczynamy i co musi być gotowe przed zmianami.>
2. <Jaką ochronę testową przygotujemy/potwierdzimy i co ma ona wykazać przed zmianą kodu.>
3. <Co dokładnie zmienimy, w jakiej kolejności i dlaczego; powiązać elementy systemu i miejsca zmian.>
4. <Jak potwierdzimy rezultat oraz jak nastąpi aktywacja, jeśli potrzebna.>

**Przerwanie i odzyskanie:** <gdzie bezpiecznie się zatrzymać, co pozostaje zachowane i jaki jest rzeczywisty sposób odzyskania/rollbacku, jeśli dotyczy>.

## Jak będzie działać rozwiązanie — ścieżki wykonania

### Ścieżka główna

<Wejście/wyzwalacz → konkretne komponenty lub funkcje w kolejności → zmiany stanu → widoczny wynik i skutki uboczne. Objaśnić przejścia ludzkim językiem, nie tylko schematem.>

### Istotne odgałęzienia

| Warunek wejściowy lub zdarzenie | Przebieg w wybranym rozwiązaniu | Wynik / błąd / stan końcowy |
| --- | --- | --- |
| <istotna awaria, granica, powtórzenie, anulowanie lub inna ścieżka> | <dokładnie co zostanie wykonane i czego nie wykonujemy> | <jednoznaczny rezultat> |

<To gałęzie zachowania, nie alternatywne projekty pozostawione wykonawcy.>

## Dokładny kontrakt techniczny

### Miejsca i odpowiedzialności

| Ścieżka i symbol | Stan | Dokładna zmiana / odpowiedzialność |
| --- | --- | --- |
| <konkretny plik i symbol> | <istniejące, odczytane / planowany nowy element> | <co dodać, zmienić lub ponownie wykorzystać> |

### Interfejsy i algorytm

<Konkretne wejścia, wyjścia, typy/sygnatury, formaty, klucze i wartości domyślne, kolejność operacji oraz kompatybilność — tylko istotne elementy. Pseudokod, przykład lub schema, jeśli opis pozostawiałby różne implementacje.>

### Stan, awarie i bezpieczeństwo

<Kto posiada stan, kiedy się zmienia, jak propagują się błędy; istotne reguły retry, timeoutu, współbieżności, idempotencji, anulowania i cleanup. Uprawnienia, sekrety i zachowanie istniejących danych, jeśli dotyczy. Wartości i reguły muszą być ustalone, nie pozostawione do wyboru.>

## Kontrakt test-first

**Wymaganie obserwowalne:** <co musi zachodzić>.
**Niezależne źródło oczekiwań:** <zatwierdzone wymaganie, kontrakt, uzasadniony niezmiennik itp.; nie wyjście bieżącego kodu>.
**Warstwa i miejsce testów:** <rzeczywisty framework, pliki i seam; nowe pliki oznaczyć jako planowane>.
**Fixture, izolacja i mocki:** <konkretne przygotowanie oraz granice; co musi pozostać rzeczywistą integracją>.

| Scenariusz / stan początkowy | Działanie | Oczekiwany wynik i źródło | Jaki prawdopodobny błąd wykrywa |
| --- | --- | --- | --- |
| <wartościowy przypadek> | <wejście lub sekwencja> | <obserwowalny wynik, nie kopia implementacji> | <kontrprzykład> |

**Komenda ukierunkowana i wymagania środowiska:** <dokładna, wykonalna komenda, katalog roboczy i znane wymagane zgody>.
**Oczekiwane RED:** <sygnał luki zachowania, nie błąd środowiska; albo uzasadnienie braku sztucznego RED dla ochrony zachowania>.
**Warunek wejścia w zmianę produkcyjną:** <jakie potwierdzenie ochrony/RED jest wymagane; jawna autoryzowana alternatywa tylko jeśli rzeczywiście istnieje>.
**Warunek GREEN i regresje:** <oczekiwany wynik tych samych testów, komendy i zakres regresji>.
**Weryfikacja zaobserwowana:** <rzeczywiście wykonane komendy, stan kodu i wyniki albo NOT RUN / NOT RUN — PROHIBITED; nie mylić z oczekiwaniami>.

## Instrukcja wykonania

1. Sprawdź zatwierdzenie tej rewizji, zależności, blokady, zgodność źródeł i wymagane uprawnienia.
2. <Przygotuj wskazane testy według kontraktu scenariuszy i niezależnych oczekiwań.>
3. <Uruchom wskazaną komendę, jeśli dozwolona; potwierdź wymagane RED lub ochronę istniejącego zachowania. Przy nieoczekiwanej przyczynie zatrzymaj zależne prace.>
4. <Po spełnieniu warunku test-first wykonaj dokładnie ustalone zmiany w podanej kolejności; rozpisz na osobne kroki, gdy potrzebne.>
5. <Uruchom te same testy oraz ustalone regresje. Refaktoryzuj wyłącznie z działającą ochroną.>
6. <Przeprowadź uzgodnioną aktywację/odbiór, jeśli dotyczy i jest autoryzowany. Zapisz rzeczywisty stan wykonania i weryfikacji w planie.>

## Kryteria odbioru

- <Obserwowalny rezultat zadania wraz z istotnymi negatywnymi ścieżkami.>
- <Zachowane granice, kompatybilność i wymagane potwierdzenie ochrony testowej.>
- <Niezbędna ręczna akceptacja, jeśli rzeczywiście wymagana; nie przedstawiać oczekującego odbioru jako wykonanego.>

## Braki informacji i blokady

| Dokładne pytanie / luka | Sprawdzone dowody i przyczyna blokady | Dotknięte zadania / zależności | Kto lub co rozstrzyga | Co odblokuje |
| --- | --- | --- | --- | --- |
| <jeden konkretny brak> | <co wiadomo i czego brakuje> | <T-xx i zadania wymagające jego ukończenia> | <użytkownik / źródło / uprawniona osoba> | <konkretna odpowiedź lub dowód> |

<Jeśli nie ma blokad, napisać krótko „Brak”, bez pustej tabeli. Otwarte pytania nie mogą współistnieć ze statusem gotowości READY, jeśli wpływają na wykonanie.>

## Reguła zatrzymania wykonawcy

Nie wykonuj instrukcji niezatwierdzonej, nieaktualnej lub zablokowanej ani zadań wymagających ukończenia zablokowanego zadania. Przed zmianami sprawdź zgodność kodu z założeniami i wymagane zgody. Jeżeli instrukcja pozostawia wybór, dowody jej przeczą, nie można wykonać wymaganej weryfikacji albo RED wynika z nieoczekiwanej przyczyny, zatrzymaj zależne prace i zgłoś konkretną lukę właścicielowi planu. Nie wybieraj alternatywy, nie osłabiaj asercji i nie rozszerzaj zakresu. Zachowaj dotychczasowe zmiany. Kontynuuj tylko prace jawnie potwierdzone jako niezależne i bezpieczne; gdy nie można oddzielić wpływu, wstrzymaj całe przekazanie. Wznów po autoryzowanym i zatwierdzonym rozstrzygnięciu.
````

## Final review

Check both directions: each walkthrough step must map to a specified operation, and each important operation must be understandable in the walkthrough. Relevant runtime branches must have consistent contract and test expectations. A diagram alone does not establish either correspondence.

Remove drafting guidance and inapplicable placeholders before requesting whole-instruction approval. Keep genuine blockers visible rather than deleting them to obtain `READY`. Do not fill future verification with fabricated results. A source-access problem, unclear oracle or missing approval remains a blocker even if the rest of the document looks complete.
