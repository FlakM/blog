+++ 
draft = true
date = 2023-12-22T21:13:20+01:00
title = "Exposing blog posts to backend"
slug = ""
authors = []
tags = []
categories = []
externalLink = ""
series = ["Simple personal blog"]

description = """
Exposing blog posts from static hugo site to backend.
"""
+++

## Create hugo blog posts index

Hugo is a static web site generator. It's job is to take a bunch of markdown file and output html.
But it can be hacked to output other formats like RSS feeds or custom json.
By following the [tutorial on custom output formats)[https://gohugo.io/templates/output-formats/] we can build a custom json file that will serve as index for backend.
All changes required are:

1. Add a piece of configuration to tell about the new format:

```toml
# blog-static/config.toml
[outputFormats.BlogListJSON]
mediaType = "application/json"
baseName = "bloglist"
isPlainText = true

[outputs]
home = ["HTML", "RSS", "BlogListJSON"]
```

2. Add a hugo temlate to show how to render the json: 

```go
// blog-static/layouts/index.bloglistjson.json
{{- $.Scratch.Add "index" slice -}}
{{- range .Site.RegularPages -}}
    {{- if eq .Section "posts" }}
        {{- $.Scratch.Add "index" (dict 
            "title" .Title 
            "description" .Description 
            "date" .Date 
            "featuredImage" .Params.featured_image 
            "tags" .Params.tags 
            "url" .Permalink 
        ) -}}
    {{- end -}}
{{- end -}}
{{- $.Scratch.Get "index" | jsonify -}}
```

And voila if we build it using `nix build` inside blog-static directory we'll find the index:

```bash
❯ file result/bloglist.json
result/bloglist.json: JSON text data
```

## Exposing blog index to backend code

Since backend and static site are deployed together we can just tell the backend module where the index is located:

```nix
# backend/flake.nix
options.services.backend = {
  enable = mkEnableOption "Enables the backend HTTP service";

  domain = mkOption rec {
    type = types.str;
    default = "localhost";
    example = default;
    description = "The domain name";
  };

  # 👇 this configuration is new
  posts_path = mkOption {
    type = types.path;
    default = "./posts.json";
    description = "The path to the posts json file";
  };
};


config = mkIf cfg.enable {
  systemd.services.backend = {
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Restart = "on-failure";
      #                                  👇 pass index file path as first argument
      ExecStart = "${server}/bin/backend ${config.services.backend.posts_path}";
      DynamicUser = true;
      TemporaryFileSystem = "/:ro";
      BindPaths = "/var/lib/backend";
      StateDirectory = "backend";
      WorkingDirectory = "/var/lib/backend";
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
      NoNewPrivileges = true;
    };
    environment = {
      "RUST_LOG" = "INFO";
      "DATABASE_PATH" = "/var/lib/backend/db.sqlite3";
    };
  };
```

Just like that our program will be able to read the contents of the index in plain rust:

```rust
let posts = std::env::args().nth(1).expect("No posts file given");
info!("Posts file: {}", posts);
let posts = std::fs::read_to_string(posts).expect("Failed to read posts file");
info!("Posts: {}", posts);
```

## Parsing the posts 

We now have to define a rust
