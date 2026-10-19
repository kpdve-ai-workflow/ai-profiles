# ai-profiles

Lokalny katalog wersjonowanych profili **OpenCode**. Źródło prawdy to to repozytorium Git; każdy projekt dostaje **własną kopię** wybranego profilu. Bez marketplace i bez zależności od Herdr.

## 1. Pierwsze użycie

```bash
# Jednorazowo – repo może być prywatne, jeśli masz dostęp do GitHuba
git clone git@github.com:TWOJ_LOGIN/ai-profiles.git ~/ai-profiles

# Z katalogu ai-profiles zainstaluj profil w docelowym projekcie
cd ~/ai-profiles
make install PROFILE=backend-go PROJECT=~/projects/my-api
# lub
make install PROFILE=frontend-react PROJECT=~/projects/my-web
```

Profile: `backend-go` (Go + PostgreSQL MCP) i `frontend-react` (React + Figma MCP). W obu automatycznie instalowane są wspólne skille `make-plan`, `refine-plan` i plugin blokujący zapis.

Instalator tworzy/aktualizuje w **projekcie docelowym**: `.opencode/skills/`, `.opencode/plugins/`, `.opencode/workflow-rules.md`, `.opencode/ai-profile-gate.py`, `.opencode/ai-profile-state.json`, `opencode.json` oraz wpis w `.gitignore`. Innych skilli i MCP nie usuwa. Restartuj sesję OpenCode po zmianie profilu.

## 2. Rozwijanie i publikowanie ai-profiles

Źródła są w repozytorium:

```text
ai-profiles/
├── profiles/
│   ├── backend-go/
│   │   ├── skills/go-backend/SKILL.md
│   │   └── mcp.json
│   └── frontend-react/
│       ├── skills/react-frontend/SKILL.md
│       └── mcp.json
├── common/
│   ├── skills/make-plan/SKILL.md
│   ├── skills/refine-plan/SKILL.md
│   ├── plugins/approval-gate.js
│   └── workflow-rules.md
├── mcp/example.json
├── profile.py
├── gate.py
├── Makefile
└── tests/
```

**Dodanie dwóch nowych skilli Go:**

1. Dodaj `profiles/backend-go/skills/go-testing/SKILL.md` i `profiles/backend-go/skills/go-database/SKILL.md` (każdy skill w osobnym folderze, frontmatter `name` zgodny z nazwą folderu). Możesz umieszczać także pliki wspierające wewnątrz folderu skilla.
2. Uruchom `make validate && make test`.
3. Wypchnij zmianę: `git add . && git commit -m 'Add Go skills' && git push`.

Zmiana `common/skills/` lub `common/plugins/` trafi przy następnym `update` do **obu** typów profili. Zmiany `profiles/<profil>/` dotyczą tylko wybranego profilu.

**MCP:** definicje właściwe profilom znajdują się w `profiles/*/mcp.json`. `mcp/example.json` to wspólny przykład wszystkich MCP w repo. Gdy zmienisz lub dodasz MCP, uaktualnij także przykład – `make validate` sprawdzi zgodność. Repo nie instaluje przestarzałego `@modelcontextprotocol/server-postgres`.

## 3. Aktualizacja już zainstalowanego profilu

Na komputerze, na którym używasz projektu:

```bash
cd ~/ai-profiles
git pull --ff-only
make update PROJECT=~/projects/my-api
```

Alternatywnie **jedna komenda** w katalogu `ai-profiles`:

```bash
make pull-update PROJECT=~/projects/my-api
```

Menedżer odczytuje nazwę profilu z `.opencode/ai-profile-state.json` – nie wybierasz jej ponownie. Dodaje nowe skille, uaktualnia zmienione, usuwa wycofane **zarządzane** pliki i synchronizuje MCP/plugin. Pliki innych narzędzi zostawia bez zmian. Po udanym update stan bramki jest **LOCKED**.

Jeżeli zmodyfikujesz lokalnie zarządzany plik (np. `go-backend/SKILL.md`), `update` przerwie działanie z komunikatem o konflikcie **bez nadpisania**. Zmień go najpierw w centralnym `ai-profiles` albo rozwiąż konflikt lokalnie, a potem ponów aktualizację. Sumy SHA-256 w `ai-profile-state.json` służą do rozpoznawania takich zmian.

## 4. Pozostałe komendy

Wywołuj z katalogu `~/ai-profiles`:

```bash
make status PROJECT=~/projects/my-api
make switch PROFILE=frontend-react PROJECT=~/projects/my-api
make uninstall PROJECT=~/projects/my-api
make validate
make test
```

`switch` zmienia profil technologiczny, zachowując część wspólną. `uninstall` usuwa tylko pliki i wpisy zarządzane przez `ai-profiles`. Obie operacje sprawdzają konflikty i nie nadpisują lokalnie zmodyfikowanych elementów.

Bez Makefile działa też: `python3 ~/ai-profiles/profile.py update --project ~/projects/my-api` (analogicznie `install backend-go`, `switch frontend-react`, `uninstall`, `status`).

## 5. Akceptowanie tasków

`make-plan` i `refine-plan` przygotowują i doprecyzowują taski **w czacie**. Zapis pozostaje zablokowany, dopóki **Ty** nie otworzysz go w osobnym terminalu:

```bash
cd ~/ai-profiles
make approve PROJECT=~/projects/my-api TASK=TASK-001
make gate-status PROJECT=~/projects/my-api
# Po zakończeniu zadania, przed kolejnym:
make lock PROJECT=~/projects/my-api
```

Plugin blokuje `write`, `edit`, `apply_patch`, `multiedit` i `bash` w OpenCode przed akceptacją. To bramka aplikacyjna, **nie izolacja systemowa**: nie obejmuje każdego niestandardowego narzędzia ani zapisów przez zewnętrzne MCP. Agent nie może sam potwierdzić, czy decyzje w czacie rzeczywiście zostały przez Ciebie zaakceptowane.

## 6. Projekty zainstalowane wcześniejszym `install.py`

Poprzednie MVP nie miało sum kontrolnych. Aby przejąć istniejące pliki **bez ich nadpisywania**, wykonaj po ich ręcznym przejrzeniu:

```bash
python3 ~/ai-profiles/profile.py migrate-legacy --accept-existing --project ~/projects/my-api
make update PROJECT=~/projects/my-api
```

Ta migracja jawnie przyjmuje obecne pliki za punkt odniesienia. Warto wcześniej zrobić commit lub kopię projektu.

**Wymagania managera:** Python 3.10+ i Git; Makefile wymaga `make`. Nie modyfikuje `opencode.jsonc` (celowo przerywa, żeby nie uszkodzić komentarzy). Instalacja profilu nie uruchamia samych serwerów MCP ani nie sprawdza ich logowania.
