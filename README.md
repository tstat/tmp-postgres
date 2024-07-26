# tmp-postgres
---

`tmp-postgres` is a CLI program to aid in the creation and destruction of
temporary PostgreSQL instances.

[![asciicast](https://asciinema.org/a/FbJenFrs4AA52qNHgkgaNb7UK.svg)](https://asciinema.org/a/FbJenFrs4AA52qNHgkgaNb7UK)

Running

```
tmp-postgres <DIRECTORY>
```

Initializes a database in `<DIRECTORY>` and starts a postgres server there. By
default, no tcp ports are listened to. The server is listening on a unix socket
located in `<DIRECTORY>`. So, you can connect to it with `psql --host
<DIRECTORY>`.

Alternatively, running

```
tmp-postgres <DIRECTORY> --psql
```

Does the same thing but launches psql for you in the same terminal. Output from
child processes (e.g. `initdb`, `postgres`) is line-buffered and forwarded
along with a tag indicating which child wrote the line. If this is undesirable
then you can pass `--silent`.

Any arguments after `<DIRECTORY>` are interpreted as a command to run while the
temporary postgres server runs. So, if you have a test that requires postgres,
then you could scope a temporary postgres server around it by running

```
tmp-postgres <DIRECTORY> <TEST>
```

For example:

```
# tmp-postgres --silent --remove true /tmp/pg psql --host /tmp/pg -c 'select 1 + 1 as sum'
 sum
-----
   2
(1 row)
```

See `tmp-postgres --help` for additional information.

## Installation

### Cargo

`cd` into `src` and `cargo build` as usual.

### Nix

`nix build` will compile the binary. You can also run it directly with

    nix run "github:tstat/tmp-postgres" -- <args>

or add it to your nix registry with

    nix registry add tmp-postgres "github:tstat/tmp-postgres"

then you can run it with `nix run tmp-postgres -- <args>`
