---
title: "Proving SQLx’s Statement Cache with bpftrace"
date: 2026-08-24T11:39:24+02:00
draft: false
authors: ["Maciej Flak"]
description:
    SQLx prepares and caches every query you run, transparently, in an LRU that belongs to the connection rather than the pool. This post traces the Postgres wire protocol with bpftrace to watch what is happening under the hood.
tags: ["rust", "sqlx", "postgres", "bpftrace", "performance"]
---

During the blissful one week off I spent without touching any AI assistants, I realized I'd stopped learning new things during regular paid work.

Now my job requires me to work with the clankers, so I don't get to observe the APIs or hit my head against the invalid imagination of some gnarly problem.

I decided to spend some time restarting the habit of writing down what I discover and, more importantly, how I prove it to myself. Let's start with some surprising optimization in sqlx

Caching is on by default. With PostgreSQL, each connection keeps up to 100 statements in an LRU keyed by the SQL text. The capacity is configurable:

```rust
use sqlx::postgres::{PgConnectOptions, PgPool};

let options: PgConnectOptions = database_url.parse()?;
let pool = PgPool::connect_with(options.statement_cache_capacity(32)).await?;
```

That part is documented. I wanted to see it happen.

## Watching it happen

All I needed was a Postgres container, a small loop, and bpftrace on the socket syscalls:

```bash
docker run -d --name pgcache-lab -e POSTGRES_PASSWORD=lab -e POSTGRES_DB=lab \
  -p 5433:5432 postgres:16-bookworm

cd /tmp && cargo new pgcache && cd pgcache
cargo add sqlx --no-default-features --features postgres,runtime-tokio,tls-none
cargo add tokio --features macros,rt-multi-thread
```

I added `iter_mark` only to give bpftrace a clean symbol for a uprobe. It separates the trace output by call:

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

I built the lab with Rust's v0 symbol mangling. This keeps the readable suffixes used by the uprobes stable across rebuilds:

```bash
RUSTFLAGS="-C symbol-mangling-version=v0" cargo build
```

I used one `PgConnection`, not a pool. Otherwise, the pool would scatter the observations across several caches.

Postgres frontend messages identify themselves with their first byte (`P` = Parse, `B` = Bind). SQLx batches each flush into one syscall. That makes the first byte of every send the hit-or-miss signal. No guessing from timing. A uprobe on `get_or_prepare` adds the SQL text:

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

- **`iter_mark`** separates the output. An empty `extern "C"` function with `#[unsafe(no_mangle)]` and `#[inline(never)]` gives me a stable symbol. Its arguments land in `arg0`/`arg1`, so I pass the loop index and `cached_statements_size()` to label each call without a debugger.
- **`PgConnection::get_or_prepare`** sees every parameterized query before the cache lookup. In this build the returned future occupies the first ABI argument, followed by the connection and the `&str` pointer-length pair in `arg2`/`arg3`.
- **`executor::prepare`** is SQLx's own function. SQLx calls it *only on a miss*. When it appears below a query, that lookup was cold. The `"*executor7prepare"` glob is doing real work here. More on that below.
- **`sys_enter_sendto`** is the wire. Byte 80 is `P` (Parse), byte 66 is `B` (Bind); the statement name sits a few bytes in, at offset 5 for Parse and 6 for Bind, because Bind carries an empty portal name first.

Two details are worth copying. `/pid == cpid/` limits the trace to the process launched by `-c`. The obvious `comm == "pgcache"` is a trap. Tokio renames its worker threads, so it would miss I/O from a worker. bpftrace's `pid` is the thread group ID, which catches them all.

There is no `END` block either. bpftrace prints `@parse` and `@bind` on exit. Calling `print()` myself would only print them twice.

### Where `*executor7prepare` comes from

`prepare` is a private function, so there is no tidy exported name to attach to. The full symbol is this:

```
_RNvNtNtCshtbpjjQYzMb_13sqlx_postgres10connection8executor7prepare
```

It looks unusable, but it isn't really obfuscated. Rust's v0 mangling writes a path as length-prefixed components. The readable path is right there: `13sqlx_postgres`, `10connection`, `8executor`, `7prepare`. Each name starts with its length. That makes the symbol greppable without a demangler:

```bash
nm target/debug/pgcache | grep -oP '_R\S*executor7prepare$'
```

The `Cshtbpjj...` part is the crate disambiguator, a hash of the compilation. It changes whenever the crate is rebuilt, so hardcoding the full symbol is a bad idea. Luckily, it sits *before* the part I care about. An anchored glob skips it, and bpftrace accepts globs in the symbol position:

```
uprobe:/tmp/pgcache/target/debug/pgcache:"*executor7prepare"
```

That attaches exactly one probe. The end anchor matters. A loose `*prepare*` would also catch `prepare::{closure#0}` (the async body), its drop glue, and hashbrown's `prepare_resize`. Each miss would quietly get counted several times. Check the `Attached N probes` line. If the number is wrong, the glob is too loose.

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

The protocol here is strictly request then response, so every arrow is one round trip. Call 0 is cold and pays for two: `Parse`, then `Bind`. Calls 1, 3 and 4 pay for one. They only send `Bind` against a statement the server already holds.

Call 2 is the interesting one. The only difference is one extra placeholder in the `IN` list. That is enough for a new cache key, another `Parse`, and another server-side statement. Call 3 goes straight back to `sqlx_s_1`.

The final numbers tell the whole story: six `Bind` messages and three `Parse` messages. `@bind` counts what I asked for. `@parse` counts what it cost.

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

Call 2 is a tiny example of a real problem. A query with a variable-length `IN` list, like one built with `QueryBuilder::push_tuples` over a `Vec` of IDs, produces different SQL for every length. Each string gets its own cache key:

```rust
// 300 distinct lengths -> 300 distinct cache keys -> a miss almost every call
let placeholders: Vec<String> = (2..2 + n).map(|p| format!("${p}::int")).collect();
let sql = format!("SELECT $1::int WHERE $1::int IN ({})", placeholders.join(","));
```

Every new length costs an extra round trip. Once the cache has 100 entries, each insert also evicts a neighbour. On Postgres, that eviction is another blocking round trip because SQLx waits for `CloseComplete`. Binding one array parameter, or filtering in Rust, keeps one static statement warm.

## Watching from PostgreSQL

The Debian image includes PostgreSQL's USDT probes. The Alpine image doesn't. PostgreSQL's `statement__status` probe gives me the SQL text on Bind and Execute. I can correlate it by backend PID with the parse, plan, and execute probes:

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

`uptr(arg0)` tells bpftrace that PostgreSQL's USDT argument points into userspace. Current bpftrace returns the removed value from `delete`, so I assign it to `$removed` to avoid a warning.

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

The five loop iterations give me two misses and three hits. The final `pg_prepared_statements` query adds one more miss. Same cache behavior, this time without touching SQLx's private symbols.

## What I've learned

I've learned a couple of new things:

- It's crucial to think about the shape of prepared statements - the additional round trip to the database isn't big, but it isn't free
- Ustdts in codebases like postgres are cool, but with AI it's also easy to instrument the less polished codebases
- Creating simple reproducers and tracing them with the same tools that can be used in production is a great way to learn and understand the systems

## Notes for reproducing this

Sources: [`Query::persistent`](https://docs.rs/sqlx/latest/sqlx/query/struct.Query.html#method.persistent), [`PgConnectOptions::statement_cache_capacity`](https://docs.rs/sqlx/latest/sqlx/postgres/struct.PgConnectOptions.html#method.statement_cache_capacity), and `sqlx-postgres/src/connection/executor.rs`.
