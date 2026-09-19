{ pkgs, ... }:
{
  environment.systemPackages = with pkgs; [ android-tools scrcpy pipewire wireplumber v4l2loopback ];
  boot.kernelModules = [ "v4l2loopback" ];
  boot.extraModprobeConfig = ''
    options v4l2loopback devices=1 video_nr=10 card_label="Android Hub Camera" exclusive_caps=1
  '';
  services.pipewire = { enable = true; pulse.enable = true; wireplumber.enable = true; };
}

