{ system
, pkgs
, self
}:
let
  sharedModule = {
    # Enable graphics for browser testing
    virtualisation.graphics = true;
    virtualisation.memorySize = 2048;
  };

  browserModule = {
    # Configure basic X11 setup for browser testing
    services.xserver = {
      enable = true;
      displayManager.startx.enable = true;
    };
    
    # Create test user
    users.users.testuser = {
      isNormalUser = true;
      uid = 1000;
    };
    
    environment.systemPackages = with pkgs; [
      playwright-driver.browsers
      nodejs
      python3Packages.playwright
      xvfb-run
      chromium
      xorg.xauth
      xorg.xorgserver
    ];
    
    environment.variables = {
      PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
      PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
      DISPLAY = ":0";
    };
  };

  # Test blog posts JSON data
  testBlogPosts = pkgs.writeText "test-posts.json" ''
    [
      {
        "title": "Test Blog Post",
        "slug": "test-post",  
        "description": "A test blog post for integration testing",
        "date": "2024-01-01T12:00:00Z",
        "featuredImage": null,
        "tags": ["test", "integration"],
        "url": "https://blog.flakm.com/posts/test-post"
      }
    ]
  '';

  # Test HTML page with like button for browser testing
  testHtmlPage = pkgs.writeText "test-post.html" ''
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Test Blog Post</title>
        <style>
            body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
            .like-button { background: #007bff; color: white; border: none; padding: 10px 20px; border-radius: 5px; cursor: pointer; }
            .like-button:hover { background: #0056b3; }
            .like-button.liked { background: #28a745; }
            .like-count { margin-left: 10px; font-weight: bold; }
            .error { color: #dc3545; margin-top: 10px; }
        </style>
    </head>
    <body>
        <h1>Test Blog Post</h1>
        <p>This is a test blog post for integration testing.</p>
        
        <div class="like-section">
            <button id="like-button" class="like-button" data-slug="test-post">👍 Like</button>
            <span id="like-count" class="like-count">Loading...</span>
            <div id="error-message" class="error" style="display: none;"></div>
        </div>

        <script>
            const likeButton = document.getElementById('like-button');
            const likeCount = document.getElementById('like-count');
            const errorMessage = document.getElementById('error-message');
            const postSlug = likeButton.dataset.slug;

            // Load initial like count
            async function loadLikeCount() {
                try {
                    const response = await fetch(`/api/likes/$${postSlug}`);
                    const data = await response.json();
                    if (data.success) {
                        likeCount.textContent = `$${data.total_likes} likes`;
                    } else {
                        throw new Error(data.message);
                    }
                } catch (error) {
                    likeCount.textContent = '0 likes';
                    console.error('Failed to load like count:', error);
                }
            }

            // Handle like button click
            likeButton.addEventListener('click', async () => {
                try {
                    likeButton.disabled = true;
                    likeButton.textContent = '⏳ Liking...';
                    errorMessage.style.display = 'none';

                    const response = await fetch(`/api/like/$${postSlug}`, {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json'
                        }
                    });

                    const data = await response.json();
                    
                    if (data.success) {
                        likeButton.classList.add('liked');
                        likeButton.textContent = '✅ Liked!';
                        // Reload like count
                        await loadLikeCount();
                    } else {
                        throw new Error(data.message);
                    }
                } catch (error) {
                    errorMessage.textContent = `Error: $${error.message}`;
                    errorMessage.style.display = 'block';
                    likeButton.textContent = '👍 Like';
                } finally {
                    likeButton.disabled = false;
                }
            });

            // Load initial like count on page load
            loadLikeCount();
        </script>
    </body>
    </html>
  '';
in
{
  name = "blog-integration";
  
  # Disable type checking since we're dynamically importing modules at runtime
  skipTypeCheck = true;

  nodes = {
    server = {
      networking.firewall = {
        enable = true;
        allowedTCPPorts = [ 80 443 3000 4317 4318 5432 8080 ];
      };

      imports = [ sharedModule self.nixosModules.${system}.default ];
      
      # Configure PostgreSQL
      services.postgresql = {
        enable = true;
        package = pkgs.postgresql_15;
        
        ensureDatabases = [ "blog" ];
        ensureUsers = [
          {
            name = "blog";
            ensureDBOwnership = true;
          }
        ];
        
        authentication = pkgs.lib.mkOverride 10 ''
          local   blog        blog                    trust
          host    blog        blog    127.0.0.1/32    trust
          host    blog        blog    ::1/128         trust
          local   all         all                     trust
          host    all         all     127.0.0.1/32    ident
          host    all         all     ::1/128         ident
        '';
      };

      # Configure OpenTelemetry Collector
      services.opentelemetry-collector = {
        enable = true;
        package = pkgs.opentelemetry-collector-contrib;
        
        settings = {
          receivers = {
            otlp = {
              protocols = {
                grpc = {
                  endpoint = "0.0.0.0:4317";
                };
                http = {
                  endpoint = "0.0.0.0:4318";
                };
              };
            };
            # File log receiver for comprehensive log collection (with readable paths)
            filelog = {
              include = [ "/var/log/messages" "/var/log/syslog" ];
              include_file_name = false;
              include_file_path = false;
            };
          };
          
          processors = {
            # Optimized batch processor for Coralogix
            batch = {
              send_batch_size = 1024;
              send_batch_max_size = 2048;
              timeout = "1s";
            };
            resource = {
              attributes = [
                {
                  key = "service.name";
                  value = "blog-backend-test";
                  action = "upsert";
                }
              ];
            };
          };
          
          exporters = {
            # Use debug exporter for integration tests - no external dependencies
            debug = {
              verbosity = "detailed";
            };
          };
          
          service = {
            pipelines = {
              traces = {
                receivers = [ "otlp" ];
                processors = [ "resource" "batch" ];
                # Use only debug exporter for integration tests to avoid external dependencies
                exporters = [ "debug" ];
              };
              metrics = {
                receivers = [ "otlp" ];
                processors = [ "resource" "batch" ];
                exporters = [ "debug" ];
              };
              logs = {
                # Focus on OTLP logs only for integration tests
                receivers = [ "otlp" ];
                processors = [ "resource" "batch" ];
                exporters = [ "debug" ];
              };
            };
          };
        };
      };
      
      # No external environment variables needed for debug-only testing
      
      # Configure backend service with OpenTelemetry endpoint
      services.backend = {
        enable = true;
        domain = "server";
        posts_path = "${testBlogPosts}";
      };
      
      # Configure backend to send telemetry to local collector
      systemd.services.backend.environment = {
        OTEL_EXPORTER_OTLP_ENDPOINT = "http://localhost:4317";
        SERVICE_NAME = "blog-backend-test";
        SERVICE_VERSION = "test";
      };
      
      # Configure nginx with static blog serving and API proxy
      services.nginx = {
        enable = true;
        virtualHosts."server" = {
          locations = {
            # Serve test HTML page
            "= /posts/test-post" = {
              alias = "${testHtmlPage}";
              extraConfig = ''add_header Content-Type "text/html; charset=utf-8";'';
            };
            
            # API endpoints to backend
            "~ ^/api/(health|like|likes)" = {
              proxyPass = "http://127.0.0.1:3000";
              extraConfig = ''
                proxy_set_header Host $host;
                proxy_set_header X-Real-IP $remote_addr;
                proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                rewrite ^/api(/.*) $1 break;
              '';
            };
            
            # Default fallback to backend
            "/" = {
              proxyPass = "http://127.0.0.1:3000";
              extraConfig = ''
                proxy_set_header Host $host;
                proxy_set_header X-Real-IP $remote_addr;
                proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
              '';
            };
          };
        };
      };

      # Add postgresql to system packages for testing
      environment.systemPackages = with pkgs; [
        postgresql_15
        curl
        jq
      ];
    };
    
    client = {
      imports = [ sharedModule browserModule ];
      environment.systemPackages = with pkgs; [
        curl
        jq
        python3
        python3Packages.playwright
        python3Packages.termcolor
        nodejs
      ];
      
      # Ensure Python packages are available in PYTHONPATH
      environment.variables = {
        PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
        PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
        DISPLAY = ":0";
      };
    };
  };

  extraPythonPackages = p: [ p.termcolor p.playwright ];

  testScript = 
    let
      integrationTestModule = pkgs.writeText "integration_test.py" (builtins.readFile ./integration_test.py);
      browserTestModule = pkgs.writeText "browser_e2e_tests.py" (builtins.readFile ../browser_e2e_tests.py);
    in ''
    start_all()
    
    # Copy test modules to temporary location
    import tempfile
    import os
    import sys
    
    with tempfile.TemporaryDirectory() as tmp_dir:
        # Copy integration test module
        with open(os.path.join(tmp_dir, "integration_test.py"), "w") as f:
            f.write(open("${integrationTestModule}").read())
        
        # Copy browser test module  
        with open(os.path.join(tmp_dir, "browser_e2e_tests.py"), "w") as f:
            f.write(open("${browserTestModule}").read())
        
        # Add to Python path
        sys.path.insert(0, tmp_dir)
        
        from integration_test import run_integration_tests
        
        # Run all integration tests
        run_integration_tests(server, client)
  '';
}