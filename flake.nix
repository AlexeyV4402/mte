{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }: 
  let 
    system = "x86_64-linux";
    
    # 1. Применяем оверлей к nixpkgs
    overlays = [ (import rust-overlay) ];
    pkgs = import nixpkgs { inherit system overlays; };

    # 2. Собираем свежий Nightly-тулчейн со всеми нужными компонентами
    rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
      extensions = [ 
        "rust-src"       # Исходники std (автоматически настроят автодополнение)
        "rust-analyzer"  # Линтер, скомпилированный именно под этот nightly-релиз
        "clippy"         # Свежий клиппи
        "rustfmt"        # Форматтер
      ];
    };
  in
  {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        # Вместо кучи отдельных пакетов используем наш комплексный тулчейн
        rustToolchain 
        
        # Системные зависимости для сборки большинства крейтов
        pkg-config
        openssl

        wayland
        libxkbcommon
        # Если в будущем wgpu потребует Vulkan/X11 драйверы:
        vulkan-loader
        xorg.libX11
        xorg.libXcursor
        xorg.libXrandr
        xorg.libXi
      ];
      
      # 3. Переменная для rust-analyzer теперь берется прямо из нашего nightly-тулчейна
      env.RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
      
      # Пробрасываем пути к динамическим библиотекам C (исправляет большинство os error 2 при сборке)
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
        stdenv.cc.cc.lib
        openssl
        
        wayland
        libxkbcommon
        vulkan-loader
      ]);
    };
  };
}