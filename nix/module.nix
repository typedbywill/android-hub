{ config, pkgs, ... }:
{
  # This module only provisions the kernel-backed virtual camera. PipeWire and
  # desktop packages remain under the host distribution's configuration.
  environment.systemPackages = with pkgs; [ v4l-utils ];
  boot.kernelModules = [ "v4l2loopback" ];
  boot.extraModulePackages = [ config.boot.kernelPackages.v4l2loopback ];
  boot.extraModprobeConfig = ''
    options v4l2loopback devices=1 video_nr=10 card_label="Android Hub Camera" exclusive_caps=1
  '';
  services.pipewire = { enable = true; pulse.enable = true; wireplumber.enable = true; };
}
