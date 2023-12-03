# backend

To setup a project run:

```bash
direnv allow
```

## Building and testing



```bash
# build
nix build

# run all tests
nix flake check


# run a specific integration test
nix run .\#checks.x86_64-linux.integration


# run a integration test in an interactive mode
nix run -L .\#checks.x86_64-linux.integration.driverInteractive
```
