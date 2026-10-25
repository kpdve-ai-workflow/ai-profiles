# Roadmapa migracji: Brain → Writer → Implementer + Decision Guard + Herdr

## Cel i przyjęte rozstrzygnięcia

Przystosować **istniejące** środowisko do OpenCode V2. Mocny Brain analizuje, wyjaśnia rozwiązanie człowiekowi i po akceptacji zwraca **jeden kompletny tekstowy HANDOFF**, zamiast wykonywać wiele operacji zapisu. Tani Writer odtwarza z tego HANDOFF wszystkie potrzebne pliki Markdown bez zmiany decyzji. Tani Implementer wykonuje dokładnie jedno zadanie `T-xx` na uruchomienie, pod kontrolą Decision Guard. **Herdr jest warstwą uruchamiania, komunikacji i obserwacji sesji** — nie zastępuje kontraktów OpenCode, `plan.md` ani Decision Guard.

**Jednoznaczne rozstrzygnięcie dotyczące pliku `implementer-prompt.md`: pozostaje elementem docelowego pakietu.** Zawartość przygotowuje Brain wewnątrz HANDOFF, a zapisuje Writer. Nie kasować istniejących plików tego typu w aktywnych `todos/`. Początkowa uwaga z załączonego opisu, by ten plik usunąć, była sprzeczna z dalszą roadmapą i została zastąpiona tą zasadą.

**Bez** własnego wersjonowania handoffów, dodatkowej bazy danych, nowego frameworku orkiestracji i kompatybilności z OpenCode V1. Używamy istniejącego Herdr. Do tworzenia planu nie trzeba uruchamiać Implementera.

## Co zachowujemy z obecnych skilli

- `brain-to-implementer`: Brain odpowiada za wymagania, analizę, architekturę, kontrakty, podział zadań i sposób weryfikacji, a nie implementację.
- `make-plan`: sprawdzone fakty oddzielone od wymagań użytkownika i decyzji technicznych; gotowość zadania zależy od kompletności specyfikacji.
- `test-gap-discovery`: niezależne oczekiwania/oracles, test-first tam, gdzie właściwe, brak sztucznego RED i brak osłabiania testów.
- `todos/<slug>/plan.md` jako **jedyny** rejestr kolejki, zależności, statusów, rzeczywistych wyników i dokładnie jednej `next action`.
- `tasks/T-xx.md` jako niemodyfikowalne w sensie statusów kontrakty wykonania; `implementer-prompt.md` jako instrukcja startowa dla nowej sesji.
- Jedno `T-xx` na jedno uruchomienie Implementera; blokada przy brakach, wymaganych testach i sprzecznościach; wznowienie z persisted state + working tree.
- Bez nadpisywania aktywnych planów i bez kasowania historii postępu.

## Zmiana doświadczenia użytkownika: najpierw wyjaśnienie, potem plan

Skill `brain-to-implementer` ma najpierw **prostym językiem** opisać, co feature zmieni, jak będzie działał i co uzyska użytkownik. Unikać pustych ogólników. Przy refaktorze pokazać tabelę **Przed / Problem / Po / Korzyść**. Wyraźnie wymienić kwestie wymagające decyzji użytkownika. **Nie emitować od razu zadań TODO i pełnego HANDOFF przed akceptacją podejścia.** Po zatwierdzeniu Brain przygotowuje szczegółowy HANDOFF zawierający kompletną treść każdego pliku. To zatwierdzenie kierunku nie zastępuje późniejszych decyzji Human Judge dotyczących niejasnych operacji.

## Role i odpowiedzialności

