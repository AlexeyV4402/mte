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

    runtimeLibs = with pkgs; [
          vulkan-loader
          vulkan-validation-layers
          renderdoc
          libGL
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          wayland
          libxkbcommon
        ];
  in
  {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        renderdoc

        vulkan-loader
        vulkan-validation-layers
        vulkan-tools

        rustToolchain 
        
        pkg-config
        openssl

        wayland
        libxkbcommon

        xorg.libX11
        xorg.libXcursor
        xorg.libXrandr
        xorg.libXi
      ];

      shellHook = ''
        #export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d"
        export VK_INSTANCE_LAYERS="VK_LAYER_KHRONOS_validation"

        export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibs}:$LD_LIBRARY_PATH"

        export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d:${pkgs.renderdoc}/share/vulkan/explicit_layer.d:$VK_LAYER_PATH"
            
        export XDG_DATA_DIRS="${pkgs.renderdoc}/share:$XDG_DATA_DIRS"
      '';
      
      env.RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
      
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