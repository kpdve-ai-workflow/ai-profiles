# Test Gap Discovery — portable agent skill

One skill and supporting references/scripts. **Does not require a configured multi-agent harness.** English instructions for the agent; installation guide in Polish below.

## Instalacja w istniejącym repo

Po rozpakowaniu kopiujesz **cały katalog** `test-gap-discovery` (z `SKILL.md`, `references/`, `scripts/`) do katalogu skilli wykrywanego przez Twojego agenta, np.:

```bash
mkdir -p .pi/skills
cp -R test-gap-discovery .pi/skills/
```

Jeśli Twoja konfiguracja używa `.agents/skills/`, skopiuj tam. Nie kopiuj tego jednocześnie do obu lokalizacji bez sprawdzenia discovery. Restart sesji może być potrzebny. Samo skopiowanie nie dowodzi, że loader wykrył skill — sprawdź konfigurację Pi i listę dostępnych skilli.

## Jak używać

W sesji agenta otwartej w repo, np.:

**Wizja bez uruchamiania testów:**

```text
Use the test-gap-discovery skill in vision mode. Analyze this repository and explain which behaviors and contrasting use cases should be tested first. Do not modify files or create a report.
```

**Testy szukające luk (tylko kiedy polityka projektu zezwala na zapis/uruchomienie):**

```text
Use the test-gap-discovery skill in gaps mode for <MODULE/PATH>. Derive cases from independent oracles, implement the highest-value behavior tests, run only commands permitted by repository policy, and never weaken assertions to match current code. Do not modify production code or create a formal report.
```

**Review testów:**

```text
Use the test-gap-discovery skill in review mode for <PATH/DIFF>. Find missing meaningful use cases and implementation-mirroring assertions. Keep feedback brief and evidence-based.
```

## Czy zadziała na niepełnym MVP?

Tak, jako **procedura wykonywana w pojedynczej sesji agenta** z dostępem do repo. Tryby `vision` i read-only `review` nie potrzebują Herdr/Lead→Worker, `project_*`, mutatora ani CI. Tryb `gaps` wymaga możliwości edycji i uruchamiania testów oraz zgody wynikającej z realnej polityki repo; może się ograniczyć do zaproponowania scenariuszy, gdy uruchamianie jest zabronione. Nie ma automatycznej orkiestracji ani obietnicy wykrycia błędów.

## Skrypty pomocnicze

- `scripts/inventory-tests.sh [ścieżka]`: bezpieczny odczyt i szacunkowa inwentaryzacja nazw plików testowych.
- `scripts/mutation-check.sh [ścieżka]`: wyświetla dostępne narzędzia i bezpieczną procedurę; **nie modyfikuje kodu**.
- `scripts/coverage-map.sh [ścieżka]`: tylko wykrywanie wskazówek związanych z coverage; **nie uruchamia testów**.

Skrypty są opcjonalne. Stosują wyłącznie odczyt repo i nie instalują zależności.