| Rola | Myślenie | Uprawnienia | Uruchamianie |
| --- | --- | --- | --- |
| Migration Orchestrator | High | Audyt, aktualizacja skilli/profili/pluginów w uzgodnionym zakresie; bez destrukcyjnych zmian | Sesja główna, korzysta z Herdr do uruchamiania innych |
| Brain | High | Odczyt repo, analiza, objaśnienie, HANDOFF; **bez zapisu** | Oddzielny agent OpenCode w Herdr |
| Writer | Low | Zapis tylko plików dokumentacji handoffu | Oddzielny agent OpenCode w Herdr |
| Implementer | Low | Jedno aktywne `T-xx`, kod/testy i aktualizacja `plan.md`; egzekwowany Guard | Oddzielny agent OpenCode w Herdr |
| LLM Judge | Low / szybki | Ocena semantyczna i rekomendacja, bez autoryzowania operacji | Preferuj wywołanie z pluginu V2, nie osobny stały terminal Herdr |
| Agent Judge (opcjonalnie) | Odpowiedni oszczędny model | Głębsza inspekcja **read-only** tylko gdy brakuje dowodów | Sesja oceny uruchamiana tylko w razie potrzeby |
| Human Judge | Człowiek | `Approve once`, `Reject/Stop`, `Escalate to Brain` | UI OpenCode / zatwierdzone żądanie, **nie** automatyczne wejście Herdr |

**Konfiguracja `thinking` należy do OpenCode V2, nie do Herdr.** Profile agentów utworzyć/zmienić w faktycznie wykrywanej konfiguracji, np. `.opencode/agents/*.md` lub `opencode.json(c)`, wybierając `provider/model#variant` ze wspieranego katalogu. `high` i `low` nie są uniwersalne dla każdego modelu; najpierw sprawdzić `opencode models`, listę wariantów oraz model efektywnie wybrany przez każdą uruchomioną sesję. Wybrany model sesji primary może być niezależny od profilu; nie uznawać pliku profilu za dowód faktycznego thinking.

## Fazy migracji

### Etap 0 — Audyt i baseline

**Praca:** odczytaj rzeczywisty checkout, `AGENTS.md`, skille i referencje, aktywne konfiguracje, dostępne modele i profile, pluginy, polityki testów, working tree, `opencode --version`, `herdr --version`, `herdr status` i wynik `herdr agent list`. Zidentyfikuj lokalne/globalne konfiguracje oraz możliwe konflikty. Zmierz bazowy czas, liczbę wywołań i operacji zapisu dla reprezentatywnego flow, jeśli pomiar jest możliwy.

**Done:** raport rzeczywistych ścieżek, aktywnych możliwości i pomiarów — bez niezweryfikowanych twierdzeń.

### Etap 1 — Provisioning agentów i integracja Herdr

**Praca:** Migration Orchestrator sam tworzy lub aktualizuje profile Brain, Writer i Implementer oraz ustala konfigurację Judge. Nadaje model, zweryfikowany wariant thinking, uprawnienia i instrukcje. Audytuje Herdr i jego integrację z OpenCode V2; tam, gdzie potrzebne i autoryzowane, konfiguruje `herdr integration install opencode` zgodnie z dokumentacją i stanem lokalnym (przed zmianą globalnej konfiguracji wymaga zgody). Nie instaluje ponownie istniejących komponentów bez powodu.

Sprawdza przy pomocy Herdr utworzenie workspace/pane, uruchomienie OpenCode (`herdr agent start ... --kind opencode --pane ...`), wysłanie małego testowego promptu, `herdr agent wait` oraz `herdr agent read`. Pane trzeba utworzyć **przed** `agent start`; ID pozyskiwać z faktycznych odpowiedzi JSON, nie zgadywać. Argumenty OpenCode przekazywane po `--` wolno stosować dopiero po sprawdzeniu lokalnego CLI. Nie uruchamiać Implementera do rzeczywistych zmian przed aktywacją Guard.

**Done:** agentów można zidentyfikować, uruchomić i obserwować z Herdr; rzeczywiste modele i warianty w sesjach są zweryfikowane; integracja nie popsuła dotychczasowego OpenCode.

### Etap 2 — Brain: rozmowa o rozwiązaniu i jeden HANDOFF

**Praca:** zmień `brain-to-implementer` (bez utraty reguł `make-plan`). Faza A: proste objaśnienie efektu i flow; dla refaktoru tabela Przed/Problem/Po/Korzyść; pytania blokujące, następnie zgoda na podejście. Faza B: **jedna pełna odpowiedź HANDOFF**, zawierająca roadmapę, wymagania, fakty z repo, decyzje, kontrakty, testy, zależności i **pełną docelową treść** `plan.md`, `implementer-prompt.md`, `tasks/T-xx.md`. Żadnych zapisów z Brain. Niewyjaśnione kwestie = BLOCKED, nie zgadywanie.

