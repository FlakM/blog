---
title: "One Lock to Rule Them All"
date: 2026-09-10T16:51:41+02:00
draft: false
authors: ["Maciej Flak"]
description:
    How to simplify migrations thanks to postgres advisory locks using rust and sqlx. Observing the locks with bpftrace.
tags: ["rust", "sqlx", "postgres"]
series: ["TIL"]
---

I have been doing a lot of byte shoveling lately... Stuff that we've always postponed because it wasn't realistic to spend a couple of days on. Now it's faster and easier to finish those cleanup scripts and finally get rid of those legacy tables and databases.

In my previous blog post [Proving SQLx’s Statement Cache with bpftrace](/posts/sqlx_caches_til/), I used bpftrace to observe the statements being prepared on the database side. This time, I was concerned about how client inactivity timeouts affect advisory locks, so I reached for the same tools.

What needs to be done is to: run something, run it once, possibly for a prolonged period of time, handle errors, and not kill the patient while operating on it.

How can it be done alternatively:

1. Simple internal RPC endpoint - easy to restart and retry, but it requires human monitoring
2. A Kubernetes Job separates the migration from the application lifecycle and can be wired into Argo CD hooks, but retries and possible duplicate execution still require the work to be safe to repeat.

But I wanted something more generic that can match our other components already in place. I decided to use a similar approach to what sqlx uses for migrations, with a small twist.

## Advisory locks in PostgreSQL

