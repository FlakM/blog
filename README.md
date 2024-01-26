# blog_deployment


```bash
# enable direnv integration
direnv allow


# check if env variables are setup in current shell without passing them to history
source  ./setup.sh


# unset if you've made a mistake
tofu init
tofu plan
```

Once the `tofu apply` is done there should be a ssh key in the directory that enables us to connect to the machine:

```bash
ssh root@blog.flakm.com
```

## Updating the provisioned nixos host

After provisioning NixOs host you might just modify any of `*.nix` files and use:

```bash
nixos-rebuild switch --target-host root@blog.flakm.com --flake .#blog
```


