{
  description = "Eva-daemon development environment with Ollama for Qwen 2.5";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true; # Разрешаем несвободные пакеты для драйверов NVIDIA/CUDA
            cudaSupport = true;
          };
        };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            sqlite
            dbus
            alsa-lib
            cudaPackages.cudatoolkit
            cudaPackages.cudnn
          ];

          shellHook = ''
            echo "================================================="
            echo " Окружение Евы успешно инициализировано."
            echo " Инструментарий Rust (Rig) готов к работе."
            echo " Движок llama-cpp работает как системный демон."
            echo "================================================="
          '';
        };
      }
    );
}