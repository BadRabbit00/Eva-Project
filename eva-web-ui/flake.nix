{
  description = "Eva Web UI - Independent Frontend for Eva OS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        nodejs_22
      ];

      shellHook = ''
        echo "================================================="
        echo " Eva Web UI Environment Loaded."
        echo " Node.js ready. Run 'npm install' & 'npm run dev'"
        echo "================================================="
      '';
    };
  };
}
