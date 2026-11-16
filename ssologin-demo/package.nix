{ pkgs }:

let
  listener = pkgs.writeShellScriptBin "ssologin-listener" ''
    exec ${pkgs.python3}/bin/python3 ${./ssologin-listener.py} "$@"
  '';

  demo = pkgs.writeShellApplication {
    name = "demo-ssologin-takeover";
    runtimeInputs = with pkgs; [ python3 curl jq coreutils ];
    text = ''
      export SSOLOGIN_LISTENER="${./ssologin-listener.py}"
      exec ${pkgs.bash}/bin/bash ${./demo-ssologin-takeover.sh} "$@"
    '';
  };

  agent-msg = pkgs.writeShellApplication {
    name = "agent-msg";
    runtimeInputs = with pkgs; [ coreutils curl jq ];
    text = builtins.readFile ./agent-msg.sh;
  };
in
{
  inherit listener demo agent-msg;
}
