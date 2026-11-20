# Refine Plan — wspólne doprecyzowanie zadania

`refine-plan` rozszerza [`make-plan`](../make-plan/README.md): prowadzi rozmowę nad konkretnym zadaniem istniejącego planu, pomaga wybrać rozwiązanie techniczne i zapisuje zatwierdzoną instrukcję dla wykonawcy. Nie implementuje zadania.

Celem jest rozstrzygnięcie sposobu realizacji **przed przekazaniem pracy agentowi wykonującemu**. Wykonawca dostaje jedną instrukcję, a przy nieprzewidzianej rozbieżności ma się zatrzymać i zgłosić brak — nie wybierać rozwiązania samodzielnie.

## Gdzie szukać informacji

| Chcesz… | Otwórz |
| --- | --- |
| Uruchomić warsztat i poznać jego wynik | [Sposób użycia](#sposób-użycia) |
| Poznać reguły rozmowy, decyzji i zatwierdzania | [SKILL.md](SKILL.md) |
| Zobaczyć strukturę szczegółowej instrukcji zadania | [Szablon instrukcji](references/task-layout.md) |
| Najpierw stworzyć plan z wymagań | [make-plan](../make-plan/README.md) |

Instrukcje skilla są po angielsku; rozmowa i wynikowe dokumenty powstają w języku użytkownika. Skill korzysta z `make-plan`, a przy zmianach zachowania z `test-gap-discovery`; te skille powinny być dostępne w projekcie.

## Sposób użycia

W Pi:

```text
/skill:refine-plan Przejdźmy wspólnie przez T-02 w todos/pi-performance/implementation-plan.md. Omawiaj jeden problem techniczny naraz. Nie implementuj zadania.
```

To przykład wywołania, nie polecenie rozpoczęcia zmian w tym planie. Bez wskazanego zadania agent proponuje jedno na podstawie planu i prosi o potwierdzenie.

1. Agent sprawdza zadanie, wcześniejsze decyzje i odpowiednie fragmenty repozytorium.
2. Przedstawia **jeden problem**, istotne możliwości, ich konsekwencje oraz rekomendację.
3. Zadaje jedno pytanie i czeka na odpowiedź. Zapisuje ustalenie, zanim przejdzie do kolejnego problemu.
4. Przygotowuje kompletną rewizję instrukcji i prosi o jej osobne zatwierdzenie. Zgody na pojedyncze decyzje nie zastępują tej akceptacji.
5. Po zatwierdzeniu łączy instrukcję z planem. Nie rozpoczyna automatycznie implementacji.

## Gdzie powstaje wynik

Przy planie `<katalog-planu>/implementation-plan.md` szczegóły zadania trafiają do **`<katalog-planu>/tasks/T-xx.md`**, z zachowaniem istniejącego ID. Plan i instrukcja linkują do siebie.

Plan pozostaje źródłem zakresu, wspólnych kontraktów, zależności oraz statusów wykonania i weryfikacji. Plik zadania zawiera:

- wybrane rozwiązanie, źródła ustaleń i rzeczywiste zgody;
- **opis dla człowieka:** jak wdrożenie przebiegnie krok po kroku oraz jak zadziałają ścieżka główna i istotne odgałęzienia;
- **dokładny kontrakt:** pliki, symbole, interfejsy, algorytm, stan, błędy i granice bezpieczeństwa w zakresie istotnym dla zadania;
- scenariusze, niezależne oczekiwania, lokalizacje testów, komendy i kolejność test-first;
- procedurę wykonania, kryteria odbioru, blokady i regułę zatrzymania.

Nie ma osobnej kolejki zadań ani kopii wszystkich szczegółów w obu dokumentach. Szkic nie zastępuje wiążącego kontraktu; rzeczywista luka odkryta w dotychczasowym planie blokuje jednak zależne prace.

## Zatwierdzenie nie oznacza wykonania

Instrukcja przechodzi przez `DRAFT` → `AWAITING APPROVAL` → `APPROVED`. Istotna zmiana zatwierdzonego kontraktu wymaga nowej akceptacji; nieaktualna zgoda jest oznaczana `STALE`.

`READY` wymaga kompletnej, zatwierdzonej i niezablokowanej instrukcji. Nie oznacza przejścia testów ani wdrożenia. Skill planuje weryfikację, ale sam warsztat nie upoważnia do pisania lub uruchamiania testów, połączeń z usługami czy zmian produkcyjnych.

## Dostępność w Pi

Katalog `.agents/skills/refine-plan/` jest standardową projektową lokalizacją skilla w Pi; nie wymaga dodatkowej paczki ani zależności npm. Po dodaniu plików do aktywnej sesji użyj `/reload`, a następnie `/skill:refine-plan`.
