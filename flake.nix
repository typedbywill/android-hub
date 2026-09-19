{
  description = "Android Hub development environment";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }: let
    systems = [ "x86_64-linux" ];
  in {
    devShells = nixpkgs.lib.genAttrs systems (system: let
      pkgs = import nixpkgs { inherit system; config = { allowUnfree = true; android_sdk.accept_license = true; }; };
      android = pkgs.androidenv.composeAndroidPackages {
        platformVersions = [ "35" ];
        buildToolsVersions = [ "35.0.0" ];
        includeNDK = false;
        includeSystemImages = false;
        includeEmulator = false;
      };
    in {
      default = pkgs.mkShell {
        packages = with pkgs; [ cargo rustc rustfmt gcc jdk17 gradle android-tools scrcpy pipewire v4l-utils android.androidsdk android.platform-tools ];
        ANDROID_HOME = "${android.androidsdk}/libexec/android-sdk";
      };
    });
  };
}
