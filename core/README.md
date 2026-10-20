# Core components

`core/` owns reusable components and their contracts. Each component keeps its
implementation isolated under `implementations/<language>/`; replacing one does
not require importing another implementation's internal modules.
