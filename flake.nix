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
            cudaPackages_12_6.cudatoolkit
            cudaPackages_12_6.cudnn
            nodejs_22
          ];

          shellHook = ''
            export CUDA_ROOT=${pkgs.cudaPackages_12_6.cudatoolkit}
            export CUDA_PATH=${pkgs.cudaPackages_12_6.cudatoolkit}
            echo "================================================="
            echo " Окружение Евы успешно инициализировано."
            echo " Инструментарий Rust готов к работе."
            echo " Движок инференса: HuggingFace Candle (Pure Rust)."
            echo "================================================="
          '';
        };
      }
    );
}