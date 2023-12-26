+++ 
draft = true
date = 2023-12-20T14:16:11+01:00
title = "Making the blog federated"
slug = ""
authors = []
tags = []
categories = []
externalLink = ""

series = ["Simple personal blog"]
description = """
Live blog federation using ActivityPub.
"""
+++

## Live federation

To make blog more interesting we can make it federated using [ActivityPub](https://www.w3.org/TR/activitypub/).
The ActivityPub is a decentralized social networking protocol.
This should in theory allow to make a static blog way more dynamic.

It should become an active participant of wider ecosystem. You would be able to receive posts from the favorite mastodon client.
Over the time I'd also like to show the comments and likes just aside the content itself.

To get the things started quickly enough I'll use the examples written by amazing lemmy community: [ActivityPub-Federation](https://github.com/LemmyNet/activitypub-federation-rust).
The project brings a high level framework with all the federation functionality without any prior knowledge.

Initial version will be just a [live federation example](https://github.com/LemmyNet/activitypub-federation-rust/tree/main/examples/live_federation) with small tweaks.
Only changed things are the local username and domain and handling of Follow activity from local federation example.

## Implementing the storage

Since our appliaction is a single process on a single host we can use sqlite as a database.
It's a nice mix between full blown sql server like postgres and rolling our own storage.

Fortunately rust has a very strong support for databases. Let's use `sqlx`:

```toml
# backend/Cargo.toml
sqlx = { version = "0.7", features = [ "runtime-tokio", "migrate", "sqlite", "chrono" , "json"] }
```

Since the schema will evolve over time we can use migrations:

```rust
// backend/src/main.rs
let database_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "./db.sqlite".into());
let options = SqliteConnectOptions::new()
    .filename(database_path)
    .create_if_missing(true);
let pool = SqlitePool::connect_with(options).await?;
// Run migrations in ./migrations
sqlx::migrate!().run(&pool).await?;
```

To include sqlx into our nix build we'll have to adjust main `flake.nix`

```nix
# flake.nix
...
devShell.${system} = pkgs.mkShell {
  # 👇  we need to tell our binary and sqlx compile time checks
  #     where our database should be located
  DATABASE_PATH = "./db.sqlite";
  DATABASE_URL = "sqlite://./db.sqlite";

  buildInputs = with pkgs; [
    
    sqlx-cli
  ];
};
```

And in backend's `flake.nix` we have to tell crane about our migrations:

```nix
# backend/flake.nix
craneLib = crane.lib.${system};

sqlFilter = path: _type: null != builtins.match ".*sql$" path;
sqlOrCargo = path: type: (sqlFilter path type) || (craneLib.filterCargoSources path type);

src = lib.cleanSourceWith {
  src = craneLib.path ./.; # The original, unfiltered source
  filter = sqlOrCargo;
};
```

Based on database schema (not shown here for brevity) we can create a trait that will be used in http routes.


```rust
pub type Repository = Arc<dyn Db + Send + Sync>;

#[async_trait]
pub trait Db {
    async fn blog_user(&self) -> Result<DbUser, Error>;

    async fn user_by_name(&self, name: &str) -> Result<Option<DbUser>, Error>;

    async fn user_by_object_id(&self, object_id: &str) -> Result<Option<DbUser>, Error>;

    async fn user_followers(&self, user: &DbUser) -> Result<Vec<Follower>, Error>;

    async fn save_user(&self, user: &DbUser) -> Result<SavedUser, Error>;

    async fn remove_user_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error>;

    async fn add_user_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error>;
}
```

## Debuging the messages

It's easy to follow the example and get the rest responses using curl.
Harder thing is hacking on what actual messages are sent between the servers and our little backend.
Let's use a small trick to peak under the covers. So after deploying the example we can go to our mastodon client and type the username `@blog_test@fedi.flakm.com` and press follow:


```bash
❯ ssh root@hetzner-blog
Last login: Wed Dec 20 11:00:59 2023 from 100.99.19.38
nix  flakes run nixpkgs#tshark
[root@nixos:~]# nix run nixpkgs#tshark -- -i lo  -f "tcp port 3000"  -Y http
Running as user "root" and group "root". This could be dangerous.
Capturing on 'Loopback: lo'
    4 0.000093194    127.0.0.1 → 127.0.0.1    HTTP 1114 POST /blog_test2/inbox HTTP/1.0  (application/activity+json)
    6 0.103275143    127.0.0.1 → 127.0.0.1    HTTP 141 HTTP/1.0 200 OK
   14 0.223853980    127.0.0.1 → 127.0.0.1    HTTP 799 GET /blog_test2 HTTP/1.0
   16 0.224439328    127.0.0.1 → 127.0.0.1    HTTP 1235 HTTP/1.0 200 OK  (application/activity+json)
   24 0.320410582    127.0.0.1 → 127.0.0.1    HTTP 362 GET /.well-known/webfinger?resource=acct:blog_test2@fedi.flakm.com HTTP/1.0
   26 0.320989568    127.0.0.1 → 127.0.0.1    HTTP/JSON 464 HTTP/1.0 200 OK , JSON (application/json)
```

Ok, we have some action. Lets dig into the messages themself:

```bash
[root@nixos:~]# nix run nixpkgs#tshark -- -i lo  -f "tcp port 3000"  -T fields -e http.file_data -w out.pcap
[root@nixos:~]# nix run nixpkgs#tshark -- -r out.pcap  -T fields -e http.file_data | xxd -r -p | jq
```

And the bodies:

```json
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "id": "https://hachyderm.io/ca7b8bb3-a304-4d8d-9958-6d57440ea39e",
  "type": "Follow",
  "actor": "https://hachyderm.io/users/flakm",
  "object": "https://fedi.flakm.com/blog_test2"
}
...
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "id": "https://hachyderm.io/users/flakm#follows/3469756/undo",
  "type": "Undo",
  "actor": "https://hachyderm.io/users/flakm",
  "object": {
    "id": "https://hachyderm.io/ca7b8bb3-a304-4d8d-9958-6d57440ea39e",
    "type": "Follow",
    "actor": "https://hachyderm.io/users/flakm",
    "object": "https://fedi.flakm.com/blog_test2"
  }
}
```

Based on that information we can create new handler activity type `Unfollow` (`Follow` is already copied from local example):

```rust
// backend/src/activities/undo_follow.rs
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Unfollow {
    pub(crate) actor: ObjectId<DbUser>,
    pub(crate) object: Follow,
    #[serde(rename = "type")]
    kind: UndoType,
    id: Url,
}
```

And handle the message:

```rust
// backend/src/activities/undo_follow.rs
#[async_trait::async_trait]
impl ActivityHandler for Unfollow {
    type DataType = Repository;
    type Error = crate::error::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        tracing::info!("Received unfollow from {}", self.actor.inner());

        let local_user = data.blog_user().await?;
        let follower = self.actor.dereference(data).await?;

        // remove the follower
        data.remove_user_follower(&local_user, &follower).await?;

        // send back an accept
        let id = generate_object_id(data.domain())?;
        let accept = Accept::new(local_user.ap_id.clone(), self.object, id.clone());
        tracing::info!("Sending unfollow accept to {}", follower.ap_id);
        local_user
            .send(accept, vec![follower.shared_inbox_or_inbox()], data)
            .await?;
        Ok(())
    }
}
```

And let's verify the follow/unfollow sequence works:

```bash
[root@nixos:~]# nix run nixpkgs#tshark -- -i lo  -f "tcp port 3000"  -Y http
Running as user "root" and group "root". This could be dangerous.
Capturing on 'Loopback: lo'
    4 0.000084167    127.0.0.1 → 127.0.0.1    HTTP 1114 POST /blog_test2/inbox HTTP/1.0  (application/activity+json)
    6 0.203666266    127.0.0.1 → 127.0.0.1    HTTP 141 HTTP/1.0 200 OK
   14 0.273492222    127.0.0.1 → 127.0.0.1    HTTP 799 GET /blog_test2 HTTP/1.0
   16 0.274159654    127.0.0.1 → 127.0.0.1    HTTP 1235 HTTP/1.0 200 OK  (application/activity+json)
   24 0.293918022    127.0.0.1 → 127.0.0.1    HTTP 362 GET /.well-known/webfinger?resource=acct:blog_test2@fedi.flakm.com HTTP/1.0
   26 0.294671706    127.0.0.1 → 127.0.0.1    HTTP/JSON 464 HTTP/1.0 200 OK , JSON (application/json)
   34 0.928463746    127.0.0.1 → 127.0.0.1    HTTP 1243 POST /blog_test2/inbox HTTP/1.0  (application/activity+json)
   36 1.023983303    127.0.0.1 → 127.0.0.1    HTTP 141 HTTP/1.0 200 OK
```







