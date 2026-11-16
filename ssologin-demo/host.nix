{ pkgs, ... }:

{
  imports = [ ./module.nix ];

  services.ssologinDemo = {
    enable = true;
    sts = "https://api.eu2.coralogix.com";

    nginx = {
      hostName = "c4c.flakm.com";
      serverAliases = [ "coralogix.flakm.com" ];
      enableACME = true;
      forceSSL = true;
    };

    entry = {
      enable = true;
      team = "c4c";
      dashboardFormat = "https://{team}.app.eu2.coralogix.com";
    };

    ntfy = {
      enable = true;
      # Tapping the notification starts a fresh SSO login on the demo host; the
      # listener appends the captured login_flow_id.
      clickUrl = "https://coralogix.flakm.com/";
    };
  };

  sops.secrets.ntfy_agent_token = {
    mode = "0400";
  };

  # So the operator can also send a test from the host.
  environment.systemPackages = [ (import ./package.nix { inherit pkgs; }).agent-msg ];

  security.acme.certs."c4c.flakm.com" = {
    email = "me@flakm.com";
  };
}
