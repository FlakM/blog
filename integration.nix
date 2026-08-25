{ pkgs
, system
, backend
, static
}:
let
  testPort = 443;
  hosts = ''
    192.168.2.101 blog.local
    192.168.2.102 mastodon.local
  '';
  certificate = pkgs.runCommand "fediverse-test-certificate"
    {
      nativeBuildInputs = [ pkgs.openssl ];
    } ''
    mkdir -p $out
    openssl req -x509 -newkey rsa:2048 -nodes \
      -keyout ca-key.pem -out $out/ca.pem -days 1 \
      -subj '/CN=Fediverse Test CA' \
      -addext 'basicConstraints=critical,CA:TRUE'
    openssl req -newkey rsa:2048 -nodes \
      -keyout $out/key.pem -out request.pem \
      -subj '/CN=blog.local'
    openssl x509 -req -in request.pem -CA $out/ca.pem -CAkey ca-key.pem \
      -CAcreateserial -out $out/cert.pem -days 1 \
      -extfile <(printf '%s\n' \
        'subjectAltName=DNS:blog.local,DNS:mastodon.local' \
        'basicConstraints=critical,CA:FALSE' \
        'keyUsage=critical,digitalSignature,keyEncipherment' \
        'extendedKeyUsage=serverAuth')
  '';
  sharedModule = {
    virtualisation.graphics = false;
    networking.extraHosts = hosts;
    security.pki.certificateFiles = [ "${certificate}/ca.pem" ];
  };
  emptyBlogPosts = pkgs.writeText "empty-test-posts.json" "[]";
  testBlogPosts = pkgs.writeText "test-posts.json" ''
    [
      {
        "title": "SQLx caches prepared statements per connection",
        "slug": "sqlx_caches_til",
        "description": "A real blog post used for integration testing",
        "date": "2026-08-24T12:00:00Z",
        "featuredImage": null,
        "tags": ["test", "integration"],
        "url": "https://blog.local/posts/sqlx_caches_til/"
      }
    ]
  '';
  fallbackBlogPost = pkgs.writeText "fallback-test-post.json" ''
    [
      {
        "title": "Fallback Test Post",
        "slug": "fallback-test-post",
        "description": "A post published while Mastodon resolution is unavailable",
        "date": "2026-08-25T12:00:00Z",
        "featuredImage": null,
        "tags": ["test"],
        "url": "https://blog.local/posts/fallback-test-post/"
      }
    ]
  '';
