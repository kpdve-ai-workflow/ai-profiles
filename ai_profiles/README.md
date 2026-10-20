# Pakiet menedżera profili

`cli.py` obsługuje argumenty i kieruje polecenia do operacji, statusu,
walidacji lub migracji. `operations.py` odpowiada za instalację i usuwanie;
`profiles.py` za metadane i wersje, `payload.py` za pliki dostarczane,
`project.py` za stan i konfigurację projektu, a `mcp.py` za konfigurację MCP.
`filesystem.py` zawiera wspólne operacje plikowe i ochronę ścieżek.
`validation.py` sprawdza źródła, a `legacy.py` obsługuje migrację.

Zależności prowadzą od entrypointów przez CLI i operacje do modułów domenowych,
a następnie do kontekstu, stałych, błędów i narzędzi plikowych. `RepositoryContext`
jawnie wskazuje checkout źródłowy; polecenia z repozytorium używają kontekstu
zamiast globalnej ścieżki. Implementacji polecenia należy szukać w `cli.py`,
a właściwej logiki w module odpowiedzialnym za daną domenę.
