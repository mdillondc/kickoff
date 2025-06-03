Based on the original kickoff launcher [found here](https://github.com/j0ru/kickoff), this enhanced version adds several quality-of-life improvements (see Features below).

---

Heavily inspired by rofi.

## Install

#### Cargo

```bash
cargo clean
cargo build --release
cargo install --path . --force
```

## Features

- Wayland native (only wlroots based compositors though)
- Fuzzy search
- Fast and snappy
- Remembers often used applications
- Argument support for launched programs
- Paste support
- Custom Input via stdin
- Built-in calculator with basic arithmetic operations

## How does it search

Kickoff searches for applications from multiple sources to provide comprehensive results:

1. **$PATH executables** - All executable programs found in your $PATH directories. This includes your additions to $PATH as long as they are done before you launch kickoff or the program that launches kickoff (i.e. your window manager).

2. **Desktop applications** - Applications with `.desktop` files from standard locations:
   - `/usr/share/applications`
   - `/usr/local/share/applications` 
   - `~/.local/share/applications`

3. **Flatpak applications** - Installed Flatpak apps discovered via `flatpak list --app`

4. **Snap packages** - Installed Snap packages discovered via `snap list` (excluding core/system snaps)

5. **Settings applications** - Some applications with `NoDisplay=true` are included if they are Settings applications, making system configuration more accessible.

## Calculator

Kickoff includes a built-in calculator that automatically detects mathematical expressions. Simply type an arithmetic expression and see the result:

- **Basic operations**: `10-5`, `2+3*4`, `(1+2)*3`
- **Decimal numbers**: `3.14*2`, `10/3`
- **Negative numbers**: `-5+10`, `(-2)*3`

When you type a mathematical expression, the result will be displayed at the top of the results list. Press Enter while the calculator result is selected to copy the result to your clipboard.

## Configuration

A default configuration will be placed at `~/.config/kickoff/config.toml`. See sample [here](https://github.com/mdillondc/kickoff/blob/main/assets/default_config.toml).

## Script integration

If you want to adapt kickoff for your use case, i.e. selecting an entry from a password manager,
you can use one of the `--from-*` options. If any of those options is defined, the default behavior of reading from `$PATH` is disabled as well as
saving the history. The latter can easily be reactivated by setting `--history <some path>`.

|Option|Argument|Usage|
|------|--------|-----|
|`--from-stdin`|None| Reads a list of items from stdin |
|`--from-file`|Path| Reads a list of items from a file |
|`--from-path`|None| Walks all `$PATH` directories and adds all executables as selectable items |
|`--stdout`|None| Prints the selected result to stdout instead of trying to execute it |

These can also be combined, for example, if you want to add custom commands to your usual list of programs.
```bash
echo 'Big kitty = kitty -o "font_size=20"' | kickoff --from-stdin --from-path --history ".cache/kickoff/custom_history.csv"
```

### Input Format

Reading from file or stdin follows a very simple format,
spaces around the equals sign can be dropped:
```
Small kitty = kitty -o "font_size=5"
Big kitty = kitty -o "font_size=20"
^=======^   ^=====================^
    |                  |
Displayed Name         |
                       |
              Executed Command
```

### Magic Words

When reading from a file or stdin, you can use magic words to influence the generated items.
Currently, there is only one, but more might be added someday:

|Word|Argument|Usage|Default|
|----|--------|-----|-------|
|%base_score| number | Sets the base score for all following entries, can be overwritten later | 0 |

In this example, `Small kitty` has a base score of 0, while the others have a score of 5.
```
Small kitty = kitty -o "font_size=5"
%base_score = 5
Big kitty = kitty -o "font_size=20"
Medium kitty = kitty -o "font_size=12"
```
