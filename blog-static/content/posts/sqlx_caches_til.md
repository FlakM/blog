---
title: "SQLx caches prepared statements per connection"
date: 2026-08-24T11:39:24+02:00
draft: true
authors: ["Maciej Flak"]
description:
    SQLx prepares and caches every query you run, transparently, in an LRU that belongs to the connection rather than the pool. This post traces the Postgres wire protocol with bpftrace to watch what is happening under the hood.
tags: ["rust", "sqlx", "postgres", "bpftrace", "performance"]
---

Today I learned that [`sqlx::query()`](https://docs.rs/sqlx/latest/sqlx/fn.query.html) prepares and caches sql statements transparently. The cache belongs to each *connection*, so a pool does not have one shared statement cache — it has one per connection. With `max_connections: 40`, a statement that is warm on one connection is cold on the other 39.

Caching is on by default, and for PostgreSQL each connection keeps up to 100 distinct statements in an LRU keyed by the SQL text. The capacity is configurable:

```rust
use sqlx::postgres::{PgConnectOptions, PgPool};

let options: PgConnectOptions = database_url.parse()?;
let pool = PgPool::connect_with(options.statement_cache_capacity(32)).await?;
```

That is all documented. What I actually wanted to do was to observe that behavior.

## Watching it happen

A Postgres container, a couple-line loop, and bpftrace on the socket syscalls:

```bash
docker run -d --name pgcache-lab -e POSTGRES_PASSWORD=lab -e POSTGRES_DB=lab \
  -p 5433:5432 postgres:16-bookworm

cd /tmp && cargo new pgcache && cd pgcache
cargo add sqlx --no-default-features --features postgres,runtime-tokio,tls-none
cargo add tokio --features macros,rt-multi-thread
```

The `iter_mark` function exists only to give bpftrace a clean symbol to hang a uprobe on, so the trace output is delimited per call:

```rust
use sqlx::{Connection, PgConnection, Row};

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn iter_mark(_i: u64, _cache: u64) {}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let mut c = PgConnection::connect("postgres://postgres:lab@127.0.0.1:5433/lab").await?;

    for i in 0..5 {
        iter_mark(i, c.cached_statements_size() as u64);

        // call 2 gets a two-element IN list: different SQL text, so a different cache key
        let n = if i == 2 { 2 } else { 1 };
        let placeholders: Vec<String> = (2..2 + n).map(|p| format!("${p}::int")).collect();
        let sql = format!("SELECT $1::int WHERE $1::int IN ({})", placeholders.join(", "));

        let mut q = sqlx::query(&sql).bind(1i32);
        for _ in 0..n {
            q = q.bind(1i32);
        }
        q.fetch_all(&mut c).await?;
    }

    let names: Vec<String> = sqlx::query("SELECT name FROM pg_prepared_statements ORDER BY name")
        .fetch_all(&mut c)
        .await?
        .iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();
    println!("client cache={} server={:?}", c.cached_statements_size(), names);
    Ok(())
}
```

I built the lab with Rust's v0 symbol mangling so the readable suffixes used by the uprobes are stable across rebuilds:

```bash
RUSTFLAGS="-C symbol-mangling-version=v0" cargo build
```

A single `PgConnection`, not a pool — the cache is per connection, and a pool would scatter the observations.

Postgres frontend messages are self-identifying by their first byte (`P` = Parse, `B` = Bind), and sqlx batches each flush into one syscall. So the first byte of every send *is* the hit-or-miss signal — no need to guess from timing. A uprobe on `get_or_prepare` adds the SQL text for both paths:

```awk
// statement-cache trace: one arrow per round trip
uprobe:/tmp/pgcache/target/debug/pgcache:iter_mark {
  printf("\ncall %d  cache=%d\n", arg0, arg1);
}

uprobe:/tmp/pgcache/target/debug/pgcache:"*14get_or_prepare" {
  printf("  query: %s\n", str(arg2, arg3 + 1));
}

uprobe:/tmp/pgcache/target/debug/pgcache:"*executor7prepare" {
  printf("  cache: miss\n");
}

tracepoint:syscalls:sys_enter_sendto /pid == cpid/ {
  $p = (uint8 *)args->buff;
  $t = *uptr($p);
  if ($t == 80) { printf("  -> Parse %s\n", str($p + 5, 12)); @parse = count(); }
  if ($t == 66) { printf("  -> Bind  %s\n", str($p + 6, 12)); @bind  = count(); }
}
```

Four probes, each answering a different question:

- **`iter_mark`** is a marker function I put in the program purely to delimit the output. An empty `extern "C"` function with `#[unsafe(no_mangle)]` and `#[inline(never)]` gives a stable, unmangled symbol whose arguments land in `arg0`/`arg1` — so passing the loop index and `cached_statements_size()` labels every line that follows and prints the cache occupancy without a debugger.
- **`PgConnection::get_or_prepare`** sees every parameterized query before the cache lookup. In this build the returned future occupies the first ABI argument, followed by the connection and the `&str` pointer-length pair in `arg2`/`arg3`.
- **`executor::prepare`** is sqlx's own function, and sqlx calls it *only on a miss*. Its presence directly below a query line classifies that lookup as cold. The `"*executor7prepare"` glob is doing real work here — see below.
- **`sys_enter_sendto`** is the wire. Byte 80 is `P` (Parse), byte 66 is `B` (Bind); the statement name sits a few bytes in, at offset 5 for Parse and 6 for Bind, because Bind carries an empty portal name first.

Two details worth copying. `/pid == cpid/` scopes everything to the process launched by `-c`; the obvious-looking `comm == "pgcache"` is a trap, because tokio renames its worker threads and I/O on a worker would be missed, whereas bpftrace's `pid` is the thread group id and catches every thread. And there is no `END` block — bpftrace auto-prints `@parse` and `@bind` on exit, which is both shorter and avoids the double-printing you get by calling `print()` yourself.

### Where `*executor7prepare` comes from

`prepare` is a private function, so there is no tidy exported name to attach to. The full symbol is this:

```
_RNvNtNtCshtbpjjQYzMb_13sqlx_postgres10connection8executor7prepare
```

Which looks unusable until you notice it is not really obfuscated. Rust's v0 mangling writes a path as a sequence of length-prefixed components, so the readable path is sitting right there in plain text: `13sqlx_postgres`, `10connection`, `8executor`, `7prepare` — each name preceded by its own length. That makes the symbol greppable without a demangler:

```bash
nm target/debug/pgcache | grep -oP '_R\S*executor7prepare$'
```

The `Cshtbpjj...` in the middle is the crate disambiguator, a hash of the compilation. It changes whenever the crate is rebuilt, which is what makes hardcoding the full symbol a bad idea. But since it sits *before* the part we care about, an anchored glob skips it entirely — and bpftrace accepts globs in the symbol position:

```
uprobe:/tmp/pgcache/target/debug/pgcache:"*executor7prepare"
```

That attaches exactly one probe. It has to be anchored at the end, because `*prepare*` unanchored would also catch `prepare::{closure#0}` (the async body), its drop glue, and hashbrown's `prepare_resize`, and you would silently count each miss several times. Check the count in the `Attached N probes` line: if it is not the number you expect, the glob is too loose.

The same trick gets the cache itself, for counting lookups rather than misses:

```
uprobe:/tmp/pgcache/target/debug/pgcache:"*statement_cache*7get_mut*"
```

```console
❯ sudo bpftrace trace.bt -c /tmp/pgcache/target/debug/pgcache
Attached 4 probes

call 0  cache=0
  query: SELECT $1::int WHERE $1::int IN ($2::int)
  cache: miss
  -> Parse sqlx_s_1
  -> Bind  sqlx_s_1

call 1  cache=1
  query: SELECT $1::int WHERE $1::int IN ($2::int)
  -> Bind  sqlx_s_1

call 2  cache=1
  query: SELECT $1::int WHERE $1::int IN ($2::int, $3::int)
  cache: miss
  -> Parse sqlx_s_2
  -> Bind  sqlx_s_2

call 3  cache=2
  query: SELECT $1::int WHERE $1::int IN ($2::int)
  -> Bind  sqlx_s_1

call 4  cache=2
  query: SELECT $1::int WHERE $1::int IN ($2::int)
  -> Bind  sqlx_s_1
  query: SELECT name FROM pg_prepared_statements ORDER BY name
  cache: miss
  -> Parse sqlx_s_3
  -> Bind  sqlx_s_3
client cache=3 server=["sqlx_s_1", "sqlx_s_2", "sqlx_s_3"]

@bind: 6
@parse: 3
```

Because the protocol here is strictly request then response, every arrow is one round trip. Call 0 is cold and pays two, `Parse` then `Bind`. Calls 1, 3 and 4 pay one — `Bind` alone against a statement the server already holds.

Call 2 is the one worth staring at. The only difference is a single extra placeholder in the `IN` list, and that is enough to make it a different cache key, a second `Parse`, and a second server-side statement. Call 3 then goes straight back to `sqlx_s_1`, undisturbed by the miss in between. The tail — six `Bind` against three `Parse` — is the whole thing in two numbers: `@bind` counts what you asked for, `@parse` counts what it cost.

| Call | Messages | Round trips |
|---|---|---|
| Miss | `Parse`+`Describe`+`Sync`, then `Bind`+`Execute`+`Sync` | 2 |
| Hit | `Bind`+`Execute`+`Sync` | 1 |

Server-side the statements are real named objects, and the client cache is just a map from SQL text to those names:

```sql
SELECT name, statement FROM pg_prepared_statements;
--  sqlx_s_1 | SELECT $1::int WHERE $1::int IN ($2::int)
--  sqlx_s_2 | SELECT $1::int WHERE $1::int IN ($2::int, $3::int)
```

`pg_prepared_statements` is session-scoped, which is why the program queries it on its own connection. From a separate `psql` session it is empty.

## Why any of this matters

Call 2 above is a two-element toy, but it is the whole problem in miniature. A query built with a variable-length `IN` list — the shape you get from `QueryBuilder::push_tuples` over a `Vec` of ids — produces a different SQL string, and so a different cache key, for every distinct length:

```rust
// 300 distinct lengths -> 300 distinct cache keys -> a miss almost every call
let placeholders: Vec<String> = (2..2 + n).map(|p| format!("${p}::int")).collect();
let sql = format!("SELECT $1::int WHERE $1::int IN ({})", placeholders.join(","));
```

Every novel length costs the extra round trip, and once past the 100-entry capacity each insert also evicts a neighbour — and on Postgres that eviction is itself a blocking round trip, since sqlx waits for `CloseComplete`. Binding a single array parameter, or filtering in Rust, keeps one static statement that stays warm.

## Watching from PostgreSQL

The Debian image used above includes PostgreSQL's USDT probes, while the Alpine image does not. PostgreSQL's `statement__status` probe supplies the SQL text on Bind and Execute, so it can be correlated by backend PID with parse, plan, and execute probes:

```bash
PG_BIN="/proc/$(docker inspect -f '{{.State.Pid}}' pgcache-lab)/root/usr/lib/postgresql/16/bin/postgres"
sudo bpftrace -e "
usdt:$PG_BIN:postgresql:statement__status
/arg0/
{
  @sql[pid] = str(uptr(arg0));
}

usdt:$PG_BIN:postgresql:query__parse__start {
  @parsed[pid] = 1;
  @parse_started[pid] = nsecs;
  printf(\"prepare pid=%d sql=%s\\n\", pid, str(uptr(arg0)));
}

usdt:$PG_BIN:postgresql:query__parse__done
/@parse_started[pid]/
{
  printf(\"parsed  pid=%d duration_us=%d\\n\",
         pid, (nsecs - @parse_started[pid]) / 1000);
  \$removed = delete(@parse_started, pid);
}

usdt:$PG_BIN:postgresql:query__plan__start {
  @plan_started[pid] = nsecs;
}

usdt:$PG_BIN:postgresql:query__plan__done
/@plan_started[pid]/
{
  printf(\"planned pid=%d duration_us=%d sql=%s\\n\",
         pid, (nsecs - @plan_started[pid]) / 1000, @sql[pid]);
  \$removed = delete(@plan_started, pid);
}

usdt:$PG_BIN:postgresql:query__execute__start {
  @execute_started[pid] = nsecs;
  if (@parsed[pid]) {
    printf(\"execute pid=%d cache=miss sql=%s\\n\", pid, @sql[pid]);
    \$removed = delete(@parsed, pid);
  } else {
    printf(\"execute pid=%d cache=hit  sql=%s\\n\", pid, @sql[pid]);
  }
}

usdt:$PG_BIN:postgresql:query__execute__done
/@execute_started[pid]/
{
  printf(\"done    pid=%d duration_us=%d\\n\",
         pid, (nsecs - @execute_started[pid]) / 1000);
  \$removed = delete(@execute_started, pid);
}

END {
  clear(@sql);
  clear(@parsed);
  clear(@parse_started);
  clear(@plan_started);
  clear(@execute_started);
}"
```

`uptr(arg0)` tells bpftrace that PostgreSQL's USDT argument points into userspace. On current bpftrace, `delete` returns the removed value, so assigning it to `$removed` avoids a discarded-value warning.

```console
Attached 8 probes
# 👇 First SQL shape: prepare, plan, then execute with a cold cache
prepare pid=1138669 sql=SELECT $1::int WHERE $1::int IN ($2::int)
parsed  pid=1138669 duration_us=2328
planned pid=1138669 duration_us=510 sql=SELECT $1::int WHERE $1::int IN ($2::int)
execute pid=1138669 cache=miss sql=SELECT $1::int WHERE $1::int IN ($2::int)
done    pid=1138669 duration_us=19
# 👇 Same SQL text: no parse, and the execution is a cache hit
planned pid=1138669 duration_us=9 sql=SELECT $1::int WHERE $1::int IN ($2::int)
execute pid=1138669 cache=hit  sql=SELECT $1::int WHERE $1::int IN ($2::int)
done    pid=1138669 duration_us=2
# 👇 One extra placeholder creates a new cache key and another miss
prepare pid=1138669 sql=SELECT $1::int WHERE $1::int IN ($2::int, $3::int)
parsed  pid=1138669 duration_us=23
planned pid=1138669 duration_us=319 sql=SELECT $1::int WHERE $1::int IN ($2::int, $3::int)
execute pid=1138669 cache=miss sql=SELECT $1::int WHERE $1::int IN ($2::int, $3::int)
done    pid=1138669 duration_us=2
# 👇 Returning to the first shape reuses its cached statement
planned pid=1138669 duration_us=3 sql=SELECT $1::int WHERE $1::int IN ($2::int)
execute pid=1138669 cache=hit  sql=SELECT $1::int WHERE $1::int IN ($2::int)
done    pid=1138669 duration_us=1
planned pid=1138669 duration_us=2 sql=SELECT $1::int WHERE $1::int IN ($2::int)
execute pid=1138669 cache=hit  sql=SELECT $1::int WHERE $1::int IN ($2::int)
done    pid=1138669 duration_us=1
# 👇 The introspection query is a third SQL shape, so it also misses
prepare pid=1138669 sql=SELECT name FROM pg_prepared_statements ORDER BY name
parsed  pid=1138669 duration_us=73
planned pid=1138669 duration_us=1391 sql=SELECT name FROM pg_prepared_statements ORDER BY name
execute pid=1138669 cache=miss sql=SELECT name FROM pg_prepared_statements ORDER BY name
done    pid=1138669 duration_us=950
```

The five loop iterations produce two misses and three hits. The final `pg_prepared_statements` query adds one more miss. This server-side view confirms the same cache behavior without depending on sqlx's private symbols.

## Notes for reproducing this

Sources: [`Query::persistent`](https://docs.rs/sqlx/latest/sqlx/query/struct.Query.html#method.persistent), [`PgConnectOptions::statement_cache_capacity`](https://docs.rs/sqlx/latest/sqlx/postgres/struct.PgConnectOptions.html#method.statement_cache_capacity), and `sqlx-postgres/src/connection/executor.rs`.
