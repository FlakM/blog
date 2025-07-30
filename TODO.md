## Desing the e2e

Figure out how to test end to end the whole integration of the blog, backend, and infrastructure
Maybe in itg test from main directory?

Suggestions:
- it could use web browser api (nixos maybe has a support for that) to access the blog and backend by liking a post

## Include the backend check into main flake checks

This will ensure that the backend is built and tested as part of the main flake checks, providing confidence that the backend functionality is always working.

## Prepare deployment

1. Go over the configuration.nix and analyze if something might be missing


## Prepare the backup solution for the database

It might be automated from odroid to reach the hetzner server and backup the database to a zfs pool
Or it could be sent to a s3 storage somewhere

1. Instructions on how to install it
2. Automations that will check if it has been done
3. Test instructions to verify that the backup is working - ie import the database from the backup 

## Deploy the new like functionality
## Backup the files for tata sub page - install plausible there
## Write an export for plausible data


## Simplify the code & deployment

1. The main calls into 9090 port exposed by the same backend
2. No need to keep the ips of the users - use hash
3. extract_password should not be used - use local sockets instead
4. check if rate limiting is implemented in memory not database with some bucket size
5. use sops to encrypt and ship the secrets


## Write down the important things about the project for a blog

1. Nixos and deployment as code
