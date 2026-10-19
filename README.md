# ai-profiles

Lokalne repozytorium Git profili OpenCode. Każdy projekt otrzymuje własną kopię
wybranego profilu. Wymagania: Python 3.10+, Git i opcjonalnie Make.

## Instalacja i komendy

Uruchamiaj z katalogu sklonowanego `ai-profiles`:

```bash
make install PROFILE=backend-go PROJECT=~/projects/my-api
make status PROJECT=~/projects/my-api
make update PROJECT=~/projects/my-api
make pull-update PROJECT=~/projects/my-api
make switch PROFILE=frontend-react PROJECT=~/projects/my-api
make uninstall PROJECT=~/projects/my-api
make validate
make test
```

`pull-update` wykonuje `git pull --ff-only`, a następnie zwykłe `update`.
`status` i `update` korzystają z lokalnych źródeł — same nie pobierają zmian z sieci.
Przed sprawdzeniem najnowszej opublikowanej wersji wykonaj `git pull --ff-only`.
Bez Make: `python3 profile.py update --project ~/projects/my-api`
(analogicznie `install backend-go`, `status`, `switch frontend-react`, `uninstall`).

Po instalacji, aktualizacji lub zmianie profilu zrestartuj OpenCode.
Profile nie instalują bramki blokującej zapis ani shell. Zwykła praca agenta nie
wymaga odblokowania zadania. Aktualizacja usuwa stare zarządzane pliki bramki;
`approve`, `lock` i `gate-status` są wyłącznie zgodnościowymi aliasami informacyjnymi.

## Lokalne ustawienia MCP

Profil zarządza definicją serwera, a projekt własnymi połączeniami i zmiennymi.
Opcjonalny plik `.opencode/ai-profile.local.json` jest odczytywany przez menedżer
i adapter MCP, **nie bezpośrednio przez OpenCode**. Przykład:

```json
{
  "mcp": {
    "postgres": {
      "env_file": ".env.stage",
      "environment": {
        "DATABASE_URI": "{env:STAGE_DATABASE_URL}"
      }
    }
  }
}
```

Każdy projekt sam wybiera nazwę pliku env i zmiennych. `env_file` jest opcjonalną
ścieżką wewnątrz projektu (bez `..` i symlinków), dostępną tylko dla lokalnych MCP.
Plik env może zawierać np. `STAGE_DATABASE_URL=...`; dodaj go do projektowego
`.gitignore`. Instalator nie tworzy, nie nadpisuje ani nie usuwa tych plików.
`mcp/local.example.json` jest przykładem do ręcznego skopiowania, nie częścią instalacji.
Nie umieszczaj haseł w konfiguracji lokalnej; używaj referencji do środowiska.

Kolejność wartości: środowisko odziedziczone → wskazany plik env → mapowanie
`environment`. Referencje `{env:NAZWA}` w mapowaniu są rozwiązywane przez adapter
po odczycie pliku. Brak wymaganej zmiennej lub wskazanego pliku zatrzymuje start MCP.
Parser obsługuje pojedyncze linie `KEY=value`, cudzysłowy, komentarze i `export`;
nie wykonuje kodu powłoki ani interpolacji `$VAR`/`${VAR}`, nie obsługuje wartości wieloliniowych.

Obiekty są scalane po kluczach, natomiast tablice (np. `command`) zastępowane w całości.
Można nadpisać m.in. `command`, `environment`, `cwd`, `enabled`/`disabled`, `timeout`,
a dla zdalnego MCP także `url`, `headers` i `oauth`. Nie można zmienić typu transportu
ani skonfigurować tym plikiem MCP niezarządzanego przez profil — takie serwery
konfiguruj niezależnie w OpenCode.

PostgreSQL MCP domyślnie używa `unrestricted` (read/write, w granicach uprawnień
użytkownika bazy). Lokalny wybór trybu nie wymaga nadpisania komendy:

```json
{"mcp": {"postgres": {"access_mode": "restricted"}}}
```

Dozwolone są `restricted` i `unrestricted`. Jedynym źródłem trybu jest pole
`access_mode`, domyślnie `unrestricted`. Adapter usuwa stary argument trybu
z `command` i przekazuje dokładnie jeden `--access-mode=...` wyliczony z tego pola.
Także stare lokalne `--access-mode=restricted` zostaje zastąpione; aby wybrać
read-only, ustaw jawnie `access_mode: "restricted"`. Pole `access_mode` jest
ustawieniem ai-profiles, nie polem OpenCode. Powielone argumenty oraz błędne
wartości powodują błąd przed zapisem lub uruchomieniem.
Read/write nie oznacza zgody na niezamówione operacje destrukcyjne ani migracje.

