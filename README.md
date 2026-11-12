# dgi

A Rust CLI to download .gitignore files from [Github's repository](https://github.com/github/gitignore) easily.

***

# How to Install

Requires a Rust toolchain (`cargo`):

```console
cargo install --git https://github.com/mahadevxo/download-gitignore
```

***

# How to use

Run `dgi` with no arguments to show help.

### Download a combined .gitignore

```console
dgi --download python c c++
```

Names are case-insensitive. If a `.gitignore` already exists, `dgi` shows a diff and asks whether to **combine** (append, skipping rules you already have), **overwrite**, or **abort** (the default).

### View a template in vim

```console
dgi --show python
```

### Detect languages in the current codebase

```console
dgi --languages
```

Files ignored by `.gitignore` (e.g. `node_modules`, `venv`) are skipped. To download a combined .gitignore for the detected languages:

```console
dgi --languages --download
```

You can also add extra languages: `dgi --languages --download terraform`.

### List all available templates

```console
dgi --list
```
