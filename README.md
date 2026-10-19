# AI Profiles MVP — OpenCode + Herdr

Jedno repozytorium GitHub hostuje dwa profile **backend-go** i **frontend-react** oraz wspólny workflow.
Marketplace Herdr **nie jest wymagany**. Herdr pozostaje menedżerem terminali/sesji; mechanizmy skilli, MCP i hooków wykonuje OpenCode.

## Instalacja profilu w projekcie

1. Wrzuć zawartość tego katalogu do repozytorium GitHub (może być prywatne).
2. Pobierz aktualną wersję, np. `git clone https://github.com/TWOJ_LOGIN/ai-profiles.git`.
3. W pobranym repozytorium uruchom:

```bash
python3 install.py backend-go /sciezka/do/projektu
# ALBO
python3 install.py frontend-react /sciezka/do/projektu
```

Ponowne wywołanie aktualizuje wybrany profil, usuwa zarządzany skill/MCP poprzedniego profilu i **resetuje akceptację**. Nie usuwaj niepowiązanych plików projektu. Skrypt **nie zmienia globalnej konfiguracji** OpenCode.

Konfiguracja jest kopiowana do projektu do `.opencode/` oraz do `opencode.json`; następnie uruchom ponownie OpenCode z tego projektu (również w sesji Herdr).

## MCP

- **Go**: Postgres MCP Pro w trybie `restricted` (tylko odczyt). Wymaga `uvx` i zmiennej środowiskowej `DATABASE_URI` z danymi połączenia. Zalecany użytkownik bazy o prawach tylko do odczytu. Nie zapisuj URL w repo.
- **React**: Figma remote MCP `https://mcp.figma.com/mcp`. OpenCode może wymagać autoryzacji przez `opencode mcp auth figma`, a **Figma ogranicza wsparcie klientów zdalnych**. Jeśli zdalna integracja OpenCode nie zadziała, możesz zmienić `profiles/frontend-react/mcp.json` na desktop URL `http://127.0.0.1:3845/mcp`, który wymaga włączonego MCP w aplikacji desktopowej Figma i kwalifikującego się planu.

Nie instaluj przestarzałego `@modelcontextprotocol/server-postgres` — implementacja referencyjna została zarchiwizowana.

## Workflow (plan najpierw, pliki później)

1. Agent wywołuje skill `implementation-breakdown` i omawia w czacie plan całego zadania, bez tworzenia plików.
2. Agent wywołuje `refine-plan`, przechodzi task po tasku, ustala z Tobą decyzje techniczne i ich akceptację.
3. **Ty**, w osobnym terminalu (nie przez agenta), jawnie otwierasz zapis dla konkretnego taska:

```bash
python3 gate.py /sciezka/do/projektu approve TASK-001
```

4. Agent może implementować zatwierdzony task. Przed kolejnym taskiem **Ty** zamykasz bramkę:

```bash
python3 gate.py /sciezka/do/projektu lock
```

Status: `python3 gate.py /sciezka/do/projektu status`.

Hook `.opencode/plugins/approval-gate.js` **fail-closed** blokuje wywołania `write`, `edit`, `apply_patch`, `multiedit` oraz całe `bash` przy braku prawidłowego znacznika zgody. Dotyczy to również dokumentacji. Planowanie w czacie działa bez zapisu na dysk. Jeśli wpisujesz prośby o testy/komendy shell przed zgodą, pamiętaj: shell jest zablokowany; to świadomy kompromis MVP.

## Ograniczenia MVP

- Hook egzekwuje zapis przy użyciu *wbudowanych* narzędzi **OpenCode**. Nie jest sandboxem: nie obejmuje wszystkich własnych narzędzi, zapisu przez MCP ani innych klientów uruchamianych jako samodzielne CLI (np. `codex` / `claude`). Używaj Codex/Claude jako **providerów wewnątrz OpenCode**.
- Zatwierdzenie jest zapisem lokalnego tokena; hook **nie potrafi sam zweryfikować jakości rozmowy** ani tego, że każdy task został omówiony. Kolejne taski wymagają ręcznego `lock`.
- Po odblokowaniu shell agent posiada uprawnienia procesu użytkownika. Dla rzeczywiście twardej granicy bezpieczeństwa potrzebujesz osobnego sandboxa, ograniczonych uprawnień i/lub CI.
- Instalator zachowuje inne MCP/skille w plikach projektu, lecz **nie wyłącza MCP skonfigurowanych globalnie**. Własny `opencode.jsonc` z komentarzami nie jest automatycznie modyfikowany (skrypt kończy się błędem, by uniknąć niejasnych konfliktów).
- Na macOS/Linux wymagany Python 3.10+. Skrypt jest przenośny, lecz nie sprawdza podłączenia ani uprawnień usług MCP.

## Linki do dokumentacji

- https://docs.opencode.ai/docs/skills/
- https://docs.opencode.ai/docs/plugins/
- https://docs.opencode.ai/docs/mcp-servers/
- https://herdr.dev/docs/marketplace/
- https://developers.figma.com/docs/figma-mcp-server/remote-server-installation/
- https://github.com/crystaldba/postgres-mcp
# ai-profiles
