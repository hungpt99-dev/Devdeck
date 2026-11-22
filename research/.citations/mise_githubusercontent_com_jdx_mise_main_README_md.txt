# (no title)
URL: https://raw.githubusercontent.com/jdx/mise/main/README.md



 
 
 
 
 
 
 
 mise-en-place
 
 

 
 
 
 
 
 

 
 
 
 
 

 Dev tools, env vars, and tasks in one CLI 

 
 Getting started •
 Docs •
 Dev tools •
 Environments •
 Tasks •
 Bootstrap 
 

 
 Sponsored by 
 
 
 
 
 
 
    
 
 
 
 
 
 
 
 View all sponsors 
 

 

 

## What is mise?

mise (pronounced "meez", short for _mise-en-place_) manages a project's dev
tools, environment variables, and tasks from one `mise.toml` that works across
your shell, editor, and CI:

```toml
[tools]
node = "24"
python = "3.14"
rust = "stable"
"cargo:cargo-nextest" = "0.9"
"github:BurntSushi/ripgrep" = "15"
"npm:prettier" = "3"

[env]
NODE_ENV = "development"
_.file = ".env"
_.path = "./node_modules/.bin"

[tasks.build]
description = "Build the application"
run = "npm run build"

[tasks.test]
description = "Run the test suite"
depends = ["build"]
run = "pytest && npm test"
```

- Install [dev tools](https://mise.jdx.dev/dev-tools/) such as Node.js, Python,
 Go, Rust, and Terraform from a
 [registry of 1,000+ tools](https://mise.jdx.dev/registry.html) or directly
 from [backends](https://mise.jdx.dev/dev-tools/backends/) like GitHub
 releases, Aqua, Cargo, npm, and PyPI, with separate versions per project.
- Load [environment variables](https://mise.jdx.dev/environments/), `.env`
 files, `PATH` entries, virtualenvs, and
 [secrets](https://mise.jdx.dev/environments/secrets/) automatically as you
 enter a directory.
- Run [tasks](https://mise.jdx.dev/tasks/) defined in `mise.toml` or as scripts
 in `mise-tasks/`, with dependencies, parallel execution, file watching, and
 the project's tools and environment already configured.
- Set up a machine with [`mise bootstrap`](https://mise.jdx.dev/bootstrap.html)
 and [`mise dot apply`](https://mise.jdx.dev/dotfiles.html) to apply system
 packages, dotfiles, Git repositories, macOS defaults, and background services.

Adopt only the parts you need. Start with a single tool or task, and keep any
existing `.tool-versions` files as they are; mise also reads `.nvmrc`,
`.python-version`, and other
[idiomatic version files](https://mise.jdx.dev/dev-tools/versions.html#idiomatic-version-files)
once enabled.

[![Demo of mise managing tools](./docs/tapes/demo.gif)](https://mise.jdx.dev/demo.html)

See the [demo transcript](https://mise.jdx.dev/demo.html#transcript) for the
commands shown above.

## Quickstart

### 1. Install mise

On macOS or Linux:

```sh
curl -fsSL https://mise.run | sh
~/.local/bin/mise --version
```

On Windows, run `winget install jdx.mise`. For Homebrew, apt, dnf, Scoop, and
other package managers, see
[Installing mise](https://mise.jdx.dev/installing-mise.html).

The installer places the binary at `~/.local/bin/mise`. If `~/.local/bin` is not
on your `PATH` yet, run `~/.local/bin/mise` in place of `mise` until you
activate mise in step 4.

### 2. Run a tool without changing your shell

```sh
mise exec node@24 -- node --version
# v24.x.x
```

[`mise exec`](https://mise.jdx.dev/cli/exec.html) downloads Node.js 24 if
nee