`update` nakłada lokalne ustawienia na aktualną definicję centralną. Gdy upstream
zmienia nadpisane pole, wyświetla ostrzeżenie bez ujawniania wartości; lokalne ustawienie
wygrywa. Zmiany pliku lokalnego nie są konfliktem. Nie edytuj wygenerowanych wpisów
w `opencode.json`: ich ręczne modyfikacje nadal blokują update.
Przy wycofaniu serwera lub zmianie profilu usuń/przenieś niepasujące nadpisanie;
menedżer zgłosi je przed zapisem, zamiast je ignorować. `uninstall` zachowuje plik lokalny.

Instalator dodaje zarządzany adapter `.opencode/ai-profile-mcp.py`, który odczytuje
definicje MCP z `mcp_base` w `.opencode/ai-profile-state.json` i nakłada lokalne
nadpisania. Osobny `ai-profile-mcp.json` nie jest potrzebny; update usuwa jego
wcześniej zarządzaną kopię. Adapter uruchamia komendę bez powłoki, przekazuje
stdio i rozwiązuje środowisko wyłącznie dla procesu MCP. Nie zmienia środowiska OpenCode.
Zdalne MCP pozostają uruchamiane natywnie przez OpenCode (bez `env_file`).
Po zmianie pliku env lub mapowania zrestartuj MCP. Po zmianie ustawień OpenCode
(np. `enabled`, `url`) wykonaj `update` i zrestartuj sesję. Adapter wymaga Python 3.10+;
sam pakiet i zależności serwera MCP muszą być dostępne niezależnie.

## Tworzenie profili i zarządzanie skillami

```text
profiles/<nazwa>/
├── profile.json
├── mcp.json
└── skills/<nazwa-skilla>/SKILL.md
common/
├── skills/
└── workflow-rules.md
```

Każdy katalog w `profiles/` jest wykrywany automatycznie. Dodając profil,
utwórz powyższą strukturę oraz metadane:

```json
{
  "name": "backend-go",
  "version": "1.0.0",
  "description": "Backend Golang development profile"
}
```

`name` musi odpowiadać nazwie katalogu. `mcp.json` zawiera niepusty obiekt
definicji z unikalnymi nazwami; ich wspólna suma musi odpowiadać `mcp/example.json`.

Dodaj skill jako `profiles/<profil>/skills/<skill>/SKILL.md` z frontmatter:

```yaml
---
name: go-testing
description: Go testing practices
---
```

Nazwa skilla musi odpowiadać nazwie folderu. Pliki pomocnicze umieszczaj w tym
samym folderze. Aby wycofać skill, usuń jego folder ze źródeł. Kolejny `update`
usunie wyłącznie wcześniej zarządzane pliki, pozostawiając własne pliki użytkownika.
Skille wspólne trafiają do `common/skills/`; zmiany wspólnych plików dotyczą
wszystkich profili. Nie używaj symlinków ani kolidujących nazw skilli.

## Wersje i publikowanie

Każdy profil ma niezależną wersję `MAJOR.MINOR.PATCH`. Numer wylicza i zapisuje
komenda `bump`, bez ręcznej edycji `profile.json`. Wybierasz rodzaj wydania:

- **MAJOR**: zmiana niekompatybilna, np. usunięcie wymaganego skilla lub zmiana kontraktu.
- **MINOR**: kompatybilne rozszerzenie, np. dodanie nowych skilli.
- **PATCH**: poprawki istniejących instrukcji lub konfiguracji.

Obsługiwane są stabilne wersje z trzema liczbami bez zer wiodących.
Wersje prerelease i metadane build nie są obsługiwane. Porównanie jest numeryczne:
`1.10.0` jest nowsze od `1.9.0`. Zmiana pliku nie zwiększa wersji automatycznie.
Przy publikowaniu zmian wspólnych wykonaj bump dla wszystkich dotkniętych profili.

```bash
make bump PROFILE=backend-go PART=patch
make bump PROFILE=backend-go PART=minor
make bump PROFILE=backend-go PART=major
```

To trzy alternatywy — wybierz jedną dla danego wydania. Domyślne `PART` to `patch`.
`minor` zeruje PATCH, a `major` zeruje MINOR i PATCH. Bump zmienia tylko metadane
wybranego profilu; nie aktualizuje projektów, nie wykonuje commitów ani nie tworzy tagów.
Bez Make: `python3 profile.py bump backend-go --part patch`.