in
{
  name = "integration";

  nodes = {
    server = {
      imports = [
        sharedModule
        backend.nixosModules.x86_64-linux.default
        static.nixosModules.x86_64-linux.default
      ];

      networking = {
        interfaces.eth1.ipv4.addresses = [{
          address = "192.168.2.101";
          prefixLength = 24;
        }];
        firewall = {
          enable = true;
          allowedTCPPorts = [ 80 443 ];
        };
      };

      systemd = {
        tmpfiles.rules = [
          "d /var/lib/blog 0755 root root -"
          "L+ /var/lib/blog/posts.json - - - - ${emptyBlogPosts}"
        ];
        services.backend.environment = {
          FEDIVERSE_DEBUG = "true";
          SSL_CERT_FILE = "/etc/ssl/certs/ca-certificates.crt";
        };
      };

      services = {
        backend = {
          enable = true;
          domain = "blog.local";
          fediverse_domain = "blog.local";
          preferred_mastodon_instance = "mastodon.local";
          mastodon_access_token_file = "/run/mastodon-access-token";
          posts_path = "/var/lib/blog/posts.json";
        };
        static-website = {
          enable = true;
          domain = "blog.local";
        };
        nginx = {
          enable = true;
          virtualHosts."blog.local" = {
            forceSSL = true;
            sslCertificate = "${certificate}/cert.pem";
            sslCertificateKey = "${certificate}/key.pem";
          };
        };
        postgresql = {
          enable = true;
          package = pkgs.postgresql_15;
          ensureDatabases = [ "blog" ];
          ensureUsers = [{
            name = "blog";
            ensureDBOwnership = true;
          }];
          authentication = pkgs.lib.mkOverride 10 ''
            local   blog        blog                    trust
            host    blog        blog    127.0.0.1/32    trust
            host    blog        blog    ::1/128         trust
            local   all         all                     trust
            host    all         all     127.0.0.1/32    ident
            host    all         all     ::1/128         ident
          '';
        };
      };
    };

    mastodon = { config, ... }: {
      imports = [ sharedModule ];

      virtualisation.memorySize = 2048;
      networking = {
        interfaces.eth1.ipv4.addresses = [{
          address = "192.168.2.102";
          prefixLength = 24;
        }];
        firewall.allowedTCPPorts = [ 80 443 ];
      };

      services = {
        mastodon = {
          enable = true;
          configureNginx = true;
          localDomain = "mastodon.local";
          enableUnixSocket = false;
          streamingProcesses = 1;
          smtp = {
            createLocally = false;
            fromAddress = "mastodon@mastodon.local";
          };
          extraConfig = {
            ALLOWED_PRIVATE_ADDRESSES = "192.168.2.0/24";
            AUTHORIZED_FETCH = "true";
            EMAIL_DOMAIN_ALLOWLIST = "example.com";
          };
        };
        nginx.virtualHosts."mastodon.local" = {
          enableACME = pkgs.lib.mkForce false;
          sslCertificate = "${certificate}/cert.pem";
          sslCertificateKey = "${certificate}/key.pem";
        };
      };

      systemd.services.mastodon-test-user = {
        description = "Create the Mastodon integration test user";
        wantedBy = [ "multi-user.target" ];
        after = [ "mastodon-init-db.service" ];
        requires = [ "mastodon-init-db.service" ];
        environment = builtins.removeAttrs config.systemd.services.mastodon-web.environment [ "PATH" ] // {
          HOME = "/var/lib/mastodon";
        };
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          User = "mastodon";
          Group = "mastodon";
          WorkingDirectory = "${pkgs.mastodon}";
          EnvironmentFile = "/var/lib/mastodon/.secrets_env";
        };
        script = ''
          ${pkgs.mastodon}/bin/rails runner '
            user = User.find_by(email: "alice@example.com")
            unless user
              account = Account.new(username: "alice")
              user = User.new(email: "alice@example.com", password: "integration-test", agreement: true, confirmed_at: Time.now.utc, bypass_registration_checks: true)
              user.account = account
              user.save!
              user.mark_email_as_confirmed!
              user.approve!
            end
            application = Doorkeeper::Application.find_by!(superapp: true)
            token = Doorkeeper::AccessToken.create!(application: application, resource_owner_id: user.id, scopes: "read write follow")
            File.write("/var/lib/mastodon/test-token", token.token)
          '
        '';
      };
    };

    client = {
      imports = [ sharedModule ];
      networking.interfaces.eth1.ipv4.addresses = [{
        address = "192.168.2.103";
        prefixLength = 24;
      }];
      environment.systemPackages = [ pkgs.jq ];
    };
  };

  testScript = ''
    import json

    start_all()

    server.wait_for_unit("backend.service")
    server.wait_for_unit("nginx.service")
    server.wait_for_open_port(${toString testPort})
    mastodon.wait_for_unit("mastodon-web.service")
    mastodon.wait_for_unit("mastodon-sidekiq-all.service")
    mastodon.wait_for_unit("mastodon-test-user.service")
    mastodon.wait_for_open_port(${toString testPort})

    assert client.succeed("curl -fsS https://blog.local/api/health") == "OK"
    mastodon_webfinger = json.loads(server.succeed(
        "curl -fsS 'https://mastodon.local/.well-known/webfinger?resource=acct:alice@mastodon.local'"
    ))
    mastodon_actor_url = next(link["href"] for link in mastodon_webfinger["links"] if link["rel"] == "self")
    server.fail("curl -fsS -H 'Accept: application/activity+json' " + mastodon_actor_url)
    webfinger = json.loads(client.succeed(
        "curl -fsS 'https://blog.local/.well-known/webfinger?resource=acct:blog@blog.local'"
    ))
    assert webfinger["subject"] == "acct:blog@blog.local"

    actor = json.loads(client.succeed(
        "curl -fsS -H 'Accept: application/activity+json' https://blog.local/blog"
    ))
    assert actor["type"] == "Service"
    assert actor["preferredUsername"] == "blog"
    assert actor["name"] == "FlakM blog"
    assert actor["url"] == "https://blog.local/"
    assert actor["icon"] == {
        "type": "Image",
        "mediaType": "image/jpeg",
        "url": "https://blog.local/images/avatar.jpg",
        "name": "Portrait of Maciek Flak",
    }
    assert actor["image"] == {
        "type": "Image",
        "mediaType": "image/png",
        "url": "https://blog.local/images/fediverse-header.png",
        "name": "FlakM blog homepage in its dark theme",
    }
    assert "Technical notes by Maciek Flak" in actor["summary"]
    assert "https://blog.local/" in actor["summary"]
    assert actor["discoverable"]
    assert actor["indexable"]
    assert actor["inbox"] == "https://blog.local/blog/inbox"

    token = mastodon.succeed("cat /var/lib/mastodon/test-token").strip()
    server.succeed("install -m 0400 /dev/null /run/mastodon-access-token")
    server.succeed("printf '%s' '" + token + "' > /run/mastodon-access-token")
    authorization = "Authorization: Bearer " + token
    search_command = (
        "curl -fsS -G -H '" + authorization + "' "
        "--data-urlencode 'q=@blog@blog.local' --data 'resolve=true' "
        "https://mastodon.local/api/v2/search"
    )
    client.wait_until_succeeds(
        search_command + " | jq -e '.accounts[] | select(.acct == \"blog@blog.local\")'",
        timeout=60,
    )
    search = json.loads(client.succeed(search_command))
    blog_account = next(account for account in search["accounts"] if account["acct"] == "blog@blog.local")
    assert blog_account["display_name"] == "FlakM blog"
    assert blog_account["bot"]
    assert blog_account["discoverable"]
    assert blog_account["indexable"]
    assert "Technical notes by Maciek Flak" in blog_account["note"]
    assert not blog_account["avatar"].endswith("missing.png")
    assert not blog_account["header"].endswith("missing.png")
    account_id = blog_account["id"]

    follow_command = (
        "curl -fsS -X POST -H '" + authorization + "' "
        "https://mastodon.local/api/v1/accounts/" + account_id + "/follow"
    )
    follow = json.loads(client.succeed(follow_command))
    assert follow["following"] or follow["requested"]

    client.wait_until_succeeds(
        "curl -fsS https://blog.local/blog/followers | jq -e '.totalItems == 1'",
        timeout=60,
    )
    server.succeed(
        "sudo -u postgres psql blog -tAc 'SELECT count(*) FROM fediverse_followers' | grep -qx 1"
    )
    client.wait_until_succeeds(
        "curl -fsS -G -H '" + authorization + "' "
        "--data-urlencode 'id[]=" + account_id + "' "
        "https://mastodon.local/api/v1/accounts/relationships | jq -e '.[0].following == true'",
        timeout=60,
    )

    server.succeed("cp --remove-destination ${testBlogPosts} /var/lib/blog/posts.json")
    server.succeed("systemctl restart backend.service")
    server.wait_for_open_port(3000)
    server.wait_until_succeeds(
        "sudo -u postgres psql blog -tAc \"SELECT count(*) FROM fediverse_published_posts WHERE slug = 'sqlx_caches_til'\" | grep -qx 1",
        timeout=60,
    )
    mastodon_status = json.loads(client.wait_until_succeeds(
        "curl -fsS -H '" + authorization + "' https://mastodon.local/api/v1/timelines/home "
        "| jq -e '.[] | select(.uri == \"https://blog.local/blog/posts/sqlx_caches_til\" and .url == \"https://blog.local/blog/posts/sqlx_caches_til\")'",
        timeout=60,
    ))
    assert len(mastodon_status["media_attachments"]) == 1
    assert mastodon_status["media_attachments"][0]["type"] == "image"
    anonymous_status_search = (
        "curl -sS -o /tmp/anonymous-status-search.json -w '%{http_code}' -G "
        "--data-urlencode 'q=https://blog.local/blog/posts/sqlx_caches_til' "
        "--data 'type=statuses' --data 'resolve=true' https://mastodon.local/api/v2/search"
    )
    assert client.succeed(anonymous_status_search) == "401"
    status_search_command = (
        "curl -fsS -G -H '" + authorization + "' "
        "--data-urlencode 'q=https://blog.local/blog/posts/sqlx_caches_til' "
        "--data 'type=statuses' --data 'resolve=true' https://mastodon.local/api/v2/search"
    )
    resolved_status = json.loads(client.wait_until_succeeds(
        status_search_command + " | jq -e '.statuses[] | select(.uri == \"https://blog.local/blog/posts/sqlx_caches_til\")'",
        timeout=60,
    ))
    interaction_url = (
        "https://mastodon.local/authorize_interaction?uri="
        "https%3A%2F%2Fblog.local%2Fblog%2Fposts%2Fsqlx_caches_til"
    )
    discussion = json.loads(client.wait_until_succeeds(
        "curl -fsS https://blog.local/api/discussions/sqlx_caches_til "
        "| jq -e '.links[] | select(.source == \"mastodon\")'",
        timeout=30,
    ))
    assert discussion == {
        "source": "mastodon",
        "label": "Mastodon",
        "url": interaction_url,
    }
    server.succeed(
        "sudo -u postgres psql blog -tAc \"SELECT source || '|' || url FROM blog_post_discussion_links WHERE post_slug = 'sqlx_caches_til'\" "
        "| grep -qx 'mastodon|" + interaction_url + "'"
    )
    server.fail(
        "sudo -u postgres psql blog -v ON_ERROR_STOP=1 -c \"INSERT INTO blog_post_discussion_links "
        "(post_slug, source, label, url) VALUES ('sqlx_caches_til', 'invalid', 'Invalid', 'javascript:alert(1)')\""
    )
    server.succeed(
        "sudo -u postgres psql blog -c \"INSERT INTO blog_post_discussion_links "
        "(post_slug, source, label, url) VALUES ('sqlx_caches_til', 'hacker_news', 'Hacker News', "
        "'https://news.ycombinator.com/item?id=1')\""
    )
    client.succeed(
        "curl -fsS https://blog.local/api/discussions/sqlx_caches_til "
        "| jq -e '.links | length == 2 and any(.[]; .source == \"hacker_news\" and .url == \"https://news.ycombinator.com/item?id=1\")'"
    )
    article = client.succeed("curl -fsS https://blog.local/posts/sqlx_caches_til/")
    assert 'data-discussion-api-base=/api' in article
    assert 'data-discussion-slug=sqlx_caches_til' in article
    server.succeed(
        "sudo -u postgres psql blog -c \"DELETE FROM blog_post_discussion_links WHERE post_slug = 'sqlx_caches_til'\""
    )
    mastodon.succeed("systemctl stop mastodon-web.service")
    server.succeed("systemctl restart backend.service")
    server.wait_for_open_port(3000)
    client.wait_until_succeeds(
        "curl -fsS https://blog.local/api/discussions/sqlx_caches_til "
        "| jq -e '.links == [{\"source\":\"fediverse\",\"label\":\"Fediverse\",\"url\":\"https://blog.local/blog/posts/sqlx_caches_til\"}]'",
        timeout=60,
    )
    mastodon.succeed("systemctl start mastodon-web.service")
    mastodon.wait_for_unit("mastodon-web.service")
    client.wait_until_succeeds(status_search_command + " | jq -e '.statuses | length == 1'", timeout=60)
    server.succeed("systemctl restart backend.service")
    server.wait_for_open_port(3000)
    client.wait_until_succeeds(
        "curl -fsS https://blog.local/api/discussions/sqlx_caches_til "
        "| jq -e --arg url '" + interaction_url + "' '.links == [{\"source\":\"mastodon\",\"label\":\"Mastodon\",\"url\":$url}]'",
        timeout=60,
    )

    server.succeed("printf '%s' 'invalid-token' > /run/mastodon-access-token")
    server.succeed("cp --remove-destination ${fallbackBlogPost} /var/lib/blog/posts.json")
    server.succeed("systemctl restart backend.service")
    server.wait_for_open_port(3000)
    server.wait_until_succeeds(
        "sudo -u postgres psql blog -tAc \"SELECT count(*) FROM fediverse_published_posts WHERE slug = 'fallback-test-post'\" | grep -qx 1",
        timeout=60,
    )
    client.wait_until_succeeds(
        "curl -fsS -H '" + authorization + "' https://mastodon.local/api/v1/timelines/home "
        "| jq -e '.[] | select(.uri == \"https://blog.local/blog/posts/fallback-test-post\")'",
        timeout=60,
    )
    client.succeed(
        "curl -fsS https://blog.local/api/discussions/fallback-test-post "
        "| jq -e '.links == [{\"source\":\"fediverse\",\"label\":\"Fediverse\",\"url\":\"https://blog.local/blog/posts/fallback-test-post\"}]'"
    )

    unfollow_command = (
        "curl -fsS -X POST -H '" + authorization + "' "
        "https://mastodon.local/api/v1/accounts/" + account_id + "/unfollow"
    )
    unfollow = json.loads(client.succeed(unfollow_command))
    assert not unfollow["following"]
    client.wait_until_succeeds(
        "curl -fsS https://blog.local/blog/followers | jq -e '.totalItems == 0'",
        timeout=60,
    )
    server.succeed(
        "sudo -u postgres psql blog -tAc 'SELECT count(*) FROM fediverse_followers' | grep -qx 0"
    )
  '';
}
