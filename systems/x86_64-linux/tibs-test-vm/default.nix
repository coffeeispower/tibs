{ inputs, pkgs, system, ... }: {
  imports = [
    # inputs.self.nixosModules.tibs
    "${inputs.nixpkgs}/nixos/modules/virtualisation/qemu-vm.nix"
    
  ];
  # tibs.enable = true;

  services.qemuGuest.enable = true;

  users.users.tibs = {
    isNormalUser = true;
    password = "tibs";
    extraGroups = [ "wheel" ];
  };

  networking.useDHCP = true;

  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    curl
    inputs.self.packages.${system}.tibs
  ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  hardware.graphics.enable = true;
  hardware.graphics.extraPackages = [ pkgs.mesa.drivers ];
  system.stateVersion = "25.05";
}