**Done:** Brain ma technicznie ograniczony zapis; dopiero po akceptacji może wyprodukować kompletny HANDOFF; nie generuje częściowych, nieodtwarzalnych wskazówek.

### Etap 3 — Writer: materializacja 1:1

**Praca:** Writer działa z niskim thinking i zapisuje wyłącznie docelowy pakiet w `todos/<slug>/`. Nie dopowiada braków. Weryfikuje listę ścieżek, treści, ID, odnośniki, placeholdery i brak utraty bloków. Preferuj deterministyczne kopiowanie kompletnych bloków plikowych; LLM Writer może koordynować, ale nie parafrazować.

**Ważne dla Herdr:** odczyt terminalowy `herdr agent read` i stan `done` nie są gwarancją otrzymania całego długiego HANDOFF. Migration Orchestrator musi zweryfikować bezstratny kanał przekazania pełnej odpowiedzi (np. kontrolowany odczyt wyniku sesji OpenCode przez oficjalne API i przekazanie Writerowi; ewentualnie zewnętrzny zapis przez orchestrator, ale nie przez Brain) wraz z kontrolą integralności/liczby bloków. Jeśli nie ma bezstratnego kanału, **BLOCKED**. Nie polegać na ograniczonym scrollbacku.

**Done:** komplet docelowych dokumentów odpowiada treści HANDOFF; niekompletny pakiet nie zostaje opublikowany jako READY.

### Etap 4 — Implementer: jedno zadanie, bez przeprojektowania

**Praca:** popraw profil i reusable `implementer-prompt.md`; zadanie czyta `plan.md` oraz jedno `T-xx`. Wymusza zależności, testy, wpisy rzeczywistych wyników, status i stop. Przed uruchomieniem następnego zadania orchestrator sprawdza zapisany `next action`, working tree i rzeczywistą akceptację, nie tylko status Herdr `done`.

**Done:** ukończenie T-01 nie uruchamia T-02 bez nowej, jawnej inwokacji; przerwane zadania wznawiane bez nadpisywania.

### Etap 5 — OpenCode V2 Decision Guard: hard rules

**Praca:** plugin kontroluje operacje Implementera zgodnie ze sprawdzonym API V2. Rozpoznaje rolę, sesję, wybrany task i dozwolony zakres. Fast Rules: jasne `ALLOW`, twarde `DENY`, niejasne → LLM Judge. Zbadaj wywołania edit/write/patch, shell, custom tools i MCP oraz izolację innych kanałów zapisu. Hard DENY nie może zostać odblokowane przez model lub człowieka bez autoryzowanej zmiany polityki.

**Done:** testy potwierdzają ochronę obsługiwanych punktów wykonania, również alternatywnymi kanałami; brak auto-allow przy awarii pluginu.

### Etap 6 — LLM Judge → Human Judge

**Praca:** dla każdej niejasnej operacji użyj krótkiej oceny LLM (`ctx.generate.text()` wyłącznie jeśli dostępne w zainstalowanym API). Jeśli pojedyncza ocena nie wystarczy, opcjonalny read-only Agent Judge bada niezbędne dowody. **Każda operacja skierowana na tę ścieżkę następnie musi przejść Human Judge.** LLM tylko rekomenduje. Człowiek wybiera: `Approve once`, `Reject/Stop`, `Escalate to Brain`. Zgoda tylko na dokładnie określony zamiar; przy zmianie argumentów konieczna ponowna decyzja. Brak UI, błąd LLM/RPC i timeout → STOP, nigdy auto-approval. `Escalate` uruchamia analizę Brain przez Herdr tylko jako odczyt i propozycję rozstrzygnięcia; zmiana handoffu wymaga autoryzacji i Writera.

**Done:** test E2E pokazuje, że człowiek może odrzucić rekomendację LLM i że bez jego decyzji nie ma niejasnej operacji.

### Etap 7 — Orkiestracja docelowego workflow przez Herdr

