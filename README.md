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
Istniejące komendy `approve`, `lock` i `gate-status` pozostają bez zmian;
udany `update` nadal ustawia bramkę na LOCKED.

## Tworzenie profili i zarządzanie skillami

```text
profiles/<nazwa>/
├── profile.json
├── mcp.json
└── skills/<nazwa-skilla>/SKILL.md
common/
├── skills/
├── plugins/approval-gate.js
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

Każdy profil ma niezależną wersję `MAJOR.MINOR.PATCH`, zwiększaną **ręcznie**:

- **MAJOR**: zmiana niekompatybilna, np. usunięcie wymaganego skilla lub zmiana kontraktu.
- **MINOR**: kompatybilne rozszerzenie, np. dodanie nowych skilli.
- **PATCH**: poprawki istniejących instrukcji lub konfiguracji.

Obsługiwane są stabilne wersje z trzema liczbami bez zer wiodących.
Wersje prerelease i metadane build nie są obsługiwane. Porównanie jest numeryczne:
`1.10.0` jest nowsze od `1.9.0`. Zmiana pliku nie zwiększa wersji automatycznie.
Przy publikowaniu zmian wspólnych zwiększ odpowiednio wersje wszystkich dotkniętych profili.

```bash
make validate
make test
git add profiles common mcp README.md profile.py tests
git commit -m 'Release backend-go 1.1.0'
git tag backend-go-v1.1.0
git push
git push origin backend-go-v1.1.0
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
zarządzane wpisy `mcp` oraz informacje potrzebne do bezpiecznego usuwania.
Zmiany źródeł są rozpoznawane przez porównanie pełnych map plików i konfiguracji,
nie przez sam numer wersji ani SHA commita. Uwzględniane są też pliki wspólne.

`source_commit` to SHA lokalnego HEAD repozytorium źródłowego w chwili operacji.
Niezacommitowane zmiany również są instalowane; SHA nie opisuje ich treści,
dlatego wydania publikuj z czystego checkout. Bez dostępnego repo Git pole ma
wartość `null`, a status pokazuje `UNKNOWN`; hashe plików nadal działają.

## Aktualizacje i konflikty

`update` dodaje nowe pliki, synchronizuje istniejące skille, konfigurację i pluginy
oraz usuwa wycofane pliki zarządzane. Zachowuje obce pliki i konfigurację.
Ponowienie aktualizacji bez zmian daje tę samą zawartość; bramka nadal jest blokowana.
Downgrade tego samego profilu jest odrzucany. `switch` pozwala wybrać inny profil
z jego niezależną wersją.

Przed zapisem menedżer sprawdza SHA-256 wszystkich dotychczas zarządzanych plików,
wpisy MCP i kolizje ścieżek. Wykryty konflikt przerywa operację bez częściowego
zastosowania aktualizacji i bez zmiany zapisanej wersji/SHA. Brak opcji wymuszającej
nadpisanie lokalnych zmian. Zapis stanu następuje dopiero po zapisaniu plików i konfiguracji.

Przy konflikcie:

1. Zrób kopię lokalnych zmian lub commit projektu.
2. Przenieś potrzebne zmiany do źródeł `ai-profiles` i opublikuj nową wersję.
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

Starsze MVP bez hashy wymaga świadomego przejęcia plików po ich przeglądzie i backupie:

```bash
python3 profile.py migrate-legacy --accept-existing --project ~/projects/my-api
make update PROJECT=~/projects/my-api
```

Migracja uznaje istniejące pliki za punkt odniesienia. Historyczny skill
`implementation-breakdown` jest następnie zastępowany przez `make-plan`.
