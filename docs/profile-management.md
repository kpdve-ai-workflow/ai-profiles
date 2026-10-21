# Zarządzanie profilami i instalacjami

## Tworzenie i struktura profilu

Profile są wykrywane automatycznie z `profiles/<nazwa>/`. Profil zawiera
`profile.json`, `mcp.json` i opcjonalne `skills/<skill>/SKILL.md`. `name` w
metadanych musi odpowiadać nazwie katalogu. Najprościej utworzyć profil poleceniem:

```bash
make create PROFILE=backend-python DESCRIPTION="Backend Python development profile"
```

Nazwa jest wymagana; opis jest opcjonalny. Komenda tworzy metadane w wersji
`1.0.0`, pusty wybór MCP oraz zachowuje pusty katalog skills. Nie instaluje
profilu ani nie nadpisuje istniejącego. Szczegóły formatu wyboru MCP znajdują się
w [dokumentacji MCP](../mcp/README.md).

Skill dodaj jako `profiles/<profil>/skills/<skill>/SKILL.md` z frontmatter `name`
i `description`; nazwa musi odpowiadać katalogowi. Pliki pomocnicze pozostają
w katalogu skilla. Wspólne skille umieszczaj w `common/skills/`; ich zmiana
dotyczy wszystkich profili. Nie używaj symlinków ani kolidujących nazw.
Narzędzia użytkownika profilu umieszczaj w `profiles/<profil>/scripts/`, nie
w skillach. Są instalowane bez automatycznego wykonywania.

Skille przeznaczone wyłącznie dla tego repozytorium umieszczaj w
`.opencode/skills/`; OpenCode wykrywa je natywnie. Nie są częścią `common/`
ani żadnego profilu i nie są dostarczane do projektów. Workflow tego checkoutu
jest oddzielony od zawartości profili.

## Wersje i publikowanie

Każdy profil ma niezależną stabilną wersję `MAJOR.MINOR.PATCH`. Aktualizuj ją
poleceniem `make bump PROFILE=<nazwa> PART=patch|minor|major` (domyślnie `patch`):

- **MAJOR** — niekompatybilna zmiana;
- **MINOR** — kompatybilne rozszerzenie;
- **PATCH** — poprawka istniejących instrukcji lub konfiguracji.

Wersje prerelease i build metadata nie są obsługiwane. Zmiana plików nie zwiększa
wersji automatycznie. Zmiany wspólne wymagają świadomego bumpu wszystkich
dotkniętych profili; przy zmianie definicji MCP bumpuj tylko profile, które ją
wybierają. Bump zmienia wyłącznie metadane: nie aktualizuje projektów, nie
wykonuje commitów ani nie tworzy tagów.

## Instalacja i aktualizacja

Instaluj przez `make install PROFILE=<profil> PROJECT=<projekt>`. Make buduje
manager i runtime Rust w `core/rust/target/release/` przed operacją. Adapter MCP,
launcher MariaDB i restore WordPress trafiają do projektu jako samodzielne
pliki wykonywalne. Python pozostaje wyłącznie w repozytorium jako kod
referencyjny. Aktualizacja usuwa dawne zarządzane `.py` po sprawdzeniu hashy;
obce i lokalnie zmienione pliki zachowują dotychczasową ochronę.

Make wymaga `PROFILE` przy `create`, `install`, `switch` i `bump`, oraz `PROJECT`
przy komendach projektowych. CLI domyślnie działa na bieżącym katalogu.
`status` i `update` korzystają z lokalnego checkoutu i nie pobierają zmian;
`pull-update` wykonuje `git pull --ff-only`, a następnie `update`. Przed oceną
najnowszej wersji upstream pobierz zmiany jawnie.

`update` dodaje i synchronizuje zarządzane pliki oraz usuwa wycofane pliki,
zachowując pliki obce i lokalną konfigurację. Powtórzenie bez zmian daje tę samą
zawartość. Downgrade tego samego profilu jest blokowany; `switch` wybiera inny
profil. `uninstall` usuwa wyłącznie zarządzaną zawartość.

Stan `.opencode/ai-profile-state.json` śledzi wersję, źródło, SHA-256 plików,
zarządzane wpisy MCP i dane potrzebne do bezpiecznej synchronizacji. Status
rozróżnia aktualność wersji od zmian treści oraz pokazuje lokalne modyfikacje.
SHA commit nie opisuje niezacommitowanych zmian; do publikowania używaj czystego
checkoutu.

Wycofany `.opencode/workflow-rules.md` oraz dawniej wspólnie dostarczany
`brain-to-implementer` są usuwane z istniejącej instalacji podczas update/switch
lub uninstall tylko wtedy, gdy ich hashe zgadzają się ze stanem instalatora.
Wpis `workflow-rules.md` w `instructions` jest usuwany przy update/switch
wyłącznie, gdy stan potwierdza, że dodał go instalator (`instructions_added`).
Własny wpis użytkownika pozostaje; po usunięciu wpisu instalatora flaga
ownership jest zerowana.

Przed zapisem menedżer sprawdza hashe plików, wpisy MCP i kolizje ścieżek.
Konflikt przerywa operację bez częściowego zastosowania; nie ma opcji wymuszającej
nadpisanie lokalnych zmian. Zrób kopię lub commit, przenieś zmiany wspólne do
repozytorium, a projektowe ustawienia MCP do `.opencode/ai-profile.local.json`.
Przywróć zarządzany plik/wpis do zainstalowanej wartości i ponów aktualizację.
Nie edytuj hashy stanu ani zarządzanych wpisów w `opencode.json`. Menedżer odmawia
pracy z `opencode.jsonc`, by nie utracić komentarzy. Zapis nie jest transakcją
odporną na awarię dysku lub przerwanie procesu.

## Migracja starszej instalacji

Stan formatu 1 bez wersji/SHA commita jest obsługiwany: status pokaże
`VERSION UNKNOWN`, a udany update uzupełni metadane po sprawdzeniu hashy.
Starsze stany bez `mcp_base` są również aktualizowane po sprawdzeniu starej
konfiguracji. Instalacje MVP bez hashy wymagają przeglądu i backupu przed
świadomym przejęciem:

```bash
make migrate-legacy PROJECT=~/projects/my-api
make update PROJECT=~/projects/my-api
```

Migracja uznaje istniejące pliki za punkt odniesienia. Walidację repozytorium
uruchom przez `make validate`; szczegóły walidacji MCP są w dokumentacji MCP.