```bash
make validate
make test
make bump PROFILE=backend-go PART=minor
git add profiles common mcp README.md profile.py tests
git commit -m 'Release backend-go'
git push
```

Tag jest opcjonalną etykietą wydania; menedżer czyta `profile.json` w aktualnym
checkout, nie rozwiązuje tagów i nie wybiera automatycznie wydania.

## Status i śledzenie instalacji

Przykład `make status PROJECT=...`:

```text
Profile: backend-go
Installed: 1.0.0
Available: 1.1.0
Status: UPDATE AVAILABLE
```

Status rozróżnia `UP TO DATE`, `UPDATE AVAILABLE`,
`CONTENT CHANGED WITHOUT VERSION BUMP`, `INSTALLED NEWER (downgrade blocked)`
oraz `VERSION UNKNOWN` dla starszych instalacji.
Osobno pokazuje zmianę zawartości i lokalnie zmienione/usunięte pliki lub wpisy MCP.
Nowszy numer wersji nie oznacza automatycznie zmiany plików.

`.opencode/ai-profile-state.json` zachowuje format 1 i zapisuje:
`profile`, `version`, `source_commit`, mapę `files` (ścieżka → SHA-256),
zarządzane wygenerowane wpisy `mcp`, definicję centralną `mcp_base`, hash lokalnych
nadpisań `mcp_local_hash` oraz informacje potrzebne do bezpiecznego usuwania.
Zmiany źródeł są rozpoznawane przez porównanie pełnych map plików i konfiguracji,
nie przez sam numer wersji ani SHA commita. Uwzględniane są też pliki wspólne.

`source_commit` to SHA lokalnego HEAD repozytorium źródłowego w chwili operacji.
Niezacommitowane zmiany również są instalowane; SHA nie opisuje ich treści,
dlatego wydania publikuj z czystego checkout. Bez dostępnego repo Git pole ma
wartość `null`, a status pokazuje `UNKNOWN`; hashe plików nadal działają.

## Aktualizacje i konflikty

`update` dodaje nowe pliki, synchronizuje istniejące skille, konfigurację i pluginy
oraz usuwa wycofane pliki zarządzane. Zachowuje obce pliki i konfigurację.
Ponowienie aktualizacji bez zmian daje tę samą zawartość.
Downgrade tego samego profilu jest odrzucany. `switch` pozwala wybrać inny profil
z jego niezależną wersją.

Przed zapisem menedżer sprawdza SHA-256 wszystkich dotychczas zarządzanych plików,
wpisy MCP i kolizje ścieżek. Wykryty konflikt przerywa operację bez częściowego
zastosowania aktualizacji i bez zmiany zapisanej wersji/SHA. Brak opcji wymuszającej
nadpisanie lokalnych zmian. Zapis stanu następuje dopiero po zapisaniu plików i konfiguracji.

Przy konflikcie:

1. Zrób kopię lokalnych zmian lub commit projektu.
2. Przenieś zmiany wspólne do `ai-profiles`, a projektowe ustawienia MCP do
   `.opencode/ai-profile.local.json`.
3. Przywróć w projekcie zarządzany plik do zainstalowanej wersji (lub wpis MCP
   do wartości ze stanu), następnie ponów `update`.

Nie edytuj hashy w stanie, aby ukryć konflikt. `uninstall` i `switch` także chronią
lokalne modyfikacje. Menedżer odmawia pracy z `opencode.jsonc`, aby nie utracić komentarzy.
Nie zapewnia transakcji odpornej na awarię dysku lub przerwanie procesu podczas zapisu;
przed większą aktualizacją wykonaj commit lub kopię projektu.

## Migracja istniejących instalacji

Stan formatu 1 z hashami, ale bez wersji i SHA commita, jest obsługiwany bez
dodatkowych kroków. `status` pokaże `VERSION UNKNOWN`; udany `update` sprawdzi
dotychczasowe hashe i uzupełni metadane bieżącego profilu. Nie zgaduje historycznej wersji.
Dotychczasowe stany bez `mcp_base` są również obsługiwane: po sprawdzeniu starej
konfiguracji update instaluje adapter i zapisuje nowy podział konfiguracji.

Starsze MVP bez hashy wymaga świadomego przejęcia plików po ich przeglądzie i backupie:

```bash
python3 profile.py migrate-legacy --accept-existing --project ~/projects/my-api
make update PROJECT=~/projects/my-api
```

Migracja uznaje istniejące pliki za punkt odniesienia. Historyczny skill
`implementation-breakdown` jest następnie zastępowany przez `make-plan`.