**Praca:** główny orchestrator tworzy/wykorzystuje panele, odpala Brain, odbiera kompletne HANDOFF, deleguje materializację Writerowi, po jej weryfikacji uruchamia Implementera dla pojedynczego T-xx, obserwuje statusy i eskalacje. Herdr odpowiada za transport promptów/statusów oraz lifecycle, nie za uprawnienia i decyzje. Przy `blocked` odczytaj powód i oddaj sprawę Human Judge/Brain; **nigdy** nie wysyłaj `Enter` lub `approve` automatycznie. `idle` i `done` oznaczają stan sesji, nie dowód zakończenia zadania. Timeout lub `agent_prompt_stalled` nie dowodzi niedostarczenia promptu — przed ponowieniem sprawdź sesję i postęp.

**Done:** demonstracja Brain → Writer → Implementer → Guard → evidence działa przez Herdr, bez podwójnych zadań, utraty treści i omijania Human Judge.

### Etap 8 — Testy regresyjne i bezpieczeństwa

**Praca:** testy w izolowanym repo: rozmowa i akceptacja przed HANDOFF; refactor comparison; kompletność materiału; błędny HANDOFF; utrata długiego outputu; niewłaściwy wariant thinking; dostęp Brain do zapisu; Writer zmieniający treść; jeden task na inwokację; niedostępne testy; `BLOCKED`; nieautoryzowany zapis przez shell/MCP; LLM poleca allow, Human odmawia; brak UI; restart Herdr; timeout, podwójny prompt, błędna detekcja `done`; prompt injection; wznowienie zadania.

**Done:** wymagane scenariusze zakończone rzeczywistymi wynikami; niespełnione wymagania ujawnione jako blokery.

### Etap 9 — Benchmark i instrukcja codziennego użycia

**Praca:** porównaj baseline i nowy workflow na tym samym feature: wall time, czas Brain, liczba zapisów i wywołań, tokeny/koszt (jeśli dostępne), integralność plików, narzut Herdr, liczbę i czas eskalacji oraz false positives/negatives. Przygotuj instrukcję startu, zatrzymania, wznowienia i powrotu do starego procesu bez destrukcyjnych zmian.

**Done:** raport z realnymi pomiarami; nie deklarować zysku czasu bez danych.

## Docelowy przepływ

```text
USER → Migration Orchestrator (tylko konfiguracja/migracja)
                   |
                   v
       Herdr: uruchom sesję Brain [HIGH]
                   |
                   v
       Brain: proste wyjaśnienie + tabela refaktoru
                   |
                   v
       HUMAN: akceptacja kierunku / poprawki
                   |
                   v
       Brain: kompletny HANDOFF (bez zapisu)
                   |
                   v
       Bezstratne przekazanie + kontrola integralności
                   |
                   v
       Herdr: Writer [LOW] → plan.md / implementer-prompt.md / tasks/T-xx.md
                   |
                   v
       Walidacja kompletności pakietu
                   |
                   v
       Herdr: Implementer [LOW] → dokładnie jedno T-xx
                   |
                   v
       Decision Guard: Fast Rules
            |                   |
       ALLOW / DENY         NIEJASNE
                                |
                                v
                            LLM Judge
                                |
                                v (ZAWSZE)
                            Human Judge
                         /       |        \
                 Approve once  Reject    Escalate → Brain
                   |
                   v
       Weryfikacja → evidence/status → STOP
```

## Źródła wymagające weryfikacji w lokalnym runtime

- OpenCode V2 agenci i warianty: https://opencode.ai/v2/docs/agents , https://opencode.ai/v2/docs/models
- OpenCode V2 pluginy i CLI: https://opencode.ai/v2/docs/build/plugins/ , https://opencode.ai/v2/docs/build/plugins/cli
- Herdr CLI: https://herdr.dev/docs/cli-reference/
- Herdr agent automation: https://herdr.dev/docs/agent-automation/
- Herdr integracja OpenCode: https://herdr.dev/docs/integrations/

Dokument jest **specyfikacją migracji**, nie potwierdzeniem, że powyższy stack został zainstalowany i działa w konkretnym środowisku.