According to the [documentation](https://www.postgresql.org/docs/current/explicit-locking.html) postgres apart from table and row-level locks, also supports application-level locks - called advisory locks. You can obtain them at two levels: session (explicitly released) and transaction (automatically released at the end of the transaction). The state of the locks can be checked using the `pg_locks` view.

For our case, the migration can take a significant amount of time, so the session-level locks with exclusive mode are the best fit since we want to have a single instance of the migration - possibly opening and closing a large number of transactions during its lifetime.

Seeing it in action:

### Terminal 1 - Postgres (Debian, foreground so the log streams)

```bash
docker run --rm --name pg --pid=host -p 5433:5432 \
  -e POSTGRES_PASSWORD=x \
  postgres:16-bookworm \
  -c log_connections=on -c log_disconnections=on -c "log_line_prefix=%m [%p] app=%a "
```

`--pid=host` is what lines the PIDs up. Not `-alpine` - musl build, no USDT.

### Terminal 2 - bpftrace (start this before you connect, so it sees the events)

```bash
sudo bpftrace /tmp/lock_trace.bt \
  /proc/$(docker inspect -f '{{.State.Pid}}' pg)/root/usr/lib/postgresql/16/bin/postgres
```

The script traces lock acquisition, explicit release, and session cleanup. For example, this probe reads the lock key when PostgreSQL enters the blocking lock function:

```bpftrace
uprobe:$1:pg_advisory_lock_int8
{
  @key[tid] = *(int64*)uptr(arg0 + 32);
  @t0[tid] = nsecs;
}
```

{{< details summary="Show the full /tmp/lock_trace.bt script" >}}

```bpftrace
BEGIN
{
  printf("%-8s %-6s %-14s %s\n", "TIME", "PID", "EVENT", "KEY");
}

uprobe:$1:pg_try_advisory_lock_int8,
uprobe:$1:pg_advisory_lock_int8
{
  @key[tid] = *(int64*)uptr(arg0 + 32);
  @t0[tid] = nsecs;
}

uretprobe:$1:pg_try_advisory_lock_int8
/@t0[tid]/
{
  printf("%-8s %-6d %-14s %lld\n",
         strftime("%H:%M:%S", nsecs), pid,
         retval ? "TAKE" : "TAKE denied", @key[tid]);
  if (retval) { @held[pid]++; }
  $_ = delete(@key, tid);
  $_ = delete(@t0, tid);
}

uretprobe:$1:pg_advisory_lock_int8
/@t0[tid]/
{
  printf("%-8s %-6d %-14s %lld  (blocked %d ms)\n",
         strftime("%H:%M:%S", nsecs), pid, "TAKE", @key[tid],
         (nsecs - @t0[tid]) / 1000000);
  @held[pid]++;
  $_ = delete(@key, tid);
  $_ = delete(@t0, tid);
}

uprobe:$1:pg_advisory_unlock_int8
{
  @ukey[tid] = *(int64*)uptr(arg0 + 32);
  @unlocking[tid] = 1;
}

uretprobe:$1:pg_advisory_unlock_int8
/@unlocking[tid]/
{
  printf("%-8s %-6d %-14s %lld\n",
         strftime("%H:%M:%S", nsecs), pid,
         retval ? "RELEASE" : "RELEASE ?!", @ukey[tid]);
  if (retval && @held[pid] > 0) { @held[pid]--; }
  $_ = delete(@ukey, tid);
  $_ = delete(@unlocking, tid);
}

uprobe:$1:LockReleaseAll
/arg0 == 2 && arg1 == 1 && @held[pid] > 0/
{
  printf("%-8s %-6d %-14s %d lock(s) dropped by session exit\n",
         strftime("%H:%M:%S", nsecs), pid, "RELEASE (died)", @held[pid]);
}

uprobe:$1:LockReleaseAll
/arg0 == 2 && arg1 == 1/
{
  $_ = delete(@held, pid);
}

END
{
  clear(@key);
  clear(@t0);
  clear(@ukey);
  clear(@unlocking);
  clear(@held);
}
```

{{< /details >}}

Wait for the `Attached ... probes` line. The `/proc/<pid>/root/...` path is required: uprobes key on the inode inside the container's filesystem.

### Terminal 3 `psql`

```bash
PGPASSWORD=x psql -h localhost -p 5433 -U postgres
```

```sql
SET application_name = 'lab';
SELECT pg_backend_pid();                    -- note the number
SELECT pg_try_advisory_lock(5784863001);    -- T3: TAKE
SELECT pg_advisory_unlock(5784863001);      -- T3: RELEASE
SELECT pg_try_advisory_lock(5784863001);    -- take it again…
\q
```


Here is the corresponding bpftrace output:

```console
Attached 10 probes
TIME     PID    EVENT          KEY
16:00:16 879502 TAKE           5784863001
16:00:27 879502 RELEASE        5784863001
16:00:37 879502 TAKE           5784863001
...
```

### How did I build this trace?

Building the bpftrace scripts is a bit of a black art. I'm using a lot of ai guidance and examples from the [bpftrace tools](https://github.com/bpftrace/bpftrace/tree/master/tools).
If you are interested in the details of how I figured out the symbol names and the `+32` offset, read on. If you want to use the script, skip to the next section.

{{< details summary="Technical walkthrough: building the bpftrace probe" >}}

I did not know the symbol name or the `+32` offset. I started with the SQL function and followed it down through PostgreSQL.

First, PostgreSQL can tell us which C function implements a built-in SQL function:

```sql
SELECT oid::regprocedure, prosrc
FROM pg_proc
WHERE proname IN (
    'pg_advisory_lock',
    'pg_try_advisory_lock',
    'pg_advisory_unlock'
)
ORDER BY 1;
```

The interesting rows are:

```text
pg_advisory_lock(bigint)     | pg_advisory_lock_int8
pg_try_advisory_lock(bigint) | pg_try_advisory_lock_int8
pg_advisory_unlock(bigint)   | pg_advisory_unlock_int8
```

I then checked that these symbols really exist in the binary I am about to trace:

```bash
PG_BIN=/proc/$(docker inspect -f '{{.State.Pid}}' pg)/root/usr/lib/postgresql/16/bin/postgres

sudo nm -D "$PG_BIN" | grep advisory_lock
sudo bpftrace -l "uprobe:$PG_BIN:pg_*advisory*lock_int8"
```

That gives us this part:

```bpftrace
uprobe:$1:pg_advisory_lock_int8
```

`uprobe` means "run when this userspace function is entered". `$1` is the path to the PostgreSQL binary passed after the script name. The final part is the symbol found above.

The argument took a little more digging. In PostgreSQL's [`lockfuncs.c`](https://github.com/postgres/postgres/blob/REL_16_STABLE/src/backend/utils/adt/lockfuncs.c), the function starts like this:

```c
Datum
pg_advisory_lock_int8(PG_FUNCTION_ARGS)
{
    int64 key = PG_GETARG_INT64(0);
```

`PG_FUNCTION_ARGS` looks descriptive, but it is a macro. Following it into [`fmgr.h`](https://github.com/postgres/postgres/blob/REL_16_STABLE/src/include/fmgr.h) reveals the real function argument:

```c
#define PG_FUNCTION_ARGS FunctionCallInfo fcinfo
#define PG_GETARG_DATUM(n) (fcinfo->args[n].value)
#define PG_GETARG_INT64(n) DatumGetInt64(PG_GETARG_DATUM(n))
```

On x86-64, bpftrace exposes the first C argument as `arg0`, so `arg0` is the `fcinfo` pointer, not the lock key itself. The key lives in `fcinfo->args[0].value`.

The remaining question is where `args` begins in memory. I could add up the sizes of the fields before it, but the compiler may insert padding to keep them aligned. The standard C `offsetof(Type, field)` macro does that calculation correctly.

This tiny program includes PostgreSQL's structure definition and asks: "How many bytes are there between the start of `FunctionCallInfoBaseData` and its `args` field?"

```c
#include "postgres.h"
#include "fmgr.h"

#include <stddef.h>
#include <stdio.h>

int main(void)
{
    printf("args starts at byte %zu\n",
           offsetof(FunctionCallInfoBaseData, args));
}
```

I saved it as `/tmp/fcinfo_offset.c`, then compiled it against the server headers for the same PostgreSQL version:

```bash
cc -I"$(pg_config --includedir-server)" /tmp/fcinfo_offset.c -o /tmp/fcinfo_offset
/tmp/fcinfo_offset
```

```console
args starts at byte 32
```

The program is not reading a live PostgreSQL process. It is letting the C compiler lay out the structure exactly as PostgreSQL's headers describe it. The output means that, given a pointer to the start of `fcinfo`, its `args` array begins 32 bytes later.

The `args` field is an array with one `NullableDatum` for every SQL argument. PostgreSQL defines each element like this:

```c
typedef struct NullableDatum
{
    Datum value;
    bool isnull;
} NullableDatum;
```

`Datum` is PostgreSQL's generic container for a SQL value. It is large enough to hold either a small value directly or a pointer to a larger value. The separate `isnull` flag is needed because SQL `NULL` is not an ordinary value.

For `pg_advisory_lock(5784863001)`, `args[0]` represents the first SQL argument. On this 64-bit build its `value` field contains the `bigint` key directly. Because `value` is the first field in `NullableDatum`, it adds no extra offset: `args[0].value` starts at the same byte as `args[0]`, which is byte 32 in `fcinfo`.

Now the strange-looking line is just a translation of PostgreSQL's own `PG_GETARG_INT64(0)`:

```bpftrace
@key[tid] = *(int64*)uptr(arg0 + 32);
```

Starting at the `fcinfo` address in `arg0`, it moves 32 bytes to `args[0].value`, marks that address as a userspace pointer with `uptr`, reads the eight-byte lock key as an `int64`, and stores it under the current thread ID. The matching return probe uses that thread ID to find the right key when several PostgreSQL backends run at once.

Finally, `@t0[tid] = nsecs` remembers when the function was entered. The return probe subtracts it from the current time, which tells us how long a blocking `pg_advisory_lock` waited before it acquired the lock.

The `32` is not a stable PostgreSQL API. It belongs to this PostgreSQL 16, 64-bit structure layout.

{{< /details >}}

## Experiments we can now do with the trace

So now that we have the way of seeing what happens with the locks, we can experiment with the questions we have about the implementation to confirm our mental model.
For example, I was wondering what would happen if the process that holds the lock dies. Will the database see the immediate release of the lock?

```console
postgres=# SELECT pg_try_advisory_lock(5784863001);
 pg_try_advisory_lock 
----------------------
 t
(1 row)

postgres=# exit;
```


```console
TIME     PID    EVENT          KEY
09:39:29 115879 TAKE           5784863001
09:39:41 115879 RELEASE (died) 1 lock(s) dropped by session exit
```

What if another process tries to take the lock while the first one is holding it? Will it block, or will it fail immediately?

```console
postgres=# SELECT pg_try_advisory_lock(5784863001);
 pg_try_advisory_lock 
----------------------
 f
(1 row)
```

```console
TIME     PID    EVENT          KEY
09:41:58 118668 TAKE           5784863001
09:42:09 119173 TAKE denied    5784863001
```


The second process (PID 119173) tried to take the lock and was denied immediately. The SQL output also confirms that the `pg_try_advisory_lock` returned `false` for the second process, and it didn't block according to the documentation.

We can also check what will happen if we take the lock and then leave it on for a long time with session timeout, simulate network issues, and many other scenarios - having the ability to ask those questions and see the answers invites curiosity and better final design.

## The sqlx wrapper

I've created a small wrapper around sqlx that allows us to run those migrations in a single instance using sqlx - it follows the same pattern as sqlx migrations.

```rust
use std::{future::Future, time::Duration};

use sqlx::{
    postgres::{PgAdvisoryLock, PgConnection, Postgres},
    Acquire, Connection, Either,
};
use tokio::time::{interval, MissedTickBehavior};

pub async fn run_once<'a, A, M, F>(connection: A, migration: M) -> sqlx::Result<bool>
where
    A: Acquire<'a, Database = Postgres>,
    M: FnOnce() -> F,
    F: Future<Output = sqlx::Result<()>>,
{
    let lock = PgAdvisoryLock::new("data-migration");
    let mut conn = connection.acquire().await?;

    let Either::Left(mut guard) = lock.try_acquire(&mut *conn).await? else {
        return Ok(false); // another replica is already running it
    };

    let migration = migration();
    tokio::pin!(migration);

    let mut keep_alive = interval(Duration::from_secs(30));
    keep_alive.set_missed_tick_behavior(MissedTickBehavior::Delay);
    keep_alive.tick().await; // discard interval's immediate first tick

    let result = loop {
        tokio::select! {
            result = &mut migration => break result,
            _ = keep_alive.tick() => guard.as_mut().ping().await?,
        }
    };

    guard.release_now().await?;
    result?;

    Ok(true)
}
```

The migration itself is just a closure, so the wrapper does not know what kind of work it runs. The lock connection can come from a pool:

```rust {title="Using a pool"}
let work_pool = pool.clone();
run_once(&pool, || async move { backfill_users(work_pool).await }).await?;
```

Or it can be an existing direct connection:

```rust {title="Using a connection"}
let mut lock_conn = PgConnection::connect(&database_url).await?;
let work_pool = pool.clone();
run_once(&mut lock_conn, || async move { backfill_users(work_pool).await }).await?;
```

Every replica may call `run_once`. The first one gets `Either::Left` with a guard and runs the migration. The others get `Either::Right` and return `false` immediately - someone else is already holding the lock.

`Acquire` abstracts where the connection comes from: either a pool or an existing connection. Because `A::Database` is `Postgres`, it guarantees that the acquired value dereferences to `PgConnection`; `&mut *conn` borrows that concrete connection for the lock guard. The migration closure can use a separate pool while the acquired connection remains borrowed for the lock's lifetime.


Every 30 seconds, `PgConnection::ping` sends a PostgreSQL protocol `Sync` message on that connection. 
It creates keep-alive traffic to prevent overly eager routers and load balancers from killing the connection after some inactivity timeout.


It creates traffic without executing a SQL query, which keeps the session active through idle network infrastructure.

The ping proves that the session is still alive, not that it still owns the advisory lock. PostgreSQL does not reap a lock from a live session, but the heartbeat is not fencing: if the lock session is lost, another replica may acquire the lock before the original worker notices. Work running through other connections may outlive the lock session, and dropping the migration future does not guarantee that already-submitted or externally spawned work stops. This wrapper is intended for retry-safe migrations, not strict exactly-once execution.

I save the migration result before calling `release_now`, so a failed migration still takes the explicit unlock path.

And we can now observe the lock being taken and released in the bpftrace output (first run of the empty migration, second stopped by ctrl-c):

```console
TIME     PID    EVENT          KEY
12:24:44 253918 TAKE           5377830020611403474
12:24:54 253918 RELEASE        5377830020611403474
12:29:41 259814 TAKE           5377830020611403474
12:29:42 259814 RELEASE (died) 1 lock(s) dropped by session exit
```


### Conclusions

The postgres advisory locks are a great tool for building simple yet correct code, since that particular database is almost universally present in the stack - it's a powerful primitive that we can build upon.
I've used bpftrace to confirm my understanding of the locks in the database - I was able to investigate the difference between session and transaction locks and see the client timeouts in action.

I love this approach of using an observability tool to zoom in from the outside of the application. What's most important is that it makes me want to play with the system - it brings the joy of coding back! 💗
